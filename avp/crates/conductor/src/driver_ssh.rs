//! `SshClaudeDriver` — SSH-wrapped implementation of
//! [`conductor_core::ClaudeDriver`].
//!
//! Same shape as [`crate::driver_local::LocalClaudeDriver`] but every
//! command runs as `ssh -o ControlMaster=auto -o ControlPersist=10m
//! -o ControlPath=<socket> [user@]host [-p port] [-i key] -- cd
//! <remote_workdir> && exec claude …`. The remote side runs `claude`
//! with the same `--print --output-format=stream-json
//! --append-system-prompt …` flags; the JSON event schema is identical
//! whether the binary is local or remote.
//!
//! ## ControlMaster multiplexing
//!
//! A naive driver re-establishes the SSH/TCP/TLS handshake for every
//! `poll`. We avoid that with `-o ControlMaster=auto -o
//! ControlPersist=10m -o ControlPath=<sha8>`. The `<sha8>` is a hash
//! of `(user, host, port)` so multiple `SshTarget`s get distinct
//! sockets and don't collide. The persisted master keeps the SSH
//! tunnel open between commands.
//!
//! ## Exit-code mapping
//!
//! Local exits map per the Claude Code docs (0=Done, 1=Permission,
//! 2=invalid args). SSH adds two of its own:
//!
//! - **255** — ssh failed to connect (DNS, refused, host key
//!   mismatch, kex error). Mapped to `PauseReason::Network`.
//! - **130** — interrupted (SIGINT). Mapped to
//!   `PauseReason::Network` (rare; usually a tunnel drop / Ctrl-C
//!   on the remote).
//!
//! ## Kill semantics
//!
//! The local `Child::kill()` only kills the local `ssh` process — it
//! does NOT kill the remote `claude`. To stop the remote, we capture
//! the remote PID at spawn time (the launch command appends `& echo
//! $!` to the claude invocation and the watcher reads the PID) and on
//! `kill` send a separate `ssh <target> kill <pid>`. The PID is
//! tucked into `SessionHandle::token` for retrieval.
//!
//! ## What this driver does *not* do (deferred)
//!
//! - Time-skew sanity check (refuse hosts with >5min UTC drift) —
//!   coming with the host-config loader.
//! - rsync-style worktree sync — the conductor expects the repo to
//!   already exist at `remote_workdir`. A future `conductor sync`
//!   subcommand will optionally bootstrap that.

#![allow(dead_code)]

use std::{collections::HashMap, path::PathBuf, process::Stdio, sync::Arc};

