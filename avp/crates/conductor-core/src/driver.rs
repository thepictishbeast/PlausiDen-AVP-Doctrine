//! `ClaudeDriver` trait — the boundary between supervisor and the
//! actual Claude Code subprocess.
//!
//! The real impl lives in the `conductor` binary crate (it shells to
//! `claude --print --output-format=json`). Tests use [`MockDriver`]
//! which simulates the protocol deterministically — no network, no
//! flaky child processes.

use std::{collections::VecDeque, sync::Mutex};

use async_trait::async_trait;
use thiserror::Error;
use tracing::trace;

use crate::{policy::PauseReason, session::SessionId};

/// Opaque handle to a running Claude Code session. The supervisor holds
/// these per [`Session`]; drivers may store implementation-specific
/// state behind it.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SessionHandle {
    /// Stable session id (mirrors `Session::id`).
    pub id: SessionId,
    /// Driver-specific opaque token. The real driver puts a PID +
    /// session resume token here.
    pub token: String,
}

/// One unit of progress observed from a session. Emitted by `poll`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DriverEvent {
    /// Subprocess emitted a log line (stdout or stderr).
    Log {
        /// Line text, stripped of trailing newline.
        line: String,
    },
    /// Session paused for the named reason. The supervisor will consult
    /// `RecoveryPolicy` to decide what to do.
    Paused {
        /// Pause taxonomy.
        reason: PauseReason,
    },
    /// Session completed successfully (subprocess exited 0).
    Done,
    /// Session failed irrecoverably (subprocess exited non-zero, output
    /// schema corrupted, etc.).
    Failed {
        /// Human reason.
        reason: String,
    },
}

/// Interface every Claude Code subprocess driver must implement.
#[async_trait]
pub trait ClaudeDriver: Send + Sync + std::fmt::Debug {
    /// Spawn a new session. The conductor passes the session id, the
    /// worktree directory, and an opaque system prompt (typically
    /// the `.avp-intent.toml` rendered as briefing text).
    async fn start(
        &self,
        id: &SessionId,
        worktree: &std::path::Path,
        system_prompt: &str,
    ) -> Result<SessionHandle, DriverError>;

    /// Drain any pending events for a session. Returns an empty vec
    /// when nothing is available; the supervisor calls this in a loop
    /// with a small sleep.
    async fn poll(&self, handle: &SessionHandle) -> Result<Vec<DriverEvent>, DriverError>;

    /// Resume a paused session (e.g. after a rate-limit backoff).
    /// For `claude` CLI this is `claude --continue <session-id>`.
    async fn resume(&self, handle: &SessionHandle) -> Result<(), DriverError>;

    /// Forcibly terminate a session.
    async fn kill(&self, handle: &SessionHandle) -> Result<(), DriverError>;
}

/// Driver errors.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DriverError {
    /// Subprocess spawn failed.
    #[error("driver spawn failed: {0}")]
    Spawn(String),
    /// I/O error talking to the subprocess.
    #[error("driver I/O: {0}")]
    Io(String),
    /// Session id is unknown to the driver.
    #[error("driver: no such session {0}")]
    UnknownSession(SessionId),
    /// Subprocess produced output the driver couldn't parse.
    #[error("driver: protocol error: {0}")]
    Protocol(String),
}

// ─────────────────────────────────────────────────────────────────────────
// MockDriver
// ─────────────────────────────────────────────────────────────────────────

/// In-memory deterministic driver. Tests script a sequence of events
/// per session id; calls to `poll` drain that sequence in order.
///
/// `kill` and `resume` are recorded for inspection but produce no
/// further events automatically — tests script them too.
#[derive(Debug, Default)]
pub struct MockDriver {
    inner: Mutex<MockState>,
}

#[derive(Debug, Default)]
struct MockState {
    /// Per-session event queues.
    events: std::collections::HashMap<SessionId, VecDeque<DriverEvent>>,
    /// Per-session resume call counts (for assertions).
    resumes: std::collections::HashMap<SessionId, u32>,
    /// Per-session kill flag.
    killed: std::collections::HashSet<SessionId>,
}

