//! Gate identity, taxonomy, and the `Gate` trait.
//!
//! Every check the toolchain runs is identified by a stable, kebab-case
//! [`GateId`]. Adding a new gate means adding a variant here, implementing
//! the [`Gate`] trait in a language-specific crate, and updating
//! `avp gate list` (which iterates `GateId::ALL`).

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{finding::Finding, newtype::CrateName, repo::RepoRoot};

/// Stable kebab-case identifier for every gate the toolchain enforces.
///
/// AVP-2 §SHIP-DECISION: this enum is `#[non_exhaustive]` so future doctrine
/// updates can add gates without it being a breaking change for consumers
/// that match on it. Match arms must include `_ => …`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
pub enum GateId {
    /// Every public fn must have a `BUG ASSUMPTION:` comment within 20 lines
    /// preceding its signature. Doctrine: AVP-2 §Annotations.
    BugAssumption,
    /// Forbidden calls in library code: `unwrap`, `expect`, `panic!`,
    /// `todo!`, `unimplemented!`, `dbg!`, `println!`, `eprintln!`. Allowed
    /// when the line carries a `// SAFETY:`, `// test-only`, `// AVP-PASS-`,
    /// `// SHIP-DECISION:`, or `// FOSS-ABSORBED:` justification.
    ForbiddenCall,
    /// Any tracked file containing `DEBUG-REMOVE:` literal fails the build.
    /// **Not ratchetable** — this is the doctrine's pre-release lint.
    DebugRemove,
    /// Every `unsafe { ... }` block and every `unsafe fn` declaration must
    /// have a `// SAFETY:` proof comment within the 5 lines preceding it.
    UnsafeProof,
    /// Aggregate `tests / public_fns` ratio across the workspace must be
    /// ≥ the configured threshold (doctrine default: 4.0).
    TestDensityAggregate,
    /// Every individual public fn must have at least one paired test.
    /// (Per-fn density; complementary to the aggregate gate.)
    TestDensityPerFn,
}

impl GateId {
    /// Every defined gate, in stable order. Iterate this for introspection
    /// (`avp gate list`, drift-check, completeness tests).
    pub const ALL: &'static [Self] = &[
        Self::BugAssumption,
        Self::ForbiddenCall,
        Self::DebugRemove,
        Self::UnsafeProof,
        Self::TestDensityAggregate,
        Self::TestDensityPerFn,
    ];

    /// Whether the gate accepts ratchet overrides at all. The
    /// [`Self::DebugRemove`] gate is the lone non-ratchetable hard gate;
    /// every other gate can be temporarily overridden via
    /// `avp-ratchet.toml`.
    #[must_use]
    pub const fn ratchetable(self) -> bool {
        !matches!(self, Self::DebugRemove)
    }

    /// One-line human description, used by `avp gate list` and `avp explain`.
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::BugAssumption => "Public fns must have a `BUG ASSUMPTION:` annotation.",
            Self::ForbiddenCall => {
                "Forbidden calls in library code (unwrap/expect/panic/dbg/println)."
            }
            Self::DebugRemove => "DEBUG-REMOVE markers must be stripped before merge.",
            Self::UnsafeProof => "Every `unsafe` site must have a `// SAFETY:` proof comment.",
            Self::TestDensityAggregate => "Aggregate (tests / public_fns) ratio ≥ threshold.",
            Self::TestDensityPerFn => "Every public fn has at least one paired test.",
        }
    }

    /// The stable kebab-case identifier as a `&'static str` — what TOML files
    /// and CI annotations write.
    #[must_use]
    pub const fn as_kebab(self) -> &'static str {
        match self {
            Self::BugAssumption => "bug-assumption",
            Self::ForbiddenCall => "forbidden-call",
            Self::DebugRemove => "debug-remove",
            Self::UnsafeProof => "unsafe-proof",
            Self::TestDensityAggregate => "test-density-aggregate",
            Self::TestDensityPerFn => "test-density-per-fn",
        }
    }
}

impl fmt::Display for GateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_kebab())
    }
}

impl std::str::FromStr for GateId {
    type Err = UnknownGate;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .iter()
            .copied()
            .find(|g| g.as_kebab() == s)
            .ok_or_else(|| UnknownGate(s.to_owned()))
    }
}

/// Error returned by `GateId::from_str` when the name is unknown.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown gate id: {0:?} (known: {known:?})", known = GateId::ALL.iter().map(|g| g.as_kebab()).collect::<Vec<_>>())]
pub struct UnknownGate(pub String);

// ─────────────────────────────────────────────────────────────────────────
// Gate trait + Context
// ─────────────────────────────────────────────────────────────────────────

/// Contextual input every [`Gate::run`] receives. Lives long enough that
/// gates can borrow from it freely.
#[derive(Debug)]
#[non_exhaustive]
pub struct Context<'a> {
    /// Resolved repo root.
    pub repo: &'a RepoRoot,
    /// Optional restricted scope: only this crate's sources are inspected.
    pub crate_scope: Option<CrateName>,
    /// Whether the gate is running in CI (controls verbosity / annotation format).
    pub in_ci: bool,
}

impl<'a> Context<'a> {
    /// Construct a context.
    #[must_use]
    pub const fn new(repo: &'a RepoRoot, in_ci: bool) -> Self {
        Self {
            repo,
            crate_scope: None,
            in_ci,
        }
    }

    /// Restrict the gate to a single crate.
    #[must_use]
    pub fn with_crate(mut self, c: CrateName) -> Self {
        self.crate_scope = Some(c);
        self
    }
}

/// A check that can be run against a [`Context`] and returns findings.
/// Implementations live in language-specific crates (`avp-rust`, etc.).
pub trait Gate: Send + Sync + std::fmt::Debug {
    /// Stable identity of this gate.
    fn id(&self) -> GateId;

    /// Run the gate, returning all findings (empty vec = clean).
    /// Gates do not return Result — operational errors are themselves
    /// findings (with [`crate::finding::Severity::Error`]) so a single
    /// reporter pass surfaces both code violations and infra issues.
    fn run(&self, ctx: &Context<'_>) -> Vec<Finding>;

    /// Human-friendly name for headers / status lines. Default = `Display`.
    fn name(&self) -> String {
        self.id().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_gates_have_unique_kebab() {
        let mut seen = std::collections::HashSet::new();
        for g in GateId::ALL {
            assert!(seen.insert(g.as_kebab()), "duplicate kebab for {g:?}");
        }
    }

    #[test]
    fn all_gates_round_trip() {
        for g in GateId::ALL {
            let s = g.to_string();
            let parsed: GateId = s.parse().expect("known kebab parses");
            assert_eq!(parsed, *g);
        }
    }

    #[test]
    fn unknown_gate_errors() {
        let err = "no-such-gate".parse::<GateId>().unwrap_err();
        assert_eq!(err.0, "no-such-gate");
    }

    #[test]
    fn debug_remove_is_unique_unratchetable() {
        for g in GateId::ALL {
            if matches!(g, GateId::DebugRemove) {
                assert!(!g.ratchetable());
            } else {
                assert!(g.ratchetable(), "{g} should be ratchetable");
            }
        }
    }

    #[test]
    fn descriptions_are_non_empty() {
        for g in GateId::ALL {
            assert!(!g.description().is_empty());
        }
    }

    #[test]
    fn serde_round_trip_kebab() {
        let g = GateId::BugAssumption;
        let json = serde_json::to_string(&g).unwrap();
        assert_eq!(json, "\"bug-assumption\"");
        let back: GateId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, g);
    }
}
