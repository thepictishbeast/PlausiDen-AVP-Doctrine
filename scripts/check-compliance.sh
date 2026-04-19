#!/usr/bin/env bash
# check-compliance.sh — verify that a repo follows the AVP Doctrine.
# Checks: CLAUDE.md exists and references doctrine, .github/workflows
# contains avp.yml, docs/AVP-LINK.md present, no agent loops are running
# without a registry entry.
# Exit 0 if compliant; exit 1 with a list of findings otherwise.

set -euo pipefail

REPO="${1:-$(pwd)}"
DOCTRINE_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REGISTRY="$DOCTRINE_ROOT/loops/REGISTRY.md"

failed=0
fail() { echo "::error::$REPO: $*" >&2; failed=$((failed + 1)); }

check_doctrine_ref() {
    local file="$1"
    [[ -f "$REPO/$file" ]] || { fail "missing $file"; return; }
    grep -q "PlausiDen-AVP-Doctrine" "$REPO/$file" \
        || fail "$file does not reference PlausiDen-AVP-Doctrine"
}

check_doctrine_ref "CLAUDE.md"
check_doctrine_ref "docs/AVP-LINK.md"

[[ -f "$REPO/.github/workflows/avp.yml" ]] || fail "missing .github/workflows/avp.yml"

if [[ "${1:-}" == "--loops" ]]; then
    echo "check-compliance: loop registration scan"
    while IFS= read -r line; do
        pid=$(echo "$line" | awk '{print $2}')
        cmd=$(echo "$line" | sed 's/^[^/]*//')
        if ! grep -qF "$cmd" "$REGISTRY"; then
            fail "running unregistered loop: pid=$pid cmd=$cmd"
        fi
    done < <(pgrep -af 'PlausiDen-AVP-Doctrine/loops' 2>/dev/null || true)
fi

if [[ "$failed" -eq 0 ]]; then
    echo "check-compliance: $REPO compliant"
    exit 0
fi
echo "check-compliance: $failed finding(s) for $REPO" >&2
exit 1
