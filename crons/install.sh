#!/usr/bin/env bash
# Install / uninstall PlausiDen crons.
# Tagged with `# managed by PlausiDen-AVP-Doctrine` so we can find and remove
# our entries without touching unrelated cron jobs.

set -euo pipefail

CRON_DIR="$(cd "$(dirname "$0")" && pwd)"
TAG="# managed by PlausiDen-AVP-Doctrine"
STATE_DIR="${HOME}/.local/share/plausiden/cron-state"

available=(
  daily-audit
  weekly-audit
  hourly-heartbeat
  nightly-ipc-archive
  weekly-foss-update-check
)

usage() {
  cat <<USAGE
PlausiDen cron installer.

Usage:
  $0                            interactive — asks per cron
  $0 --yes <a,b,c>              non-interactive, install the listed crons
  $0 --uninstall                remove every PlausiDen-managed cron
  $0 --list                     show currently-installed PlausiDen crons

Available crons:
$(printf '  - %s\n' "${available[@]}")
USAGE
}

current_crontab() {
  crontab -l 2>/dev/null || true
}

list_installed() {
  current_crontab | grep -F "$TAG" || true
}

uninstall_all() {
  local existing
  existing=$(current_crontab)
  if ! echo "$existing" | grep -qF "$TAG"; then
    echo "no PlausiDen crons installed."
    return 0
  fi
  echo "$existing" | grep -vF "$TAG" | crontab -
  echo "removed all PlausiDen-managed crons."
}

install_one() {
  local name="$1"
  local file="$CRON_DIR/${name}.cron"
  if [[ ! -f "$file" ]]; then
    echo "::error::no such cron: $name (looked for $file)" >&2
    return 1
  fi
  mkdir -p "$STATE_DIR"
  local existing snippet
  existing=$(current_crontab)
  snippet=$(grep -v '^[[:space:]]*#' "$file" | grep -v '^[[:space:]]*$' | head -1)
  if [[ -z "$snippet" ]]; then
    echo "::error::$file has no schedule line" >&2
    return 1
  fi
  if echo "$existing" | grep -qF "$snippet"; then
    echo "$name: already installed."
    return 0
  fi
  printf '%s\n%s %s\n' "$existing" "$snippet" "$TAG" \
    | crontab -
  echo "$name: installed."
}

interactive() {
  for name in "${available[@]}"; do
    read -r -p "Install '${name}' cron? [y/N] " ans
    case "$ans" in
      [Yy]*) install_one "$name" ;;
      *) echo "$name: skipped." ;;
    esac
  done
}

main() {
  case "${1:-}" in
    "" )      interactive ;;
    --list)   list_installed ;;
    --uninstall) uninstall_all ;;
    --yes)
      shift
      [[ -n "${1:-}" ]] || { usage; exit 2; }
      IFS=',' read -ra names <<<"$1"
      for name in "${names[@]}"; do
        install_one "$name"
      done
      ;;
    -h|--help) usage ;;
    *) usage; exit 2 ;;
  esac
}

main "$@"
