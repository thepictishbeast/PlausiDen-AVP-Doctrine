# CLAUDE-md-template.md — Standard framing block for every repo's CLAUDE.md

Every PlausiDen-managed repo has a `CLAUDE.md` at its root. This file is
loaded as context at the start of every Claude Code session that opens
the repo. **The first ~30 lines are load-bearing**: they're what an
agent reads before doing anything, especially after a context compact
or a fresh session. Inconsistent or missing framing wastes the first
N tool calls re-deriving what the framing block should have said.

This file is a **template**. Copy it to `<repo>/CLAUDE.md`, replace the
`{{ … }}` placeholders, then add repo-specific content below the
"Repo-specific guidance" line. Keep the header section structurally
intact — agents grep for the section names.

## The framing block (required, in this order)

```markdown
# CLAUDE.md — {{ repo-name }}

> **Read this file before doing anything.** If a session was
> recently compacted or this is a fresh session, the conversation
> history may be stale or absent. The framing block below is the
> source of truth.

## Identity

- **Repo:** `{{ repo-name }}` ({{ one-line purpose, e.g. "core data pollution library for the PlausiDen ecosystem" }})
- **Part of:** PlausiDen ecosystem ({{ link to PlausiDen-AVP-Doctrine README }})
- **Doctrine:** all changes governed by [PlausiDen-AVP-Doctrine](https://github.com/thepictishbeast/PlausiDen-AVP-Doctrine).
  Required audits at [PlausiDen-Audits](https://github.com/thepictishbeast/PlausiDen-Audits).

## Stop conditions

If the user issues a stop word (stop, halt, kill, abort, freeze,
wait, pause, enough), halt every agent immediately. See
[stop-conditions.md](https://github.com/thepictishbeast/PlausiDen-AVP-Doctrine/blob/main/standing-orders/stop-conditions.md).

## Before making any changes

1. {{ language-specific build / test command, e.g. `cargo test --workspace` }}
2. {{ lint / type-check command, e.g. `cargo clippy -- -D warnings` }}
3. {{ if multi-target: cross-compile check, e.g. `cargo check --target wasm32-unknown-unknown` }}
4. Read any `ARCHITECTURE.md`, `SECURITY.md`, `OPSEC.md` in the repo root.

## After making changes

1. Re-run the build + test commands above.
2. Update `CHANGELOG.md` for any user-visible change.
3. Update `ARCHITECTURE.md` if architecture changed (see the
   Out-of-Scope pattern in the AVP-Doctrine examples directory).
4. Annotate any new `unsafe`, `unwrap`, `expect`, or non-obvious
   security choice with the standard inline annotations
   (`SAFETY:`, `BUG ASSUMPTION:`, `SECURITY:`, etc.) — see
   [annotations/README.md](https://github.com/thepictishbeast/PlausiDen-AVP-Doctrine/blob/main/annotations/README.md).
```

## Repo-specific guidance

Below the framing block, add what's unique to this repo: the crate
map, the architectural decisions agents need to know, narrative
framing rules, file-layout conventions, etc. Keep these short and
punchy — every line costs context tokens.

## Why a framing block at all

Without it, the first thing every fresh session does is:

- Re-read `Cargo.toml` to figure out what the repo is.
- Re-read `README.md` to figure out what the project does.
- Re-read several source files to figure out the architecture.
- Re-derive that AVP-2 governs the codebase from scattered hints.
- Sometimes miss the doctrine entirely and produce a PR that
  fails CI on the first push.

The framing block collapses all of that to a single read at the
top of the file. The cost is a ~40-line CLAUDE.md header per repo;
the saving is the first 5–10 tool calls of every session.

## Discipline

- The framing block sections (`Identity`, `Stop conditions`,
  `Before making any changes`, `After making changes`) MUST appear,
  in that order, in every PlausiDen repo's CLAUDE.md.
- The names of those sections are grep targets — don't paraphrase
  them. A future audit (`framing-block-audit`) will check for them
  by exact-string match.
- Repo-specific guidance below the framing block is open-form;
  keep it short.

## Ratchet

When this template changes, every existing repo's CLAUDE.md becomes
slightly stale. Don't sweep them all in one PR — let drift
accumulate, then fix in a single sweep when the framing-block-audit
flags it. The audit's "first failing repo" gets fixed; the rest
queue up for the next sweep.
