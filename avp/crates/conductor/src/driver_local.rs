//! `LocalClaudeDriver` — local-subprocess implementation of
//! [`conductor_core::ClaudeDriver`].
//!
//! Runs `claude --print --output-format=stream-json
//! --append-system-prompt <prompt>` as a tokio subprocess in the
//! session's worktree, parses NDJSON events from stdout, and pushes
//! them as `DriverEvent`s on the supervisor's poll cycle.
//!
//! ## Lifecycle
//!
//! - `start`: spawn the subprocess, kick off a background reader task
//!   that streams stdout into an `mpsc::UnboundedSender<DriverEvent>`,
//!   and a watcher task that awaits child exit + pushes a terminal
//!   event (Done / Failed) when the child reaps.
//! - `poll`: drain the receiver into the supervisor's view.
//! - `resume`: tear down the existing subprocess (best-effort kill),
//!   re-spawn with `--resume <session-id>`. The Claude API persists
//!   session state server-side; resume picks up where the prior
//!   process left off.
//! - `kill`: send SIGTERM via `Child::kill().await`.
//!
//! ## Configuring `claude` binary location
//!
//! Default: `claude` resolved from `PATH`.
//! Override via [`LocalClaudeDriver::with_bin`] for tests / pinned
//! releases / `~/.local/bin/claude` style installs.
//!
//! ## Done-determination
//!
//! Exit code `0` → `DriverEvent::Done`.
//! Exit code `1` → `DriverEvent::Paused { reason: Permission }`
//! (`--print` mode exits 1 on permission failures, per docs).
//! Exit code `2` → `DriverEvent::Failed { reason: "claude: invalid args" }`.
//! SIGTERM / signal exit → `DriverEvent::Failed { reason: "killed" }`.
//! Anything else → `DriverEvent::Failed { reason: format!("exit {n}") }`.

// AVP-PASS-2026-05-01: dead_code allowed during the Local driver's
// initial wiring; the conductor binary's main consumes this module via
// the lib re-export but doesn't yet pin the driver to its supervisor.
// The integration tests in `tests/driver_local.rs` exercise everything.
#![allow(dead_code)]

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
};

use async_trait::async_trait;
use conductor_core::{
    ClaudeDriver, DriverError, DriverEvent, PauseReason, SessionHandle, SessionId,
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    sync::{Mutex, mpsc},
    task::JoinHandle,
};
use tracing::{debug, info, instrument, warn};

use crate::claude_event::{Mapped, MappedDriverEvent, map_event, parse_line};

// ─────────────────────────────────────────────────────────────────────────
// Driver
// ─────────────────────────────────────────────────────────────────────────

/// Local-subprocess driver. Holds per-session state behind an async
/// mutex so `poll`/`resume`/`kill` calls from the supervisor can race
/// safely.
#[derive(Debug)]
pub struct LocalClaudeDriver {
    /// Path to the `claude` binary. Default: bare `"claude"` (PATH lookup).
    claude_bin: PathBuf,
    /// Per-session state.
    sessions: Mutex<HashMap<SessionId, Arc<Mutex<LocalSession>>>>,
}

#[derive(Debug)]
struct LocalSession {
    /// Child process handle (None between resume() tear-down + re-spawn).
    child: Option<Child>,
    /// Receiver for events emitted by the reader/exit-watcher tasks.
    rx: mpsc::UnboundedReceiver<DriverEvent>,
    /// Sender retained so resume() can re-spawn into the same channel.
    tx: mpsc::UnboundedSender<DriverEvent>,
    /// Worktree CWD (needed for resume).
    worktree: PathBuf,
    /// System prompt (pinned at start, unchanged on resume).
    system_prompt: String,
    /// Claude-side session id, captured from `system.init`.
    claude_session_id: Option<String>,
    /// Background tasks (reader, exit watcher). Kept so we can abort
    /// them on drop/kill.
    tasks: Vec<JoinHandle<()>>,
}

impl Default for LocalClaudeDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalClaudeDriver {
    /// Construct with the default `claude` binary lookup.
    #[must_use]
    pub fn new() -> Self {
        Self::with_bin("claude")
    }

    /// Construct pointing at a specific `claude` binary path. Used by
    /// tests (fake fixture binary) and for pinned-version installs.
    #[must_use]
    pub fn with_bin(claude_bin: impl Into<PathBuf>) -> Self {
        Self {
            claude_bin: claude_bin.into(),
            sessions: Mutex::new(HashMap::new()),
        }
    }

    /// Build the argument vector for a fresh launch of a session.
    fn launch_args(system_prompt: &str, resume_id: Option<&str>) -> Vec<String> {
        let mut args = vec![
            "--print".to_owned(),
            "--output-format=stream-json".to_owned(),
            "--include-partial-messages".to_owned(),
            "--append-system-prompt".to_owned(),
            system_prompt.to_owned(),
        ];
        if let Some(id) = resume_id {
            args.push("--resume".to_owned());
            args.push(id.to_owned());
        }
        args
    }

