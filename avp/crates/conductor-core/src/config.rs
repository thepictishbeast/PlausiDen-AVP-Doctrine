//! `~/.config/conductor/hosts.toml` — declared targets + routing rules.
//!
//! This module defines the on-disk schema and the routing logic. It
//! has no async / subprocess code so non-conductor tools (e.g. an
//! `avp` subcommand inspecting the config) can consume it freely.
//!
//! ## TOML schema
//!
//! ```toml
//! # Default routing target. "local" is always implicitly available;
//! # other names must match a [[host]] entry below.
//! default = "local"
//!
//! [[host]]
//! name = "vps-eu-1"
//! host = "vps-eu-1.plausiden.com"
//! user = "william"
//! port = 22
//! identity_file = "~/.ssh/id_ed25519_plausiden"
//! remote_workdir = "/srv/plausiden/PlausiDen-Engine"
//! ssh_options = ["ServerAliveInterval=30"]
//!
//! [[host]]
//! name = "macbook"
//! host = "192.168.1.42"
//! user = "william"
//! remote_workdir = "/Users/william/Development/PlausiDen/PlausiDen-Engine"
//!
//! [[rule]]
//! match = "tests-*"
//! host  = "vps-eu-1"
//!
//! [[rule]]
//! match = "browser-*"
//! host  = "macbook"
//! ```
//!
//! Rule match order: first matching pattern wins; if none match, the
//! `default` host is used. The `match` pattern is a `globset` glob
//! against the session's `agent_id` (which mirrors
//! `avp_core::AgentId`).

use std::{collections::HashMap, fs, path::Path};

use globset::{Glob, GlobMatcher};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, instrument};

use crate::host::{Host, SshTarget};

// ─────────────────────────────────────────────────────────────────────────
// Top-level config
// ─────────────────────────────────────────────────────────────────────────

/// Loaded + validated hosts config.
///
/// `hosts` is keyed by name; "local" is implicit (always present, can be
/// shadowed by a `[[host]]` entry named `"local"` only if it has the
/// `local = true` discriminator — otherwise the conductor refuses).
#[derive(Debug)]
#[non_exhaustive]
pub struct HostsConfig {
    /// Name → host. Always contains an entry for `"local"`.
    pub hosts: HashMap<String, Host>,
    /// Name of the default host (must be present in `hosts`).
    pub default: String,
    /// Compiled rules, in declaration order.
    pub rules: Vec<CompiledRule>,
}

/// One compiled routing rule.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct CompiledRule {
    /// Original glob source (preserved for diagnostics).
    pub pattern: String,
    /// Compiled matcher.
    pub matcher: GlobMatcher,
    /// Target host name.
    pub host_name: String,
}

