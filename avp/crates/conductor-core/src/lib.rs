//! # conductor-core
//!
//! Types and traits for the conductor — the long-running supervisor that
//! drives N Claude Code subprocesses across PlausiDen worktrees.
//!
//! ## Architecture
//!
//! - [`session::Session`] — one work unit (one `.avp-intent.toml`,
//!   one Claude Code subprocess, one branch).
//! - [`session::SessionState`] — explicit FSM with named pause variants;
//!   no implicit "stuck" state, no silent retries.
//! - [`driver::ClaudeDriver`] — the boundary between conductor and the
//!   actual `claude` CLI. The real impl shells to `claude --print
//!   --output-format=json` (in `conductor` binary); the test impl
//!   [`driver::MockDriver`] simulates the protocol deterministically.
//! - [`supervisor::Supervisor`] — owns a `Vec<Session>`, steps each
//!   session per the policy in [`policy::RecoveryPolicy`], and emits
//!   [`event::SupervisorEvent`]s for the CLI / external observer to
//!   render.
//!
//! ## Doctrine
//!
//! Per `cross-repo/multi-instance.md`, every Claude Code session
//! launched by the conductor:
//!
//! 1. Has a per-repo `.claude/settings.json` curated so 90 % of
//!    permission prompts simply don't fire (read-ops + routine edits
//!    allowlisted; destructive ops gated).
//! 2. Owns exactly one branch's `.avp-intent.toml` worth of work.
//! 3. Pauses for one of four named reasons (rate, context, permission,
//!    blocked) — each with a deterministic recovery policy.
//! 4. Escalates genuine blockers via PR comment / GH issue, never via
//!    silent retry.
//!
//! `--dangerously-skip-permissions` is forbidden. The doctrine answer
//! is "configure correctly," not "bypass."

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

pub mod driver;
pub mod event;
pub mod host;
pub mod policy;
pub mod session;
pub mod supervisor;

pub use driver::{ClaudeDriver, DriverError, DriverEvent, MockDriver, SessionHandle};
pub use event::{SupervisorEvent, SupervisorEventKind};
pub use host::{Host, HostError, SshTarget};
pub use policy::{PauseReason, RecoveryAction, RecoveryPolicy};
pub use session::{Session, SessionId, SessionState};
pub use supervisor::{Supervisor, SupervisorError};

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod smoke {
    use super::*;

    #[test]
    fn version_populated() {
        assert!(!VERSION.is_empty());
    }
}
