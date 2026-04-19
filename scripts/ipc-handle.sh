#!/usr/bin/env bash
# ipc-handle.sh — process a single IPC message piped on stdin.
# Args: <agent-id>
# Reads a JSON line from stdin and dispatches based on `kind`.
# Stub: logs and acks. Real handlers per-agent live in the agent's repo.

set -euo pipefail

AGENT="${1:?usage: ipc-handle.sh <agent-id>}"
BUS="${IPC_BUS:-/home/user/Development/PlausiDen/.ipc/bus.jsonl}"

while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    id=$(echo "$line" | python3 -c 'import sys, json; print(json.loads(sys.stdin.read()).get("id","?"))' 2>/dev/null || echo "?")
    kind=$(echo "$line" | python3 -c 'import sys, json; print(json.loads(sys.stdin.read()).get("kind","?"))' 2>/dev/null || echo "?")
    subj=$(echo "$line" | python3 -c 'import sys, json; print(json.loads(sys.stdin.read()).get("subject",""))' 2>/dev/null || echo "")
    echo "ipc-handle: agent=$AGENT kind=$kind id=$id subject=$subj"
    ack_id=$(uuidgen 2>/dev/null || python3 -c 'import uuid; print(uuid.uuid4())')
    ts=$(date -u +%Y-%m-%dT%H:%M:%SZ)
    printf '{"id":"%s","ts":"%s","from":"%s","to":["all"],"kind":"ack","subject":"ack","body":"acked %s","refs":{"original":"%s"}}\n' \
        "$ack_id" "$ts" "$AGENT" "$id" "$id" >> "$BUS"
done
