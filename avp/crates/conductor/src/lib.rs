//! `conductor` — long-running supervisor for parallel Claude Code
//! sessions across PlausiDen worktrees.
//!
//! The binary's `main` lives in `bin/conductor.rs` (via `[[bin]]` in
//! Cargo.toml's `name = "conductor"` entry, path `src/main.rs`).
//! This `lib.rs` re-exports the modules so integration tests in
//! `tests/` can drive the supervisor + drivers without spawning the
//! binary.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![allow(clippy::disallowed_macros)]
#![allow(clippy::redundant_pub_crate)]

pub mod claude_event;
pub mod driver_local;
pub mod driver_ssh;
