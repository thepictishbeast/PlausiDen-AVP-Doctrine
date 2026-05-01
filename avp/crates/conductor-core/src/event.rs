//! Outward-facing supervisor events. The CLI / external observer
//! subscribes to these to render progress and to pipe escalations into
//! GitHub issue creation.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{policy::PauseReason, session::SessionId};

/// One observable supervisor event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SupervisorEvent {
    /// Which session this event pertains to (None for supervisor-wide).
    pub session: Option<SessionId>,
    /// What happened.
    pub kind: SupervisorEventKind,
    /// Wall-clock time of emission.
    #[serde(with = "rfc3339")]
    pub at: OffsetDateTime,
}

impl SupervisorEvent {
    /// Construct stamped to "now".
    #[must_use]
    pub fn now(session: Option<SessionId>, kind: SupervisorEventKind) -> Self {
        Self {
            session,
            kind,
            at: OffsetDateTime::now_utc(),
        }
    }
}

/// Discriminator for [`SupervisorEvent`]. Named so JSON consumers can
/// pattern-match on the `kind.tag` field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tag", rename_all = "kebab-case")]
#[non_exhaustive]
pub enum SupervisorEventKind {
    /// Session entered the queue.
    Queued,
    /// Subprocess started.
    Started,
    /// Subprocess emitted a log line.
    Log {
        /// Line text.
        line: String,
    },
    /// Session paused.
    Paused {
        /// Why.
        reason: PauseReason,
    },
    /// Supervisor scheduled a resume.
    ResumeScheduled {
        /// Backoff seconds.
        delay_seconds: u64,
    },
    /// Session escalated to human.
    Escalated {
        /// Reason.
        reason: String,
    },
    /// Session reached a terminal state.
    Terminal {
        /// `done` or `failed`.
        outcome: TerminalOutcome,
    },
}

/// Terminal outcome label.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum TerminalOutcome {
    /// Subprocess exited 0 + success_test passed.
    Done,
    /// Anything else.
    Failed,
}

mod rfc3339 {
    use serde::{Deserialize, Deserializer, Serializer};
    use time::{OffsetDateTime, format_description::well_known::Rfc3339};

    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub(super) fn serialize<S: Serializer>(dt: &OffsetDateTime, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&dt.format(&Rfc3339).map_err(serde::ser::Error::custom)?)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        de: D,
    ) -> Result<OffsetDateTime, D::Error> {
        let s = String::deserialize(de)?;
        OffsetDateTime::parse(s.trim(), &Rfc3339).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_serde_round_trip() {
        let e = SupervisorEvent::now(
            Some(SessionId("alpha".into())),
            SupervisorEventKind::Paused {
                reason: PauseReason::RateLimit,
            },
        );
        let json = serde_json::to_string(&e).unwrap();
        let back: SupervisorEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, e);
    }

    #[test]
    fn kind_tag_visible_in_json() {
        let e = SupervisorEvent::now(None, SupervisorEventKind::Queued);
        let v: serde_json::Value = serde_json::to_value(&e).unwrap();
        assert_eq!(v["kind"]["tag"], "queued");
    }

    #[test]
    fn terminal_outcome_kebab() {
        let v = serde_json::to_value(TerminalOutcome::Done).unwrap();
        assert_eq!(v, serde_json::json!("done"));
    }
}
