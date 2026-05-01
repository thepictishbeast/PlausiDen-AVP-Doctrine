//! Parser for `claude --output-format=stream-json` NDJSON events.
//!
//! Schema reference: doctrine memory `reference_claude_cli_schema.md`,
//! sourced from Claude Code's CLI / headless / streaming-output docs.
//!
//! Defensive parsing: the schema is stable but field additions are
//! expected. Every variant carries `#[serde(other)]` fallbacks so a
//! future CLI release adding a new event type doesn't break our parser
//! — unknown events surface as `Other` and are mapped to
//! `DriverEvent::Log`.
//!
//! Done-determination is **not** event-based. `message_stop` can occur
//! multiple times within one session (tool-use loops). The driver
//! therefore signals `DriverEvent::Done` only when the child process
//! exits with code 0; this parser is concerned only with the streaming
//! lifecycle, not with terminal classification.

// AVP-PASS-2026-05-01: dead_code allowed module-wide while the
// LocalClaudeDriver (next commit) is in flight. Once the driver wires
// up parse_line + map_event, this attribute becomes redundant and
// will be removed.
#![allow(dead_code)]

use conductor_core::PauseReason;
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────
// Top-level event
// ─────────────────────────────────────────────────────────────────────────

/// One NDJSON line from `claude --output-format=stream-json`.
///
/// AVP-PASS-2026-05-01: tagged with `type`. Forward-compat fallback via
/// `Other`. Field shapes match the agent-sdk/streaming-output reference.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum ClaudeEvent {
    /// `type: "system"` event — session lifecycle, retry signals, plugin
    /// install progress, compaction boundaries.
    System(SystemEvent),
    /// `type: "stream_event"` — wraps a raw Anthropic API streaming event.
    StreamEvent(StreamEventOuter),
    /// Forward-compat: any unknown `type` value.
    #[serde(other)]
    Other,
}

// ─────────────────────────────────────────────────────────────────────────
// System events
// ─────────────────────────────────────────────────────────────────────────

/// `type: "system"` event payload. The `subtype` field disambiguates;
/// `details` is a free-form JSON value carrying subtype-specific fields
/// (we don't need their shapes for the FSM mapping).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub(crate) struct SystemEvent {
    /// Variant identifier (`init`, `api_retry`, …).
    pub subtype: SystemSubtype,
    /// Short uuid for this event (used for trace correlation).
    #[serde(default)]
    pub uuid: Option<String>,
    /// Claude-side session id. Always present for `init`.
    #[serde(default)]
    pub session_id: Option<String>,
    /// Subtype-specific fields. We deserialize into a `serde_json::Value`
    /// so we can pluck `error`, `retry_delay_ms`, etc. without a richer
    /// schema per variant.
    #[serde(flatten)]
    pub details: serde_json::Value,
}

/// Known `subtype` values for `type: "system"` events.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub(crate) enum SystemSubtype {
    /// Session initialization. The `session_id` on the wrapper is
    /// authoritative; capture it for resume.
    Init,
    /// Rate-limit / retryable-error indicator. The CLI surfaces this
    /// before retrying internally; we treat it as a pause signal so the
    /// supervisor can decide whether to wait or escalate.
    ApiRetry,
    /// Plugin install progress (irrelevant for the FSM).
    PluginInstall,
    /// Compaction boundary marker (informational).
    CompactBoundary,
    /// Future variants land here.
    #[serde(other)]
    Other,
}

impl SystemEvent {
    /// Pull the `error` string from a `subtype: api_retry` event.
    /// Returns `None` if the field is absent or not a string.
    #[must_use]
    pub(crate) fn retry_error(&self) -> Option<&str> {
        self.details.get("error").and_then(|v| v.as_str())
    }

