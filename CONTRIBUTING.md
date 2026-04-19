# Contributing to PlausiDen-AVP-Doctrine

This repo *is* the doctrine. Changes here ripple to every PlausiDen repo
because every other repo's `CLAUDE.md` and CI references back to here.
Open with care.

## Before opening a PR

- Read [`AVP2_PROTOCOL.md`](AVP2_PROTOCOL.md) end-to-end.
- Read the standing-orders file matching your role.
- Run `bash scripts/check-compliance.sh` against this repo.

## What kinds of PRs are welcome

- New standing-orders for a new agent role.
- New entry in `annotations/` (with grep-usage example).
- New language gate file in `gates/`.
- New cron in `crons/` (with `install.sh` updated).
- New loop in `loops/` (with `REGISTRY.md` updated).
- New prompt template in `prompts/`.
- Fixes to the FOSS / cross-repo / ship-decision protocols based on
  real-world findings.

## What's out of scope here

- Concrete project code — those changes go in the relevant PlausiDen repo.
- Audit checklists — those go in [PlausiDen-Audits](https://github.com/thepictishbeast/PlausiDen-Audits).
- Test utilities — those go in [Testing-Framework](https://github.com/thepictishbeast/Testing-Framework).

## Versioning

Doctrine PRs that change agent behavior bump the protocol version in the
affected file's frontmatter. Major bumps (breaking semantics) trigger a
broadcast on the IPC bus subject `DOCTRINE-UPDATED`.
