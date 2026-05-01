//! `claude-fixture` — a fake `claude` binary used by the conductor
//! crate's `LocalClaudeDriver` tests.
//!
//! Driven by env var `FIXTURE=<scenario>` (passed via the test's
//! `system_prompt` argument, which the fixture inspects). Scenarios:
//!
//! - `happy` — emits a session.init event, a text_delta, then exits 0.
//! - `permission` — exits 1 after one log line (simulates a permission
//!   denial in `--print` mode).
//! - `rate_limit` — emits a `system.api_retry` rate_limit event, then
//!   exits 0.
//! - `invalid_args` — exits 2 (simulates malformed CLI flags).
//! - `hang` — sleeps until killed (used by kill() tests).
//!
//! The fixture reads the `--append-system-prompt <text>` argument and
//! parses `FIXTURE=<scenario>` out of it. This pattern keeps the fixture
//! invocation identical to a real claude call.

#![forbid(unsafe_code)]
#![allow(clippy::disallowed_macros)] // this *is* a binary; printing is the whole point

use std::{io::Write as _, process::ExitCode};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let prompt = args
        .iter()
        .position(|a| a == "--append-system-prompt")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_default();
    let scenario = prompt
        .split_whitespace()
        .find_map(|tok| tok.strip_prefix("FIXTURE="))
        .unwrap_or("happy");

    match scenario {
        "permission" => permission(),
        "rate_limit" => rate_limit(),
        "invalid_args" => ExitCode::from(2),
        "hang" => hang(),
        // "happy" and any unknown scenario fall through to the
        // happy-path fixture. Unknown scenarios should be loud in tests
        // (assertion failure on missing event) rather than silent.
        _ => happy(),
    }
}

fn emit(line: &str) {
    let stdout = std::io::stdout();
    let mut h = stdout.lock();
    let _ = writeln!(h, "{line}");
    let _ = h.flush();
}

fn happy() -> ExitCode {
    emit(r#"{"type":"system","subtype":"init","uuid":"u1","session_id":"sess-happy"}"#);
    emit(
        r#"{"type":"stream_event","uuid":"u2","session_id":"sess-happy","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"hello world"}}}"#,
    );
    emit(
        r#"{"type":"stream_event","uuid":"u3","session_id":"sess-happy","event":{"type":"message_stop"}}"#,
    );
    ExitCode::SUCCESS
}

fn permission() -> ExitCode {
    emit(r#"{"type":"system","subtype":"init","uuid":"u1","session_id":"sess-perm"}"#);
    let stderr = std::io::stderr();
    let mut h = stderr.lock();
    let _ = writeln!(h, "Tool 'BashWrite' requires permission");
    let _ = h.flush();
    ExitCode::from(1)
}

fn rate_limit() -> ExitCode {
    emit(r#"{"type":"system","subtype":"init","uuid":"u1","session_id":"sess-rate"}"#);
    emit(
        r#"{"type":"system","subtype":"api_retry","uuid":"u2","session_id":"sess-rate","attempt":1,"max_retries":4,"retry_delay_ms":15000,"error":"rate_limit","error_status":429}"#,
    );
    ExitCode::SUCCESS
}

fn hang() -> ExitCode {
    // Sleep forever; the test will kill us.
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}
