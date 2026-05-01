# Multi-instance coordination protocol

> **Status:** v0.1 design. Implementation lands as `avp intent {claim|overlap|merge-order|verify}` and is consumed by the conductor.
> **Owner:** William Armstrong
> **Last updated:** 2026-05-01

## Problem

When N agents (Claude instances, humans, scheduled jobs) work on the same
PlausiDen sibling at once, three things go wrong without coordination:

1. **Stomp.** Two branches modify the same file; merge produces a conflict
   that's silently resolved with the wrong answer.
2. **Scope creep.** An agent declares it'll fix one bug, then quietly
   touches twelve unrelated files that show up at PR review with no
   justification.
3. **Order chaos.** Three branches need to land in a specific order
   (intent A creates the type, intent B uses it, intent C tests both).
   Without explicit ordering, a maintainer has to reverse-engineer the
   DAG by reading every diff.

The protocol below — declared up front, machine-checked — turns each of
these from a 3am surprise into a CI failure.

## `.avp-intent.toml`

Per-branch manifest, committed at the repo root of each working branch.
Schema is canonical (parsed by `avp-core::intent::IntentFile`).

```toml
# Required.
agent_id       = "claude-2026-05-01-T01"      # unique identifier
branch         = "claude/wire-avp-engine"     # the branch this intent lives on
goal           = "Wire avp into PlausiDen-Engine CI."
success_test   = "cargo run -p avp -- check rust --workspace"
opened_at      = "2026-05-01T20:00:00Z"
declared_files = [
    ".github/workflows/avp.yml",
    "avp-ratchet.toml",
]

# Optional.
allows_overlap_with = []                      # other agent_ids whose overlap is OK
expected_pr         = "thepictishbeast/PlausiDen-Engine#42"
expires_after       = "2026-05-15"             # auto-stale; conductor archives past this
```

### Field rules

- **`agent_id`** — stable, unique per parallel work-unit. Convention:
  `<persona>-<ISO date>-T<short hash>`. Two intents with the same
  `agent_id` is a fatal schema error.
- **`branch`** — the git branch this intent governs. The intent file
  must literally exist on this branch (verified by `avp intent overlap`
  via `git show <branch>:.avp-intent.toml`).
- **`goal`** — one-sentence statement. PR reviewers read this; keep it
  human.
- **`success_test`** — a runnable command that returns 0 iff the goal
  is achieved. The conductor runs this after merging; failures revert.
- **`declared_files`** — repo-relative paths. Globs allowed
  (`crates/*/src/lib.rs`). Empty list = "I will touch nothing" (used
  by audit-only intents).
- **`opened_at`** — RFC-3339 timestamp. Anchors expiry math.
- **`allows_overlap_with`** — list of `agent_id`s whose declared-files
  overlap is intentional (e.g., paired refactor on shared module).
  Without this, any overlap is a `::error::` from `avp intent overlap`.
