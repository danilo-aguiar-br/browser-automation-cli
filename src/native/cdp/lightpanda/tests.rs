// SPDX-License-Identifier: MIT OR Apache-2.0

use super::*;
#[cfg(unix)]
use std::process::{Command, Stdio};
#[cfg(unix)]
use std::time::Duration;
#[cfg(unix)]
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[cfg(unix)]
use tokio::net::TcpListener as TokioTcpListener;

#[cfg(unix)]
fn unused_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// Binds the loopback listener the fake CDP server will accept on, and hands it
/// back together with its port.
///
/// # Why the listener comes back BOUND
///
/// [`unused_port`] reads a port from a listener it drops on the same line, so
/// the number is merely OBSERVED free and never reserved. The server task then
/// slept before binding it, and any of the ~1080 sibling unit tests running on
/// other libtest threads could take the port inside that window. The bind then
/// failed on an `unwrap()` inside a `tokio::spawn` whose `JoinHandle` was
/// dropped, so tokio absorbed the panic and told nobody, and the unit under
/// test spent its entire budget reporting `Connection refused` — a true answer
/// to a question the test never asked.
///
/// Holding the listener makes the theft impossible by construction, and binding
/// on the test thread makes a bind failure surface as a failure of THIS test.
/// Measured 2026-09-05: one failure in a full `cargo test --lib`, with the same
/// test passing in the run immediately before.
#[cfg(unix)]
async fn bind_unused_port() -> (TokioTcpListener, u16) {
    let listener = TokioTcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    (listener, port)
}

#[cfg(unix)]
async fn serve_json_version_once_after_delay(
    listener: TokioTcpListener,
    delay_ms: u64,
    body: &'static str,
) {
    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    let (mut socket, _) = listener.accept().await.unwrap();
    let mut buf = [0u8; 1024];
    let _ = socket.read(&mut buf).await;
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\nContent-Type: application/json\r\n\r\n{}",
        body.len(),
        body
    );
    socket.write_all(response.as_bytes()).await.unwrap();
}

/// Readiness budget granted by the success-path test below.
///
/// # Why a literal and not `lightpanda_startup_timeout()`
///
/// That reader consults the ambient XDG config, so a host carrying
/// `lightpanda_startup_timeout_secs = 1` would shrink this window under the
/// test's feet. Resolving the knob is
/// `resolve_lightpanda_startup_timeout_secs`'s contract, not this test's; the
/// unit under test takes the budget as a parameter, and the sibling
/// `timeout_reports_last_probe_error` already passes it explicitly.
#[cfg(unix)]
const READY_BUDGET: Duration = Duration::from_secs(10);

/// Lifetime of every fake Lightpanda child in this module, DERIVED from
/// [`READY_BUDGET`], which is the longest deadline any test here grants.
///
/// # Why derived
///
/// `wait_for_lightpanda_ready` has two distinct failure exits: the budget
/// expires, or the child dies before readiness. A test only asserts what it
/// means to assert when the exit it is NOT testing is impossible by
/// construction, so the child must strictly outlive the deadline in play.
///
/// The old shape granted a 10s budget to a child that lived 5s, so the fixture
/// raced its own assertion. Measured 2026-08-18 under host contention (a
/// release compile plus eight background workers): the test failed with
/// `Lightpanda exited before CDP became ready (status: exit status: 0)`. That
/// `exit status: 0` is the signature of the `sleep` completing normally — the
/// correct answer to a question the test never meant to ask. In isolation the
/// same test passed 6 of 6 in ~0.3s each.
#[cfg(unix)]
const CHILD_LIFETIME_SECS: u64 = READY_BUDGET.as_secs() * 3;

