#!/usr/bin/env bash
# Install / uninstall PlausiDen loops as systemd user units.
# Units installed under ~/.config/systemd/user/plausiden-<name>.service
# Services use `type=simple`, `Restart=on-failure`.

set -euo pipefail

DOCTRINE_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SYSTEMD_DIR="${HOME}/.config/systemd/user"
PREFIX="plausiden-"

available=(
  audit-loop
  ipc-poll-loop
  sprint-progress-loop
  drift-detect-loop
)

usage() {
  cat <<EOF
PlausiDen loop systemd installer.

Usage:
  $0                       interactive — asks per loop
  $0 --yes <a,b,c>         non-interactive — only install the listed loops
  $0 --uninstall           remove every PlausiDen-managed user unit
  $0 --list                show currently-installed PlausiDen user units

Available loops:
$(printf '  - %s\n' "${available[@]}")
EOF
}

write_unit() {
  local name="$1"
  local script="$DOCTRINE_ROOT/loops/${name}.sh"
  local unit="$SYSTEMD_DIR/${PREFIX}${name}.service"
  if [[ ! -x "$script" ]]; then
    chmod +x "$script"
  fi
  mkdir -p "$SYSTEMD_DIR"
  cat > "$unit" <<UNIT
[Unit]
Description=PlausiDen ${name} (managed by PlausiDen-AVP-Doctrine)
After=network-online.target

[Service]
Type=simple
ExecStart=${script}
Restart=on-failure
RestartSec=30
StandardOutput=journal
StandardError=journal
# managed by PlausiDen-AVP-Doctrine

[Install]
WantedBy=default.target
UNIT
  systemctl --user daemon-reload
  systemctl --user enable --now "${PREFIX}${name}.service"
  echo "${name}: installed and started"
}

remove_unit() {
  local name="$1"
  local unit="$SYSTEMD_DIR/${PREFIX}${name}.service"
  systemctl --user disable --now "${PREFIX}${name}.service" 2>/dev/null || true
  rm -f "$unit"
  echo "${name}: removed"
}

uninstall_all() {
  for unit in "$SYSTEMD_DIR"/${PREFIX}*.service; do
    [[ -f "$unit" ]] || continue
    local name
    name=$(basename "$unit" .service)
    name=${name#$PREFIX}
    remove_unit "$name"
  done
  systemctl --user daemon-reload || true
}

list_installed() {
  ls "$SYSTEMD_DIR"/${PREFIX}*.service 2>/dev/null \
    | xargs -I{} basename {} .service \
    | sed "s/^${PREFIX}//"
}

interactive() {
  for name in "${available[@]}"; do
    read -r -p "Install '${name}' as user unit? [y/N] " ans
    case "$ans" in
      [Yy]*) write_unit "$name" ;;
      *) echo "$name: skipped." ;;
    esac
  done
}

case "${1:-}" in
  "" )      interactive ;;
  --list)   list_installed ;;
  --uninstall) uninstall_all ;;
  --yes)
    shift
    [[ -n "${1:-}" ]] || { usage; exit 2; }
    IFS=',' read -ra names <<<"$1"
    for name in "${names[@]}"; do write_unit "$name"; done
    ;;
  -h|--help) usage ;;
  *) usage; exit 2 ;;
esac