- **`expected_pr`** — optional GitHub reference for tracking.
- **`expires_after`** — RFC-3339 date. After expiry, the conductor
  archives the intent (moves the branch to `archive/<branch>`,
  unblocks others' overlap claims).

### Strict parser

Required fields are mandatory. Typos in field names fail the parse
loudly — the same paranoia applied to `avp-ratchet.toml`. The intent
file is privileged metadata; a silent typo could un-claim work.

## Lifecycle

```
                ┌────────────┐
                │  proposed  │ — agent decides to start work, no .avp-intent.toml yet
                └─────┬──────┘
                      │ avp intent claim ...
                      ▼
                ┌────────────┐
            ┌── │  claimed   │ — file written, branch created, no overlap with active intents
            │   └─────┬──────┘
overlap-    │         │ work + commits + push
detected    │         ▼
            │   ┌────────────┐
            └── │   active   │ — at least one declared file touched
                └─────┬──────┘
                      │ PR opened
                      ▼
                ┌────────────┐
                │   review   │
                └─────┬──────┘
                      │ avp intent verify (CI gate on the PR)
                      ▼
                ┌────────────┐
                │  verified  │ — actual diff ⊆ declared_files
                └─────┬──────┘
                      │ merge
                      ▼
                ┌────────────┐
                │  archived  │ — intent file deleted from main; record kept in audit log
                └────────────┘
```

States are derivable from git+intent; we don't store them as state in
the file. `avp intent overlap` re-derives the picture every run.

## Subcommands

### `avp intent claim`

Validates the proposed intent against active siblings, then writes
`.avp-intent.toml` to the current working branch.

```sh
avp intent claim \
  --branch claude/wire-avp-engine \
  --goal 'Wire avp into PlausiDen-Engine CI.' \
  --success-test 'cargo run -p avp -- check rust --workspace' \
  --files .github/workflows/avp.yml \
  --files avp-ratchet.toml
```

Pre-claim checks:

1. The proposed `branch` doesn't already have an active `.avp-intent.toml`.
2. The proposed `declared_files` don't overlap with any other active
   intent (unless that intent's `allows_overlap_with` includes this
   intent's `agent_id`, or vice-versa).

Failure path: `avp intent claim` exits non-zero with an `::error::`
naming the conflicting intent. The agent must either coordinate
(add to `allows_overlap_with`), wait, or pick a different scope.

### `avp intent overlap`

Scans every branch's `.avp-intent.toml`, builds the file → intents
map, and reports collisions.

```text
$ avp intent overlap
overlap: claude/wire-avp-engine ↔ jordan/refactor-engine-cli
  shared files:
    - .github/workflows/avp.yml
  resolution: add `allows_overlap_with = ["claude-..."]` to one side, or merge order
```

Exit 0 if no overlaps remain after `allows_overlap_with` resolution;
non-zero otherwise.

### `avp intent merge-order`

Topologically sort active intents into a merge order. Two intents that
share files have an implicit edge; the order is determined by `opened_at`
(earlier first) unless an explicit `merge_after = [...]` field overrides.

Output is a DAG-as-text:

```text
1. claude-2026-05-01-T01  (claude/wire-avp-engine)
   declared: .github/workflows/avp.yml, avp-ratchet.toml
   blocks:   [jordan-2026-05-01-T02]

2. jordan-2026-05-01-T02  (jordan/refactor-engine-cli)
   declared: src/cli.rs, .github/workflows/avp.yml
   waits-on: [claude-2026-05-01-T01]
```

The conductor consumes this as JSON (`--format=json`).

### `avp intent verify`

Runs as a CI gate on every PR. Reads the branch's `.avp-intent.toml`
and the PR's actual diff. Fails if:

1. The branch has no `.avp-intent.toml` (every branch managed by the
   conductor must have one).
2. The diff touches files outside `declared_files` (and not in the
   `allowed_overage` allowlist for legacy migration cases).
3. The intent has `expired_after` past today.

This turns "I lied about my scope" from a midnight merge surprise into
a CI red mark.

## Storage + git integration

- `.avp-intent.toml` is committed at branch root. It's *part of the
  branch's diff*, deliberately — every commit explicitly records the
  intent state.
- Discovery: `git for-each-ref refs/heads/* | git show <ref>:.avp-intent.toml`
  reads every branch's claim without checking it out.
- Worktrees: `git worktree list --porcelain` is the canonical source
  for "which agent is on which branch right now". The conductor uses
  this to spawn one Claude Code session per worktree.
- Archive on merge: the intent file is deleted in the merge commit.
  An audit log at `PlausiDen-Audits/intent-log.toml` keeps a
  permanent record (intent → merge SHA → outcome).

## Conductor integration

The conductor (separate doc, separate binary) consumes intents this way:

1. **Spawn**: for each open intent, create a worktree at the branch
   tip, launch a Claude Code session with that intent as part of its
   system prompt. Permission scopes derived from `declared_files`.
2. **Watch**: subscribe to the intent's `opened_at` + `expires_after`
   for archive triggers.
3. **Choreograph**: on overlap detection, the conductor pauses the
   later-claimed branch (sets it to `paused-blocked`) and notifies
   the operator via PR comment.
4. **Merge**: when `avp intent verify` passes on a PR and the topological
   order says it's next, the conductor merges, archives the intent,
   and runs `success_test` against main.

## Threat model

- **Stale claims**: an agent claims work then dies/disappears. Mitigated
  by `expires_after` + the conductor's archive sweep.
- **Forged claims**: an agent writes a `.avp-intent.toml` claiming a
  scope it intends to violate. Mitigated by `avp intent verify`'s
  diff-vs-manifest check at PR time. The CI failure is on-record.
- **Collusion**: two agents add each other to `allows_overlap_with`
  to bypass overlap checks. This is *intentional* — overlap is fine
  when explicitly authorized; the audit log preserves the decision.
- **Race on claim**: two agents try to claim the same files
  simultaneously. Last-writer-wins via git push (rejected by remote
  if non-fast-forward). Locally, `avp intent claim` re-fetches before
  writing.

## Out-of-scope for v0.1

- Cross-repo intents (an intent that spans `PlausiDen-Engine` +
  `PlausiDen-Inject` simultaneously). Tracked separately.
- Web UI for browsing live intents. Conductor's `--format=json` output
  is the v0.1 interface.
- Permission-scoped Claude Code launches based on `declared_files`.
  Lands with conductor.

## v0.1 deliverable checklist

- [x] design (this doc)
- [ ] `IntentFile` + `IntentEntry` parser/validator in `avp-core`
- [ ] `avp intent claim` (write + pre-claim checks)
- [ ] `avp intent overlap` (scan + collision report)
- [ ] `avp intent merge-order` (toposort)
- [ ] `avp intent verify` (CI gate)
- [ ] tests for each: unit + tempdir-based integration
- [ ] integration test: simulate two overlapping intents end-to-end
