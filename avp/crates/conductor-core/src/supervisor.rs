//! `Supervisor` — owns a list of sessions, steps each one per the
//! recovery policy, emits events.
//!
//! The supervisor is intentionally pure-async + driver-agnostic. Tests
//! drive it with a [`MockDriver`]; production wires a `RealClaudeDriver`
//! that shells to `claude --print`.

use std::sync::Arc;

use thiserror::Error;
use tokio::sync::Mutex;
use tracing::{debug, info, instrument};

use crate::{
    driver::{ClaudeDriver, DriverError, DriverEvent, SessionHandle},
    event::{SupervisorEvent, SupervisorEventKind, TerminalOutcome},
    policy::{PauseReason, RecoveryAction, RecoveryPolicy},
    session::{Session, SessionId, SessionState},
};

/// Top-level supervisor. Owns the sessions; the driver is pluggable.
#[derive(Debug)]
#[non_exhaustive]
pub struct Supervisor<D: ClaudeDriver> {
    /// The driver used to spawn / poll / resume / kill sessions.
    pub driver: Arc<D>,
    /// Recovery policy.
    pub policy: RecoveryPolicy,
    sessions: Mutex<Vec<TrackedSession>>,
    events: Mutex<Vec<SupervisorEvent>>,
}

#[derive(Debug)]
struct TrackedSession {
    session: Session,
    handle: Option<SessionHandle>,
}

impl<D: ClaudeDriver> Supervisor<D> {
    /// Construct with the doctrine-default policy.
    #[must_use]
    pub fn new(driver: Arc<D>) -> Self {
        Self::with_policy(driver, RecoveryPolicy::default())
    }

    /// Construct with a custom policy (e.g. tighter `max_resumes` for
    /// flaky environments).
    #[must_use]
    pub fn with_policy(driver: Arc<D>, policy: RecoveryPolicy) -> Self {
        Self {
            driver,
            policy,
            sessions: Mutex::new(Vec::new()),
            events: Mutex::new(Vec::new()),
        }
    }

    /// Add a session to the supervisor's tracking set, emitting a
    /// `Queued` event.
    pub async fn enroll(&self, session: Session) {
        let id = session.id.clone();
        self.sessions.lock().await.push(TrackedSession {
            session,
            handle: None,
        });
        self.emit(Some(id), SupervisorEventKind::Queued).await;
    }

    /// Drain the events buffer (for inspection by the CLI / tests).
    pub async fn drain_events(&self) -> Vec<SupervisorEvent> {
        std::mem::take(&mut *self.events.lock().await)
    }

    /// Snapshot of current session states. Used by tests + the CLI's
    /// status table.
    pub async fn snapshot(&self) -> Vec<(SessionId, SessionState, u32)> {
        self.sessions
            .lock()
            .await
            .iter()
            .map(|t| {
                (
                    t.session.id.clone(),
                    t.session.state.clone(),
                    t.session.resume_count,
                )
            })
            .collect()
    }

    /// Step every non-terminal session once.
    ///
    /// "Step" means:
    /// - Queued → start subprocess via driver, transition to Running.
    /// - Running → poll the driver; for each event, advance the FSM
    ///   (Log → log; Paused → consult policy; Done → terminal Done;
    ///   Failed → terminal Failed).
    /// - Resumed → call driver.resume(), transition to Running.
    /// - Paused → no-op (the supervisor's `await_recoveries` task
    ///   handles wakeups separately in production).
    ///
    /// This async fn is supervisor's primary unit of work; tests call
    /// it in a loop until `is_done()` returns true.
    #[instrument(level = "debug", skip(self))]
    pub async fn step(&self) -> Result<(), SupervisorError> {
        let mut sessions = self.sessions.lock().await;
        let mut events = Vec::<SupervisorEvent>::new();

        for tracked in sessions.iter_mut() {
            if tracked.session.state.is_terminal() {
                continue;
            }
            match &tracked.session.state {
                SessionState::Queued => {
                    self.start_session(tracked, &mut events).await?;
                }
                SessionState::Running => {
                    self.poll_session(tracked, &mut events).await?;
                }
                SessionState::Resumed => {
                    self.do_resume(tracked, &mut events).await?;
                }
                SessionState::Paused(_) | SessionState::Done | SessionState::Failed { .. } => {
                    // Paused waits for an external wake; terminals are filtered above.
                }
            }
        }

        drop(sessions);
        {
            let mut buf = self.events.lock().await;
            buf.extend(events);
        }
        Ok(())
    }