    /// Spawn the subprocess + the reader/watcher tasks. Returns the
    /// `LocalSession` ready to be inserted into `self.sessions`.
    fn spawn_session(
        &self,
        worktree: &Path,
        system_prompt: String,
        resume_id: Option<&str>,
    ) -> Result<LocalSession, DriverError> {
        let args = Self::launch_args(&system_prompt, resume_id);
        debug!(bin = %self.claude_bin.display(), ?args, cwd = %worktree.display(), "claude spawn");

        let mut child = Command::new(&self.claude_bin)
            .args(&args)
            .current_dir(worktree)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .spawn()
            .map_err(|e| DriverError::Spawn(format!("{}: {e}", self.claude_bin.display())))?;

        let (tx, rx) = mpsc::unbounded_channel::<DriverEvent>();

        // Reader: stdout NDJSON → parsed events.
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| DriverError::Io("missing child stdout".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| DriverError::Io("missing child stderr".into()))?;
        let reader_task = tokio::spawn(stream_stdout(stdout, tx.clone()));
        let stderr_task = tokio::spawn(stream_stderr(stderr, tx.clone()));

        Ok(LocalSession {
            child: Some(child),
            rx,
            tx,
            worktree: worktree.to_path_buf(),
            system_prompt,
            claude_session_id: None,
            tasks: vec![reader_task, stderr_task],
        })
    }

    /// Spawn an exit watcher. Pulls the child out of the LocalSession
    /// and awaits it; pushes a terminal event when the child reaps.
    fn spawn_exit_watcher(session: Arc<Mutex<LocalSession>>) {
        tokio::spawn(async move {
            // Wait for exit. Acquire the lock, take the child, drop the
            // lock so poll/resume can still run during the wait.
            let mut child = {
                let mut g = session.lock().await;
                g.child.take()
            };
            let Some(child) = child.as_mut() else { return };
            let status = child.wait().await;
            let event = match status {
                Ok(s) => match s.code() {
                    Some(0) => DriverEvent::Done,
                    Some(1) => DriverEvent::Paused {
                        reason: PauseReason::Permission,
                    },
                    Some(2) => DriverEvent::Failed {
                        reason: "claude: invalid arguments (exit 2)".to_owned(),
                    },
                    Some(n) => DriverEvent::Failed {
                        reason: format!("claude exited {n}"),
                    },
                    None => DriverEvent::Failed {
                        reason: "claude killed by signal".to_owned(),
                    },
                },
                Err(e) => DriverEvent::Failed {
                    reason: format!("await child: {e}"),
                },
            };
            let g = session.lock().await;
            // tx may have been replaced by resume(); send is best-effort.
            let _ = g.tx.send(event);
        });
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Stream readers
// ─────────────────────────────────────────────────────────────────────────

async fn stream_stdout(
    stdout: tokio::process::ChildStdout,
    tx: mpsc::UnboundedSender<DriverEvent>,
) {
    let mut lines = BufReader::new(stdout).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        emit_for_line(&line, &tx);
    }
    debug!("stdout reader ended");
}

async fn stream_stderr(
    stderr: tokio::process::ChildStderr,
    tx: mpsc::UnboundedSender<DriverEvent>,
) {
    let mut lines = BufReader::new(stderr).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        // stderr is *always* surfaced as a Log so users can see CLI
        // diagnostics; we don't try to parse it as JSON.
        let _ = tx.send(DriverEvent::Log {
            line: format!("(stderr) {line}"),
        });
    }
    debug!("stderr reader ended");
}

fn emit_for_line(raw: &str, tx: &mpsc::UnboundedSender<DriverEvent>) {
    let parsed = parse_line(raw);
    let Some(event) = parsed else {
        // Unparseable line — surface as a log so users see what claude
        // printed. Empty-string lines are skipped silently.
        if !raw.trim().is_empty() {
            let _ = tx.send(DriverEvent::Log {
                line: raw.to_owned(),
            });
        }
        return;
    };
    match map_event(&event) {
        Mapped::DriverEvent(e) => {
            let de = match e {
                MappedDriverEvent::Log { line } => DriverEvent::Log { line },
                MappedDriverEvent::Paused { reason } => DriverEvent::Paused { reason },
            };
            let _ = tx.send(de);
        }
        // SessionId capture is deferred — for now we rely on the
        // `--resume` flow being driven from outside (next slice
        // captures the claude session id at the driver level).
        Mapped::SessionId(_) | Mapped::Skip => {}
    }
}

// ─────────────────────────────────────────────────────────────────────────
// ClaudeDriver impl
// ─────────────────────────────────────────────────────────────────────────

