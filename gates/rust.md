# Rust Coding Gates

Mandatory on every Rust crate in the PlausiDen ecosystem.

## Crate-level

- `#![forbid(unsafe_code)]` at the crate root unless `unsafe` is genuinely
  required. If required, narrow to the smallest module possible.
- `#![deny(missing_docs)]` on every public module.
- `#![warn(clippy::all, clippy::pedantic)]` at minimum; `clippy::nursery`
  encouraged.
- Edition `2021` minimum, `2024` preferred. Note: `gen` is a reserved
  keyword in 2024.

## Functions

- Every public function has a `///` doc comment.
- Every public function returning `Result` / `Option` is `#[must_use]`.
- Every public function carries a `BUG ASSUMPTION:` comment above it.
- Every public function has at least one test case.

## Error handling

- `thiserror` for library errors; `anyhow` only at binary boundaries.
- No `unwrap()` / `expect()` in non-test code without an inline `// SAFETY:`
  or `// test-only` justification.
- No `panic!()` / `todo!()` / `unimplemented!()` in code paths reachable at
  runtime.

## Types

- Every `String` parameter that has constraints becomes a newtype.
- Every `usize`/`i32` that has a valid range becomes a newtype.
- Every `bool` parameter becomes an enum (no boolean blindness).
- Every public enum is `#[non_exhaustive]` when the variant set may grow.

## Concurrency

- Every shared mutable state has documented locking discipline.
- Every `async` boundary is cancellation-safe.
- Tests run under `--test-threads=1` AND under max parallelism.
- Critical sections audited under `miri` and (where applicable) `loom`.

## Dependencies

- `Cargo.toml` declares `[workspace.dependencies]` for shared deps; sub-crates
  inherit.
- No new dep without `cargo audit` clean and `cargo geiger` reviewed.
- `Cargo.lock` committed for binary crates, omitted for libraries.
- All crypto from audited libraries (`ring`, `ed25519-dalek`,
  `chacha20poly1305`, `subtle`, `rustcrypto`).

## Logging

- `tracing` for structured logs. No ad-hoc `println!` / `eprintln!` /
  `dbg!` in library code.
- ERROR for actionable failures, WARN for degraded state, INFO for
  significant events, DEBUG/TRACE stripped or gated in release.

## Build / lint / test

- `cargo fmt --all -- --check` clean.
- `cargo clippy --workspace -- -D warnings` clean.
- `cargo test --workspace` green.
- `cargo doc --workspace --no-deps` clean.
- Where applicable: `cargo check --target wasm32-unknown-unknown` clean.

## Annotations

- `BUG ASSUMPTION:` on every public fn.
- `SAFETY:` on every `unsafe` block.
- `SECURITY:` on every defense-in-depth.
- `FOSS-ABSORBED:` on every vendored crate.

See [`../annotations/README.md`](../annotations/README.md) for the full set.
