#!/usr/bin/env bash
# drift-detect-loop.sh — watch for drift between vendored FOSS and upstream.
# Re-runs every 12h. File a finding under PlausiDen-Audits/audits/foss/findings/
# whenever a vendored crate's upstream has new commits since our last check.

set -euo pipefail

DOCTRINE_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RUNTIME_DIR="${DOCTRINE_ROOT}/loops/runtime"
LOG_DIR="${HOME}/.local/share/plausiden/loop-state"
REPOS_ROOT="${REPOS_ROOT:-/home/user/Development/PlausiDen}"
SLEEP_SECS="${SLEEP_SECS:-43200}"  # 12 hours
PID_FILE="$RUNTIME_DIR/drift-detect.pid"
STOP_FILE="$RUNTIME_DIR/STOP"
LOG_FILE="$LOG_DIR/drift-detect.log"

mkdir -p "$RUNTIME_DIR" "$LOG_DIR"

log() {
  printf '[%s] %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*" | tee -a "$LOG_FILE"
}

cleanup() { rm -f "$PID_FILE"; log "drift-detect: exit"; }
trap cleanup EXIT
trap 'exit 0' INT TERM

if [[ "${1:-}" == "--stop" ]]; then
  [[ -f "$PID_FILE" ]] && kill "$(cat "$PID_FILE")" 2>/dev/null || true
  touch "$STOP_FILE"
  exit 0
fi

if [[ -f "$PID_FILE" ]] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
  log "drift-detect: already running"
  exit 0
fi
echo "$$" > "$PID_FILE"
rm -f "$STOP_FILE"
log "drift-detect: started (sleep=${SLEEP_SECS}s)"

while true; do
  [[ -f "$STOP_FILE" ]] && { log "drift-detect: STOP, exit"; exit 0; }
  log "drift-detect: scanning $REPOS_ROOT for vendored deps"
  found=0
  while IFS= read -r vendor; do
    [[ -z "$vendor" ]] && continue
    found=$((found + 1))
    if [[ -f "$vendor/VENDORING.md" ]]; then
      log "  ↳ $vendor (VENDORING.md present)"
    else
      log "  ↳ $vendor — MISSING VENDORING.md (will file finding)"
      bash "$DOCTRINE_ROOT/scripts/file-finding.sh" foss \
        "missing VENDORING.md in $vendor" 2>/dev/null || true
    fi
  done < <(find "$REPOS_ROOT" -type d -name vendor -not -path '*/node_modules/*' -not -path '*/target/*' 2>/dev/null)
  log "drift-detect: scanned $found vendor directories"
  for ((i = 0; i < SLEEP_SECS; i += 60)); do
    sleep 60
    [[ -f "$STOP_FILE" ]] && break
  done
done
