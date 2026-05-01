//! Per-repo `avp-ratchet.toml` overrides — temporary, signed, time-boxed
//! exemptions from specific gates.
//!
//! AVP-2 §SHIP-DECISION:
//! > After a minimum of 36 passes I may interrupt the loop with a
//! > `SHIP-DECISION:` annotation listing accepted residual risks, the
//! > mutant survival rate, the coverage number, and my name.
//!
//! Ratchets are the machine-readable form of that decision. Each entry has a
//! gate, an optional scope (crate / file regex), a free-text reason, an
//! accountable signer email, an opening date, and a hard expiry date after
//! which the gate re-engages and CI fails.
//!
//! # Schema
//!
//! ```toml
//! [[overrides]]
//! gate          = "bug-assumption"
//! crate         = "engine-fs"          # optional
//! file          = "src/legacy/.*"      # optional, Rust regex
//! reason        = "Pending RFC-0007 deprecation."
//! signed_by     = "william@plausiden.com"
//! opened        = "2026-04-30"
//! expires_after = "2026-06-30"
//! ```

use std::{
    fs,
    path::{Path, PathBuf},
};

use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::Date;
use tracing::{debug, instrument, trace, warn};

use crate::{
    gate::{GateId, UnknownGate},
    newtype::{CrateName, NewtypeError, SignerEmail},
};

// ─────────────────────────────────────────────────────────────────────────
// File container
// ─────────────────────────────────────────────────────────────────────────

/// Top-level deserialized form of `avp-ratchet.toml`.
///
/// BUG ASSUMPTION: a missing file is *not* an error — the absence of
/// overrides is the default state. A malformed file IS an error: typos and
/// bad dates must surface loudly so a developer cannot accidentally disable
/// a gate they didn't mean to.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RatchetFile {
    /// Override entries, in source order.
    #[serde(default, rename = "overrides")]
    pub entries: Vec<RatchetEntry>,
}

impl RatchetFile {
    /// Load (and validate) a ratchet file from disk. A missing file yields
    /// an empty [`RatchetFile`]; this is intentional.
    #[instrument(level = "debug", skip_all, fields(path = %path.as_ref().display()))]
    pub fn load(path: impl AsRef<Path>) -> Result<Self, RatchetError> {
        let path = path.as_ref();
        let text = match fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                debug!("ratchet file not found; returning empty");
                return Ok(Self::default());
            }
            Err(e) => {
                return Err(RatchetError::Io {
                    path: path.to_path_buf(),
                    source: e,
                });
            }
        };
        let mut me: Self = toml::from_str(&text)?;
        me.validate()?;
        debug!(entries = me.entries.len(), "ratchet file loaded");
        Ok(me)
    }

    /// Validate every entry and compile the `file` regexes. Idempotent.
    #[instrument(level = "debug", skip_all)]
    pub fn validate(&mut self) -> Result<(), RatchetError> {
        for (idx, entry) in self.entries.iter_mut().enumerate() {
            entry.validate(idx)?;
        }
        Ok(())
    }

    /// All currently *active* (unexpired) entries as of `now`.
    #[must_use]
    pub fn active(&self, now: Date) -> Vec<&RatchetEntry> {
        self.entries.iter().filter(|e| !e.is_expired(now)).collect()
    }

    /// All *expired* entries as of `now` — these are CI-failing.
    #[must_use]
    pub fn expired(&self, now: Date) -> Vec<&RatchetEntry> {
        self.entries.iter().filter(|e| e.is_expired(now)).collect()
    }

    /// Entries whose expiry is within `window_days` days of `now`.
    #[must_use]
    pub fn expiring_soon(&self, now: Date, window_days: u16) -> Vec<&RatchetEntry> {
        self.entries
            .iter()
            .filter(|e| {
                let days = e.days_until_expiry(now);
                days >= 0 && days <= i64::from(window_days)
            })
            .collect()
    }

    /// Whether any active entry covers a given gate / scope.
    #[instrument(level = "trace", skip_all, fields(gate = %gate))]
    pub fn covers(
        &self,
        gate: GateId,
        crate_scope: Option<&CrateName>,
        path: Option<&Path>,
        now: Date,
    ) -> bool {
        for entry in &self.entries {
            if entry.is_expired(now) {
                continue;
            }
            if entry.matches(gate, crate_scope, path) {
                trace!("ratchet hit");
                return true;
            }
        }
        false
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Entry
// ─────────────────────────────────────────────────────────────────────────

/// One `[[overrides]]` entry.
///
/// `PartialEq` is hand-written: `regex::Regex` does not implement `PartialEq`,
/// and the validated newtype caches are derived state — equality is defined
/// over the *source-of-truth* TOML fields only.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RatchetEntry {
    /// The gate this override targets.
    pub gate: GateId,

    /// Optional crate-name scope. When set, only findings inside this crate
    /// are covered. Stored as a raw string so a typo in the user's TOML
    /// doesn't get auto-validated — we cross-check on construction in
    /// `validate`.
    #[serde(default, rename = "crate")]
    pub crate_scope: Option<String>,

    /// Optional file-path regex (matched against repo-relative paths).
    #[serde(default)]
    pub file: Option<String>,

    /// Free-text justification.
    pub reason: String,

    /// Accountable signer email.
    pub signed_by: String,

    /// When the override was opened.
    #[serde(with = "iso_date")]
    pub opened: Date,

    /// Hard expiry (inclusive). On the day *after* this date, the override
    /// stops applying and any matching gate finding fails CI.
    #[serde(with = "iso_date")]
    pub expires_after: Date,

    /// Compiled regex (filled in by `validate`).
    #[serde(skip)]
    file_re: Option<Regex>,

    /// Validated newtype form of `crate_scope`.
    #[serde(skip)]
    crate_validated: Option<CrateName>,

    /// Validated newtype form of `signed_by`.
    #[serde(skip)]
    signed_by_validated: Option<SignerEmail>,
}