#[async_trait]
impl ClaudeDriver for LocalClaudeDriver {
    #[instrument(level = "debug", skip_all, fields(id = %id, worktree = %worktree.display()))]
    async fn start(
        &self,
        id: &SessionId,
        worktree: &Path,
        system_prompt: &str,
    ) -> Result<SessionHandle, DriverError> {
        let session = self.spawn_session(worktree, system_prompt.to_owned(), None)?;
        let session_arc = Arc::new(Mutex::new(session));
        Self::spawn_exit_watcher(session_arc.clone());
        {
            let mut sessions = self.sessions.lock().await;
            sessions.insert(id.clone(), session_arc);
        }
        info!(%id, "session spawned");
        Ok(SessionHandle::new(id.clone(), format!("local-{id}")))
    }

    async fn poll(&self, handle: &SessionHandle) -> Result<Vec<DriverEvent>, DriverError> {
        let session_arc = {
            let sessions = self.sessions.lock().await;
            sessions
                .get(&handle.id)
                .cloned()
                .ok_or_else(|| DriverError::UnknownSession(handle.id.clone()))?
        };
        let mut s = session_arc.lock().await;
        let mut drained = Vec::new();
        while let Ok(ev) = s.rx.try_recv() {
            // Capture session id from any DriverEvent that names it —
            // not done here since DriverEvent doesn't carry session_id;
            // the next slice extends the capture path by sniffing the
            // raw NDJSON for a `system.init` event in `emit_for_line`.
            drained.push(ev);
        }
        Ok(drained)
    }

    #[instrument(level = "debug", skip_all, fields(id = %handle.id))]
    async fn resume(&self, handle: &SessionHandle) -> Result<(), DriverError> {
        let session_arc = {
            let sessions = self.sessions.lock().await;
            sessions
                .get(&handle.id)
                .cloned()
                .ok_or_else(|| DriverError::UnknownSession(handle.id.clone()))?
        };
        // Tear down old child + tasks.
        let (worktree, system_prompt, resume_id) = {
            let mut s = session_arc.lock().await;
            if let Some(child) = s.child.as_mut() {
                let _ = child.kill().await;
            }
            for h in s.tasks.drain(..) {
                h.abort();
            }
            (
                s.worktree.clone(),
                s.system_prompt.clone(),
                s.claude_session_id.clone(),
            )
        };
        // Re-spawn with `--resume <id>` if we captured the session id;
        // otherwise fall back to a fresh launch with same prompt.
        let new_session = self.spawn_session(&worktree, system_prompt, resume_id.as_deref())?;
        {
            let mut s = session_arc.lock().await;
            s.child = new_session.child;
            s.rx = new_session.rx;
            s.tx = new_session.tx;
            s.tasks = new_session.tasks;
        }
        Self::spawn_exit_watcher(session_arc.clone());
        info!(id = %handle.id, "session resumed");
        Ok(())
    }

    #[instrument(level = "debug", skip_all, fields(id = %handle.id))]
    async fn kill(&self, handle: &SessionHandle) -> Result<(), DriverError> {
        let session_arc = {
            let mut sessions = self.sessions.lock().await;
            sessions.remove(&handle.id)
        };
        let Some(session_arc) = session_arc else {
            return Err(DriverError::UnknownSession(handle.id.clone()));
        };
        {
            let mut s = session_arc.lock().await;
            if let Some(mut child) = s.child.take() {
                let _ = child.kill().await;
            }
            for h in s.tasks.drain(..) {
                h.abort();
            }
        }
        warn!(id = %handle.id, "session killed");
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────
//
// Subprocess-spawning tests live in `tests/driver_local.rs` because
// `CARGO_BIN_EXE_claude-fixture` is only available to integration
// tests. Pure-logic tests stay here.

#[cfg(test)]
mod tests {
    use super::*;

    fn driver_no_spawn() -> LocalClaudeDriver {
        LocalClaudeDriver::with_bin("/nonexistent")
    }

    #[test]
    fn launch_args_includes_required_flags() {
        let args = LocalClaudeDriver::launch_args("hi", None);
        assert!(args.iter().any(|a| a == "--print"));
        assert!(args.iter().any(|a| a == "--output-format=stream-json"));
        assert!(args.iter().any(|a| a == "--append-system-prompt"));
        assert!(args.iter().any(|a| a == "hi"));
        assert!(!args.iter().any(|a| a == "--resume"));
    }

    #[test]
    fn launch_args_with_resume_id() {
        let args = LocalClaudeDriver::launch_args("hi", Some("sess-123"));
        let mut iter = args.iter();
        let resume_pos = iter.position(|a| a == "--resume");
        assert!(resume_pos.is_some(), "{args:?}");
        assert_eq!(iter.next().map(String::as_str), Some("sess-123"));
    }

    #[tokio::test]
    async fn spawn_with_missing_binary_errors() {
        let d = driver_no_spawn();
        let cwd = std::env::temp_dir();
        let id = SessionId("ghost".into());
        let err = d.start(&id, &cwd, "x").await.unwrap_err();
        assert!(matches!(err, DriverError::Spawn(_)));
    }
}