    /// Pull the `retry_delay_ms` from a `subtype: api_retry` event.
    #[must_use]
    pub(crate) fn retry_delay_ms(&self) -> Option<u64> {
        self.details
            .get("retry_delay_ms")
            .and_then(serde_json::Value::as_u64)
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Stream events (wraps Anthropic API events)
// ─────────────────────────────────────────────────────────────────────────

/// Outer wrapper for `type: "stream_event"`. Carries metadata + the
/// nested raw API event.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub(crate) struct StreamEventOuter {
    /// Short uuid.
    #[serde(default)]
    pub uuid: Option<String>,
    /// Claude-side session id (consistent with the init event).
    #[serde(default)]
    pub session_id: Option<String>,
    /// Optional parent tool use id (for nested tool-use sequences).
    #[serde(default)]
    pub parent_tool_use_id: Option<String>,
    /// The raw API event payload.
    pub event: ApiStreamEvent,
}

/// Raw Anthropic API streaming event types we care about. Anything we
/// don't model lands in `Other`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum ApiStreamEvent {
    /// Beginning of a message (assistant or user).
    MessageStart {
        /// Raw `message` payload.
        #[serde(default)]
        message: serde_json::Value,
    },
    /// Beginning of a content block (text or tool use).
    ContentBlockStart {
        /// Block index in the message.
        #[serde(default)]
        index: Option<u32>,
        /// `content_block` payload.
        #[serde(default)]
        content_block: serde_json::Value,
    },
    /// Incremental update to a content block.
    ContentBlockDelta {
        /// Block index.
        #[serde(default)]
        index: Option<u32>,
        /// Delta payload — `text_delta` for text, `input_json_delta`
        /// for tool-use args.
        #[serde(default)]
        delta: ContentDelta,
    },
    /// End of a content block.
    ContentBlockStop {
        /// Block index.
        #[serde(default)]
        index: Option<u32>,
    },
    /// Message-level update (stop reason, usage).
    MessageDelta {
        /// Delta payload.
        #[serde(default)]
        delta: serde_json::Value,
        /// Optional usage tally.
        #[serde(default)]
        usage: serde_json::Value,
    },
    /// Message complete. Doesn't necessarily mean session-done — only
    /// the child process exit code does.
    MessageStop,
    /// Forward-compat fallback.
    #[serde(other)]
    Other,
}

/// `delta` payload for `content_block_delta`. We surface the actual
/// text / tool-input fragments so the supervisor can show them in the
/// log stream.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub(crate) enum ContentDelta {
    /// Streaming text from the assistant.
    TextDelta {
        /// New text fragment.
        #[serde(default)]
        text: String,
    },
    /// Streaming JSON for a tool's input.
    InputJsonDelta {
        /// New JSON fragment (raw string).
        #[serde(default)]
        partial_json: String,
    },
    /// Forward-compat fallback.
    #[serde(other)]
    #[default]
    Other,
}

// ─────────────────────────────────────────────────────────────────────────
// Parsing
// ─────────────────────────────────────────────────────────────────────────

/// Parse one NDJSON line. Empty / whitespace-only lines yield `None`
/// (so callers can skip them without ceremony). JSON parse failures
/// also yield `None` — the line then falls back to a `Log` event in
/// the driver, preserving the user's view of what claude printed.
#[must_use]
pub(crate) fn parse_line(line: &str) -> Option<ClaudeEvent> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    serde_json::from_str(trimmed).ok()
}

// ─────────────────────────────────────────────────────────────────────────
// Mapping into DriverEvent
// ─────────────────────────────────────────────────────────────────────────

/// Map one parsed `ClaudeEvent` to the supervisor-facing event types.
///
/// Returns `Mapped::DriverEvent(...)` when the line should produce a
/// `DriverEvent` for the supervisor; `Mapped::SessionId(...)` when the
/// line carried a Claude-side session id we should capture (for
/// resume); `Mapped::Skip` when the line is just informational.
pub(crate) fn map_event(event: &ClaudeEvent) -> Mapped {
    match event {
        ClaudeEvent::System(sys) => map_system(sys),
        ClaudeEvent::StreamEvent(outer) => map_stream(outer),
        ClaudeEvent::Other => Mapped::Skip,
    }
}

fn map_system(sys: &SystemEvent) -> Mapped {
    match sys.subtype {
        SystemSubtype::Init => sys
            .session_id
            .as_ref()
            .map_or(Mapped::Skip, |id| Mapped::SessionId(id.clone())),
        SystemSubtype::ApiRetry => {
            // `error: "rate_limit"` → RateLimit. Anything else under
            // api_retry (auth, server, network) → Network.
            let reason = match sys.retry_error() {
                Some("rate_limit") => PauseReason::RateLimit,
                Some(_) | None => PauseReason::Network,
            };
            Mapped::DriverEvent(MappedDriverEvent::Paused { reason })
        }
        SystemSubtype::CompactBoundary => Mapped::DriverEvent(MappedDriverEvent::Paused {
            reason: PauseReason::ContextCompaction,
        }),
        SystemSubtype::PluginInstall | SystemSubtype::Other => Mapped::Skip,
    }
}

