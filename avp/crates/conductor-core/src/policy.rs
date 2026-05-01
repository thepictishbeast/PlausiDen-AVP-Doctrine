//! Pause taxonomy + recovery policy.
//!
//! Every Claude Code pause falls into one of four named buckets. Each
//! bucket has a deterministic recovery action — never silent retry,
//! never blanket bypass. The supersociety answer to "why does Claude
//! pause" is *configuration*, not *circumvention*.

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Why a session paused. `#[non_exhaustive]` so the doctrine can
/// add new categories without breaking consumers.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
pub enum PauseReason {
    /// Anthropic API rate-limited us. Recovery: exponential backoff.
    RateLimit,
    /// Local network blip / DNS / TLS handshake fail. Recovery: short
    /// backoff, fewer retries than a rate-limit.
    Network,
    /// Context window saturated; Claude Code triggered compaction.
    /// Recovery: `claude --continue <session-id>` once compaction completes.
    ContextCompaction,
    /// Permission prompt that *isn't* on the per-repo allowlist (i.e.
    /// not a routine edit; could be destructive). Recovery: escalate
    /// to a human; no auto-allow.
    Permission,
    /// Subprocess hit an irrecoverable error (parser corruption,
    /// 5xx storm, hostile diff). Recovery: escalate.
    Blocked,
}

impl PauseReason {
    /// Stable kebab-case label for logs.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::RateLimit => "rate-limit",
            Self::Network => "network",
            Self::ContextCompaction => "context-compaction",
            Self::Permission => "permission",
            Self::Blocked => "blocked",
        }
    }

    /// Whether the supervisor should attempt automatic recovery
    /// (vs. escalating immediately on first occurrence).
    #[must_use]
    pub const fn auto_recoverable(self) -> bool {
        matches!(
            self,
            Self::RateLimit | Self::Network | Self::ContextCompaction
        )
    }
}

/// What the supervisor should do given a (PauseReason, retry-count) pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecoveryAction {
    /// Wait `delay`, then resume the session (`Resumed → Running`).
    ResumeAfter {
        /// Duration to wait before resume.
        delay: Duration,
    },
    /// Escalate the session to a human (open GH issue, mark Failed).
    Escalate {
        /// Reason text for the escalation message.
        reason: &'static str,
    },
}

/// Policy governing recovery decisions. Default values come from the
/// doctrine; siblings can override per-conductor-config in v0.2.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct RecoveryPolicy {
    /// Max number of automatic resumes before escalating, regardless of
    /// PauseReason. Bounds runaway backoff.
    pub max_resumes: u32,
    /// Initial backoff duration for `RateLimit` and `Network`.
    pub base_backoff: Duration,
    /// Backoff cap (no single sleep is longer than this).
    pub max_backoff: Duration,
}

impl Default for RecoveryPolicy {
    fn default() -> Self {
        Self {
            max_resumes: 6,
            base_backoff: Duration::from_secs(15),
            max_backoff: Duration::from_secs(15 * 60),
        }
    }
}

impl RecoveryPolicy {
    /// Decide what to do given a pause and how many times this session
    /// has already been resumed.
    #[must_use]
    pub fn decide(self, reason: PauseReason, resume_count: u32) -> RecoveryAction {
        if resume_count >= self.max_resumes {
            return RecoveryAction::Escalate {
                reason: "max resumes exceeded; supervisor giving up",
            };
        }
        if !reason.auto_recoverable() {
            return RecoveryAction::Escalate {
                reason: match reason {
                    PauseReason::Permission => "non-allowlisted permission prompt",
                    PauseReason::Blocked => "irrecoverable blocked state",
                    _ => "unhandled pause",
                },
            };
        }
        // Exponential backoff: base * 2^resume_count, capped.
        let factor: u64 = 1u64.checked_shl(resume_count.min(31)).unwrap_or(u64::MAX);
        let raw = self
            .base_backoff
            .saturating_mul(factor.try_into().unwrap_or(u32::MAX));
        let delay = raw.min(self.max_backoff);
        RecoveryAction::ResumeAfter { delay }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_recoverable_set() {
        assert!(PauseReason::RateLimit.auto_recoverable());
        assert!(PauseReason::Network.auto_recoverable());
        assert!(PauseReason::ContextCompaction.auto_recoverable());
        assert!(!PauseReason::Permission.auto_recoverable());
        assert!(!PauseReason::Blocked.auto_recoverable());
    }

    #[test]
    fn permission_escalates_immediately() {
        let p = RecoveryPolicy::default();
        let a = p.decide(PauseReason::Permission, 0);
        assert!(matches!(a, RecoveryAction::Escalate { .. }));
    }

    #[test]
    fn rate_limit_first_resume_uses_base_backoff() {
        let p = RecoveryPolicy::default();
        let RecoveryAction::ResumeAfter { delay } = p.decide(PauseReason::RateLimit, 0) else {
            panic!("expected resume");
        };
        assert_eq!(delay, p.base_backoff);
    }

    #[test]
    fn rate_limit_doubles_with_each_resume() {
        let p = RecoveryPolicy::default();
        let RecoveryAction::ResumeAfter { delay: d1 } = p.decide(PauseReason::RateLimit, 1) else {
            panic!()
        };
        let RecoveryAction::ResumeAfter { delay: d2 } = p.decide(PauseReason::RateLimit, 2) else {
            panic!()
        };
        assert_eq!(d1, p.base_backoff * 2);
        assert_eq!(d2, p.base_backoff * 4);
    }

    #[test]
    fn backoff_caps_at_max() {
        // Use a large max_resumes so we exercise the *backoff* cap, not
        // the *resume-count* cap — those are different limits.
        let p = RecoveryPolicy {
            max_resumes: u32::MAX,
            ..RecoveryPolicy::default()
        };
        let RecoveryAction::ResumeAfter { delay } = p.decide(PauseReason::RateLimit, 30) else {
            panic!("expected ResumeAfter after large resume_count")
        };
        assert_eq!(delay, p.max_backoff);
    }

    #[test]
    fn max_resumes_escalates() {
        let p = RecoveryPolicy {
            max_resumes: 3,
            ..RecoveryPolicy::default()
        };
        let a = p.decide(PauseReason::RateLimit, 3);
        assert!(matches!(a, RecoveryAction::Escalate { .. }));
        let a = p.decide(PauseReason::RateLimit, 100);
        assert!(matches!(a, RecoveryAction::Escalate { .. }));
    }

    #[test]
    fn label_is_stable() {
        assert_eq!(PauseReason::RateLimit.label(), "rate-limit");
        assert_eq!(PauseReason::ContextCompaction.label(), "context-compaction");
    }

    #[test]
    fn pause_reason_serde_round_trip() {
        let r = PauseReason::ContextCompaction;
        let s = serde_json::to_string(&r).unwrap();
        assert_eq!(s, "\"context-compaction\"");
        let back: PauseReason = serde_json::from_str(&s).unwrap();
        assert_eq!(back, r);
    }
}
