# ARCHITECTURE-md-template.md — Out-of-Scope pattern for every repo's ARCHITECTURE.md

Every PlausiDen-managed repo (or every repo with non-trivial
architecture) carries an `ARCHITECTURE.md` describing what's in the
codebase, the major design decisions, and the seams between modules.

This template adds **one section that's missing in most**: an explicit
**Out of Scope** block. Without it, agents and contributors keep
re-asking the same questions ("does this repo handle X?", "should this
PR add feature Y?") and re-deriving the same answer from absence.

The Out-of-Scope block makes the ABSENCE of features explicit, so a
reader at first glance knows what NOT to PR.

## The pattern (required section, near the top)

```markdown
## Out of Scope

This crate does **not** handle:

- **{{ specific feature }}** — owned by `{{ sibling crate }}`. {{ link to that crate's ARCHITECTURE.md if applicable }}
- **{{ specific feature }}** — deferred indefinitely; tracked at #{{ issue number }} but not on the roadmap.
- **{{ specific feature }}** — explicitly rejected. Reason: {{ one-sentence why this would be wrong here, e.g. "binds the wire format to a network protocol we don't control" }}.

Adding any of the above to this crate is a doctrine violation,
not a missing feature. If you need it, work in the listed sibling
or open a doctrine RFC first.
```

## Why "Out of Scope" specifically

Most ARCHITECTURE.md files describe what IS in the codebase. The
gap is what ISN'T. Common consequences of the gap:

- A new contributor proposes a PR that adds feature X here when X
  belongs in sibling crate Y. Reviewer has to explain the seam in
  PR comments instead of pointing at a documented decision.
- An AI agent doing autonomous work picks the WRONG layer to add a
  feature to (because the ARCHITECTURE.md describes layer A in
  detail and is silent on layer B's existence), and the reviewer
  has to back it out.
- A new auditor or carrier reviewer asks "does this codebase touch
  PII / ML / network / payments?" and the answer requires reading
  the source rather than reading the architecture doc.

An explicit Out-of-Scope block answers all three at first read.

## Examples of well-formed Out-of-Scope items

```markdown
## Out of Scope

- **Network I/O.** This crate is library-only and pure. Network
  consumers live in `engine-pipeline` (artifact persistence) and
  `plausiden-inject` (OS-level write). A new network call here is
  a doctrine violation — open an issue against the right consumer
  instead.
- **Cryptographic primitives.** We use audited crates only
  (`ring`, `ed25519-dalek`, `chacha20poly1305`). Adding a custom
  cipher, hash, or KDF here is rejected on principle; even
  "small" ones widen the audit surface beyond what we can
  responsibly maintain.
- **Telemetry / metrics.** This crate emits `tracing` events at
  defined levels but never aggregates, never sends, never
  persists. Aggregation is `engine-obs`'s job; persistence is
  `engine-pipeline`'s.
- **WASM-incompatible APIs.** Every public API must compile to
  `wasm32-unknown-unknown`. The browser extension consumes us as
  WASM; breaking that is a P0.
```

## Discipline

- The section heading is `## Out of Scope` (capital O, capital S,
  no trailing colon). Grep target.
- It belongs near the top of ARCHITECTURE.md — after the project
  identity / mission, before the crate map / module breakdown.
- Each item is a **noun phrase** + **owner / reason** + (optional)
  **why-not-here clause**. Resist vague items like "advanced
  features" — be specific.
- When a sibling crate gains the feature you've listed as
  out-of-scope, update the link in the Out-of-Scope item; don't
  remove the entry. The historical "this isn't here, here's where
  it lives" answer remains useful.

## Ratchet

When the architecture genuinely shifts (a feature graduates IN to
this crate from a sibling), update the Out-of-Scope block in the
same PR that does the move. A future `out-of-scope-staleness`
audit may sample architecture files and check for items that have
since become in-scope.

## Companion file

See `examples/CLAUDE-md-template.md` for the framing block that
should head every CLAUDE.md. The two templates are designed to
work together: CLAUDE.md tells the agent how to behave;
ARCHITECTURE.md tells the agent what the codebase does (and
doesn't do).
