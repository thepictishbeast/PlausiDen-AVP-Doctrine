#!/usr/bin/env bash
# file-finding.sh — append a finding to PlausiDen-Audits/audits/<slug>/findings/.

set -euo pipefail

SLUG="${1:?usage: file-finding.sh <audit-slug> <message>}"
shift
MSG="$*"
AUDITS_REPO="${AUDITS_REPO:-/home/user/Development/PlausiDen/PlausiDen-Audits}"
AGENT="${AGENT:-$(hostname -s)}"
DATE=$(date -u +%Y-%m-%d)
DIR="$AUDITS_REPO/audits/$SLUG/findings"

[[ -d "$DIR" ]] || { echo "::error::no audit folder $DIR" >&2; exit 1; }

OUT="$DIR/${DATE}-${AGENT}.md"
{
    echo "# Finding ${DATE} (${AGENT})"
    echo
    echo "$MSG"
    echo
    echo "_filed by file-finding.sh_"
} >> "$OUT"
echo "filed: $OUT"