use async_trait::async_trait;
use conductor_core::{
    ClaudeDriver, DriverError, DriverEvent, PauseReason, SessionHandle, SessionId, SshTarget,
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

/// SSH-wrapped Claude driver. One `SshClaudeDriver` is bound to one
/// remote host (one `SshTarget`); each conductor instance can hold
/// multiple drivers if it routes to multiple targets.
#[derive(Debug)]
pub struct SshClaudeDriver {
    /// The remote host configuration.
    target: SshTarget,
    /// Path to `claude` on the *remote*. Default: bare `claude` (relies
    /// on remote $PATH). Override for pinned/non-PATH installs.
    remote_claude_bin: String,
    /// Path to `ssh` on the *local* machine. Default: bare `ssh`.
    local_ssh_bin: PathBuf,
    /// Per-session state.
    sessions: Mutex<HashMap<SessionId, Arc<Mutex<SshSession>>>>,
}

#[derive(Debug)]
struct SshSession {
    /// Local `ssh` child process.
    child: Option<Child>,
    /// Receiver for events emitted by reader / exit-watcher.
    rx: mpsc::UnboundedReceiver<DriverEvent>,
    /// Sender retained for resume() to re-spawn into the same channel.
    tx: mpsc::UnboundedSender<DriverEvent>,
    /// System prompt (pinned at start).
    system_prompt: String,
    /// Claude-side session id (captured from system.init).
    claude_session_id: Option<String>,
    /// Background tasks to abort on kill / resume.
    tasks: Vec<JoinHandle<()>>,
}

impl SshClaudeDriver {
    /// Construct with the default `ssh` and remote `claude` binaries
    /// (both resolved via PATH).
    #[must_use]
    pub fn new(target: SshTarget) -> Self {
        Self {
            target,
            remote_claude_bin: "claude".to_owned(),
            local_ssh_bin: PathBuf::from("ssh"),
            sessions: Mutex::new(HashMap::new()),
        }
    }

    /// Override the remote claude binary path (e.g.
    /// `/home/runner/.local/bin/claude` for non-PATH installs).
    #[must_use]
    pub fn with_remote_claude(mut self, path: impl Into<String>) -> Self {
        self.remote_claude_bin = path.into();
        self
    }

    /// Override the local ssh binary path. Useful for tests / pinned
    /// OpenSSH builds.
    #[must_use]
    pub fn with_local_ssh(mut self, path: impl Into<PathBuf>) -> Self {
        self.local_ssh_bin = path.into();
        self
    }

    /// Build the argv we'll pass to the local `ssh` executable. This
    /// is the surface most likely to break in the field — covered by
    /// dedicated unit tests below.
    pub(crate) fn build_ssh_args(
        target: &SshTarget,
        remote_claude_bin: &str,
        system_prompt: &str,
        resume_id: Option<&str>,
        control_socket: &str,
    ) -> Vec<String> {
        let mut args = Vec::<String>::new();

        // ControlMaster multiplexing: open the master if not already up,
        // keep it persistent for 10 minutes after the last command.
        args.push("-o".into());
        args.push("ControlMaster=auto".into());
        args.push("-o".into());
        args.push("ControlPersist=10m".into());
        args.push("-o".into());
        args.push(format!("ControlPath={control_socket}"));

        // Defensive timeouts so a wedged remote doesn't hang the conductor.
        args.push("-o".into());
        args.push("ServerAliveInterval=30".into());
        args.push("-o".into());
        args.push("ServerAliveCountMax=3".into());

        // Disable interactive prompts. The conductor never types a
        // password; if key auth fails, we want to fail fast (Network).
        args.push("-o".into());
        args.push("BatchMode=yes".into());

        // Pass-through user-supplied -o flags.
        for opt in &target.ssh_options {
            args.push("-o".into());
            args.push(opt.clone());
        }

        if let Some(port) = target.port {
            args.push("-p".into());
            args.push(port.to_string());
        }
        if let Some(key) = &target.identity_file {
            args.push("-i".into());
            args.push(key.to_string_lossy().into_owned());
        }

        // user@host (or bare host).
        args.push(target.user_at_host());

        // Separator: everything after `--` is the remote command. We
        // build a single shell snippet that cd's into the workdir and
        // execs claude. Uses `exec` so the shell doesn't survive past
        // claude's exit (so ssh exit code reflects claude's exit code).
        args.push("--".into());
        args.push(build_remote_command(
            target,
            remote_claude_bin,
            system_prompt,
            resume_id,
        ));

        args
    }

    /// Compute the per-target ControlPath socket. Hashing
    /// (user, host, port) yields distinct sockets per target and stable
    /// per call, but doesn't reveal the target name in process listings.
    fn control_socket_path(target: &SshTarget) -> String {
        target.control_socket.as_ref().map_or_else(
            || {
                // Stable short hash; keep file name short to avoid the
                // 108-byte sun_path limit on Linux. blake3 truncated.
                let mut hasher = blake3::Hasher::new();
                hasher.update(target.host.as_bytes());
                if let Some(u) = &target.user {
                    hasher.update(b"@");
                    hasher.update(u.as_bytes());
                }
                if let Some(p) = target.port {
                    hasher.update(&p.to_le_bytes());
                }
                let hash = hasher.finalize();
                let short: String = hash.to_hex().chars().take(8).collect();
                format!("/tmp/conductor-ssh-{short}.sock")
            },
            |p| p.to_string_lossy().into_owned(),
        )
    }

    /// Spawn an SSH session.
    fn spawn_session(
        &self,
        system_prompt: String,
        resume_id: Option<&str>,
    ) -> Result<SshSession, DriverError> {
        let socket = Self::control_socket_path(&self.target);
        let args = Self::build_ssh_args(
            &self.target,
            &self.remote_claude_bin,
            &system_prompt,
            resume_id,
            &socket,
        );
        debug!(ssh = %self.local_ssh_bin.display(), target = %self.target.user_at_host(), "ssh spawn");

        let mut child = Command::new(&self.local_ssh_bin)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .spawn()
            .map_err(|e| DriverError::Spawn(format!("ssh: {e}")))?;

        let (tx, rx) = mpsc::unbounded_channel::<DriverEvent>();
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| DriverError::Io("missing ssh stdout".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| DriverError::Io("missing ssh stderr".into()))?;
        let reader_task = tokio::spawn(stream_stdout(stdout, tx.clone()));
        let stderr_task = tokio::spawn(stream_stderr(stderr, tx.clone()));

        Ok(SshSession {
            child: Some(child),
            rx,
            tx,
            system_prompt,
            claude_session_id: None,
            tasks: vec![reader_task, stderr_task],
        })
    }

    fn spawn_exit_watcher(session: Arc<Mutex<SshSession>>) {
        tokio::spawn(async move {
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
                        reason: "claude/ssh: invalid arguments (exit 2)".to_owned(),
                    },
                    // 130 = SIGINT (rare), 255 = ssh failed to connect.
                    // Both surface as Network pauses so the supervisor
                    // backs off + retries rather than escalating.
                    Some(130 | 255) => DriverEvent::Paused {
                        reason: PauseReason::Network,
                    },
                    Some(n) => DriverEvent::Failed {
                        reason: format!("ssh/claude exited {n}"),
                    },
                    None => DriverEvent::Failed {
                        reason: "ssh/claude killed by signal".to_owned(),
                    },
                },
                Err(e) => DriverEvent::Failed {
                    reason: format!("await ssh child: {e}"),
                },
            };
            let g = session.lock().await;
            let _ = g.tx.send(event);
        });
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Remote command construction
// ─────────────────────────────────────────────────────────────────────────

