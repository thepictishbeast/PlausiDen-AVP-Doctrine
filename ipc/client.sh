#!/usr/bin/env bash
# ipc/client.sh — reference IPC client.
#
# Subcommands:
#   post <kind> <to-csv> <subject> <body>   — append a message
#   tail [-f] [agent]                       — print bus, optionally follow
#   ack <original-id>                       — post an ack for a message id
#   search <substring>                      — grep the bus
#   where                                   — print the resolved bus path

set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
# shellcheck disable=SC1091
source "$HERE/discovery.sh"

AGENT="${AGENT:-$(whoami)@$(hostname -s)}"

uuid() {
    uuidgen 2>/dev/null || python3 -c 'import uuid; print(uuid.uuid4())'
}

iso_now() {
    date -u +%Y-%m-%dT%H:%M:%SZ
}

cmd_post() {
    local kind="$1" to_csv="$2" subject="$3" body="${4:-}"
    local bus; bus=$(ipc_resolve_write) || { echo "::error::no writable bus" >&2; exit 1; }
    local id ts to_json
    id=$(uuid); ts=$(iso_now)
    to_json=$(printf '%s' "$to_csv" | python3 -c 'import json, sys; print(json.dumps([s.strip() for s in sys.stdin.read().split(",") if s.strip()]))')
    python3 - "$id" "$ts" "$AGENT" "$to_json" "$kind" "$subject" "$body" >>"$bus" <<'PY'
import json, sys
id_, ts, frm, to, kind, subject, body = sys.argv[1:8]
print(json.dumps({
    "id": id_, "ts": ts, "from": frm,
    "to": json.loads(to), "kind": kind,
    "subject": subject, "body": body, "refs": {}
}, separators=(",", ":")))
PY
    echo "posted: $id"
}

cmd_tail() {
    local bus; bus=$(ipc_resolve_read) || { echo "::error::no readable bus" >&2; exit 1; }
    if [[ "${1:-}" == "-f" ]]; then
        shift
        tail -n 50 -f "$bus"
    else
        local agent="${1:-}"
        if [[ -n "$agent" ]]; then
            grep "\"to\":[^]]*\"$agent\"" "$bus" || true
        else
            tail -n 100 "$bus"
        fi
    fi
}

cmd_ack() {
    local original="${1:?usage: ack <original-id>}"
    cmd_post "ack" "all" "ack" "acked $original"
}

cmd_search() {
    local needle="${1:?usage: search <substring>}"
    local bus; bus=$(ipc_resolve_read) || { echo "::error::no readable bus" >&2; exit 1; }
    grep -F "$needle" "$bus" || true
}

cmd_where() {
    if r=$(ipc_resolve_read); then echo "read : $r"; else echo "read : (none)"; fi
    if w=$(ipc_resolve_write); then echo "write: $w"; else echo "write: (none writable)"; fi
}

usage() {
    cat <<EOF
IPC reference client. Uses fallback discovery — see ipc/README.md.

Usage:
  $0 post <kind> <to-csv> <subject> <body>
     kind: task | ack | finding | broadcast | heartbeat | halt
     to-csv: "claude-0,claude-1" or "all" or "human"

  $0 tail              show the last 100 lines
  $0 tail -f           follow the bus
  $0 tail <agent>      show lines addressed to <agent>
  $0 ack <id>          post an ack for a prior message id
  $0 search <text>     grep the bus
  $0 where             show resolved read + write bus paths

Env:
  AGENT    — this agent's id (default: <user>@<host>)
  IPC_BUS  — explicit bus path (otherwise discovered)
EOF
}

case "${1:-}" in
    post)   shift; cmd_post "$@" ;;
    tail)   shift; cmd_tail "$@" ;;
    ack)    shift; cmd_ack "$@" ;;
    search) shift; cmd_search "$@" ;;
    where)  shift; cmd_where ;;
    ""|-h|--help) usage ;;
    *) usage; exit 2 ;;
esac
