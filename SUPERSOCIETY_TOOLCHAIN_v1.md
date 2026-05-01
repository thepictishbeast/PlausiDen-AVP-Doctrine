# SUPERSOCIETY_TOOLCHAIN — v1 design document

> **Status:** living document. Last updated: 2026-04-30.
> **Owner:** William Armstrong &lt;william@plausiden.com&gt; (`thepictishbeast`).
> **Persistence purpose:** this file exists so a fresh Claude session (or
> any new contributor) can pick up the work from the doc alone, without
> re-deriving it from chat history. Update it in the same PR as any
> structural change.

---

## Why this exists

The PlausiDen portfolio has ~80 sibling repos. The AVP-2 doctrine
(`AVP2_PROTOCOL.md`, `SUPERSOCIETY_STACK.md`) describes how every line of
code should be written, audited, and shipped. But the *enforcement* of that
doctrine has historically been markdown-only — every sibling copy-pastes
`composition.yml` + `security.yml` from the others, every `deny.toml` is a
slightly different copy, every `.github/workflows/avp.yml` (where it exists
at all) is a fail-open `echo '::warning::TODO'` placeholder. Doctrine
without execution is theatre.

This document defines the supersociety toolchain that ends the theatre: one
Rust workspace, one set of binaries, one set of canonical configs,
distributed by tag, drift-checked daily.

---

## Stack mandate

Every binary and library here follows `SUPERSOCIETY_STACK.md`:

- **Rust everywhere.** No bash, no python, no shell scripts shipped as
  build artifacts. Bash/Python is acceptable only as throwaway design
  spikes (deleted before merge) or as one-line GitHub Action `run:`
  shims that exec a Rust binary.
- `#![forbid(unsafe_code)]` at every crate root. If `unsafe` is *required*
  (FFI, extreme perf), narrow it to the smallest possible module and
  carry a `// SAFETY:` proof.
- **Audited crypto only**: `chacha20poly1305`, `blake3`, `argon2`,
  `ed25519-dalek`, `subtle`, `zeroize`, `rand_chacha`. No hand-rolled
  crypto.
- **Reproducible builds.** Targeted: a Nix flake + static-musl release
  pipeline for the binaries.
- **Frontend (where present): Leptos → WASM.** No React, no Svelte. CSS
  via tooling that's itself Rust (`lightningcss`).

---

## The toolchain — what gets built

Six components, all in supersociety stack, all sharing infrastructure:

| # | Component | Form | Status | Notes |
|---|-----------|------|--------|-------|
| 1 | **`avp`** | static-musl Rust binary | **v0.1.0-dev — workspace + ratchet + CLI scaffold landed** | Core gate enforcer. Subcommands: `check {rust,ts,py}`, `ratchet`, `drift`, `install`, `gate`, `explain`, `intent`, `completions`. |
| 2 | **`conductor`** | long-running Rust daemon | scaffold pending | Spawns Claude Code subprocesses across N PlausiDen sibling worktrees in parallel; FSM-supervised; auto-resumes transient pauses; escalates genuine blockers to GitHub issues; drives merge order via the AVP gate. |
| 3 | **`avp intent`** *(subcommand of `avp`)* | shipped as part of `avp` | scaffold pending | `.avp-intent.toml` per worktree (declared files + goal + success-test + agent-id + branch). Pre-claim conflict detection, post-hoc diff-vs-manifest verification. Drives `conductor`'s choreography. |
| 4 | **PlausiDen-Annotator** | browser extension + Tauri shell | not started | Visual UI inspector: click-to-select elements (Chrome devtools-style), capture DOM/console/network/performance, attach user comments, relay structured findings back to Claude/conductor. Likely extends `PlausiDen-Browser-Ext` (already MV3+WASM). Optional integration with `PlausiDen-Crawler` for headless capture at scale. |
| 5 | **PlausiDen-Loom-Backend** *(working name)* | Rust workspace + Leptos UI | not started | Counterpart to `PlausiDen-Loom` (which is UI-side) for backend code workflows: data jobs, batch processes, scheduled tasks, service contracts. Visual + CLI workflow editor, code-as-content. Integrates with `avp` and `conductor`. |
| 6 | **PlausiDen-CMS-Backend** *(working name)* | Rust workspace + Leptos UI | not started | Counterpart to `PlausiDen-CMS` (UI content) for backend code-as-content: schemas, contracts, prompt libraries, dataset manifests, fixture libraries, FOSS-absorbed crate manifests. Cross-references the doctrine repo and `PlausiDen-Audits`. |