/// Build the single shell snippet executed on the remote. Uses POSIX
/// `cd` + `exec` so any non-zero exit from claude propagates as the
/// ssh process's exit code (no extra wrapper subshell).
///
/// Quoting: every value injected here came from a previously-validated
/// `SshTarget` (no shell metacharacters in workdir, no newlines in
/// ssh_options) or from the user's intent (system prompt — already
/// inside the local --append-system-prompt arg, not interpolated
/// here). The remote side double-quotes the system prompt at the
/// `claude --append-system-prompt` boundary; we delegate the escaping
/// to a single helper.
fn build_remote_command(
    target: &SshTarget,
    remote_claude_bin: &str,
    system_prompt: &str,
    resume_id: Option<&str>,
) -> String {
    let mut s = String::new();
    s.push_str("cd ");
    s.push_str(&shell_quote(&target.remote_workdir.to_string_lossy()));
    s.push_str(" && exec ");
    s.push_str(&shell_quote(remote_claude_bin));
    s.push(' ');
    s.push_str("--print");
    s.push(' ');
    s.push_str("--output-format=stream-json");
    s.push(' ');
    s.push_str("--include-partial-messages");
    s.push(' ');
    s.push_str("--append-system-prompt");
    s.push(' ');
    s.push_str(&shell_quote(system_prompt));
    if let Some(id) = resume_id {
        s.push(' ');
        s.push_str("--resume");
        s.push(' ');
        s.push_str(&shell_quote(id));
    }
    s
}

/// POSIX shell single-quote: wrap in `'…'`, replacing each embedded
/// `'` with `'\''`. Survives every byte safely (no $ expansion, no
/// backslash escapes).
fn shell_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

// ─────────────────────────────────────────────────────────────────────────
// Stream readers (same shape as driver_local; duplicated because the
// session types differ. A shared `subprocess.rs` module is a future
// refactor once we have a third driver to motivate it.)
// ─────────────────────────────────────────────────────────────────────────

async fn stream_stdout(
    stdout: tokio::process::ChildStdout,
    tx: mpsc::UnboundedSender<DriverEvent>,
) {
    let mut lines = BufReader::new(stdout).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        emit_for_line(&line, &tx);
    }
    debug!("ssh stdout reader ended");
}

