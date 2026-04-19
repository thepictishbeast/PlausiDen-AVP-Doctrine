#!/usr/bin/env bash
# ipc/discovery.sh — resolve the IPC bus path across primary + fallback
# locations. Source this from other scripts:
#
#   # shellcheck disable=SC1091
#   source /path/to/PlausiDen-AVP-Doctrine/ipc/discovery.sh
#   bus=$(ipc_resolve_write) || { echo "no writable bus"; exit 1; }
#
# Or run it directly to print the first match:
#
#   $ discovery.sh --read      # first existing + readable
#   $ discovery.sh --write     # first writable (parent dir creatable)
#   $ discovery.sh --list      # print all candidates, one per line

set -euo pipefail

ipc_candidates() {
    local list=()
    [[ -n "${IPC_BUS:-}" ]] && list+=("$IPC_BUS")
    list+=(
        "/home/user/Development/PlausiDen/.ipc/bus.jsonl"
        "${XDG_RUNTIME_DIR:-/run/user/$UID}/plausiden/bus.jsonl"
        "$HOME/.local/share/plausiden/ipc/bus.jsonl"
        "$HOME/.cache/plausiden/ipc/bus.jsonl"
        "/tmp/claude-ipc/bus.jsonl"
    )
    printf '%s\n' "${list[@]}"
}

ipc_resolve_read() {
    while IFS= read -r p; do
        [[ -f "$p" && -r "$p" ]] && { echo "$p"; return 0; }
    done < <(ipc_candidates)
    return 1
}

ipc_resolve_write() {
    while IFS= read -r p; do
        local dir
        dir=$(dirname "$p")
        if [[ -d "$dir" && -w "$dir" ]]; then
            echo "$p"; return 0
        fi
        if mkdir -p "$dir" 2>/dev/null && [[ -w "$dir" ]]; then
            echo "$p"; return 0
        fi
    done < <(ipc_candidates)
    return 1
}

ipc_drop_breadcrumb() {
    local bus="$1"
    local mark="$HOME/.local/share/plausiden/ipc/bus.where"
    mkdir -p "$(dirname "$mark")" 2>/dev/null || return 0
    echo "$bus" > "$mark"
}

if [[ "${BASH_SOURCE[0]:-}" == "${0}" ]]; then
    case "${1:-}" in
        --read)
            if r=$(ipc_resolve_read); then echo "$r"; else
                echo "::error::no readable IPC bus found" >&2; exit 1; fi ;;
        --write)
            if w=$(ipc_resolve_write); then
                echo "$w"; ipc_drop_breadcrumb "$w"
            else
                echo "::error::no writable IPC bus location" >&2; exit 1; fi ;;
        --list) ipc_candidates ;;
        -h|--help|*)
            cat <<EOF
Usage: $0 --read | --write | --list

--read   print the first existing + readable candidate (fail if none)
--write  print the first writable candidate (create parent dir if needed)
--list   print all candidates in priority order

Env:
  IPC_BUS  — if set, becomes candidate #1.
EOF
            [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]] || exit 2 ;;
    esac
fi
