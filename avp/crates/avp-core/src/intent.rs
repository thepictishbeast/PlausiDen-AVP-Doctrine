//! `.avp-intent.toml` — per-branch coordination manifest.
//!
//! Schema and lifecycle are documented at
//! `PlausiDen-AVP-Doctrine/cross-repo/multi-instance.md`. This module is
//! the type + parser + validator. Higher-level discovery (scan-branches,
//! overlap detection, merge-order topo sort) lives in the binary's
//! `crate::intent` module since it shells to `git`.

use std::{fs, path::Path};

use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::{Date, OffsetDateTime};
use tracing::{debug, instrument};

use crate::newtype::NewtypeError;

// ─────────────────────────────────────────────────────────────────────────
// AgentId newtype
// ─────────────────────────────────────────────────────────────────────────

/// Unique identifier for one parallel work unit.
///
/// Convention: `<persona>-<ISO date>-T<short hash>` (e.g.
/// `claude-2026-05-01-T01`). We don't enforce the convention — only
/// shape (non-empty, ASCII-printable, no whitespace, 1..=128 chars).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentId(String);

impl AgentId {
    /// Construct, validating shape.
    pub fn new(s: impl Into<String>) -> Result<Self, NewtypeError> {
        let s = s.into();
        if s.is_empty() {
            return Err(NewtypeError::AgentIdEmpty);
        }
        if s.len() > 128 {
            return Err(NewtypeError::AgentIdTooLong(s));
        }
        for c in s.chars() {
            if c.is_whitespace() || !c.is_ascii_graphic() {
                return Err(NewtypeError::AgentIdShape(s));
            }
        }
        Ok(Self(s))
    }

    /// Borrowed `&str` view.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for AgentId {
    type Err = NewtypeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

// ─────────────────────────────────────────────────────────────────────────
// IntentFile
// ─────────────────────────────────────────────────────────────────────────

/// Top-level deserialized form of `.avp-intent.toml`.
///
/// One file = one intent. Parser is strict: typos in field names fail
/// loudly. The `compiled_globs` field is `#[serde(skip)]` derived state
/// populated by [`IntentFile::validate`].
#[derive(Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct IntentFile {
    /// Unique identifier for this work unit.
    pub agent_id: AgentId,
    /// Git branch this intent governs.
    pub branch: String,
    /// One-sentence goal (human-readable, used in PR descriptions).
    pub goal: String,
    /// Runnable command that returns 0 iff the goal is achieved.
    pub success_test: String,
    /// When the intent was opened (RFC-3339).
    #[serde(with = "rfc3339")]
    pub opened_at: OffsetDateTime,
    /// Declared file scope. Glob patterns matched against repo-relative
    /// paths via `globset`. Empty = "I will touch nothing."
    pub declared_files: Vec<String>,
    /// Other agent ids whose declared-files overlap is intentional.
    #[serde(default)]
    pub allows_overlap_with: Vec<AgentId>,
    /// Optional GH PR reference for tracking (e.g. `org/repo#42`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_pr: Option<String>,
    /// Optional auto-archive date.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "iso_date_opt"
    )]
    pub expires_after: Option<Date>,

    /// Compiled globset (skipped during serde; built by `validate`).
    #[serde(skip)]
    compiled_globs: Option<GlobSet>,
}

impl PartialEq for IntentFile {
    fn eq(&self, other: &Self) -> bool {
        // SECURITY: equality compares only user-input fields. The compiled
        // globset is derived state and lacks PartialEq.
        self.agent_id == other.agent_id
            && self.branch == other.branch
            && self.goal == other.goal
            && self.success_test == other.success_test
            && self.opened_at == other.opened_at
            && self.declared_files == other.declared_files
            && self.allows_overlap_with == other.allows_overlap_with
            && self.expected_pr == other.expected_pr
            && self.expires_after == other.expires_after
    }
}

impl Eq for IntentFile {}

