#!/usr/bin/env bash
# install-doctrine.sh — drop AVP-Doctrine references into the current repo.
# Creates: CLAUDE.md (if absent) referencing the doctrine,
#          .github/workflows/avp.yml (skeleton calling the audit routines),
#          docs/AVP-LINK.md pointing contributors at the doctrine.
# Idempotent: skips files that already exist.

set -euo pipefail

DOCTRINE_URL="https://github.com/thepictishbeast/PlausiDen-AVP-Doctrine"
AUDITS_URL="https://github.com/thepictishbeast/PlausiDen-Audits"

write_if_absent() {
    local path="$1" content="$2"
    if [[ -e "$path" ]]; then
        echo "skip: $path already exists"
        return
    fi
    mkdir -p "$(dirname "$path")"
    printf '%s' "$content" > "$path"
    echo "wrote: $path"
}

write_if_absent "CLAUDE.md" "# CLAUDE.md — AVP-managed repo

This repo is governed by the [PlausiDen AVP Doctrine]($DOCTRINE_URL).
Read the doctrine before opening a PR. Required audits live at
[$AUDITS_URL]($AUDITS_URL).

## Stop conditions

If the user issues a stop word (stop, halt, kill, abort, freeze, wait,
pause, enough), halt every agent immediately. See
[$DOCTRINE_URL/blob/main/standing-orders/stop-conditions.md]($DOCTRINE_URL/blob/main/standing-orders/stop-conditions.md).
"

write_if_absent ".github/workflows/avp.yml" "name: avp

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  pre-merge:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run pre-merge audit routine
        run: |
          # Resolve from PlausiDen-Audits/routines/pre-merge.toml.
          # Until per-audit ci.yml stanzas are wired up, this is a
          # placeholder that fails open. Replace with the per-audit
          # checks from $AUDITS_URL once the target repo opts in to
          # specific audits.
          echo '::warning::pre-merge audit routine not yet wired — see $AUDITS_URL'
"

write_if_absent "docs/AVP-LINK.md" "# AVP Doctrine

This project is governed by the [PlausiDen AVP Doctrine]($DOCTRINE_URL).

- Standing orders for AI agents: $DOCTRINE_URL/tree/main/standing-orders
- Coding gates: $DOCTRINE_URL/tree/main/gates
- Inline annotations: $DOCTRINE_URL/blob/main/annotations/README.md
- Audit catalog: $AUDITS_URL
"

echo "install-doctrine: done."
