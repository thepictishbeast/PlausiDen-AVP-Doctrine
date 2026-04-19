#!/usr/bin/env bash
# post-heartbeat.sh — post a single heartbeat message to the IPC bus.

set -euo pipefail

DOCTRINE_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck disable=SC1091
source "$DOCTRINE_ROOT/ipc/discovery.sh"

BUS=$(ipc_resolve_write) || { echo "::error::no writable IPC bus" >&2; exit 1; }
AGENT="${AGENT:-$(hostname -s)}"
id=$(uuidgen 2>/dev/null || python3 -c 'import uuid; print(uuid.uuid4())')
ts=$(date -u +%Y-%m-%dT%H:%M:%SZ)
printf '{"id":"%s","ts":"%s","from":"%s","to":["all"],"kind":"heartbeat","subject":"alive","body":"%s","refs":{}}\n' \
  "$id" "$ts" "$AGENT" "host=$(hostname -s) load=$(uptime | awk -F'load average:' '{print $2}' | xargs)" >> "$BUS"
echo "heartbeat posted by $AGENT"