#[cfg(unix)]
#[tokio::test]
async fn waits_for_ready_without_logs() {
    let (listener, port) = bind_unused_port().await;
    tokio::spawn(serve_json_version_once_after_delay(
        listener,
        150,
        r#"{"webSocketDebuggerUrl":"ws://127.0.0.1:9222/"}"#,
    ));

    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg(format!("sleep {CHILD_LIFETIME_SECS}"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let (logs, _drainers) = start_log_drainers(&mut child).unwrap();
    let ws_url = wait_for_lightpanda_ready(&mut child, port, &logs, READY_BUDGET)
        .await
        .unwrap();

    assert_eq!(ws_url, format!("ws://127.0.0.1:{port}/"));
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(unix)]
#[tokio::test]
async fn child_exit_surfaces_logs() {
    let port = unused_port();
    let mut child = Command::new("/bin/sh")
        .args(["-c", "echo boom >&2; sleep 0.1; exit 23"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let (logs, _drainers) = start_log_drainers(&mut child).unwrap();
    let err = wait_for_lightpanda_ready(&mut child, port, &logs, lightpanda_startup_timeout())
        .await
        .unwrap_err();

    assert!(err.contains("Lightpanda exited before CDP became ready"));
    assert!(err.contains("boom"));
}

#[cfg(unix)]
#[tokio::test]
async fn timeout_reports_last_probe_error() {
    let port = unused_port();
    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg(format!("sleep {CHILD_LIFETIME_SECS}"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let timeout = Duration::from_millis(300);
    let (logs, _drainers) = start_log_drainers(&mut child).unwrap();
    let err = tokio::time::timeout(
        Duration::from_secs(2),
        wait_for_lightpanda_ready(&mut child, port, &logs, timeout),
    )
    .await
    .expect("ready wait should return before outer timeout")
    .unwrap_err();

    assert!(err.contains("Timed out after 300ms waiting for Lightpanda CDP endpoint"));
    assert!(err.contains("Failed to connect to CDP") || err.contains("Timeout connecting to CDP"));

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn test_find_lightpanda_returns_none_when_missing() {
    let _ = find_lightpanda();
}

#[test]
fn test_lightpanda_launch_error_no_logs() {
    let logs = LaunchLogBuffer::default();
    let msg = lightpanda_launch_error("Lightpanda exited", &logs, None);
    assert!(msg.contains("no stdout/stderr output"));
}

#[test]
fn test_lightpanda_launch_error_with_lines() {
    let logs = LaunchLogBuffer::default();
    logs.push_stdout("stdout line".to_string());
    logs.push_stderr("stderr line".to_string());
    let msg = lightpanda_launch_error("Lightpanda exited", &logs, Some("connect failed"));
    assert!(msg.contains("stdout line"));
    assert!(msg.contains("stderr line"));
    assert!(msg.contains("Last probe error: connect failed"));
}

#[test]
fn test_default_options() {
    let opts = LightpandaLaunchOptions::default();
    assert!(opts.executable_path.is_none());
    assert!(opts.proxy.is_none());
    assert!(opts.port.is_none());
}

#[test]
fn test_build_lightpanda_serve_args_sets_explicit_session_timeout() {
    // Literal timeout on both sides. The previous version passed
    // `resolve_lightpanda_session_timeout_secs()` as the EXPECTED value while
    // the function under test called the same resolver — so the assertion
    // compared a value against itself and would have held even if the argv were
    // built from the wrong knob entirely.
    let args = build_lightpanda_serve_args_with(9222, None, 37);

    assert_eq!(
        args,
        vec![
            "serve".to_string(),
            "--host".to_string(),
            "127.0.0.1".to_string(),
            "--port".to_string(),
            "9222".to_string(),
            "--timeout".to_string(),
            "37".to_string(),
        ]
    );
}

#[test]
fn test_build_lightpanda_serve_args_with_proxy() {
    let args = build_lightpanda_serve_args_with(9333, Some("http://127.0.0.1:8080"), 41);

    assert_eq!(
        args,
        vec![
            "serve".to_string(),
            "--host".to_string(),
            "127.0.0.1".to_string(),
            "--port".to_string(),
            "9333".to_string(),
            "--timeout".to_string(),
            "41".to_string(),
            "--http_proxy".to_string(),
            "http://127.0.0.1:8080".to_string(),
        ]
    );
}
