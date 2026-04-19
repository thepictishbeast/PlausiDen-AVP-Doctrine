#!/usr/bin/env bash
# foss-upstream-check.sh — walk every vendored dep, diff against upstream's
# latest release, file findings if drift detected. Stub: scaffolds the
# walker; per-crate diff requires upstream URL pinned in VENDORING.md.

set -euo pipefail

REPOS_ROOT="${REPOS_ROOT:-/home/user/Development/PlausiDen}"
echo "foss-upstream-check: scanning $REPOS_ROOT for vendored deps"

found=0
while IFS= read -r vendor; do
    [[ -z "$vendor" ]] && continue
    found=$((found + 1))
    if [[ -f "$vendor/VENDORING.md" ]]; then
        upstream=$(grep -E '^upstream[[:space:]]*=' "$vendor/VENDORING.md" 2>/dev/null \
            | head -1 | sed -E 's/^upstream[[:space:]]*=[[:space:]]*//' || true)
        echo "  ↳ $vendor  upstream=${upstream:-unknown}"
    else
        echo "  ↳ $vendor  MISSING VENDORING.md"
    fi
done < <(find "$REPOS_ROOT" -type d -name vendor \
    -not -path '*/node_modules/*' -not -path '*/target/*' 2>/dev/null)

echo "foss-upstream-check: $found vendor directories scanned"