    /// True when every session has reached a terminal state.
    pub async fn is_done(&self) -> bool {
        self.sessions
            .lock()
            .await
            .iter()
            .all(|t| t.session.state.is_terminal())
    }

    /// External wake: move a paused session into `Resumed`. Called by
    /// the supervisor's timer tasks when a backoff completes; tests
    /// call it directly.
    pub async fn wake(&self, id: &SessionId) -> Result<(), SupervisorError> {
        let mut sessions = self.sessions.lock().await;
        let tracked = sessions
            .iter_mut()
            .find(|t| &t.session.id == id)
            .ok_or_else(|| SupervisorError::UnknownSession(id.clone()))?;
        if matches!(tracked.session.state, SessionState::Paused(_)) {
            tracked.session.transition(SessionState::Resumed);
            tracked.session.record_resume();
        }
        drop(sessions);
        Ok(())
    }

    // ─── internals ───────────────────────────────────────────────────

    async fn emit(&self, session: Option<SessionId>, kind: SupervisorEventKind) {
        let event = SupervisorEvent::now(session, kind);
        debug!(?event, "emit");
        self.events.lock().await.push(event);
    }

    async fn start_session(
        &self,
        tracked: &mut TrackedSession,
        events: &mut Vec<SupervisorEvent>,
    ) -> Result<(), SupervisorError> {
        let prompt = render_system_prompt(&tracked.session);
        let handle = self
            .driver
            .start(&tracked.session.id, &tracked.session.worktree, &prompt)
            .await
            .map_err(|e| SupervisorError::Driver {
                id: tracked.session.id.clone(),
                source: e,
            })?;
        tracked.handle = Some(handle);
        tracked.session.transition(SessionState::Running);
        events.push(SupervisorEvent::now(
            Some(tracked.session.id.clone()),
            SupervisorEventKind::Started,
        ));
        Ok(())
    }

    async fn poll_session(
        &self,
        tracked: &mut TrackedSession,
        events: &mut Vec<SupervisorEvent>,
    ) -> Result<(), SupervisorError> {
        let handle = tracked
            .handle
            .clone()
            .ok_or_else(|| SupervisorError::Inconsistent {
                id: tracked.session.id.clone(),
                msg: "running without handle",
            })?;
        let drained = self
            .driver
            .poll(&handle)
            .await
            .map_err(|e| SupervisorError::Driver {
                id: tracked.session.id.clone(),
                source: e,
            })?;
        for ev in drained {
            self.apply_driver_event(tracked, ev, events);
            if tracked.session.state.is_terminal() {
                break;
            }
        }
        Ok(())
    }

    fn apply_driver_event(
        &self,
        tracked: &mut TrackedSession,
        ev: DriverEvent,
        events: &mut Vec<SupervisorEvent>,
    ) {
        match ev {
            DriverEvent::Log { line } => {
                tracked.session.last_log_line = Some(line.clone());
                events.push(SupervisorEvent::now(
                    Some(tracked.session.id.clone()),
                    SupervisorEventKind::Log { line },
                ));
            }
            DriverEvent::Paused { reason } => {
                tracked.session.transition(SessionState::Paused(reason));
                events.push(SupervisorEvent::now(
                    Some(tracked.session.id.clone()),
                    SupervisorEventKind::Paused { reason },
                ));
                self.handle_pause(tracked, reason, events);
            }
            DriverEvent::Done => {
                tracked.session.transition(SessionState::Done);
                events.push(SupervisorEvent::now(
                    Some(tracked.session.id.clone()),
                    SupervisorEventKind::Terminal {
                        outcome: TerminalOutcome::Done,
                    },
                ));
                info!(id = %tracked.session.id, "session done");
            }
            DriverEvent::Failed { reason } => {
                tracked.session.transition(SessionState::Failed {
                    reason: reason.clone(),
                });
                events.push(SupervisorEvent::now(
                    Some(tracked.session.id.clone()),
                    SupervisorEventKind::Terminal {
                        outcome: TerminalOutcome::Failed,
                    },
                ));
                events.push(SupervisorEvent::now(
                    Some(tracked.session.id.clone()),
                    SupervisorEventKind::Escalated { reason },
                ));
            }
        }
    }