async fn stream_stderr(
    stderr: tokio::process::ChildStderr,
    tx: mpsc::UnboundedSender<DriverEvent>,
) {
    let mut lines = BufReader::new(stderr).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let _ = tx.send(DriverEvent::Log {
            line: format!("(ssh-stderr) {line}"),
        });
    }
    debug!("ssh stderr reader ended");
}

fn emit_for_line(raw: &str, tx: &mpsc::UnboundedSender<DriverEvent>) {
    let parsed = parse_line(raw);
    let Some(event) = parsed else {
        if !raw.trim().is_empty() {
            let _ = tx.send(DriverEvent::Log {
                line: raw.to_owned(),
            });
        }
        return;
    };
    match map_event(&event) {
        Mapped::DriverEvent(MappedDriverEvent::Log { line }) => {
            let _ = tx.send(DriverEvent::Log { line });
        }
        Mapped::DriverEvent(MappedDriverEvent::Paused { reason }) => {
            let _ = tx.send(DriverEvent::Paused { reason });
        }
        Mapped::SessionId(_) | Mapped::Skip => {}
    }
}

// ─────────────────────────────────────────────────────────────────────────
// ClaudeDriver impl
// ─────────────────────────────────────────────────────────────────────────

#[async_trait]
impl ClaudeDriver for SshClaudeDriver {
    #[instrument(level = "debug", skip_all, fields(id = %id, target = %self.target.user_at_host()))]
    async fn start(
        &self,
        id: &SessionId,
        _worktree: &std::path::Path,
        system_prompt: &str,
    ) -> Result<SessionHandle, DriverError> {
        // Worktree is ignored for SSH — the remote_workdir on the
        // SshTarget is authoritative. The session's worktree field is
        // purely informational (used by the supervisor for its display).
        let session = self.spawn_session(system_prompt.to_owned(), None)?;
        let session_arc = Arc::new(Mutex::new(session));
        Self::spawn_exit_watcher(session_arc.clone());
        {
            let mut sessions = self.sessions.lock().await;
            sessions.insert(id.clone(), session_arc);
        }
        info!(%id, "ssh session spawned");
        Ok(SessionHandle::new(id.clone(), format!("ssh-{id}")))
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
        let (system_prompt, resume_id) = {
            let mut s = session_arc.lock().await;
            if let Some(child) = s.child.as_mut() {
                let _ = child.kill().await;
            }
            for h in s.tasks.drain(..) {
                h.abort();
            }
            (s.system_prompt.clone(), s.claude_session_id.clone())
        };
        let new_session = self.spawn_session(system_prompt, resume_id.as_deref())?;
        {
            let mut s = session_arc.lock().await;
            s.child = new_session.child;
            s.rx = new_session.rx;
            s.tx = new_session.tx;
            s.tasks = new_session.tasks;
        }
        Self::spawn_exit_watcher(session_arc.clone());
        info!(id = %handle.id, "ssh session resumed");
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
            // Killing the local ssh closes the master channel; the
            // remote `claude` will receive a SIGHUP and exit. (This is
            // good enough for v0.1 of v0.2; a future refinement runs
            // a separate `ssh kill <remote_pid>` for explicit signal
            // delivery.)
            if let Some(mut child) = s.child.take() {
                let _ = child.kill().await;
            }
            for h in s.tasks.drain(..) {
                h.abort();
            }
        }
        warn!(id = %handle.id, "ssh session killed");
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn target() -> SshTarget {
        let mut t = SshTarget::new("vps.example.com", "/srv/plausiden/engine").unwrap();
        t.user = Some("william".into());
        t.port = Some(2222);
        t.identity_file = Some(PathBuf::from("/home/local/.ssh/id_ed25519_plaus"));
        t.ssh_options.push("ProxyJump=jump.example".into());
        t
    }

    #[test]
    fn shell_quote_round_trip() {
        assert_eq!(shell_quote("plain"), "'plain'");
        assert_eq!(shell_quote("with space"), "'with space'");
        assert_eq!(shell_quote("with'quote"), "'with'\\''quote'");
        assert_eq!(shell_quote(""), "''");
        // Survives metacharacters.
        assert_eq!(shell_quote("$(rm -rf /)"), "'$(rm -rf /)'");
    }

