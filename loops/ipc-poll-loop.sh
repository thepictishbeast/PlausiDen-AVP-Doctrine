#!/usr/bin/env bash
# ipc-poll-loop.sh — poll the IPC bus for messages addressed to AGENT.
# Acks each new message, dispatches the body to a handler script if present.
#
# Stop conditions: SIGTERM/SIGINT, $RUNTIME_DIR/STOP file, stop word seen.

set -euo pipefail

DOCTRINE_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck disable=SC1091
source "$DOCTRINE_ROOT/ipc/discovery.sh"
RUNTIME_DIR="${DOCTRINE_ROOT}/loops/runtime"
LOG_DIR="${HOME}/.local/share/plausiden/loop-state"
# BUS resolves lazily each iteration — handles host where the canonical path
# comes back online during the loop's lifetime.
AGENT="${AGENT:-claude-1}"
SLEEP_SECS="${SLEEP_SECS:-60}"
PID_FILE="$RUNTIME_DIR/ipc-poll-${AGENT}.pid"
STOP_FILE="$RUNTIME_DIR/STOP"
CURSOR_FILE="$RUNTIME_DIR/ipc-poll-${AGENT}.cursor"
LOG_FILE="$LOG_DIR/ipc-poll-${AGENT}.log"
HANDLER="$DOCTRINE_ROOT/scripts/ipc-handle.sh"

mkdir -p "$RUNTIME_DIR" "$LOG_DIR"

log() {
  printf '[%s] %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*" | tee -a "$LOG_FILE"
}

cleanup() { rm -f "$PID_FILE"; log "ipc-poll: exit"; }
trap cleanup EXIT
trap 'exit 0' INT TERM

usage() {
  cat <<EOF
ipc-poll-loop.sh — poll IPC bus for messages addressed to an agent.

Usage:
  AGENT=<id> $0 [--background|--stop]

Env:
  IPC_BUS     — path to bus.jsonl (default: /home/user/Development/PlausiDen/.ipc/bus.jsonl)
  AGENT       — this agent's id (default: claude-1)
  SLEEP_SECS  — poll cadence (default: 60)
EOF
}

case "${1:-}" in
  -h|--help) usage; exit 0 ;;
  --stop)
    [[ -f "$PID_FILE" ]] && kill "$(cat "$PID_FILE")" 2>/dev/null || true
    touch "$STOP_FILE"
    log "ipc-poll: stop requested"
    exit 0
    ;;
  --background)
    setsid "$0" </dev/null >>"$LOG_FILE" 2>&1 &
    echo "ipc-poll: backgrounded (pid $!)"
    exit 0
    ;;
esac

if [[ -f "$PID_FILE" ]] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
  log "ipc-poll: already running (pid $(cat "$PID_FILE"))"
  exit 0
fi
echo "$$" > "$PID_FILE"
rm -f "$STOP_FILE"
log "ipc-poll: started (agent=$AGENT, bus=$BUS, sleep=${SLEEP_SECS}s)"

# Cursor tracks the last byte offset processed.
[[ -f "$CURSOR_FILE" ]] || echo 0 > "$CURSOR_FILE"

while true; do
  [[ -f "$STOP_FILE" ]] && { log "ipc-poll: STOP file, exiting"; exit 0; }
  if ! BUS=$(ipc_resolve_read); then
    log "ipc-poll: no readable bus via discovery; sleeping"
    sleep "$SLEEP_SECS"
    continue
  fi
  cursor=$(cat "$CURSOR_FILE")
  size=$(stat -c%s "$BUS" 2>/dev/null || stat -f%z "$BUS" 2>/dev/null)
  if (( size > cursor )); then
    new_lines=$(tail -c $((size - cursor)) "$BUS")
    echo "$cursor $size" | log "ipc-poll: new bytes from $cursor to $size"
    while IFS= read -r line; do
      [[ -z "$line" ]] && continue
      # Naive grep on `"to":["claude-1"]` etc; production should use jq.
      if echo "$line" | grep -q "\"to\":[^]]*\"$AGENT\""; then
        log "ipc-poll: message for $AGENT — $(echo "$line" | head -c 200)"
        if [[ -x "$HANDLER" ]]; then
          echo "$line" | "$HANDLER" "$AGENT" || log "ipc-poll: handler failed"
        fi
      fi
    done <<< "$new_lines"
    echo "$size" > "$CURSOR_FILE"
  fi
  sleep "$SLEEP_SECS"
done