    fn handle_pause(
        &self,
        tracked: &mut TrackedSession,
        reason: PauseReason,
        events: &mut Vec<SupervisorEvent>,
    ) {
        match self.policy.decide(reason, tracked.session.resume_count) {
            RecoveryAction::ResumeAfter { delay } => {
                events.push(SupervisorEvent::now(
                    Some(tracked.session.id.clone()),
                    SupervisorEventKind::ResumeScheduled {
                        delay_seconds: delay.as_secs(),
                    },
                ));
            }
            RecoveryAction::Escalate { reason: msg } => {
                tracked
                    .session
                    .transition(SessionState::Failed { reason: msg.into() });
                events.push(SupervisorEvent::now(
                    Some(tracked.session.id.clone()),
                    SupervisorEventKind::Escalated { reason: msg.into() },
                ));
                events.push(SupervisorEvent::now(
                    Some(tracked.session.id.clone()),
                    SupervisorEventKind::Terminal {
                        outcome: TerminalOutcome::Failed,
                    },
                ));
            }
        }
    }

    async fn do_resume(
        &self,
        tracked: &mut TrackedSession,
        events: &mut Vec<SupervisorEvent>,
    ) -> Result<(), SupervisorError> {
        let handle = tracked
            .handle
            .clone()
            .ok_or_else(|| SupervisorError::Inconsistent {
                id: tracked.session.id.clone(),
                msg: "resumed without handle",
            })?;
        self.driver
            .resume(&handle)
            .await
            .map_err(|e| SupervisorError::Driver {
                id: tracked.session.id.clone(),
                source: e,
            })?;
        tracked.session.transition(SessionState::Running);
        events.push(SupervisorEvent::now(
            Some(tracked.session.id.clone()),
            SupervisorEventKind::Started,
        ));
        Ok(())
    }
}

/// Render the system prompt that's handed to a freshly-spawned Claude
/// session. Today this is the rendered intent, plus a fixed preamble.
/// Future versions may layer in the per-repo `.claude/settings.json`
/// path and a recall of the prior session's tail.
fn render_system_prompt(session: &Session) -> String {
    format!(
        "You are working on branch `{branch}` in worktree `{wt}`.\n\
         Goal: {goal}\n\
         Success criterion: `{success}` must exit 0.\n\
         Declared file scope (do NOT touch anything outside this set):\n  - {files}\n",
        branch = session.intent.branch,
        wt = session.worktree.display(),
        goal = session.intent.goal,
        success = session.intent.success_test,
        files = session.intent.declared_files.join("\n  - "),
    )
}

