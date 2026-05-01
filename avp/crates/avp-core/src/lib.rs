//! # avp-core
//!
//! Core types and traits for the AVP-2 supersociety toolchain. This crate is
//! deliberately language-agnostic at the top level — language-specific gate
//! logic lives in sibling crates (`avp-rust`, `avp-ts`, `avp-py`) that
//! consume these types.
//!
//! ## Architectural overview
//!
//! - [`gate::GateId`] — every check is identified by a stable kebab-case ID.
//! - [`gate::Gate`] — trait every check implements. `run` returns a stream of
//!   [`finding::Finding`]s.
//! - [`ratchet::RatchetFile`] — the per-repo `avp-ratchet.toml` overrides
//!   with a TTL, expressing "this gate is temporarily allowed to fail in
//!   this scope, signed by this human, until this date."
//! - [`reporter::Reporter`] — pluggable output formatters (GitHub Actions
//!   annotations, JSON for editors, human-readable terminal output).
//! - [`newtype`] — domain primitives that prevent type confusion at API
//!   boundaries (per AVP-2 §6: "every `String` parameter that has constraints
//!   becomes a newtype").
//!
//! ## SECURITY: forbid(unsafe_code)
//!
//! This crate is `#![forbid(unsafe_code)]` at the crate root. There are no
//! `unsafe` blocks anywhere; if the static analysis ever reports one, the
//! supply chain has been compromised.
//!
//! ## BUG ASSUMPTION
//!
//! The ratchet TOML may be missing (treat as no overrides), malformed (fail
//! loudly so a typo can't silently disable a gate), or contain entries with
//! bad dates (also fail loudly). All public API of this crate is fallible
//! and returns concrete error enums — no panics in non-test code.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

pub mod finding;
pub mod gate;
pub mod newtype;
pub mod ratchet;
pub mod repo;
pub mod reporter;

pub use finding::{Finding, Location, Severity};
pub use gate::{Context, Gate, GateId};
pub use newtype::{CrateName, RepoRelativePath, SignerEmail};
pub use ratchet::{RatchetEntry, RatchetError, RatchetFile};
pub use repo::{RepoLanguage, RepoRoot};
pub use reporter::{GithubActionsReporter, HumanReporter, JsonReporter, Reporter};

/// Crate version, sourced from `Cargo.toml` at compile time.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Canonical name of the per-repo ratchet file.
pub const RATCHET_FILE: &str = "avp-ratchet.toml";

/// Canonical name of the per-repo intent file (multi-instance coordination).
pub const INTENT_FILE: &str = ".avp-intent.toml";

/// Canonical workflow path siblings install via `avp install`.
pub const CANONICAL_WORKFLOW: &str = ".github/workflows/avp.yml";

#[cfg(test)]
mod smoke {
    use super::*;

    #[test]
    fn version_is_populated() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn canonical_paths_are_stable() {
        assert_eq!(RATCHET_FILE, "avp-ratchet.toml");
        assert_eq!(INTENT_FILE, ".avp-intent.toml");
        assert_eq!(CANONICAL_WORKFLOW, ".github/workflows/avp.yml");
    }
}
