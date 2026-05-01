//! Where a Claude Code session runs.
//!
//! The conductor is a fleet orchestrator: it must handle local
//! subprocesses *and* remote ones reached via SSH. Drivers consume
//! [`Host`] and dispatch — the local driver runs `claude …` directly,
//! the SSH driver runs `ssh user@host claude …` with connection
//! multiplexing for poll efficiency.
//!
//! Future: Docker / k8s / cloud-run targets. The enum is
//! `#[non_exhaustive]` so adding them won't break match arms.
//!
//! Design rationale captured in `cross-repo/conductor-hosts.md`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ─────────────────────────────────────────────────────────────────────────
// Host enum
// ─────────────────────────────────────────────────────────────────────────

/// Where a [`crate::Session`] runs.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Host {
    /// Conductor's own machine; spawn locally.
    #[default]
    Local,
    /// SSH-reachable remote.
    Ssh(SshTarget),
}

impl Host {
    /// Human label for log lines / status tables.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Local => "local".to_owned(),
            Self::Ssh(t) => format!("ssh:{t}"),
        }
    }

    /// True if this host requires network connectivity.
    #[must_use]
    pub const fn is_remote(&self) -> bool {
        matches!(self, Self::Ssh(_))
    }
}

// ─────────────────────────────────────────────────────────────────────────
// SshTarget
// ─────────────────────────────────────────────────────────────────────────

/// SSH connection target. All fields except `host` and `remote_workdir`
/// are optional — when absent, the SSH driver falls through to
/// `~/.ssh/config` defaults (Host blocks, IdentityFile, ProxyJump, …).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SshTarget {
    /// Hostname or IP. Required.
    pub host: String,
    /// Login username. None ⇒ `~/.ssh/config` default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    /// SSH port. None ⇒ 22 / config default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// Identity file (private key). None ⇒ ssh-agent or config default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_file: Option<PathBuf>,
    /// Absolute path to the working directory on the remote (where
    /// `claude` is run). The repo must already be present at this path —
    /// the conductor doesn't ship code; it orchestrates work where the
    /// code already lives.
    pub remote_workdir: PathBuf,
    /// Pass-through SSH options (`-o Foo=bar`). Useful for
    /// `ProxyJump=...`, `ServerAliveInterval=...`, etc.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ssh_options: Vec<String>,
    /// ControlMaster socket path (None ⇒ driver chooses). Setting this
    /// explicitly lets multiple sessions share one TCP connection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_socket: Option<PathBuf>,
}

impl SshTarget {
    /// Construct + validate.
    pub fn new(
        host: impl Into<String>,
        remote_workdir: impl Into<PathBuf>,
    ) -> Result<Self, HostError> {
        let host = host.into();
        let remote_workdir = remote_workdir.into();
        let me = Self {
            host,
            user: None,
            port: None,
            identity_file: None,
            remote_workdir,
            ssh_options: Vec::new(),
            control_socket: None,
        };
        me.validate()?;
        Ok(me)
    }

    /// Validate field shapes:
    /// - host non-empty, no whitespace
    /// - user (if Some) ASCII-printable, no whitespace
    /// - remote_workdir absolute (no `..`)
    /// - ssh_options none containing newlines (would be shell-injectable)
    pub fn validate(&self) -> Result<(), HostError> {
        if self.host.is_empty() {
            return Err(HostError::EmptyHost);
        }
        if self.host.contains(char::is_whitespace) {
            return Err(HostError::InvalidHost(self.host.clone()));
        }
        if let Some(user) = &self.user
            && (user.is_empty()
                || user
                    .chars()
                    .any(|c| c.is_whitespace() || !c.is_ascii_graphic()))
        {
            return Err(HostError::InvalidUser(user.clone()));
        }
        if !self.remote_workdir.is_absolute() {
            return Err(HostError::WorkdirNotAbsolute(self.remote_workdir.clone()));
        }
        if self
            .remote_workdir
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(HostError::WorkdirParentTraversal(
                self.remote_workdir.clone(),
            ));
        }
        for opt in &self.ssh_options {
            if opt.contains('\n') || opt.contains('\r') {
                return Err(HostError::InjectableSshOption(opt.clone()));
            }
        }
        Ok(())
    }

    /// Render the `user@host` form ssh accepts, falling back to bare
    /// host when no user is set (config can supply one).
    #[must_use]
    pub fn user_at_host(&self) -> String {
        self.user
            .as_ref()
            .map_or_else(|| self.host.clone(), |u| format!("{u}@{}", self.host))
    }
}