fn map_stream(outer: &StreamEventOuter) -> Mapped {
    let session_capture = outer
        .session_id
        .as_ref()
        .map(|id| Mapped::SessionId(id.clone()));

    let driver = match &outer.event {
        ApiStreamEvent::ContentBlockDelta {
            delta: ContentDelta::TextDelta { text },
            ..
        } => {
            // Empty deltas are noise; skip them.
            if text.is_empty() {
                return session_capture.unwrap_or(Mapped::Skip);
            }
            Some(MappedDriverEvent::Log { line: text.clone() })
        }
        ApiStreamEvent::ContentBlockDelta {
            delta: ContentDelta::InputJsonDelta { partial_json },
            ..
        } => {
            if partial_json.is_empty() {
                return session_capture.unwrap_or(Mapped::Skip);
            }
            Some(MappedDriverEvent::Log {
                line: format!("(tool-input) {partial_json}"),
            })
        }
        ApiStreamEvent::MessageStart { .. }
        | ApiStreamEvent::ContentBlockStart { .. }
        | ApiStreamEvent::ContentBlockStop { .. }
        | ApiStreamEvent::MessageDelta { .. }
        | ApiStreamEvent::MessageStop
        | ApiStreamEvent::ContentBlockDelta { .. }
        | ApiStreamEvent::Other => None,
    };

    match (driver, session_capture) {
        (Some(d), _) => Mapped::DriverEvent(d),
        (None, Some(sc)) => sc,
        (None, None) => Mapped::Skip,
    }
}

/// Result of `map_event`. `MappedDriverEvent` mirrors `conductor_core::DriverEvent`
/// without the `Done` / `Failed` variants — those come from the child's
/// exit status, not from the JSON stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Mapped {
    /// Surface this as a supervisor-visible event.
    DriverEvent(MappedDriverEvent),
    /// Capture the Claude-side session id (for `--resume`).
    SessionId(String),
    /// Skip this line entirely.
    Skip,
}

