# Inline Annotation Standard

Every annotation is machine-grepable for CI, audit, and historical analysis.
Use the exact prefix (caps, colon) so a global `rg 'PREFIX:'` finds every
instance.

## The annotations

| Prefix | When |
|--------|------|
| `BUG ASSUMPTION:` | What could go wrong in this block. Required on every public function. |
| `AVP-PASS-N:` | A finding from AVP-2 pass N (1-36). Include date and resolution. |
| `SAFETY:` | Proof that an `unsafe` block is sound. Required on every `unsafe` block. |
| `SECURITY:` | Threat being mitigated and how. Use on every defense-in-depth measure. |
| `UX-DEBT:` | Manual UX verification required; risk if skipped. Resolved in `ux` audit. |
| `REGRESSION-GUARD:` | Why a fix exists, what broke before. Required on every regression test. |
| `FOSS-ABSORBED:` | Crate name, version, reason for vendoring. Required on every absorbed dep. |
| `SUPERSOCIETY:` | Defense-in-depth measure beyond standard practice. Documents the layer. |
| `DEBUG-REMOVE:` | Line to be stripped before release. Caught in `debug-logs` audit. |
| `SHIP-DECISION:` | Date + accepted residual risks + developer name. Required on every ship verdict. |
| `CROSSFIX:` | Source repo + description of ported fix. Required on every AVP-CROSSFIX commit. |
| `LEAK-JUSTIFIED:` | Designed-exception site for synthetic-TLD leak filter. Used in `data-leak` audit. |

## Usage

```rust
// BUG ASSUMPTION: caller may pass a path with `..` traversal; we canonicalize first.
// SAFETY: NonNull guaranteed by caller contract; checked at the entry point above.
// SECURITY: timing-safe comparison via subtle; no early return on mismatch.
// AVP-PASS-13: 2026-04-19 — added depth limit (5) to prevent JSON nesting bomb.
// SHIP-DECISION: 2026-04-30 — accepted: rate limit not yet enforced on health endpoint.
//   Residual risk: low (read-only, no state change). Owner: tian.
```

```typescript
// UX-DEBT: keyboard focus on modal close button has not been screen-reader tested.
// REGRESSION-GUARD: previously the date picker rendered locale 'en' regardless of system locale (issue #142).
```

## Discipline

- Every public function: `BUG ASSUMPTION:` is mandatory.
- Every `unsafe` block: `SAFETY:` proof is mandatory.
- Every absorbed crate: `FOSS-ABSORBED:` header in the vendor directory.
- Every ship: `SHIP-DECISION:` annotation in the release notes (and grepable
  somewhere in the repo, typically in CHANGELOG or RELEASE_NOTES).

## Audit interaction

The annotations are how the audits know what's covered and what's waived.
A `SHIP-DECISION:` is the only way to ship past a failing audit, and it
binds the agent that wrote it (or the human if the agent escalated and the
human signed).

## Adding a new annotation

1. Propose the new annotation prefix and its grep usage.
2. Update this file with the row.
3. Add it to the relevant audit's checklist if it gates that audit.
4. Open a PR and bump the doctrine version.
