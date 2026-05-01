//! `Session` + `SessionState` FSM.
//!
//! A session represents one Claude Code subprocess working on one
//! branch's `.avp-intent.toml`. The state machine is intentionally
//! explicit — every pause has a name, every transition is logged, no
//! "stuck" state without diagnosis.

use std::path::PathBuf;

use avp_core::IntentFile;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::policy::PauseReason;

/// Stable identifier for one session. Convention: agent_id from the
/// intent. We don't use a separate uuid because a session is 1:1 with
/// an intent, and the intent's agent_id is already unique.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionId(pub String);

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&avp_core::AgentId> for SessionId {
    fn from(a: &avp_core::AgentId) -> Self {
        Self(a.as_str().to_owned())
    }
}

/// Explicit session state machine.
///
/// Valid transitions:
///
/// ```text
/// Queued → Running
/// Running → Paused(reason) | Done | Failed
/// Paused(reason) → Resumed → Running
/// Paused(reason) → Failed     (when policy says "escalate")
/// ```
///
/// `#[non_exhaustive]` so future pause reasons can be added without
/// breaking match arms in consumers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SessionState {
    /// Not yet started.
    Queued,
    /// Subprocess running.
    Running,
    /// Paused for a named reason; recovery policy decides next step.
    Paused(PauseReason),
    /// Resumed from a paused state; transient — the next `step()`
    /// transitions back to `Running` (or directly to `Paused`/`Failed`).
    Resumed,
    /// Successfully completed (subprocess exited 0, success_test passed).
    Done,
    /// Failed (subprocess exited non-zero, escalated, or
    /// `verify` against the intent diff failed).
    Failed {
        /// Human-readable reason.
        reason: String,
    },
}

impl SessionState {
    /// True if the FSM has reached a terminal state.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Done | Self::Failed { .. })
    }

    /// True if the session is currently waiting to retry/resume.
    #[must_use]
    pub const fn is_paused(&self) -> bool {
        matches!(self, Self::Paused(_))
    }

    /// True if the session is actively executing.
    #[must_use]
    pub const fn is_running(&self) -> bool {
        matches!(self, Self::Running)
    }

    /// Stable label for logging/serialization.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Paused(_) => "paused",
            Self::Resumed => "resumed",
            Self::Done => "done",
            Self::Failed { .. } => "failed",
        }
    }
}

/// One conductor-managed work unit.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Session {
    /// Stable identifier (= intent.agent_id).
    pub id: SessionId,
    /// The intent this session executes.
    pub intent: IntentFile,
    /// Worktree path on disk (where the conductor will spawn `claude`).
    pub worktree: PathBuf,
    /// Current FSM state.
    pub state: SessionState,
    /// When the session entered the current state (UTC).
    pub state_since: OffsetDateTime,
    /// Number of times the session has been resumed from a paused state.
    pub resume_count: u32,
    /// Last subprocess transcript line (for diagnostics).
    pub last_log_line: Option<String>,
}

impl Session {
    /// Construct a fresh session in the `Queued` state.
    #[must_use]
    pub fn new(intent: IntentFile, worktree: PathBuf) -> Self {
        let id = SessionId::from(&intent.agent_id);
        Self {
            id,
            intent,
            worktree,
            state: SessionState::Queued,
            state_since: OffsetDateTime::now_utc(),
            resume_count: 0,
            last_log_line: None,
        }
    }

    /// Move the FSM to a new state, stamping `state_since` to now.
    /// Not `const fn`: `OffsetDateTime::now_utc()` reads the OS clock.
    #[allow(clippy::missing_const_for_fn)]
    pub fn transition(&mut self, new: SessionState) {
        self.state = new;
        self.state_since = OffsetDateTime::now_utc();
    }

    /// Increment the resume counter (used by the recovery policy to
    /// decide when to escalate after too many retries).
    pub const fn record_resume(&mut self) {
        self.resume_count = self.resume_count.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::PauseReason;

    fn intent() -> IntentFile {
        let toml = r#"
            agent_id = "claude-test"
            branch = "claude/test"
            goal = "g"
            success_test = "true"
            opened_at = "2026-05-01T00:00:00Z"
            declared_files = []
        "#;
        IntentFile::from_toml(toml).unwrap()
    }

    #[test]
    fn fresh_session_is_queued() {
        let s = Session::new(intent(), PathBuf::from("/tmp/wt"));
        assert!(matches!(s.state, SessionState::Queued));
        assert_eq!(s.resume_count, 0);
    }

    #[test]
    fn label_is_stable() {
        assert_eq!(SessionState::Queued.label(), "queued");
        assert_eq!(SessionState::Running.label(), "running");
        assert_eq!(
            SessionState::Paused(PauseReason::RateLimit).label(),
            "paused"
        );
        assert_eq!(SessionState::Resumed.label(), "resumed");
        assert_eq!(SessionState::Done.label(), "done");
        assert_eq!(
            SessionState::Failed { reason: "x".into() }.label(),
            "failed"
        );
    }

    #[test]
    fn terminal_states() {
        assert!(SessionState::Done.is_terminal());
        assert!(SessionState::Failed { reason: "x".into() }.is_terminal());
        assert!(!SessionState::Queued.is_terminal());
        assert!(!SessionState::Running.is_terminal());
        assert!(!SessionState::Paused(PauseReason::Network).is_terminal());
    }

    #[test]
    fn id_round_trips_via_serde() {
        let id = SessionId("claude-test".into());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"claude-test\"");
        let back: SessionId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn transition_updates_state_and_stamp() {
        let mut s = Session::new(intent(), PathBuf::from("/tmp/wt"));
        let before = s.state_since;
        std::thread::sleep(std::time::Duration::from_millis(2));
        s.transition(SessionState::Running);
        assert!(matches!(s.state, SessionState::Running));
        assert!(s.state_since > before);
    }

    #[test]
    fn resume_counter_saturates() {
        let mut s = Session::new(intent(), PathBuf::from("/tmp/wt"));
        s.resume_count = u32::MAX;
        s.record_resume();
        assert_eq!(s.resume_count, u32::MAX);
    }
}