impl IntentFile {
    /// Load + validate from a path. Missing file is an error here
    /// (unlike ratchet, where missing means "no overrides"). Every
    /// branch managed by the conductor MUST have an intent file.
    #[instrument(level = "debug", skip_all, fields(path = %path.as_ref().display()))]
    pub fn load(path: impl AsRef<Path>) -> Result<Self, IntentError> {
        let path = path.as_ref();
        let text = fs::read_to_string(path).map_err(|source| IntentError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_toml(&text)
    }

    /// Parse + validate from raw TOML text. Test entry point.
    pub fn from_toml(text: &str) -> Result<Self, IntentError> {
        let mut me: Self = toml::from_str(text)?;
        me.validate()?;
        debug!(agent = %me.agent_id, files = me.declared_files.len(), "intent loaded");
        Ok(me)
    }

    /// Run all validity checks + compile glob set. Idempotent.
    pub fn validate(&mut self) -> Result<(), IntentError> {
        if self.branch.trim().is_empty() {
            return Err(IntentError::EmptyField { field: "branch" });
        }
        if self.goal.trim().is_empty() {
            return Err(IntentError::EmptyField { field: "goal" });
        }
        if self.success_test.trim().is_empty() {
            return Err(IntentError::EmptyField {
                field: "success_test",
            });
        }

        // Branch refname must look ref-shaped: no whitespace, no leading/trailing slash.
        if self.branch.contains(char::is_whitespace)
            || self.branch.starts_with('/')
            || self.branch.ends_with('/')
            || self.branch.contains("..")
        {
            return Err(IntentError::InvalidBranch(self.branch.clone()));
        }

        if let Some(expires) = self.expires_after {
            // Compare to the *date* of opened_at (UTC).
            let opened_date = self.opened_at.date();
            if expires <= opened_date {
                return Err(IntentError::ExpiryBeforeOpen {
                    opened: opened_date,
                    expires,
                });
            }
        }

        let mut builder = GlobSetBuilder::new();
        for pat in &self.declared_files {
            let glob = Glob::new(pat).map_err(|source| IntentError::Glob {
                pattern: pat.clone(),
                source,
            })?;
            builder.add(glob);
        }
        self.compiled_globs = Some(builder.build().map_err(|source| IntentError::Glob {
            pattern: "(globset build)".to_owned(),
            source,
        })?);

        Ok(())
    }

    /// True if a repo-relative path is matched by any declared glob.
    /// Returns `false` if `validate` hasn't run.
    #[must_use]
    pub fn matches_path(&self, repo_relative: &str) -> bool {
        self.compiled_globs
            .as_ref()
            .is_some_and(|gs| gs.is_match(repo_relative))
    }

    /// True if `now` is past `expires_after` (None = never expires).
    #[must_use]
    pub fn is_expired(&self, now: Date) -> bool {
        self.expires_after.is_some_and(|exp| now > exp)
    }

    /// Whether this intent permits its declared-file overlap with the
    /// other intent (either side allowing the other counts).
    #[must_use]
    pub fn overlap_allowed_with(&self, other: &Self) -> bool {
        self.allows_overlap_with.contains(&other.agent_id)
            || other.allows_overlap_with.contains(&self.agent_id)
    }

    /// Whether this intent's declared files share at least one pattern
    /// with another intent's declared files. We compare the *raw*
    /// patterns rather than expanding globs across an FS — overlap is
    /// declared, not discovered.
    #[must_use]
    pub fn pattern_overlap(&self, other: &Self) -> Vec<String> {
        let me: std::collections::HashSet<&String> = self.declared_files.iter().collect();
        other
            .declared_files
            .iter()
            .filter(|f| me.contains(f))
            .cloned()
            .collect()
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────────

/// Intent load / validate errors.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum IntentError {
    /// I/O error reading the file.
    #[error("intent io error at {path}: {source}")]
    Io {
        /// Path that failed.
        path: std::path::PathBuf,
        /// Underlying io error.
        source: std::io::Error,
    },
    /// TOML parse error.
    #[error("intent TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),
    /// Required field is empty.
    #[error("intent: required field `{field}` is empty")]
    EmptyField {
        /// The field name.
        field: &'static str,
    },
    /// Branch ref shape rejected.
    #[error("intent: invalid branch refname: {0:?}")]
    InvalidBranch(String),
    /// `expires_after` not strictly after `opened_at` date.
    #[error("intent: expires_after ({expires}) must be after opened ({opened})")]
    ExpiryBeforeOpen {
        /// Open date.
        opened: Date,
        /// Expiry date.
        expires: Date,
    },
    /// Glob pattern compile failure.
    #[error("intent: glob pattern {pattern:?} invalid: {source}")]
    Glob {
        /// Offending pattern.
        pattern: String,
        /// Underlying globset error.
        #[source]
        source: globset::Error,
    },
}

// ─────────────────────────────────────────────────────────────────────────
// RFC-3339 / ISO-date serde helpers
// ─────────────────────────────────────────────────────────────────────────

mod rfc3339 {
    use serde::{Deserialize, Deserializer, Serializer};
    use time::{OffsetDateTime, format_description::well_known::Rfc3339};

    // SECURITY: serde requires the `&T` shape on these helpers; clippy's
    // trivially_copy_pass_by_ref is a false-positive in serde-with land.
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub(super) fn serialize<S: Serializer>(dt: &OffsetDateTime, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&dt.format(&Rfc3339).map_err(serde::ser::Error::custom)?)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        de: D,
    ) -> Result<OffsetDateTime, D::Error> {
        let raw = toml::Value::deserialize(de)?;
        let s: String = match raw {
            toml::Value::String(s) => s,
            toml::Value::Datetime(d) => d.to_string(),
            other => {
                return Err(serde::de::Error::custom(format!(
                    "expected RFC-3339 timestamp, got {other:?}"
                )));
            }
        };
        OffsetDateTime::parse(s.trim(), &Rfc3339).map_err(serde::de::Error::custom)
    }
}

mod iso_date_opt {
    use serde::{Deserialize, Deserializer, Serializer};
    use time::{Date, format_description::FormatItem, macros::format_description};

