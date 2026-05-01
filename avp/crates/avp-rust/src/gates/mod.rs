//! Concrete gate implementations for the Rust language.
//!
//! Each submodule provides a struct that implements [`avp_core::Gate`].
//! `super::all_gates` returns one boxed instance of every implemented gate
//! so the CLI can iterate uniformly.

pub mod bug_assumption;
pub mod debug_remove;
pub mod forbidden_call;
pub mod test_density;
pub mod unsafe_proof;

use avp_core::Gate;

/// Construct one boxed instance of every Rust gate available in this build.
///
/// Order matches the doctrine's gate declaration order — keep it stable so
/// the CI summary lines remain consistent across runs.
#[must_use]
pub fn all_gates() -> Vec<Box<dyn Gate>> {
    vec![
        Box::new(bug_assumption::BugAssumption::default()),
        Box::new(forbidden_call::ForbiddenCall),
        Box::new(debug_remove::DebugRemove),
        Box::new(unsafe_proof::UnsafeProof::default()),
        Box::new(test_density::TestDensityAggregate::default()),
    ]
}
