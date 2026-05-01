//! Findings — the structured output of a gate run.
//!
//! A [`Finding`] is the unit of communication between a gate and a reporter.
//! Reporters convert findings to GitHub Actions annotations, JSON,
//! human-readable terminal output, etc.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::gate::GateId;

/// Severity ladder. Maps directly to GitHub Actions log levels and to ANSI
/// color choices in the human reporter.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational; never fails CI.
    Notice,
    /// Soft signal; never fails CI but flagged in reports.
    Warning,
    /// Hard finding; CI fails.
    Error,
}

impl Severity {
    /// GitHub Actions annotation prefix (`::error::`, `::warning::`,
    /// `::notice::`).
    #[must_use]
    pub const fn gh_keyword(self) -> &'static str {
        match self {
            Self::Notice => "notice",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }

    /// Whether a finding at this severity should fail CI.
    #[must_use]
    pub const fn is_failing(self) -> bool {
        matches!(self, Self::Error)
    }
}

/// Where a finding lives in the source tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Location {
    /// Repo-relative path.
    pub file: PathBuf,
    /// 1-based line number, or `None` for whole-file findings.
    pub line: Option<u32>,
    /// 1-based column, or `None`.
    pub column: Option<u32>,
}

impl Location {
    /// File-only location (no line / column).
    #[must_use]
    pub fn file(file: impl Into<PathBuf>) -> Self {
        Self {
            file: file.into(),
            line: None,
            column: None,
        }
    }

    /// File + 1-based line.
    #[must_use]
    pub fn line(file: impl Into<PathBuf>, line: u32) -> Self {
        Self {
            file: file.into(),
            line: Some(line),
            column: None,
        }
    }

    /// File + 1-based line + column.
    #[must_use]
    pub fn pin(file: impl Into<PathBuf>, line: u32, column: u32) -> Self {
        Self {
            file: file.into(),
            line: Some(line),
            column: Some(column),
        }
    }
}

/// A single gate finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Finding {
    /// Which gate produced this.
    pub gate: GateId,
    /// Severity (only `Error` fails CI).
    pub severity: Severity,
    /// Where in the tree.
    pub location: Location,
    /// Free-form human message.
    pub message: String,
    /// Optional suggestion for ratcheting this finding (used by
    /// `avp ratchet add` to materialize an entry).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_ratchet: Option<RatchetSuggestion>,
}

impl Finding {
    /// Construct a hard `Error` finding.
    #[must_use]
    pub fn error(gate: GateId, location: Location, message: impl Into<String>) -> Self {
        Self {
            gate,
            severity: Severity::Error,
            location,
            message: message.into(),
            suggested_ratchet: None,
        }
    }

    /// Construct a soft `Warning` finding.
    #[must_use]
    pub fn warning(gate: GateId, location: Location, message: impl Into<String>) -> Self {
        Self {
            gate,
            severity: Severity::Warning,
            location,
            message: message.into(),
            suggested_ratchet: None,
        }
    }

    /// Construct a `Notice` finding (informational; e.g., "ratcheted").
    #[must_use]
    pub fn notice(gate: GateId, location: Location, message: impl Into<String>) -> Self {
        Self {
            gate,
            severity: Severity::Notice,
            location,
            message: message.into(),
            suggested_ratchet: None,
        }
    }

    /// Attach a suggested ratchet entry to this finding.
    #[must_use]
    pub fn with_suggestion(mut self, suggestion: RatchetSuggestion) -> Self {
        self.suggested_ratchet = Some(suggestion);
        self
    }
}

/// What `avp ratchet add` should write if the user accepts a finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RatchetSuggestion {
    /// The gate to ratchet.
    pub gate: GateId,
    /// Suggested crate scope (None = global).
    pub crate_scope: Option<String>,
    /// Suggested file regex (None = no file restriction).
    pub file_regex: Option<String>,
    /// Pre-filled reason placeholder.
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_gh_keyword_stable() {
        assert_eq!(Severity::Error.gh_keyword(), "error");
        assert_eq!(Severity::Warning.gh_keyword(), "warning");
        assert_eq!(Severity::Notice.gh_keyword(), "notice");
    }

    #[test]
    fn only_error_is_failing() {
        assert!(Severity::Error.is_failing());
        assert!(!Severity::Warning.is_failing());
        assert!(!Severity::Notice.is_failing());
    }

    #[test]
    fn location_constructors() {
        let f = Location::file("a.rs");
        assert_eq!(f.line, None);

        let l = Location::line("a.rs", 12);
        assert_eq!(l.line, Some(12));
        assert_eq!(l.column, None);

        let p = Location::pin("a.rs", 12, 5);
        assert_eq!(p.line, Some(12));
        assert_eq!(p.column, Some(5));
    }

    #[test]
    fn finding_constructors() {
        let loc = Location::line("src/lib.rs", 42);
        let e = Finding::error(GateId::BugAssumption, loc.clone(), "missing");
        assert!(e.severity.is_failing());

        let w = Finding::warning(GateId::TestDensityAggregate, loc.clone(), "low");
        assert!(!w.severity.is_failing());

        let n = Finding::notice(GateId::ForbiddenCall, loc, "ratcheted");
        assert_eq!(n.severity, Severity::Notice);
    }

    #[test]
    fn finding_serde_round_trip() {
        let f = Finding::error(
            GateId::DebugRemove,
            Location::line("src/main.rs", 10),
            "DEBUG-REMOVE marker present",
        );
        let json = serde_json::to_string(&f).unwrap();
        let back: Finding = serde_json::from_str(&json).unwrap();
        assert_eq!(f, back);
    }
}