impl PartialEq for RatchetEntry {
    fn eq(&self, other: &Self) -> bool {
        // SECURITY: equality compares only the user-input TOML fields. Cached
        // derived state (`file_re`, `crate_validated`, `signed_by_validated`)
        // is intentionally ignored — comparing them would couple equality to
        // validation order, and `Regex` itself has no `PartialEq` impl.
        self.gate == other.gate
            && self.crate_scope == other.crate_scope
            && self.file == other.file
            && self.reason == other.reason
            && self.signed_by == other.signed_by
            && self.opened == other.opened
            && self.expires_after == other.expires_after
    }
}

impl Eq for RatchetEntry {}

impl RatchetEntry {
    /// Run all validity checks. `idx` is the entry's 0-based index in the
    /// file, used for clear error messages.
    pub(crate) fn validate(&mut self, idx: usize) -> Result<(), RatchetError> {
        if self.reason.trim().is_empty() {
            return Err(RatchetError::EmptyReason { idx });
        }
        // signer
        let signer = SignerEmail::new(self.signed_by.clone())
            .map_err(|source| RatchetError::Signer { idx, source })?;
        self.signed_by_validated = Some(signer);

        // crate name (if provided)
        if let Some(name) = &self.crate_scope {
            let c = CrateName::new(name.clone())
                .map_err(|source| RatchetError::Crate { idx, source })?;
            self.crate_validated = Some(c);
        }

        // dates
        if self.expires_after <= self.opened {
            return Err(RatchetError::ExpiryBeforeOpen {
                idx,
                opened: self.opened,
                expires_after: self.expires_after,
            });
        }

        // file regex
        if let Some(pat) = &self.file {
            let re = Regex::new(pat).map_err(|source| RatchetError::Regex { idx, source })?;
            self.file_re = Some(re);
        }

        // gate must be ratchetable
        if !self.gate.ratchetable() {
            return Err(RatchetError::GateNotRatchetable {
                idx,
                gate: self.gate,
            });
        }
        Ok(())
    }

    /// True if this entry's expiry has passed.
    #[must_use]
    pub fn is_expired(&self, now: Date) -> bool {
        now > self.expires_after
    }

