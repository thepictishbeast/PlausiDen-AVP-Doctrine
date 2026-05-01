//! # avp-rust
//!
//! Rust-language gate implementations for the AVP-2 supersociety toolchain.
//! Each gate is a `Box<dyn Gate>` consumer of [`avp_core`]'s shared types.
//!
//! ## Gates landed in this crate
//!
//! - [`gates::debug_remove::DebugRemove`] — fail on any `DEBUG-REMOVE:`
//!   marker. Pure text scan, no AST.
//! - [`gates::bug_assumption::BugAssumption`] — every `pub fn` must have a
//!   `BUG ASSUMPTION:` comment in the 20-line window preceding its
//!   signature. AST-based pub-fn detection (no false positives on
//!   commented-out fns or macro-expanded code).
//!
//! ## Gates landing next (tracked tasks)
//!
//! - `forbidden-call` — `.unwrap()`, `panic!()`, `dbg!()`, etc., with
//!   justification comment overrides.
//! - `unsafe-proof` — every `unsafe { … }` and `unsafe fn` paired with a
//!   `// SAFETY:` comment.
//! - `test-density-aggregate` — workspace-level `tests / public_fns`
//!   ratio gate.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

pub mod gates;
pub mod source;

pub use source::{RustSource, SourceClass, SourceError};

/// Crate version, sourced from `Cargo.toml` at compile time.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod smoke {
    use super::*;

    #[test]
    fn version_is_populated() {
        assert!(!VERSION.is_empty());
    }
}