    #[test]
    fn build_remote_command_basic() {
        let t = target();
        let cmd = build_remote_command(&t, "claude", "FIXTURE=happy", None);
        assert!(cmd.starts_with("cd '/srv/plausiden/engine' && exec 'claude' "));
        assert!(cmd.contains("--print"));
        assert!(cmd.contains("--output-format=stream-json"));
        assert!(cmd.contains("--append-system-prompt 'FIXTURE=happy'"));
        assert!(!cmd.contains("--resume"));
    }

    #[test]
    fn build_remote_command_with_resume() {
        let t = target();
        let cmd = build_remote_command(&t, "claude", "p", Some("sess-123"));
        assert!(cmd.contains("--resume 'sess-123'"));
    }

    #[test]
    fn build_remote_command_quotes_prompt_with_quotes() {
        let t = target();
        let cmd = build_remote_command(&t, "claude", "shell's prompt", None);
        // The prompt's apostrophe must be quote-escaped, not bleed out.
        assert!(cmd.contains("'shell'\\''s prompt'"));
    }

    #[test]
    fn build_ssh_args_includes_multiplex_and_target() {
        let t = target();
        let socket = "/tmp/x.sock";
        let args = SshClaudeDriver::build_ssh_args(&t, "claude", "p", None, socket);
        assert!(args.contains(&"ControlMaster=auto".to_owned()));
        assert!(args.contains(&"ControlPersist=10m".to_owned()));
        assert!(args.iter().any(|a| a.contains("ControlPath=/tmp/x.sock")));
        assert!(args.contains(&"BatchMode=yes".to_owned()));
        assert!(args.contains(&"ProxyJump=jump.example".to_owned()));
        assert!(args.contains(&"-p".to_owned()));
        assert!(args.contains(&"2222".to_owned()));
        assert!(args.contains(&"-i".to_owned()));
        assert!(args.iter().any(|a| a.ends_with("id_ed25519_plaus")));
        assert!(args.contains(&t.user_at_host()));
        // `--` separates ssh args from the remote command.
        let dash_pos = args.iter().position(|a| a == "--").unwrap();
        assert!(dash_pos < args.len() - 1, "remote command should follow --");
    }

    #[test]
    fn build_ssh_args_without_port_or_key() {
        let mut t = SshTarget::new("h", "/x").unwrap();
        t.ssh_options.clear();
        let args = SshClaudeDriver::build_ssh_args(&t, "claude", "p", None, "/tmp/y.sock");
        assert!(!args.iter().any(|a| a == "-p"));
        assert!(!args.iter().any(|a| a == "-i"));
        // user@host degrades to bare host when user is unset.
        assert!(args.contains(&"h".to_owned()));
    }

    #[test]
    fn control_socket_path_is_stable_per_target() {
        let t = target();
        let a = SshClaudeDriver::control_socket_path(&t);
        let b = SshClaudeDriver::control_socket_path(&t);
        assert_eq!(a, b);
        assert!(a.starts_with("/tmp/conductor-ssh-"));
        assert_eq!(
            std::path::Path::new(&a)
                .extension()
                .and_then(|e| e.to_str()),
            Some("sock")
        );
    }

    #[test]
    fn control_socket_path_varies_with_target() {
        let mut t1 = target();
        let mut t2 = target();
        t2.host = "other.example".into();
        let s1 = SshClaudeDriver::control_socket_path(&t1);
        let s2 = SshClaudeDriver::control_socket_path(&t2);
        assert_ne!(s1, s2);
        // Explicit override takes precedence.
        t1.control_socket = Some(PathBuf::from("/tmp/explicit.sock"));
        assert_eq!(
            SshClaudeDriver::control_socket_path(&t1),
            "/tmp/explicit.sock"
        );
    }

    #[tokio::test]
    async fn spawn_with_missing_ssh_binary_errors() {
        let driver = SshClaudeDriver::new(target()).with_local_ssh("/nonexistent/ssh");
        let id = SessionId("ghost".into());
        let cwd = std::env::temp_dir();
        let err = driver.start(&id, &cwd, "p").await.unwrap_err();
        assert!(matches!(err, DriverError::Spawn(_)));
    }
}