    /// Days until expiry. Negative when expired.
    #[must_use]
    pub fn days_until_expiry(&self, now: Date) -> i64 {
        (self.expires_after - now).whole_days()
    }

    /// Days since opened.
    #[must_use]
    pub fn days_open(&self, now: Date) -> i64 {
        (now - self.opened).whole_days()
    }

    /// Does this entry match a particular finding?
    #[must_use]
    pub fn matches(
        &self,
        gate: GateId,
        crate_scope: Option<&CrateName>,
        path: Option<&Path>,
    ) -> bool {
        if self.gate != gate {
            return false;
        }
        if let Some(want) = self.crate_validated.as_ref() {
            match crate_scope {
                Some(have) if have == want => {}
                _ => return false,
            }
        }
        if let Some(re) = self.file_re.as_ref() {
            let Some(p) = path else { return false };
            let s = p.to_string_lossy();
            if !re.is_match(&s) {
                return false;
            }
        }
        true
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────────

/// Ratchet load / validate errors.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RatchetError {
    /// I/O error reading the file.
    #[error("ratchet file io error at {path}: {source}")]
    Io {
        /// Path that failed.
        path: PathBuf,
        /// Underlying io error.
        source: std::io::Error,
    },

    /// TOML did not parse.
    #[error("ratchet TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    /// Empty `reason` field.
    #[error("override #{idx}: reason is empty")]
    EmptyReason {
        /// Index of the offending entry.
        idx: usize,
    },

    /// `signed_by` failed validation.
    #[error("override #{idx}: signed_by invalid: {source}")]
    Signer {
        /// Index of the offending entry.
        idx: usize,
        /// Underlying newtype error.
        #[source]
        source: NewtypeError,
    },

    /// `crate` failed validation.
    #[error("override #{idx}: crate name invalid: {source}")]
    Crate {
        /// Index of the offending entry.
        idx: usize,
        /// Underlying newtype error.
        #[source]
        source: NewtypeError,
    },

    /// `expires_after` not strictly after `opened`.
    #[error("override #{idx}: expires_after ({expires_after}) must be after opened ({opened})")]
    ExpiryBeforeOpen {
        /// Index of the offending entry.
        idx: usize,
        /// `opened` date.
        opened: Date,
        /// `expires_after` date.
        expires_after: Date,
    },

    /// `file` is not a valid regex.
    #[error("override #{idx}: file regex invalid: {source}")]
    Regex {
        /// Index of the offending entry.
        idx: usize,
        /// Underlying regex error.
        #[source]
        source: regex::Error,
    },

    /// Gate id is unknown.
    #[error(transparent)]
    UnknownGate(#[from] UnknownGate),

    /// Caller tried to ratchet a non-ratchetable gate.
    #[error("override #{idx}: gate {gate} is not ratchetable")]
    GateNotRatchetable {
        /// Index of the offending entry.
        idx: usize,
        /// The non-ratchetable gate the user tried to override.
        gate: GateId,
    },
}

// ─────────────────────────────────────────────────────────────────────────
// ISO date helpers (TOML-string ⇄ time::Date)
// ─────────────────────────────────────────────────────────────────────────

mod iso_date {
    use serde::{Deserialize, Deserializer, Serializer};
    use time::{Date, format_description::FormatItem, macros::format_description};

    const FORMAT: &[FormatItem<'_>] = format_description!("[year]-[month]-[day]");

    // SECURITY: serde requires this signature shape (`&Date`); clippy's
    // trivially-copy-pass-by-ref lint is a false-positive for serde-with
    // serializers, so we silence it locally.
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub(super) fn serialize<S: Serializer>(date: &Date, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&date.format(FORMAT).map_err(serde::ser::Error::custom)?)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Date, D::Error> {
        let raw = toml::Value::deserialize(de)?;
        // Accept either a TOML local-date or a quoted ISO-8601 string.
        let s: String = match raw {
            toml::Value::String(s) => s,
            toml::Value::Datetime(d) => d.to_string(),
            other => {
                return Err(serde::de::Error::custom(format!(
                    "expected ISO-8601 date string, got {other:?}"
                )));
            }
        };
        Date::parse(s.trim(), FORMAT).map_err(serde::de::Error::custom)
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use proptest::prelude::*;
    use tempfile::NamedTempFile;
    use time::macros::date;

    use super::*;

    fn write_tmp(s: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(s.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    #[test]
    fn missing_file_is_empty_not_error() {
        let f = RatchetFile::load("/no/such/path/avp-ratchet.toml").unwrap();
        assert!(f.entries.is_empty());
    }

    #[test]
    fn minimal_well_formed_loads() {
        let s = r#"
            [[overrides]]
            gate = "bug-assumption"
            reason = "x"
            signed_by = "a@b.com"
            opened = "2026-01-01"
            expires_after = "2026-12-31"
        "#;
        let f = write_tmp(s);
        let r = RatchetFile::load(f.path()).unwrap();
        assert_eq!(r.entries.len(), 1);
        assert_eq!(r.entries[0].gate, GateId::BugAssumption);
        assert_eq!(r.entries[0].crate_scope, None);
    }

    #[test]
    fn missing_reason_rejected() {
        let s = r#"
            [[overrides]]
            gate = "bug-assumption"
            reason = "   "
            signed_by = "a@b.com"
            opened = "2026-01-01"
            expires_after = "2026-12-31"
        "#;
        let f = write_tmp(s);
        let err = RatchetFile::load(f.path()).unwrap_err();
        assert!(matches!(err, RatchetError::EmptyReason { idx: 0 }));
    }

    #[test]
    fn expiry_before_open_rejected() {
        let s = r#"
            [[overrides]]
            gate = "bug-assumption"
            reason = "x"
            signed_by = "a@b.com"
            opened = "2026-12-31"
            expires_after = "2026-01-01"
        "#;
        let f = write_tmp(s);
        let err = RatchetFile::load(f.path()).unwrap_err();
        assert!(matches!(err, RatchetError::ExpiryBeforeOpen { idx: 0, .. }));
    }

    #[test]
    fn debug_remove_not_ratchetable() {
        let s = r#"
            [[overrides]]
            gate = "debug-remove"
            reason = "x"
            signed_by = "a@b.com"
            opened = "2026-01-01"
            expires_after = "2026-12-31"
        "#;
        let f = write_tmp(s);
        let err = RatchetFile::load(f.path()).unwrap_err();
        assert!(matches!(
            err,
            RatchetError::GateNotRatchetable {
                idx: 0,
                gate: GateId::DebugRemove
            }
        ));
    }

    #[test]
    fn invalid_regex_rejected() {
        let s = r#"
            [[overrides]]
            gate = "bug-assumption"
            file = "[unclosed"
            reason = "x"
            signed_by = "a@b.com"
            opened = "2026-01-01"
            expires_after = "2026-12-31"
        "#;
        let f = write_tmp(s);
        let err = RatchetFile::load(f.path()).unwrap_err();
        assert!(matches!(err, RatchetError::Regex { idx: 0, .. }));
    }

    #[test]
    fn matches_gate_only() {
        let mut e = RatchetEntry {
            gate: GateId::BugAssumption,
            crate_scope: None,
            file: None,
            reason: "x".into(),
            signed_by: "a@b.com".into(),
            opened: date!(2026 - 01 - 01),
            expires_after: date!(2026 - 12 - 31),
            file_re: None,
            crate_validated: None,
            signed_by_validated: None,
        };
        e.validate(0).unwrap();
        assert!(e.matches(GateId::BugAssumption, None, None));
        assert!(!e.matches(GateId::ForbiddenCall, None, None));
    }

    #[test]
    fn matches_with_crate_scope() {
        let mut e = RatchetEntry {
            gate: GateId::BugAssumption,
            crate_scope: Some("engine-fs".into()),
            file: None,
            reason: "x".into(),
            signed_by: "a@b.com".into(),
            opened: date!(2026 - 01 - 01),
            expires_after: date!(2026 - 12 - 31),
            file_re: None,
            crate_validated: None,
            signed_by_validated: None,
        };
        e.validate(0).unwrap();
        let fs = CrateName::new("engine-fs").unwrap();
        let other = CrateName::new("engine-net").unwrap();
        assert!(e.matches(GateId::BugAssumption, Some(&fs), None));
        assert!(!e.matches(GateId::BugAssumption, Some(&other), None));
        assert!(!e.matches(GateId::BugAssumption, None, None));
    }

    #[test]
    fn matches_with_file_regex() {
        let mut e = RatchetEntry {
            gate: GateId::BugAssumption,
            crate_scope: None,
            file: Some("^src/legacy/".into()),
            reason: "x".into(),
            signed_by: "a@b.com".into(),
            opened: date!(2026 - 01 - 01),
            expires_after: date!(2026 - 12 - 31),
            file_re: None,
            crate_validated: None,
            signed_by_validated: None,
        };
        e.validate(0).unwrap();
        assert!(e.matches(
            GateId::BugAssumption,
            None,
            Some(Path::new("src/legacy/api.rs"))
        ));
        assert!(!e.matches(
            GateId::BugAssumption,
            None,
            Some(Path::new("src/modern.rs"))
        ));
        assert!(!e.matches(GateId::BugAssumption, None, None));
    }

    #[test]
    fn expiry_partition() {
        let mut f = RatchetFile {
            entries: vec![
                RatchetEntry {
                    gate: GateId::BugAssumption,
                    crate_scope: None,
                    file: None,
                    reason: "expired".into(),
                    signed_by: "a@b.com".into(),
                    opened: date!(2025 - 01 - 01),
                    expires_after: date!(2025 - 12 - 31),
                    file_re: None,
                    crate_validated: None,
                    signed_by_validated: None,
                },
                RatchetEntry {
                    gate: GateId::ForbiddenCall,
                    crate_scope: None,
                    file: None,
                    reason: "active".into(),
                    signed_by: "a@b.com".into(),
                    opened: date!(2026 - 01 - 01),
                    expires_after: date!(2027 - 01 - 01),
                    file_re: None,
                    crate_validated: None,
                    signed_by_validated: None,
                },
            ],
        };
        f.validate().unwrap();

        let now = date!(2026 - 06 - 01);
        assert_eq!(f.active(now).len(), 1);
        assert_eq!(f.expired(now).len(), 1);
    }

    #[test]
    fn covers_skips_expired() {
        let mut f = RatchetFile {
            entries: vec![RatchetEntry {
                gate: GateId::BugAssumption,
                crate_scope: None,
                file: None,
                reason: "x".into(),
                signed_by: "a@b.com".into(),
                opened: date!(2025 - 01 - 01),
                expires_after: date!(2025 - 12 - 31),
                file_re: None,
                crate_validated: None,
                signed_by_validated: None,
            }],
        };
        f.validate().unwrap();
        let now = date!(2026 - 06 - 01);
        assert!(!f.covers(GateId::BugAssumption, None, None, now));
    }

    proptest! {
        #[test]
        fn matches_is_strict_on_gate_id(
            gate_idx in 0usize..GateId::ALL.len(),
            other_idx in 0usize..GateId::ALL.len(),
        ) {
            let gate = GateId::ALL[gate_idx];
            let other = GateId::ALL[other_idx];
            // skip un-ratchetable
            if !gate.ratchetable() { return Ok(()); }

            let mut e = RatchetEntry {
                gate,
                crate_scope: None,
                file: None,
                reason: "x".into(),
                signed_by: "a@b.com".into(),
                opened: date!(2026 - 01 - 01),
                expires_after: date!(2026 - 12 - 31),
                file_re: None,
                crate_validated: None,
                signed_by_validated: None,
            };
            e.validate(0).unwrap();

            prop_assert_eq!(e.matches(gate, None, None), true);
            if other != gate {
                prop_assert_eq!(e.matches(other, None, None), false);
            }
        }
    }
}
