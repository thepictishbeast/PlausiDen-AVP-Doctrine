# FOSS Absorption Protocol

> Never use a third-party tool as-is. Every tool is raw material.

When the project needs a capability and a FOSS tool exists, the agent runs
this 6-step protocol. Skipping a step is a `SHIP-DECISION:` requiring an
explicit waiver.

## 1. Discover

Search across crates.io / GitHub / GitLab / Codeberg.

Prefer:
- Rust-native (or your project's primary language)
- Minimal transitive dependencies
- Active maintenance (commits in the last 6 months)
- Compatible licence (see [`../gates/README.md`](../gates/README.md) license rules)

Reject:
- Single-maintainer abandoned projects
- Heavy `unsafe` use without `// SAFETY:` proofs
- Licences incompatible with the consuming repo

Document the discovery search in the absorbed crate's `VENDORING.md`.

## 2. Evaluate

For each candidate:

```
cargo geiger              # how much unsafe is in the dep tree?
cargo tree -p <crate>     # how many transitive deps, and which?
cargo audit               # any known CVEs in the tree?
```

Read the source:
- Read every line if the crate is < 5 000 LoC.
- Read the key modules + the test suite if larger.
- Skim the issue tracker for unresolved security or correctness issues.
- Check CVE history.

Decide: **absorb**, **adapt**, or **write from scratch**.

## 3. Absorb

Never use as-is. Choose:

- **Vendor** (copy into `vendor/<crate>/`) — for stable, well-bounded deps
  you don't expect to track upstream changes for.
- **Hard fork** (separate repo with PlausiDen prefix) — for deps you'll
  evolve independently.

Either way:

- Strip features you don't use (attack-surface reduction).
- Replace every `unwrap`/`expect` with proper error handling — or annotate
  with `// SAFETY:` proof.
- Replace every `unsafe` block where possible. For the rest, write the
  `// SAFETY:` proof.
- Add bounds checking on every public input.
- Add input validation at every public API surface.
- Add structured logging where the upstream had `println!` / `eprintln!`.
- Add the tests it should have had — aim for the project's test ratio
  (≥ 4 tests per public fn).

## 4. Integrate

Wrap the absorbed code behind a thin adapter so the rest of the project
doesn't import it directly. Dependency inversion — swappable later.

```rust
// crates/our-foo/src/lib.rs — public API the rest of the project uses.
pub trait Foo { fn do_thing(&self, x: u32) -> Result<u32, Error>; }

// crates/our-foo/src/adapters/upstream.rs — the only place the absorbed
// crate is imported.
impl Foo for UpstreamFoo { ... }
```

## 5. Loop AVP

Run AVP-2 Tiers 1-3 (minimum 12 passes) on the absorbed code. It is now
ours and inherits our paranoia.

## 6. Maintain

For every upstream release:

1. Diff against our fork.
2. Cherry-pick security fixes.
3. Ignore feature churn that doesn't apply.
4. Re-run Tier 3 on every absorbed merge.

If upstream dies, our fork is canonical. Document that in `VENDORING.md`.

## Annotations

Every absorbed crate carries a `FOSS-ABSORBED:` header in its `VENDORING.md`:

```
FOSS-ABSORBED: <crate-name> <version-or-commit> <reason for vendoring>
```

The first commit that introduces an absorbed crate references the protocol
step it's at:

```
chore(vendor): absorb foo-rs v1.2.3 — Step 3 (vendored, hardened)

FOSS-ABSORBED: foo-rs v1.2.3 — needed bounds-checked parser variant.
Stripped: feature 'serde-json' (we use bincode), feature 'std' (we're no_std).
Hardened: replaced 4 unwraps with Result, added depth limit (8) to parser,
zeroize on key buffer.
```
