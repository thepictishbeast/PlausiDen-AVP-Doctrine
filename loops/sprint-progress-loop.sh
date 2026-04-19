#!/usr/bin/env bash
# sprint-progress-loop.sh — periodic sprint health check.
# Reads task tracker (defaults to PlausiDen sprint board path), summarises
# in-flight / done / blocked, posts to IPC.

set -euo pipefail

DOCTRINE_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RUNTIME_DIR="${DOCTRINE_ROOT}/loops/runtime"
LOG_DIR="${HOME}/.local/share/plausiden/loop-state"
BUS="${IPC_BUS:-/home/user/Development/PlausiDen/.ipc/bus.jsonl}"
SPRINT_FILE="${SPRINT_FILE:-/home/user/Development/PlausiDen/PRIORITIES.md}"
AGENT="${AGENT:-orchestrator}"
SLEEP_SECS="${SLEEP_SECS:-14400}"   # 4 hours
PID_FILE="$RUNTIME_DIR/sprint-progress.pid"
STOP_FILE="$RUNTIME_DIR/STOP"
LOG_FILE="$LOG_DIR/sprint-progress.log"

mkdir -p "$RUNTIME_DIR" "$LOG_DIR"

log() {
  printf '[%s] %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*" | tee -a "$LOG_FILE"
}

cleanup() { rm -f "$PID_FILE"; log "sprint-progress: exit"; }
trap cleanup EXIT
trap 'exit 0' INT TERM

if [[ "${1:-}" == "--stop" ]]; then
  [[ -f "$PID_FILE" ]] && kill "$(cat "$PID_FILE")" 2>/dev/null || true
  touch "$STOP_FILE"
  exit 0
fi

if [[ -f "$PID_FILE" ]] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
  log "sprint-progress: already running (pid $(cat "$PID_FILE"))"
  exit 0
fi
echo "$$" > "$PID_FILE"
rm -f "$STOP_FILE"
log "sprint-progress: started (sprint=$SPRINT_FILE, sleep=${SLEEP_SECS}s)"

post_to_ipc() {
  local body="$1"
  local id ts
  id=$(uuidgen 2>/dev/null || python3 -c 'import uuid; print(uuid.uuid4())')
  ts=$(date -u +%Y-%m-%dT%H:%M:%SZ)
  printf '{"id":"%s","ts":"%s","from":"%s","to":["all"],"kind":"heartbeat","subject":"sprint-progress","body":%q,"refs":{}}\n' \
    "$id" "$ts" "$AGENT" "$body" >> "$BUS"
}

while true; do
  [[ -f "$STOP_FILE" ]] && { log "sprint-progress: STOP, exit"; exit 0; }
  if [[ -f "$SPRINT_FILE" ]]; then
    in_flight=$(grep -ciE '^[[:space:]]*-[[:space:]]*\[[ x][[:space:]]?\]' "$SPRINT_FILE" || true)
    done_n=$(grep -ciE '^[[:space:]]*-[[:space:]]*\[x\]' "$SPRINT_FILE" || true)
    log "sprint-progress: $done_n done / $in_flight total"
    post_to_ipc "sprint: $done_n done / $in_flight total in $(basename "$SPRINT_FILE")"
  else
    log "sprint-progress: $SPRINT_FILE not found; skipping"
  fi
  for ((i = 0; i < SLEEP_SECS; i += 30)); do
    sleep 30
    [[ -f "$STOP_FILE" ]] && break
  done
done
