//! Domain newtypes — prevent boolean blindness and string-typing errors.
//!
//! Every parameter with constraints (a Cargo crate name, an email, a
//! repo-relative path) is wrapped here. AVP-2 §6 forbids passing bare
//! `String`s with implicit constraints across API boundaries.

use std::{fmt, path::PathBuf, str::FromStr};

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ─────────────────────────────────────────────────────────────────────────
// CrateName
// ─────────────────────────────────────────────────────────────────────────

/// A Cargo crate name (matches `[package].name` in a `Cargo.toml`).
///
/// BUG ASSUMPTION: a crate name is non-empty, ASCII, and matches
/// `[A-Za-z][A-Za-z0-9_-]*`. We validate on construction.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CrateName(String);

impl CrateName {
    /// Construct, validating the name.
    pub fn new(s: impl Into<String>) -> Result<Self, NewtypeError> {
        let s = s.into();
        if s.is_empty() {
            return Err(NewtypeError::CrateNameEmpty);
        }
        let mut chars = s.chars();
        let first = chars.next().ok_or(NewtypeError::CrateNameEmpty)?;
        if !first.is_ascii_alphabetic() {
            return Err(NewtypeError::CrateNameStart(s));
        }
        for c in chars {
            if !(c.is_ascii_alphanumeric() || c == '_' || c == '-') {
                return Err(NewtypeError::CrateNameChar(s));
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

impl fmt::Display for CrateName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for CrateName {
    type Err = NewtypeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

// ─────────────────────────────────────────────────────────────────────────
// SignerEmail
// ─────────────────────────────────────────────────────────────────────────

/// An accountable signer's email — required on every ratchet override
/// (AVP-2 §SHIP-DECISION).
///
/// BUG ASSUMPTION: a permissive `local@domain.tld` shape is enough; we are
/// not RFC-5322 conformant. Empty strings and missing `@` are rejected.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SignerEmail(String);

impl SignerEmail {
    /// Construct, validating shape.
    pub fn new(s: impl Into<String>) -> Result<Self, NewtypeError> {
        let s = s.into();
        if s.is_empty() {
            return Err(NewtypeError::SignerEmpty);
        }
        let Some(at) = s.find('@') else {
            return Err(NewtypeError::SignerNoAt(s));
        };
        let (local, domain) = (&s[..at], &s[at + 1..]);
        if local.is_empty() || domain.is_empty() || !domain.contains('.') {
            return Err(NewtypeError::SignerShape(s));
        }
        Ok(Self(s))
    }

    /// Borrowed view.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SignerEmail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for SignerEmail {
    type Err = NewtypeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

// ─────────────────────────────────────────────────────────────────────────
// RepoRelativePath
// ─────────────────────────────────────────────────────────────────────────

/// A path that is *known* to be relative to a repo root (no leading `/`,
/// no `..` traversal). Newtype guards against accidental absolute-path
/// arguments to file-matching gates.
///
/// BUG ASSUMPTION: callers construct this only after confirming the input
/// is safe. We reject `..` and absolute paths defensively at construction.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RepoRelativePath(PathBuf);

impl RepoRelativePath {
    /// Construct from any string-ish input, validating relativeness.
    pub fn new(s: impl Into<PathBuf>) -> Result<Self, NewtypeError> {
        let p = s.into();
        if p.is_absolute() {
            return Err(NewtypeError::PathAbsolute(p));
        }
        if p.components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(NewtypeError::PathParentTraversal(p));
        }
        Ok(Self(p))
    }

    /// Borrowed `&Path` view.
    #[must_use]
    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }

    /// Owned `PathBuf` view.
    #[must_use]
    pub fn into_path_buf(self) -> PathBuf {
        self.0
    }
}

impl fmt::Display for RepoRelativePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.to_string_lossy())
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────────

/// Errors raised by newtype constructors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NewtypeError {
    /// Empty crate name.
    #[error("crate name must be non-empty")]
    CrateNameEmpty,
    /// Crate name doesn't begin with an ASCII letter.
    #[error("crate name must start with [A-Za-z]: {0:?}")]
    CrateNameStart(String),
    /// Crate name has an illegal character.
    #[error("crate name has invalid char (allowed [A-Za-z0-9_-]): {0:?}")]
    CrateNameChar(String),
    /// Empty signer email.
    #[error("signed_by must be non-empty")]
    SignerEmpty,
    /// No `@` in email.
    #[error("signed_by missing '@': {0:?}")]
    SignerNoAt(String),
    /// Email shape rejected (empty local, empty domain, or domain has no dot).
    #[error("signed_by shape invalid: {0:?}")]
    SignerShape(String),
    /// Path is absolute when it must be relative.
    #[error("path must be relative to repo root: {0:?}")]
    PathAbsolute(PathBuf),
    /// Path tries to traverse upward.
    #[error("path contains parent traversal `..`: {0:?}")]
    PathParentTraversal(PathBuf),
    /// Empty agent id.
    #[error("agent_id must be non-empty")]
    AgentIdEmpty,
    /// Agent id over the 128-char cap.
    #[error("agent_id too long (>128 chars): {0:?}")]
    AgentIdTooLong(String),
    /// Agent id has whitespace or non-ASCII-graphic chars.
    #[error("agent_id has invalid char (must be ASCII-graphic, no whitespace): {0:?}")]
    AgentIdShape(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn crate_name_valid() {
        for n in ["avp", "avp-core", "engine_browser", "x", "abc123"] {
            assert!(CrateName::new(n).is_ok(), "{n} should be valid");
        }
    }

    #[test]
    fn crate_name_invalid() {
        for n in ["", "1abc", "-foo", "foo bar", "foo!"] {
            assert!(CrateName::new(n).is_err(), "{n} should be invalid");
        }
    }

    #[test]
    fn signer_email_valid() {
        for n in ["a@b.c", "william@plausiden.com", "x.y@example.co.uk"] {
            assert!(SignerEmail::new(n).is_ok(), "{n} should be valid");
        }
    }

    #[test]
    fn signer_email_invalid() {
        for n in ["", "no-at", "@nolocal.com", "nodomain@", "no-dot@host"] {
            assert!(SignerEmail::new(n).is_err(), "{n} should be invalid");
        }
    }

    #[test]
    fn repo_path_valid() {
        for n in ["src/lib.rs", "Cargo.toml", "a/b/c.rs"] {
            assert!(RepoRelativePath::new(n).is_ok(), "{n} should be valid");
        }
    }

    #[test]
    fn repo_path_invalid() {
        for n in ["/abs/path", "../escape", "a/../b"] {
            assert!(RepoRelativePath::new(n).is_err(), "{n} should be invalid");
        }
    }

    proptest! {
        #[test]
        fn crate_name_roundtrip(n in "[A-Za-z][A-Za-z0-9_-]{0,63}") {
            let c = CrateName::new(&n).expect("valid by construction");
            prop_assert_eq!(c.as_str(), n);
        }
    }
}