    const FORMAT: &[FormatItem<'_>] = format_description!("[year]-[month]-[day]");

    // SECURITY: serde-with helpers receive `&Option<T>` by contract; the
    // `ref_option` lint is a false-positive here. Same rationale as the
    // sibling rfc3339 helper.
    #[allow(clippy::trivially_copy_pass_by_ref, clippy::ref_option)]
    pub(super) fn serialize<S: Serializer>(date: &Option<Date>, ser: S) -> Result<S::Ok, S::Error> {
        match date {
            None => ser.serialize_none(),
            Some(d) => ser.serialize_str(&d.format(FORMAT).map_err(serde::ser::Error::custom)?),
        }
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Option<Date>, D::Error> {
        let raw = Option::<toml::Value>::deserialize(de)?;
        let Some(raw) = raw else { return Ok(None) };
        let s: String = match raw {
            toml::Value::String(s) => s,
            toml::Value::Datetime(d) => d.to_string(),
            other => {
                return Err(serde::de::Error::custom(format!(
                    "expected ISO-8601 date, got {other:?}"
                )));
            }
        };
        Date::parse(s.trim(), FORMAT)
            .map(Some)
            .map_err(serde::de::Error::custom)
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::{date, datetime};

    fn minimal_toml() -> &'static str {
        r#"
        agent_id = "claude-2026-05-01-T01"
        branch = "claude/wire-avp-engine"
        goal = "Wire avp into PlausiDen-Engine CI."
        success_test = "cargo test"
        opened_at = "2026-05-01T20:00:00Z"
        declared_files = [".github/workflows/avp.yml"]
        "#
    }

    #[test]
    fn agent_id_valid() {
        for s in ["a", "claude-2026-05-01-T01", "x_y"] {
            assert!(AgentId::new(s).is_ok(), "{s}");
        }
    }

    #[test]
    fn agent_id_invalid() {
        for s in ["", " ", "has space", "tab\there", &"x".repeat(129)] {
            assert!(AgentId::new(s).is_err(), "{s}");
        }
    }

    #[test]
    fn minimal_loads_and_validates() {
        let f = IntentFile::from_toml(minimal_toml()).unwrap();
        assert_eq!(f.agent_id.as_str(), "claude-2026-05-01-T01");
        assert!(f.compiled_globs.is_some());
        assert!(f.matches_path(".github/workflows/avp.yml"));
        assert!(!f.matches_path("src/lib.rs"));
    }

    #[test]
    fn empty_branch_rejected() {
        let toml = r#"
            agent_id = "a"
            branch = ""
            goal = "g"
            success_test = "t"
            opened_at = "2026-05-01T20:00:00Z"
            declared_files = []
        "#;
        let err = IntentFile::from_toml(toml).unwrap_err();
        assert!(matches!(err, IntentError::EmptyField { field: "branch" }));
    }

    #[test]
    fn invalid_branch_rejected() {
        let cases = [
            "with space",
            "/leading-slash",
            "trailing-slash/",
            "has..dots",
        ];
        for branch in cases {
            let toml = format!(
                r#"
                agent_id = "a"
                branch = "{branch}"
                goal = "g"
                success_test = "t"
                opened_at = "2026-05-01T20:00:00Z"
                declared_files = []
                "#
            );
            let err = IntentFile::from_toml(&toml).unwrap_err();
            assert!(
                matches!(err, IntentError::InvalidBranch(_)),
                "branch={branch}: got {err:?}"
            );
        }
    }

    #[test]
    fn expiry_before_open_rejected() {
        let toml = r#"
            agent_id = "a"
            branch = "x"
            goal = "g"
            success_test = "t"
            opened_at = "2026-05-15T00:00:00Z"
            declared_files = []
            expires_after = "2026-05-01"
        "#;
        let err = IntentFile::from_toml(toml).unwrap_err();
        assert!(matches!(err, IntentError::ExpiryBeforeOpen { .. }));
    }

    #[test]
    fn invalid_glob_rejected() {
        let toml = r#"
            agent_id = "a"
            branch = "x"
            goal = "g"
            success_test = "t"
            opened_at = "2026-05-01T00:00:00Z"
            declared_files = ["[unclosed"]
        "#;
        let err = IntentFile::from_toml(toml).unwrap_err();
        assert!(matches!(err, IntentError::Glob { .. }));
    }

    #[test]
    fn glob_matches_subdir() {
        let toml = r#"
            agent_id = "a"
            branch = "x"
            goal = "g"
            success_test = "t"
            opened_at = "2026-05-01T00:00:00Z"
            declared_files = ["crates/*/src/lib.rs"]
        "#;
        let f = IntentFile::from_toml(toml).unwrap();
        assert!(f.matches_path("crates/avp/src/lib.rs"));
        assert!(!f.matches_path("crates/avp/src/main.rs"));
    }

    #[test]
    fn expiry_check_works() {
        let mut f = IntentFile::from_toml(minimal_toml()).unwrap();
        f.expires_after = Some(date!(2026 - 06 - 01));
        f.validate().unwrap();
        assert!(!f.is_expired(date!(2026 - 05 - 15)));
        assert!(f.is_expired(date!(2026 - 06 - 02)));
    }

    #[test]
    fn overlap_allowed_when_either_side_lists_other() {
        let a = build("alpha", &[]);
        let b = build("beta", &["alpha".to_owned()]);
        assert!(a.overlap_allowed_with(&b));
        assert!(b.overlap_allowed_with(&a));
    }

    #[test]
    fn overlap_disallowed_by_default() {
        let a = build("alpha", &[]);
        let b = build("beta", &[]);
        assert!(!a.overlap_allowed_with(&b));
    }

    #[test]
    fn pattern_overlap_finds_shared() {
        let a = build_with_files("a", &["x.rs", "y.rs"]);
        let b = build_with_files("b", &["y.rs", "z.rs"]);
        assert_eq!(a.pattern_overlap(&b), vec!["y.rs".to_owned()]);
    }

    #[test]
    fn rfc3339_round_trip() {
        let f = IntentFile::from_toml(minimal_toml()).unwrap();
        assert_eq!(f.opened_at, datetime!(2026-05-01 20:00:00 UTC));
    }

    fn build(agent: &str, allows: &[String]) -> IntentFile {
        let toml = format!(
            r#"
            agent_id = "{agent}"
            branch = "{agent}"
            goal = "g"
            success_test = "t"
            opened_at = "2026-05-01T00:00:00Z"
            declared_files = []
            allows_overlap_with = [{}]
            "#,
            allows
                .iter()
                .map(|a| format!("\"{a}\""))
                .collect::<Vec<_>>()
                .join(", ")
        );
        IntentFile::from_toml(&toml).unwrap()
    }

    fn build_with_files(agent: &str, files: &[&str]) -> IntentFile {
        let files_toml = files
            .iter()
            .map(|f| format!("\"{f}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let toml = format!(
            r#"
            agent_id = "{agent}"
            branch = "{agent}"
            goal = "g"
            success_test = "t"
            opened_at = "2026-05-01T00:00:00Z"
            declared_files = [{files_toml}]
            "#
        );
        IntentFile::from_toml(&toml).unwrap()
    }
}
