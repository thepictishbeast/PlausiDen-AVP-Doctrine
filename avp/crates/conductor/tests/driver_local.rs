//! Integration tests for [`conductor::driver_local::LocalClaudeDriver`].
//!
//! These tests live in `tests/` (rather than inline in the `driver_local`
//! module) because the `CARGO_BIN_EXE_claude-fixture` env var that
//! resolves the test fixture's path is only set by Cargo for integration
//! tests, not for unit tests inside a bin/lib crate.
//!
//! Each test starts a session pointed at the local `claude-fixture`
//! binary (`crates/conductor/src/bin/claude-fixture.rs`) and exercises
//! one path through the FSM via that fixture's scripted scenarios.

use std::{path::PathBuf, time::Duration};

use conductor::driver_local::LocalClaudeDriver;
use conductor_core::{
    ClaudeDriver, DriverError, DriverEvent, PauseReason, SessionHandle, SessionId,
};

fn fixture_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_claude-fixture"))
}

fn driver() -> LocalClaudeDriver {
    LocalClaudeDriver::with_bin(fixture_bin())
}

fn sid(name: &str) -> SessionId {
    SessionId(name.to_owned())
}

async fn drain_until_terminal(d: &LocalClaudeDriver, h: &SessionHandle) -> Vec<DriverEvent> {
    let mut all: Vec<DriverEvent> = Vec::new();
    for _ in 0..400 {
        let batch = d.poll(h).await.unwrap();
        all.extend(batch);
        if all
            .iter()
            .any(|e| matches!(e, DriverEvent::Done | DriverEvent::Failed { .. }))
        {
            return all;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    all
}

#[tokio::test]
async fn happy_path_spawns_streams_and_completes() {
    let d = driver();
    let cwd = std::env::temp_dir();
    let h = d
        .start(&sid("happy"), &cwd, "FIXTURE=happy")
        .await
        .expect("start");
    let events = drain_until_terminal(&d, &h).await;
    assert!(
        events.iter().any(|e| matches!(e, DriverEvent::Done)),
        "expected Done, got {events:?}"
    );
    let logs: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            DriverEvent::Log { line } => Some(line.clone()),
            _ => None,
        })
        .collect();
    assert!(
        logs.iter().any(|l| l.contains("hello")),
        "expected fixture text in logs, got {logs:?}"
    );
}

#[tokio::test]
async fn permission_exit_pauses() {
    let d = driver();
    let cwd = std::env::temp_dir();
    let h = d
        .start(&sid("perm"), &cwd, "FIXTURE=permission")
        .await
        .unwrap();
    let events = drain_until_terminal(&d, &h).await;
    assert!(
        events.iter().any(|e| matches!(
            e,
            DriverEvent::Paused {
                reason: PauseReason::Permission
            }
        )),
        "expected permission pause, got {events:?}"
    );
}

#[tokio::test]
async fn rate_limit_event_pauses() {
    let d = driver();
    let cwd = std::env::temp_dir();
    let h = d
        .start(&sid("rate"), &cwd, "FIXTURE=rate_limit")
        .await
        .unwrap();
    let events = drain_until_terminal(&d, &h).await;
    assert!(
        events.iter().any(|e| matches!(
            e,
            DriverEvent::Paused {
                reason: PauseReason::RateLimit
            }
        )),
        "expected rate-limit pause, got {events:?}"
    );
}

#[tokio::test]
async fn unknown_session_poll_errors() {
    let d = driver();
    let bad = SessionHandle::new(sid("ghost"), "x".into());
    let err = d.poll(&bad).await.unwrap_err();
    assert!(matches!(err, DriverError::UnknownSession(_)));
}

#[tokio::test]
async fn kill_removes_session() {
    let d = driver();
    let cwd = std::env::temp_dir();
    let h = d.start(&sid("k"), &cwd, "FIXTURE=hang").await.unwrap();
    d.kill(&h).await.unwrap();
    let err = d.poll(&h).await.unwrap_err();
    assert!(matches!(err, DriverError::UnknownSession(_)));
}

#[tokio::test]
async fn invalid_args_exit_2_fails() {
    let d = driver();
    let cwd = std::env::temp_dir();
    let h = d
        .start(&sid("inv"), &cwd, "FIXTURE=invalid_args")
        .await
        .unwrap();
    let events = drain_until_terminal(&d, &h).await;
    assert!(
        events
            .iter()
            .any(|e| matches!(e, DriverEvent::Failed { .. })),
        "expected Failed, got {events:?}"
    );
}

#[tokio::test]
async fn captures_claude_session_id_from_init() {
    // The happy fixture emits `{"type":"system","subtype":"init",
    // "session_id":"sess-happy"}` as its first line. After the session
    // terminates the driver should have captured that id so a future
    // resume() will pass it via `--resume`.
    let d = driver();
    let cwd = std::env::temp_dir();
    let h = d
        .start(&sid("capture"), &cwd, "FIXTURE=happy")
        .await
        .unwrap();
    drain_until_terminal(&d, &h).await;
    assert_eq!(
        d.captured_session_id(&h).await.as_deref(),
        Some("sess-happy")
    );
}