Component 1 is the bedrock; everything else depends on its types and
ratchet model. Components 2–3 ship next; 4–6 are post-v1.0.

---

## Workspace layout (today)

```
/home/user/Development/PlausiDen/PlausiDen-AVP-Doctrine/
├── AVP2_PROTOCOL.md                  doctrine spec
├── SUPERSOCIETY_TOOLCHAIN_v1.md      ← THIS FILE
├── DOCTRINE.md / README.md           doctrine front-matter
├── annotations/ doctrine/ tiers/     doctrine catalogue
├── gates/{rust,ts,py,frontend}.md    per-language gate spec
├── ipc/  loops/  crons/  timers/     existing agent infra
├── standing-orders/                  agent role briefs
├── scripts/                          legacy bash (to be retired)
└── avp/                              ← THE TOOLCHAIN
    ├── Cargo.toml                    workspace + canonical deps
    ├── rust-toolchain.toml           stable channel pin
    ├── deny.toml                     cargo-deny baseline (also synced to siblings)
    ├── clippy.toml                   clippy tuning (also synced)
    ├── rustfmt.toml                  formatter (also synced)
    ├── .cargo/config.toml            build-level rustflags + cargo aliases
    ├── README.md                     dev docs for the toolchain itself
    └── crates/
        ├── avp-core/                 lib: gates, ratchet, findings, reporters, repo discovery, newtypes
        └── avp/                      bin: clap CLI, subcommand dispatch
```

When component 2 lands, expect `crates/conductor-core` + `crates/conductor`.
When component 3 lands, the `avp intent` subcommand grows; no new crate.

The `avp/` workspace's own configs (`deny.toml`, `clippy.toml`,
`rustfmt.toml`, `rust-toolchain.toml`) are the **canonical baseline** that
`avp install` will sync into siblings. Editing them touches every sibling
on the next drift-check run.

---

## Gates implemented (v0.1.0-dev)

`avp gate list` is authoritative; this table mirrors it.

| ID | Severity floor | Ratchetable | Brief |
|---|---|---|---|
| `bug-assumption` | error | yes | Every `pub fn` must have a `BUG ASSUMPTION:` comment within 20 lines preceding its signature. |
| `forbidden-call` | error | yes | No `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `dbg!`, `println!`, `eprintln!` in library code unless the line carries a `// SAFETY:` / `// test-only` / `// AVP-PASS-` / `// SHIP-DECISION:` / `// FOSS-ABSORBED:` justification. |
| `debug-remove` | error | **no** (hard) | Any `DEBUG-REMOVE:` literal fails the build. Pre-release lint. |
| `unsafe-proof` | error | yes | Every `unsafe { … }` block and `unsafe fn` declaration must have a `// SAFETY:` proof comment within 5 lines preceding it. |
| `test-density-aggregate` | error | yes | Aggregate `(tests / public_fns)` ratio across the workspace must be ≥ threshold (doctrine default: 4.0). |
| `test-density-per-fn` | error | yes | Every individual `pub fn` has at least one paired test. |

Plus the standard cargo gates, all hard: `cargo fmt --check`, `cargo clippy
--workspace --all-targets --all-features -- -D warnings -D
clippy::pedantic -D clippy::nursery`, `cargo deny check`, `cargo
nextest run` (or `cargo test`), `cargo doc -D warnings`.