/// Supervisor errors.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SupervisorError {
    /// A driver call failed for a specific session.
    #[error("session {id}: driver error: {source}")]
    Driver {
        /// The affected session.
        id: SessionId,
        /// Underlying driver error.
        #[source]
        source: DriverError,
    },
    /// State machine reached an inconsistent state (a bug we want to
    /// surface immediately rather than mask).
    #[error("session {id}: inconsistent state: {msg}")]
    Inconsistent {
        /// The affected session.
        id: SessionId,
        /// Diagnostic message.
        msg: &'static str,
    },
    /// Caller asked about a session id we don't track.
    #[error("session {0}: unknown")]
    UnknownSession(SessionId),
}

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use avp_core::IntentFile;

    use super::*;
    use crate::driver::{DriverEvent, MockDriver};

    fn intent_for(agent: &str) -> IntentFile {
        let toml = format!(
            r#"
            agent_id = "{agent}"
            branch = "claude/{agent}"
            goal = "test"
            success_test = "true"
            opened_at = "2026-05-01T00:00:00Z"
            declared_files = []
            "#
        );
        IntentFile::from_toml(&toml).unwrap()
    }

    fn session_for(agent: &str) -> Session {
        Session::new(intent_for(agent), PathBuf::from(format!("/tmp/wt-{agent}")))
    }

    #[tokio::test]
    async fn happy_path_done() {
        let driver = Arc::new(MockDriver::new());
        let sup = Supervisor::new(driver.clone());
        sup.enroll(session_for("alpha")).await;

        // step 1: queued → running
        sup.step().await.unwrap();
        let snap = sup.snapshot().await;
        assert!(matches!(snap[0].1, SessionState::Running));

        // script a Done event, step again
        driver.script(&snap[0].0, DriverEvent::Done);
        sup.step().await.unwrap();
        assert!(sup.is_done().await);

        let events = sup.drain_events().await;
        let kinds: Vec<&str> = events
            .iter()
            .map(|e| match &e.kind {
                SupervisorEventKind::Queued => "queued",
                SupervisorEventKind::Started => "started",
                SupervisorEventKind::Terminal { outcome } => match outcome {
                    TerminalOutcome::Done => "done",
                    TerminalOutcome::Failed => "failed",
                },
                _ => "other",
            })
            .collect();
        assert_eq!(kinds, vec!["queued", "started", "done"]);
    }

    #[tokio::test]
    async fn rate_limit_pauses_then_resumes() {
        let driver = Arc::new(MockDriver::new());
        let sup = Supervisor::new(driver.clone());
        sup.enroll(session_for("alpha")).await;

        sup.step().await.unwrap(); // → running
        let id = SessionId("alpha".to_owned());
        driver.script(
            &id,
            DriverEvent::Paused {
                reason: PauseReason::RateLimit,
            },
        );
        sup.step().await.unwrap(); // running → paused

        let snap = sup.snapshot().await;
        assert!(matches!(
            snap[0].1,
            SessionState::Paused(PauseReason::RateLimit)
        ));

        sup.wake(&id).await.unwrap();
        sup.step().await.unwrap(); // resumed → running
        let snap = sup.snapshot().await;
        assert!(matches!(snap[0].1, SessionState::Running));
        assert_eq!(driver.resume_count(&id), 1);
    }

    #[tokio::test]
    async fn permission_pause_escalates_immediately() {
        let driver = Arc::new(MockDriver::new());
        let sup = Supervisor::new(driver.clone());
        sup.enroll(session_for("alpha")).await;
        sup.step().await.unwrap();

        let id = SessionId("alpha".to_owned());
        driver.script(
            &id,
            DriverEvent::Paused {
                reason: PauseReason::Permission,
            },
        );
        sup.step().await.unwrap();

        let snap = sup.snapshot().await;
        assert!(matches!(snap[0].1, SessionState::Failed { .. }));
        assert!(sup.is_done().await);
    }

    #[tokio::test]
    async fn driver_failure_marks_failed() {
        let driver = Arc::new(MockDriver::new());
        let sup = Supervisor::new(driver.clone());
        sup.enroll(session_for("alpha")).await;
        sup.step().await.unwrap();

        let id = SessionId("alpha".to_owned());
        driver.script(
            &id,
            DriverEvent::Failed {
                reason: "kaboom".into(),
            },
        );
        sup.step().await.unwrap();

        let snap = sup.snapshot().await;
        let SessionState::Failed { reason } = &snap[0].1 else {
            panic!("expected Failed");
        };
        assert_eq!(reason, "kaboom");
    }

    #[tokio::test]
    async fn multiple_sessions_step_independently() {
        let driver = Arc::new(MockDriver::new());
        let sup = Supervisor::new(driver.clone());
        sup.enroll(session_for("alpha")).await;
        sup.enroll(session_for("beta")).await;

        sup.step().await.unwrap();
        driver.script(&SessionId("alpha".into()), DriverEvent::Done);
        driver.script(
            &SessionId("beta".into()),
            DriverEvent::Failed { reason: "x".into() },
        );
        sup.step().await.unwrap();

        let snap = sup.snapshot().await;
        let by_id: std::collections::HashMap<_, _> = snap
            .iter()
            .map(|(id, st, _)| (id.clone(), st.clone()))
            .collect();
        assert!(matches!(
            by_id[&SessionId("alpha".into())],
            SessionState::Done
        ));
        assert!(matches!(
            by_id[&SessionId("beta".into())],
            SessionState::Failed { .. }
        ));
    }

    #[tokio::test]
    async fn max_resumes_eventually_escalates() {
        let driver = Arc::new(MockDriver::new());
        let policy = RecoveryPolicy {
            max_resumes: 2,
            ..RecoveryPolicy::default()
        };
        let sup = Supervisor::with_policy(driver.clone(), policy);
        sup.enroll(session_for("alpha")).await;
        sup.step().await.unwrap();
        let id = SessionId("alpha".to_owned());

        for _ in 0..3 {
            driver.script(
                &id,
                DriverEvent::Paused {
                    reason: PauseReason::RateLimit,
                },
            );
            sup.step().await.unwrap();
            sup.wake(&id).await.unwrap();
            sup.step().await.unwrap();
        }
        // After exceeding max_resumes, the next pause escalates.
        driver.script(
            &id,
            DriverEvent::Paused {
                reason: PauseReason::RateLimit,
            },
        );
        sup.step().await.unwrap();
        let snap = sup.snapshot().await;
        assert!(matches!(snap[0].1, SessionState::Failed { .. }));
    }

    #[test]
    fn render_system_prompt_includes_branch_and_files() {
        let mut intent = intent_for("alpha");
        intent.declared_files = vec!["src/lib.rs".into(), "Cargo.toml".into()];
        let session = Session::new(intent, PathBuf::from("/tmp/wt"));
        let prompt = render_system_prompt(&session);
        assert!(prompt.contains("claude/alpha"));
        assert!(prompt.contains("src/lib.rs"));
        assert!(prompt.contains("Cargo.toml"));
    }
}