impl std::fmt::Display for SshTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.user_at_host())?;
        if let Some(p) = self.port {
            write!(f, ":{p}")?;
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────────

/// Validation errors for `SshTarget`.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum HostError {
    /// Host string is empty.
    #[error("ssh host must be non-empty")]
    EmptyHost,
    /// Host string contains whitespace (rejected to avoid shell injection).
    #[error("ssh host shape invalid: {0:?}")]
    InvalidHost(String),
    /// User string contains whitespace or non-ASCII-graphic chars.
    #[error("ssh user shape invalid: {0:?}")]
    InvalidUser(String),
    /// `remote_workdir` is relative (must be absolute on the remote).
    #[error("remote_workdir must be absolute: {0:?}")]
    WorkdirNotAbsolute(PathBuf),
    /// `remote_workdir` contains `..`.
    #[error("remote_workdir contains parent traversal `..`: {0:?}")]
    WorkdirParentTraversal(PathBuf),
    /// One of the `ssh_options` entries contains a newline or carriage
    /// return — those would let a malformed config inject ssh flags.
    #[error("ssh_option contains newline/carriage return (injectable): {0:?}")]
    InjectableSshOption(String),
}

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_default() {
        let h = Host::default();
        assert!(matches!(h, Host::Local));
        assert!(!h.is_remote());
        assert_eq!(h.label(), "local");
    }

    #[test]
    fn ssh_target_round_trips_via_serde() {
        let t = SshTarget::new("vps.example.com", "/srv/plausiden/engine").unwrap();
        let json = serde_json::to_string(&t).unwrap();
        let back: SshTarget = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }

    #[test]
    fn ssh_target_user_at_host() {
        let mut t = SshTarget::new("vps.example.com", "/srv/x").unwrap();
        assert_eq!(t.user_at_host(), "vps.example.com");
        t.user = Some("william".into());
        assert_eq!(t.user_at_host(), "william@vps.example.com");
    }

    #[test]
    fn ssh_target_display_with_port() {
        let mut t = SshTarget::new("h", "/x").unwrap();
        t.port = Some(2222);
        assert_eq!(format!("{t}"), "h:2222");
        t.user = Some("u".into());
        assert_eq!(format!("{t}"), "u@h:2222");
    }

    #[test]
    fn empty_host_rejected() {
        let err = SshTarget::new("", "/x").unwrap_err();
        assert!(matches!(err, HostError::EmptyHost));
    }

    #[test]
    fn whitespace_host_rejected() {
        let err = SshTarget::new("bad host", "/x").unwrap_err();
        assert!(matches!(err, HostError::InvalidHost(_)));
    }

    #[test]
    fn relative_workdir_rejected() {
        let err = SshTarget::new("h", "relative/path").unwrap_err();
        assert!(matches!(err, HostError::WorkdirNotAbsolute(_)));
    }

    #[test]
    fn parent_traversal_workdir_rejected() {
        let err = SshTarget::new("h", "/srv/../etc").unwrap_err();
        assert!(matches!(err, HostError::WorkdirParentTraversal(_)));
    }

    #[test]
    fn injectable_option_rejected() {
        let mut t = SshTarget::new("h", "/x").unwrap();
        t.ssh_options.push("foo\ninject".into());
        let err = t.validate().unwrap_err();
        assert!(matches!(err, HostError::InjectableSshOption(_)));
    }

    #[test]
    fn invalid_user_rejected() {
        let mut t = SshTarget::new("h", "/x").unwrap();
        t.user = Some("user with space".into());
        let err = t.validate().unwrap_err();
        assert!(matches!(err, HostError::InvalidUser(_)));
    }

    #[test]
    fn host_label_includes_target() {
        let t = SshTarget::new("vps.example", "/srv/x").unwrap();
        let h = Host::Ssh(t);
        assert!(h.label().starts_with("ssh:"));
        assert!(h.is_remote());
    }

    #[test]
    fn host_serde_tagged_kind() {
        let h = Host::Local;
        let v = serde_json::to_value(&h).unwrap();
        assert_eq!(v["kind"], "local");

        let h = Host::Ssh(SshTarget::new("x", "/y").unwrap());
        let v = serde_json::to_value(&h).unwrap();
        assert_eq!(v["kind"], "ssh");
    }
}