`cargo audit` is **deferred** in v0.1: the Kali self-hosted runner ships
`cargo-audit 0.21.2` (can't parse CVSS:4.0) and `cargo install --locked
cargo-audit` flakes on rustup rust-src corruption. `cargo deny check
advisories` covers the same RUSTSEC database. Re-enable in v0.2 when the
runner image bumps.

---

## Ratchet model

Every gate except `debug-remove` accepts a per-repo override via
`avp-ratchet.toml` at the repo root. The schema is canonical and lives in
`avp_core::ratchet::RatchetEntry`. Five required fields per entry:

```toml
[[overrides]]
gate          = "bug-assumption"          # required, kebab-case GateId
crate         = "engine-fs"               # optional, scope to one Cargo crate
file          = "src/legacy/.*"           # optional, Rust regex over repo paths
reason        = "Pending RFC-0007."       # required, free text
signed_by     = "william@plausiden.com"   # required, accountable human
opened        = "2026-04-30"              # required, ISO date
expires_after = "2026-06-30"              # required, ISO date; hard
```

Behaviour:

- **Unexpired & matching** → gate skipped, `::notice::` emitted.
- **Within 14 days of expiry** → `::warning::` emitted on every run.
- **Expired** → `::error::` emitted, build fails. The override is a
  forcing function, not a permanent bypass.

Non-ratchetable gates (`debug-remove`) reject `[[overrides]]` entries at
load time. Typos in `gate`, `signed_by`, dates, or regexes fail loudly —
ratchets are a privileged escape hatch and the parser is strict on
purpose.

---

## Self-hosted runner reality

All siblings target `[self-hosted, linux, x64, plausiden]`. The runner is
Kali-based and has known package-version drift. Pinned binaries are the
supersociety play: every CI workflow downloads `avp` from the doctrine
repo's GitHub release, verifies a blake3 hash, then execs it. No
`cargo install` at workflow time, no apt dependency on the runner, no
flakes from rustup rust-src corruption.

The composite GitHub Action (`.github/actions/avp-rust-gate/action.yml`,
arriving in task #17) is just:

```yaml
runs:
  using: composite
  steps:
    - name: Install avp
      shell: bash
      run: ${{ github.action_path }}/install-avp.sh ${{ inputs.version }}
    - name: avp check rust
      shell: bash
      run: avp check rust --strictness ${{ inputs.strictness }}
```

`PlausiDen-AVP-Doctrine` is private, so siblings consuming the action need
**Settings → Actions → General → Access → "Accessible from repositories
owned by the user"** flipped on.

---

## Multi-instance coordination (`avp intent`)

> When N agents (or N humans) work on one repo at once, they need to
> declare intent up front and the toolchain must catch overlap before
> merge.

`.avp-intent.toml` at branch root, written by every instance before it
touches anything:

```toml
agent_id      = "claude-2026-04-30-T01"
branch        = "claude/feat-foo-2026-04-30"
goal          = "Wire avp into PlausiDen-Engine CI"
success_test  = "cargo run -p avp -- check rust --workspace"
declared_files = [
    "PlausiDen-Engine/.github/workflows/avp.yml",
    "PlausiDen-Engine/avp-ratchet.toml",
    "PlausiDen-Engine/deny.toml",
]
opened_at     = "2026-04-30T20:00:00Z"
```

`avp intent overlap` scans all open intents and flags two branches
declaring the same `declared_files`. `avp intent verify` runs
post-hoc against a branch's actual diff and **fails any branch whose
touched-files exceed its manifest** — that turns "I lied about my scope"
into a CI failure instead of a 3am merge mess.

---

## Conductor (component 2)

Long-running Rust daemon. Spawns `claude` CLI sessions per worktree, each
with a curated `.claude/settings.json` so 90 % of permission prompts simply
don't happen. FSM with explicit states:

```
queued → running → paused-{permission,rate,context,blocked} → resumed → done | failed
```

Recovery policies:

- `paused-rate` / `paused-context` → exponential backoff + `claude --continue`.
- `paused-permission` → only if the prompt matches an allowlisted op
  (configured per repo via `.claude/settings.json`). Else escalate.
- `paused-blocked` → escalate to GitHub issue with full transcript.

**No `--dangerously-skip-permissions` anywhere.** Bypass-everything is
exactly what the doctrine forbids. Configuration solves the legitimate
prompts; escalation handles the rest.

---

## Phased delivery

| Phase | What ships | Components | When |
|---|---|---|---|
| **0.1** | `avp` binary with `check rust` working end-to-end on PlausiDen-Engine; ratchet schema; canonical configs; thin composite action; drift-check. Eats own dogfood (`avp` lints itself). | 1 | next |
| **0.2** | `avp check ts` + `avp check py` (shells to `biome`/`ruff`); `avp install` rolled out to ≥10 siblings; `cargo audit` re-enabled when runner bumps; pedantic/nursery flipped on per-sibling. | 1 | after pilot |
| **0.3** | `avp intent` lands; conductor scaffold compiles and supervises a single worktree. | 1, 2, 3 | |
| **0.4** | conductor supervises N worktrees; cross-repo merge order driven by AVP. | 2, 3 | |
| **1.0** | All ~80 siblings on `avp`; drift-check at zero; conductor in production; portfolio-wide `cargo mutants` and `cargo fuzz` campaigns running nightly. | 1, 2, 3 | |
| **post-1.0** | Annotator + Backend Loom + Backend CMS. | 4, 5, 6 | |

Tags are signed (`git tag -s`). Releases publish the `avp` binary +
blake3 manifest. Floating tags (`@v0`) are deliberately not provided —
supersociety doctrine forbids auto-updating dependencies.

---

## Threat model

The toolchain is itself a high-value target: compromising `avp` means
compromising the gate that protects ~80 sibling repos. Specific risks and
mitigations:

- **Supply chain** → `cargo deny` allowlists licenses + sources;
  `cargo audit` (when re-enabled); `cargo vet` (post-1.0); reproducible
  Nix-flake builds; blake3-pinned binary install in CI.
- **Tampered binary** → release artifacts signed; `install-avp.sh`
  rejects mismatched blake3.
- **Doctrine repo private** → action sharing toggle gates access; PAT
  scope is least-privilege.
- **Ratchet abuse** → strict TOML parser, mandatory `signed_by`, hard
  expiry, post-expiry `::error::`. No silent disables.
- **Conductor drives N Claude Code subprocesses** → each session has a
  per-repo `.claude/settings.json` with explicit allow/deny; no
  blanket bypass; destructive ops always escalate.

---

## How to resume this work

1. Pull `claude/avp-meta-runner-v0.1.0` in `PlausiDen-AVP-Doctrine`.
2. `cd avp && cargo build && cargo test` — should be 41 tests green.
3. Read `avp/README.md` for the current dev surface.
4. Read this doc top-to-bottom for the architecture.
5. Pick the next pending task from the project tracker (or
   `TaskList`-equivalent in your tooling). Tasks are numbered and have
   descriptions; #14 is the natural next one (`avp check rust` syn-based
   gate implementation).
6. Honor the supersociety stack mandate and the AVP-2 protocol on every
   change. **The verdict is always STILL BROKEN.** Shipping is explicit
   risk acceptance.

---

## Cross-references

- `AVP2_PROTOCOL.md` — the doctrine; this toolchain enforces it.
- `SUPERSOCIETY_STACK.md` — stack mandate; this toolchain is built to it.
- `gates/{rust,typescript,python,frontend}.md` — per-language gate spec.
- `cross-repo/` — crossfix protocol the conductor will eventually drive.
- `ipc/`, `loops/`, `standing-orders/` — existing agent infra the
  conductor reuses.
- `PlausiDen-Audits/` — sibling repo: 76-audit catalog the gates link to.
