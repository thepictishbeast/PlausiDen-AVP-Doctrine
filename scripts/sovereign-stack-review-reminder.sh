#!/usr/bin/env bash
# sovereign-stack-review-reminder.sh — quarterly nudge to walk
# SOVEREIGN_POLYGLOT_STACK.md and re-derive each row against current
# fitness/governance/maturity conditions.
#
# Fires first Monday of Feb / May / Aug / Nov per crons/quarterly-sovereign-stack-review.cron.
# Output: stdout (logged to ~/.local/share/plausiden/cron-state/sovereign-stack-review.log)
#         + ntfy if NTFY_URL is set.

set -euo pipefail

DOC="${SOVEREIGN_STACK_DOC:-/home/user/Development/PlausiDen/PlausiDen-AVP-Doctrine/SOVEREIGN_POLYGLOT_STACK.md}"
NOW="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

echo "[$NOW] sovereign-stack-review-reminder firing"

if [[ ! -f "$DOC" ]]; then
    echo "  ! SOVEREIGN_POLYGLOT_STACK.md not found at $DOC; skipping"
    exit 0
fi

current_version=$(grep -m1 '^Version:' "$DOC" | sed -E 's/^Version:[[:space:]]*//' || echo "unknown")
months_since_update=$( ( date -d "${current_version//./-}-01" +%s 2>/dev/null && date +%s ) | awk 'NR==1{a=$1} NR==2{b=$1} END{printf "%d\n",(b-a)/2592000}' 2>/dev/null || echo "?")

echo "  current Version: $current_version"
echo "  months since last update: $months_since_update"
echo ""
echo "  Walk every layer in $DOC and ask three questions per row:"
echo "    1. Has fitness shifted?"
echo "    2. Has governance shifted?"
echo "    3. Has a watch-list entry become production-ready?"
echo ""
echo "  Trigger events to also check for:"
echo "    - Any dependency relicensed to non-FOSS (HashiCorp/Elastic/MongoDB precedent)"
echo "    - Any watch-list language hit 1.0"
echo "    - New vendor-captive platform in our deploy surface"
echo "    - Formal verification capability landed in Rust we'd been waiting on"
echo ""
echo "  When done: bump Version: YYYY.MM, append to ## Change log, commit"
echo "  with prefix 'doctrine(sovereign-stack):' and push."

if [[ -n "${NTFY_URL:-}" ]]; then
    curl -fsS -H "Title: Sovereign Polyglot Stack quarterly review" \
              -H "Priority: default" \
              -H "Tags: clipboard" \
              -d "Version $current_version — walk SOVEREIGN_POLYGLOT_STACK.md by layer. Update protocol at bottom of doc." \
              "$NTFY_URL" >/dev/null 2>&1 || echo "  (ntfy dispatch failed)"
fi