/// Subset of `conductor_core::DriverEvent` that this parser can produce
/// from a single NDJSON line. Lacks `Done` and `Failed` because those
/// require process-exit context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MappedDriverEvent {
    /// A loggable line.
    Log {
        /// Line text.
        line: String,
    },
    /// A pause signal.
    Paused {
        /// Pause taxonomy.
        reason: PauseReason,
    },
}

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_line_is_none() {
        assert!(parse_line("").is_none());
        assert!(parse_line("   ").is_none());
        assert!(parse_line("\n").is_none());
    }

    #[test]
    fn parse_garbage_is_none() {
        assert!(parse_line("not json").is_none());
        assert!(parse_line("{bad").is_none());
    }

    #[test]
    fn parse_unknown_top_level_type_is_other() {
        let line = r#"{"type": "future_event_kind", "uuid": "x"}"#;
        let e = parse_line(line).unwrap();
        assert!(matches!(e, ClaudeEvent::Other));
    }

    #[test]
    fn parse_system_init() {
        let line = r#"{"type":"system","subtype":"init","uuid":"u","session_id":"sess-1","model":"claude"}"#;
        let e = parse_line(line).unwrap();
        let ClaudeEvent::System(sys) = e else {
            panic!()
        };
        assert_eq!(sys.subtype, SystemSubtype::Init);
        assert_eq!(sys.session_id.as_deref(), Some("sess-1"));
    }

    #[test]
    fn parse_system_api_retry_rate_limit() {
        let line = r#"{"type":"system","subtype":"api_retry","uuid":"u","session_id":"s","attempt":1,"max_retries":4,"retry_delay_ms":12000,"error":"rate_limit","error_status":429}"#;
        let e = parse_line(line).unwrap();
        let ClaudeEvent::System(sys) = e else {
            panic!()
        };
        assert_eq!(sys.subtype, SystemSubtype::ApiRetry);
        assert_eq!(sys.retry_error(), Some("rate_limit"));
        assert_eq!(sys.retry_delay_ms(), Some(12000));
    }

    #[test]
    fn map_init_captures_session_id() {
        let line = r#"{"type":"system","subtype":"init","session_id":"sess-1"}"#;
        let e = parse_line(line).unwrap();
        let m = map_event(&e);
        assert_eq!(m, Mapped::SessionId("sess-1".to_owned()));
    }

    #[test]
    fn map_api_retry_rate_limit_pauses() {
        let line = r#"{"type":"system","subtype":"api_retry","error":"rate_limit"}"#;
        let e = parse_line(line).unwrap();
        let m = map_event(&e);
        assert_eq!(
            m,
            Mapped::DriverEvent(MappedDriverEvent::Paused {
                reason: PauseReason::RateLimit
            })
        );
    }

    #[test]
    fn map_api_retry_other_error_maps_to_network() {
        let line = r#"{"type":"system","subtype":"api_retry","error":"server_error"}"#;
        let e = parse_line(line).unwrap();
        let m = map_event(&e);
        assert_eq!(
            m,
            Mapped::DriverEvent(MappedDriverEvent::Paused {
                reason: PauseReason::Network
            })
        );
    }

    #[test]
    fn map_compact_boundary_pauses_for_compaction() {
        let line = r#"{"type":"system","subtype":"compact_boundary"}"#;
        let e = parse_line(line).unwrap();
        let m = map_event(&e);
        assert_eq!(
            m,
            Mapped::DriverEvent(MappedDriverEvent::Paused {
                reason: PauseReason::ContextCompaction
            })
        );
    }

    #[test]
    fn map_plugin_install_skips() {
        let line = r#"{"type":"system","subtype":"plugin_install","name":"x","status":"started"}"#;
        let e = parse_line(line).unwrap();
        let m = map_event(&e);
        assert_eq!(m, Mapped::Skip);
    }

    #[test]
    fn parse_stream_event_text_delta() {
        let line = r#"{"type":"stream_event","uuid":"u","session_id":"s","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"hello"}}}"#;
        let e = parse_line(line).unwrap();
        let m = map_event(&e);
        assert_eq!(
            m,
            Mapped::DriverEvent(MappedDriverEvent::Log {
                line: "hello".to_owned()
            })
        );
    }

    #[test]
    fn empty_text_delta_skips_but_captures_session() {
        let line = r#"{"type":"stream_event","session_id":"sess-9","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":""}}}"#;
        let e = parse_line(line).unwrap();
        let m = map_event(&e);
        assert_eq!(m, Mapped::SessionId("sess-9".to_owned()));
    }

    #[test]
    fn stream_input_json_delta_logs_with_prefix() {
        let line = r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"input_json_delta","partial_json":"{\"foo"}}}"#;
        let e = parse_line(line).unwrap();
        let m = map_event(&e);
        if let Mapped::DriverEvent(MappedDriverEvent::Log { line }) = m {
            assert!(line.starts_with("(tool-input) "));
        } else {
            panic!("expected log");
        }
    }

    #[test]
    fn stream_message_stop_skips() {
        // message_stop alone doesn't terminate — process exit does.
        let line = r#"{"type":"stream_event","event":{"type":"message_stop"}}"#;
        let e = parse_line(line).unwrap();
        let m = map_event(&e);
        assert_eq!(m, Mapped::Skip);
    }

    #[test]
    fn stream_unknown_event_type_skips() {
        let line = r#"{"type":"stream_event","event":{"type":"future_subkind","payload":42}}"#;
        let e = parse_line(line).unwrap();
        let m = map_event(&e);
        assert_eq!(m, Mapped::Skip);
    }

    #[test]
    fn stream_message_start_skips_unless_session_present() {
        let line = r#"{"type":"stream_event","event":{"type":"message_start","message":{"role":"assistant"}}}"#;
        let e = parse_line(line).unwrap();
        let m = map_event(&e);
        assert_eq!(m, Mapped::Skip);
    }

    #[test]
    fn unknown_subtype_falls_through_to_other() {
        let line = r#"{"type":"system","subtype":"future_subtype","payload":1}"#;
        let e = parse_line(line).unwrap();
        let ClaudeEvent::System(sys) = e else {
            panic!()
        };
        assert_eq!(sys.subtype, SystemSubtype::Other);
    }

    #[test]
    fn round_trip_serde_init() {
        let line = r#"{"type":"system","subtype":"init","session_id":"s"}"#;
        let e = parse_line(line).unwrap();
        let json = serde_json::to_string(&e).unwrap();
        let back: ClaudeEvent = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, ClaudeEvent::System(_)));
    }
}
