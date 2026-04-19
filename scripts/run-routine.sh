#!/usr/bin/env bash
# run-routine.sh — execute an audit routine bundle from PlausiDen-Audits.
# Stub: dispatcher only. Each audit's automation lives in its own ci.yml /
# tooling and is invoked from here. Until per-audit automation lands, this
# prints what would run and exits 0.

set -euo pipefail

ROUTINE="${1:?usage: run-routine.sh <routine>}"
AUDITS_REPO="${AUDITS_REPO:-/home/user/Development/PlausiDen/PlausiDen-Audits}"
ROUTINE_FILE="$AUDITS_REPO/routines/${ROUTINE}.toml"

[[ -f "$ROUTINE_FILE" ]] || { echo "::error::no routine: $ROUTINE_FILE" >&2; exit 1; }

echo "run-routine: $ROUTINE in $(pwd)"
grep -E '^\s*"[a-z-]+",?$' "$ROUTINE_FILE" \
  | sed -E 's/[",]//g; s/^\s+//' \
  | while read -r audit; do
      [[ -z "$audit" ]] && continue
      echo "  - $audit (would run audits/$audit/ci.yml)"
    done
