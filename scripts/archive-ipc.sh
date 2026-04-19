#!/usr/bin/env bash
# archive-ipc.sh — roll IPC bus.jsonl entries older than 30 days into
# bus.archive/<YYYY-MM>.jsonl. Append-only, never deletes.

set -euo pipefail

DOCTRINE_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck disable=SC1091
source "$DOCTRINE_ROOT/ipc/discovery.sh"

BUS=$(ipc_resolve_read) || { echo "no existing bus found via discovery; nothing to archive"; exit 0; }
ARCHIVE_DIR="$(dirname "$BUS")/bus.archive"
mkdir -p "$ARCHIVE_DIR"

cutoff=$(date -u -d '30 days ago' +%s 2>/dev/null || date -u -v-30d +%s)
tmp=$(mktemp)
moved=0

python3 - <<PY
import json, os, sys, time
from datetime import datetime, timezone
bus = "$BUS"
archive_dir = "$ARCHIVE_DIR"
cutoff = $cutoff
tmp = "$tmp"
moved = 0
keep = []
buckets = {}
with open(bus, 'r') as f:
    for line in f:
        line = line.rstrip('\n')
        if not line:
            continue
        try:
            evt = json.loads(line)
            ts = evt.get('ts', '')
            t = datetime.fromisoformat(ts.replace('Z', '+00:00')).timestamp()
        except Exception:
            keep.append(line)
            continue
        if t < cutoff:
            month = ts[:7]
            buckets.setdefault(month, []).append(line)
            moved += 1
        else:
            keep.append(line)
for month, lines in buckets.items():
    out = os.path.join(archive_dir, f"{month}.jsonl")
    with open(out, 'a') as f:
        f.write('\n'.join(lines) + '\n')
with open(tmp, 'w') as f:
    f.write('\n'.join(keep) + ('\n' if keep else ''))
os.replace(tmp, bus)
print(f"archive-ipc: moved {moved} events into {len(buckets)} archive file(s)")
PY