impl HostsConfig {
    /// Load from `path`. A nonexistent path yields a config with only
    /// the implicit `"local"` host and `default = "local"` — useful for
    /// fresh installs where the user hasn't created the file yet.
    #[instrument(level = "debug", skip_all, fields(path = %path.display()))]
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            debug!("config absent; using implicit local-only");
            return Ok(Self::local_only());
        }
        let text = fs::read_to_string(path).map_err(|source| ConfigError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_toml(&text)
    }

    /// Implicit config when no file is present: just `"local"`.
    #[must_use]
    pub fn local_only() -> Self {
        let mut hosts = HashMap::new();
        hosts.insert("local".to_owned(), Host::Local);
        Self {
            hosts,
            default: "local".to_owned(),
            rules: Vec::new(),
        }
    }

    /// Parse + validate from raw TOML.
    pub fn from_toml(text: &str) -> Result<Self, ConfigError> {
        let raw: RawConfig = toml::from_str(text)?;
        raw.validate_and_compile()
    }

    /// Resolve a session's agent id to its target [`Host`]. The first
    /// matching rule wins; falls back to the default host.
    #[must_use]
    pub fn resolve(&self, agent_id: &str) -> &Host {
        for rule in &self.rules {
            // Rule references an unknown host should be impossible
            // (validation rejects it); the get() fallback is paranoia.
            if rule.matcher.is_match(agent_id)
                && let Some(h) = self.hosts.get(&rule.host_name)
            {
                return h;
            }
        }
        self.hosts
            .get(&self.default)
            .expect("default host present (validated)")
    }

    /// Iterate every declared host name (sorted for deterministic UX).
    pub fn host_names(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self.hosts.keys().map(String::as_str).collect();
        v.sort_unstable();
        v
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Raw TOML schema (pre-validation)
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
struct RawConfig {
    /// Optional override for the implicit default ("local").
    #[serde(default = "default_default")]
    default: String,
    #[serde(default, rename = "host")]
    hosts: Vec<RawHost>,
    #[serde(default, rename = "rule")]
    rules: Vec<RawRule>,
}

fn default_default() -> String {
    "local".to_owned()
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RawHost {
    name: String,
    /// When `true`, this entry shadows the implicit `"local"` host
    /// (rare; meant only for users who want to call something other
    /// than "local" the local host).
    #[serde(default)]
    local: bool,
    // The remaining fields mirror SshTarget — only consulted when
    // `local = false`.
    #[serde(default)]
    host: Option<String>,
    #[serde(default)]
    user: Option<String>,
    #[serde(default)]
    port: Option<u16>,
    #[serde(default)]
    identity_file: Option<std::path::PathBuf>,
    #[serde(default)]
    remote_workdir: Option<std::path::PathBuf>,
    #[serde(default)]
    ssh_options: Vec<String>,
    #[serde(default)]
    control_socket: Option<std::path::PathBuf>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RawRule {
    #[serde(rename = "match")]
    pattern: String,
    host: String,
}

impl RawConfig {
    fn validate_and_compile(self) -> Result<HostsConfig, ConfigError> {
        let mut hosts: HashMap<String, Host> = HashMap::new();
        // Implicit local first; can be overridden by a [[host]]
        // declaration with `local = true`.
        hosts.insert("local".to_owned(), Host::Local);

        for raw in self.hosts {
            if raw.name.is_empty() {
                return Err(ConfigError::EmptyName);
            }
            if hosts.contains_key(&raw.name) && raw.name != "local" {
                return Err(ConfigError::DuplicateHost(raw.name));
            }
            let host = if raw.local {
                Host::Local
            } else {
                let h = raw.host.clone().ok_or_else(|| ConfigError::MissingField {
                    host: raw.name.clone(),
                    field: "host",
                })?;
                let workdir =
                    raw.remote_workdir
                        .clone()
                        .ok_or_else(|| ConfigError::MissingField {
                            host: raw.name.clone(),
                            field: "remote_workdir",
                        })?;
                let mut t = SshTarget::new(h, workdir).map_err(|source| ConfigError::Ssh {
                    host: raw.name.clone(),
                    source,
                })?;
                t.user = raw.user;
                t.port = raw.port;
                t.identity_file = raw.identity_file;
                t.ssh_options = raw.ssh_options;
                t.control_socket = raw.control_socket;
                t.validate().map_err(|source| ConfigError::Ssh {
                    host: raw.name.clone(),
                    source,
                })?;
                Host::Ssh(t)
            };
            hosts.insert(raw.name, host);
        }

        if !hosts.contains_key(&self.default) {
            return Err(ConfigError::UnknownDefault(self.default));
        }

        let mut rules = Vec::with_capacity(self.rules.len());
        for raw in self.rules {
            if !hosts.contains_key(&raw.host) {
                return Err(ConfigError::UnknownRuleHost {
                    pattern: raw.pattern,
                    host: raw.host,
                });
            }
            let glob = Glob::new(&raw.pattern).map_err(|source| ConfigError::Glob {
                pattern: raw.pattern.clone(),
                source,
            })?;
            rules.push(CompiledRule {
                pattern: raw.pattern,
                matcher: glob.compile_matcher(),
                host_name: raw.host,
            });
        }

        Ok(HostsConfig {
            hosts,
            default: self.default,
            rules,
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────────

/// Errors from config load / parse / validate.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// I/O error opening the file.
    #[error("config io {path}: {source}")]
    Io {
        /// Path attempted.
        path: std::path::PathBuf,
        /// Underlying io error.
        source: std::io::Error,
    },
    /// TOML parse error.
    #[error("config TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
    /// Empty `name` on a `[[host]]` entry.
    #[error("[[host]] requires non-empty `name`")]
    EmptyName,
    /// Two `[[host]]` entries declare the same name.
    #[error("duplicate host name: {0:?}")]
    DuplicateHost(String),
    /// Required field missing on a `[[host]]` entry.
    #[error("[[host]] {host:?} missing required field `{field}`")]
    MissingField {
        /// Host name.
        host: String,
        /// Field name.
        field: &'static str,
    },
    /// Underlying SshTarget validation rejected the host.
    #[error("[[host]] {host:?} ssh validation: {source}")]
    Ssh {
        /// Host name.
        host: String,
        /// Underlying validator error.
        #[source]
        source: crate::host::HostError,
    },
    /// `default` references a host name not declared.
    #[error("default = {0:?} but no [[host]] declares that name")]
    UnknownDefault(String),
    /// A `[[rule]]` references an unknown host.
    #[error("[[rule]] match = {pattern:?} → host = {host:?}: no such host declared")]
    UnknownRuleHost {
        /// Glob pattern.
        pattern: String,
        /// Bad host name.
        host: String,
    },
    /// A `[[rule]] match` failed to compile as a glob.
    #[error("[[rule]] match = {pattern:?} glob compile failed: {source}")]
    Glob {
        /// Glob pattern source.
        pattern: String,
        /// Underlying compile error.
        #[source]
        source: globset::Error,
    },
}

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_file_loads_local_only() {
        let cfg = HostsConfig::from_toml("").unwrap();
        assert_eq!(cfg.host_names(), vec!["local"]);
        assert_eq!(cfg.default, "local");
        assert!(cfg.rules.is_empty());
    }

    #[test]
    fn local_only_static() {
        let cfg = HostsConfig::local_only();
        assert!(matches!(cfg.hosts.get("local"), Some(Host::Local)));
    }

    #[test]
    fn full_config_round_trips() {
        let toml = r#"
            default = "vps"

            [[host]]
            name = "vps"
            host = "vps.example.com"
            user = "claude"
            port = 22
            remote_workdir = "/srv/x"
            ssh_options = ["ServerAliveInterval=30"]

            [[rule]]
            match = "tests-*"
            host = "vps"
        "#;
        let cfg = HostsConfig::from_toml(toml).unwrap();
        assert_eq!(cfg.default, "vps");
        assert_eq!(cfg.hosts.len(), 2); // local + vps
        let Host::Ssh(t) = cfg.hosts.get("vps").unwrap() else {
            panic!("expected Ssh");
        };
        assert_eq!(t.host, "vps.example.com");
        assert_eq!(t.user.as_deref(), Some("claude"));
        assert_eq!(t.port, Some(22));
        assert_eq!(t.ssh_options, vec!["ServerAliveInterval=30".to_owned()]);
    }

    #[test]
    fn missing_default_rejected() {
        let toml = r#"
            default = "vps"
            [[host]]
            name = "other"
            local = true
        "#;
        let err = HostsConfig::from_toml(toml).unwrap_err();
        assert!(matches!(err, ConfigError::UnknownDefault(_)));
    }

    #[test]
    fn host_missing_required_field_rejected() {
        let toml = r#"
            [[host]]
            name = "vps"
            host = "x.example"
        "#;
        let err = HostsConfig::from_toml(toml).unwrap_err();
        assert!(matches!(
            err,
            ConfigError::MissingField {
                field: "remote_workdir",
                ..
            }
        ));
    }

    #[test]
    fn duplicate_host_rejected() {
        let toml = r#"
            [[host]]
            name = "vps"
            host = "x.example"
            remote_workdir = "/srv/x"

            [[host]]
            name = "vps"
            host = "y.example"
            remote_workdir = "/srv/y"
        "#;
        let err = HostsConfig::from_toml(toml).unwrap_err();
        assert!(matches!(err, ConfigError::DuplicateHost(_)));
    }

    #[test]
    fn unknown_rule_host_rejected() {
        let toml = r#"
            [[rule]]
            match = "x-*"
            host = "ghost"
        "#;
        let err = HostsConfig::from_toml(toml).unwrap_err();
        assert!(matches!(err, ConfigError::UnknownRuleHost { .. }));
    }

    #[test]
    fn unknown_field_rejected() {
        let toml = r#"
            [[host]]
            name = "vps"
            host = "x.example"
            remote_workdir = "/srv/x"
            typo_field = "oops"
        "#;
        let err = HostsConfig::from_toml(toml).unwrap_err();
        // `deny_unknown_fields` makes this a TOML parse error.
        assert!(matches!(err, ConfigError::Toml(_)));
    }

    #[test]
    fn rule_glob_compile_failure() {
        let toml = r#"
            [[rule]]
            match = "[unclosed"
            host = "local"
        "#;
        let err = HostsConfig::from_toml(toml).unwrap_err();
        assert!(matches!(err, ConfigError::Glob { .. }));
    }

    #[test]
    fn resolve_first_matching_rule_wins() {
        let toml = r#"
            default = "local"

            [[host]]
            name = "vps"
            host = "vps.example.com"
            remote_workdir = "/srv/x"

            [[host]]
            name = "macbook"
            host = "192.168.1.42"
            remote_workdir = "/Users/x/y"

            [[rule]]
            match = "browser-*"
            host = "macbook"

            [[rule]]
            match = "tests-*"
            host = "vps"
        "#;
        let cfg = HostsConfig::from_toml(toml).unwrap();
        assert!(matches!(cfg.resolve("browser-engine"), Host::Ssh(t) if t.host == "192.168.1.42"));
        assert!(matches!(cfg.resolve("tests-fast"), Host::Ssh(t) if t.host == "vps.example.com"));
        assert!(matches!(cfg.resolve("anything-else"), Host::Local));
    }

    #[test]
    fn resolve_falls_back_to_default_when_no_rule_matches() {
        let toml = r#"
            default = "vps"
            [[host]]
            name = "vps"
            host = "v.example"
            remote_workdir = "/srv/x"
        "#;
        let cfg = HostsConfig::from_toml(toml).unwrap();
        let h = cfg.resolve("anything");
        assert!(matches!(h, Host::Ssh(t) if t.host == "v.example"));
    }

    #[test]
    fn host_names_are_sorted() {
        let toml = r#"
            [[host]]
            name = "zeta"
            host = "z.example"
            remote_workdir = "/z"

            [[host]]
            name = "alpha"
            host = "a.example"
            remote_workdir = "/a"
        "#;
        let cfg = HostsConfig::from_toml(toml).unwrap();
        assert_eq!(cfg.host_names(), vec!["alpha", "local", "zeta"]);
    }

    #[test]
    fn missing_file_path_yields_local_only() {
        let cfg = HostsConfig::load(std::path::Path::new("/no/such/path.toml")).unwrap();
        assert_eq!(cfg.host_names(), vec!["local"]);
    }
}