impl MockDriver {
    /// Construct empty.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Script `event` to be returned the next time `poll` is called for
    /// `id`. Multiple scripts queue in order.
    pub fn script(&self, id: &SessionId, event: DriverEvent) {
        // SAFETY: lock is contention-free in single-threaded tests; for
        // multi-threaded tests we tolerate poisoning by panicking — that
        // *is* the test failure.
        let mut g = self.inner.lock().expect("mock lock poisoned");
        g.events.entry(id.clone()).or_default().push_back(event);
    }

    /// How many times `resume` was called for `id`.
    #[must_use]
    pub fn resume_count(&self, id: &SessionId) -> u32 {
        self.inner
            .lock()
            .expect("mock lock poisoned")
            .resumes
            .get(id)
            .copied()
            .unwrap_or(0)
    }

    /// Whether `kill` was ever called for `id`.
    #[must_use]
    pub fn was_killed(&self, id: &SessionId) -> bool {
        self.inner
            .lock()
            .expect("mock lock poisoned")
            .killed
            .contains(id)
    }
}

#[async_trait]
impl ClaudeDriver for MockDriver {
    async fn start(
        &self,
        id: &SessionId,
        _worktree: &std::path::Path,
        _system_prompt: &str,
    ) -> Result<SessionHandle, DriverError> {
        trace!(%id, "mock start");
        Ok(SessionHandle {
            id: id.clone(),
            token: format!("mock-{id}"),
        })
    }

    async fn poll(&self, handle: &SessionHandle) -> Result<Vec<DriverEvent>, DriverError> {
        let events: Vec<DriverEvent> = {
            let mut g = self.inner.lock().expect("mock lock poisoned");
            g.events
                .get_mut(&handle.id)
                .map_or_else(Vec::new, |q| q.drain(..).collect())
        };
        trace!(id = %handle.id, count = events.len(), "mock poll drained");
        Ok(events)
    }

    async fn resume(&self, handle: &SessionHandle) -> Result<(), DriverError> {
        {
            let mut g = self.inner.lock().expect("mock lock poisoned");
            *g.resumes.entry(handle.id.clone()).or_default() += 1;
        }
        trace!(id = %handle.id, "mock resume");
        Ok(())
    }

    async fn kill(&self, handle: &SessionHandle) -> Result<(), DriverError> {
        {
            let mut g = self.inner.lock().expect("mock lock poisoned");
            g.killed.insert(handle.id.clone());
        }
        trace!(id = %handle.id, "mock kill");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn id() -> SessionId {
        SessionId("alpha".to_owned())
    }

    #[tokio::test]
    async fn start_returns_handle() {
        let d = MockDriver::new();
        let h = d.start(&id(), Path::new("/tmp"), "prompt").await.unwrap();
        assert_eq!(h.id, id());
    }

    #[tokio::test]
    async fn poll_drains_scripted_events() {
        let d = MockDriver::new();
        let h = d.start(&id(), Path::new("/tmp"), "p").await.unwrap();
        d.script(
            &id(),
            DriverEvent::Log {
                line: "hello".into(),
            },
        );
        d.script(&id(), DriverEvent::Done);
        let events = d.poll(&h).await.unwrap();
        assert_eq!(events.len(), 2);
        // and a second poll yields nothing
        assert!(d.poll(&h).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn resume_records_count() {
        let d = MockDriver::new();
        let h = d.start(&id(), Path::new("/tmp"), "p").await.unwrap();
        d.resume(&h).await.unwrap();
        d.resume(&h).await.unwrap();
        assert_eq!(d.resume_count(&id()), 2);
    }

    #[tokio::test]
    async fn kill_marks_session() {
        let d = MockDriver::new();
        let h = d.start(&id(), Path::new("/tmp"), "p").await.unwrap();
        assert!(!d.was_killed(&id()));
        d.kill(&h).await.unwrap();
        assert!(d.was_killed(&id()));
    }

    #[tokio::test]
    async fn poll_unknown_session_returns_empty() {
        let d = MockDriver::new();
        let fake_handle = SessionHandle {
            id: SessionId("ghost".into()),
            token: "x".into(),
        };
        let events = d.poll(&fake_handle).await.unwrap();
        assert!(events.is_empty());
    }
}
