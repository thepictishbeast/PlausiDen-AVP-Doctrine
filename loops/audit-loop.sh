#!/usr/bin/env bash
# audit-loop.sh — continuous improvement loop.
# Walks the `daily` routine, files findings, sleeps, repeats.
#
# Stop conditions:
#   - SIGTERM / SIGINT
#   - File present at $RUNTIME_DIR/STOP
#   - User stop word seen on IPC bus addressed to this agent
#
# Register in loops/REGISTRY.md before running.

set -euo pipefail

DOCTRINE_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPOS_ROOT="${REPOS_ROOT:-/home/user/Development/PlausiDen}"
RUNTIME_DIR="${DOCTRINE_ROOT}/loops/runtime"
LOG_DIR="${HOME}/.local/share/plausiden/loop-state"
SLEEP_SECS="${SLEEP_SECS:-1800}"   # 30 min default
AGENT="${AGENT:-unspecified}"
PID_FILE="$RUNTIME_DIR/audit-loop.pid"
STOP_FILE="$RUNTIME_DIR/STOP"
LOG_FILE="$LOG_DIR/audit-loop.log"

mkdir -p "$RUNTIME_DIR" "$LOG_DIR"

cleanup() {
  rm -f "$PID_FILE"
  log "audit-loop: exiting"
}
trap cleanup EXIT
trap 'exit 0' INT TERM

log() {
  printf '[%s] %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*" | tee -a "$LOG_FILE"
}

if [[ "${1:-}" == "--stop" ]]; then
  if [[ -f "$PID_FILE" ]]; then
    kill "$(cat "$PID_FILE")" 2>/dev/null || true
    log "audit-loop: stop requested via --stop"
  fi
  touch "$STOP_FILE"
  exit 0
fi

if [[ -f "$PID_FILE" ]]; then
  if kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
    log "audit-loop: already running (pid $(cat "$PID_FILE"))"
    exit 0
  fi
  rm -f "$PID_FILE"
fi
echo "$$" > "$PID_FILE"
rm -f "$STOP_FILE"
log "audit-loop: started (agent=$AGENT, sleep=${SLEEP_SECS}s)"

iteration=0
while true; do
  if [[ -f "$STOP_FILE" ]]; then
    log "audit-loop: STOP file seen, exiting"
    exit 0
  fi
  iteration=$((iteration + 1))
  log "audit-loop: iteration $iteration"
  for repo in "$REPOS_ROOT"/PlausiDen-*/; do
    [[ -d "$repo" ]] || continue
    log "  ↳ $(basename "$repo")"
    (
      cd "$repo"
      bash "$DOCTRINE_ROOT/scripts/run-routine.sh" daily 2>&1 || true
    ) | tee -a "$LOG_FILE"
  done
  log "audit-loop: iteration $iteration complete; sleeping ${SLEEP_SECS}s"
  for ((i = 0; i < SLEEP_SECS; i += 10)); do
    sleep 10
    [[ -f "$STOP_FILE" ]] && break
  done
done
