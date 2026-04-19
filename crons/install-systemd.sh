#!/usr/bin/env bash
# install-systemd.sh — install PlausiDen scheduled routines as systemd
# system-wide timers. Idempotent. System units so they fire regardless of
# which user is logged in. OnCalendar timers with Persistent=true so missed
# runs (downtime) catch up on next boot.
#
# Usage:
#   sudo ./install-systemd.sh                 interactive — asks per job
#   sudo ./install-systemd.sh --yes <csv>     install listed jobs
#   sudo ./install-systemd.sh --all           install all jobs
#   sudo ./install-systemd.sh --uninstall     remove all plausiden-* units
#   ./install-systemd.sh --list               show which units are installed
#
# Available jobs:
#   daily-audit        0 6 * * *    Run the `daily` audit routine per repo.
#   weekly-audit       0 6 * * 1    Run the `weekly` audit routine per repo.
#   hourly-heartbeat   0 * * * *    Post agent heartbeat to IPC bus.
#   nightly-ipc-archive 0 3 * * *   Roll IPC bus.jsonl > 30 days.
#   weekly-foss-check  0 4 * * 0    Diff vendored FOSS deps vs upstream.

set -euo pipefail

DOCTRINE_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SYSTEMD_DIR="/etc/systemd/system"
PREFIX="plausiden-"
RUN_USER="${RUN_USER:-$(id -un)}"
STATE_DIR="${HOME}/.local/share/plausiden/cron-state"

# job_name:calendar:command
jobs=(
  "daily-audit:*-*-* 06:00:00:for r in /home/user/Development/PlausiDen/PlausiDen-*/; do (cd \"\$r\" && bash $DOCTRINE_ROOT/scripts/run-routine.sh daily); done"
  "weekly-audit:Mon *-*-* 06:00:00:for r in /home/user/Development/PlausiDen/PlausiDen-*/; do (cd \"\$r\" && bash $DOCTRINE_ROOT/scripts/run-routine.sh weekly); done"
  "hourly-heartbeat:hourly:bash $DOCTRINE_ROOT/scripts/post-heartbeat.sh"
  "nightly-ipc-archive:*-*-* 03:00:00:bash $DOCTRINE_ROOT/scripts/archive-ipc.sh"
  "weekly-foss-check:Sun *-*-* 04:00:00:bash $DOCTRINE_ROOT/scripts/foss-upstream-check.sh"
)

need_root() {
    if [[ $EUID -ne 0 ]]; then
        echo "::error::$1 requires root (re-run with sudo)" >&2
        exit 1
    fi
}

write_units() {
    need_root "installation"
    local name="$1" calendar="$2" cmd="$3"
    local svc="$SYSTEMD_DIR/${PREFIX}${name}.service"
    local tmr="$SYSTEMD_DIR/${PREFIX}${name}.timer"
    mkdir -p "$STATE_DIR"

    cat > "$svc" <<UNIT
[Unit]
Description=PlausiDen ${name} (managed by PlausiDen-AVP-Doctrine)
# managed by PlausiDen-AVP-Doctrine
After=network-online.target

[Service]
Type=oneshot
User=${RUN_USER}
Environment=HOME=$(eval echo ~${RUN_USER})
ExecStart=/bin/bash -lc '${cmd} >> ${STATE_DIR}/${name}.log 2>&1'
StandardOutput=journal
StandardError=journal
UNIT

    cat > "$tmr" <<UNIT
[Unit]
Description=Timer for PlausiDen ${name}
# managed by PlausiDen-AVP-Doctrine

[Timer]
OnCalendar=${calendar}
Persistent=true
AccuracySec=1min
Unit=${PREFIX}${name}.service

[Install]
WantedBy=timers.target
UNIT

    systemctl daemon-reload
    systemctl enable --now "${PREFIX}${name}.timer"
    echo "${name}: installed (next run: $(systemctl list-timers "${PREFIX}${name}.timer" --no-legend | awk '{print $1, $2}'))"
}

remove_units() {
    need_root "uninstall"
    local name="$1"
    systemctl disable --now "${PREFIX}${name}.timer" 2>/dev/null || true
    systemctl disable --now "${PREFIX}${name}.service" 2>/dev/null || true
    rm -f "$SYSTEMD_DIR/${PREFIX}${name}.service" "$SYSTEMD_DIR/${PREFIX}${name}.timer"
    systemctl daemon-reload
    echo "${name}: removed"
}

uninstall_all() {
    need_root "uninstall"
    for unit in "$SYSTEMD_DIR"/${PREFIX}*.{service,timer}; do
        [[ -f "$unit" ]] || continue
        local name
        name=$(basename "$unit")
        name=${name#$PREFIX}
        name=${name%.service}
        name=${name%.timer}
        remove_units "$name"
    done
}

list_installed() {
    systemctl list-unit-files --type=timer --no-legend 2>/dev/null \
        | awk '{print $1}' | grep "^${PREFIX}" || echo "(none installed)"
}

job_field() { printf '%s' "$1" | awk -F: "{print \$$2}"; }

find_job() {
    local name="$1"
    for j in "${jobs[@]}"; do
        [[ "${j%%:*}" == "$name" ]] && { printf '%s' "$j"; return 0; }
    done
    return 1
}

install_one() {
    local name="$1"
    local j
    if ! j=$(find_job "$name"); then
        echo "::error::no such job: $name" >&2
        return 1
    fi
    local calendar cmd
    calendar=$(printf '%s' "$j" | cut -d: -f2- | awk -F':(?=[^ ])' '{print $1}')
    # Parse: split on FIRST `:` after the name, then find the calendar vs command.
    # The `jobs` entries are `name:CALENDAR:CMD` and CALENDAR may contain `:`
    # (e.g. `*-*-* 06:00:00`), so split by field positions.
    rest=${j#${name}:}
    # CALENDAR ends at `:/` or `:bash ` or `:for ` or `:echo ` — match first `:` followed by a command token.
    calendar=$(printf '%s' "$rest" | sed -E 's/:(for |bash |echo |python ).*$//')
    cmd=${rest#${calendar}:}
    write_units "$name" "$calendar" "$cmd"
}

interactive() {
    for j in "${jobs[@]}"; do
        local name=${j%%:*}
        read -r -p "Install '${name}' as system timer? [y/N] " ans
        case "$ans" in [Yy]*) install_one "$name" ;; *) echo "$name: skipped" ;; esac
    done
}

usage() {
    cat <<EOF
PlausiDen scheduled-routine installer (systemd system timers).

Usage:
  sudo $0                        interactive
  sudo $0 --yes a,b,c            install listed jobs
  sudo $0 --all                  install every job
  sudo $0 --uninstall            remove all plausiden-* units
  $0 --list                      show installed

Jobs:
$(for j in "${jobs[@]}"; do printf "  %s\n" "${j%%:*}"; done)

State dir: $STATE_DIR
EOF
}

case "${1:-}" in
    "" )         interactive ;;
    --list)      list_installed ;;
    --uninstall) uninstall_all ;;
    --all)
        for j in "${jobs[@]}"; do install_one "${j%%:*}"; done
        ;;
    --yes)
        shift
        [[ -n "${1:-}" ]] || { usage; exit 2; }
        IFS=',' read -ra names <<<"$1"
        for n in "${names[@]}"; do install_one "$n"; done
        ;;
    -h|--help) usage ;;
    *) usage; exit 2 ;;
esac
