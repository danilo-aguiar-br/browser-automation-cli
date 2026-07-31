// SPDX-License-Identifier: MIT OR Apache-2.0

use super::*;
use crate::xdg::resolve_lightpanda_session_timeout_secs;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener as TokioTcpListener;

fn unused_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

async fn serve_json_version_once_after_delay(port: u16, delay_ms: u64, body: &'static str) {
    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    let listener = TokioTcpListener::bind(("127.0.0.1", port)).await.unwrap();
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

#[cfg(unix)]
#[tokio::test]
async fn waits_for_ready_without_logs() {
    let port = unused_port();
    tokio::spawn(serve_json_version_once_after_delay(
        port,
        150,
        r#"{"webSocketDebuggerUrl":"ws://127.0.0.1:9222/"}"#,
    ));

    let mut child = Command::new("/bin/sh")
        .args(["-c", "sleep 5"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let (logs, _drainers) = start_log_drainers(&mut child).unwrap();
    let ws_url = wait_for_lightpanda_ready(&mut child, port, &logs, lightpanda_startup_timeout())
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
        .args(["-c", "sleep 30"])
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
    let args = build_lightpanda_serve_args(9222, None);

    assert_eq!(
        args,
        vec![
            "serve".to_string(),
            "--host".to_string(),
            "127.0.0.1".to_string(),
            "--port".to_string(),
            "9222".to_string(),
            "--timeout".to_string(),
            resolve_lightpanda_session_timeout_secs().to_string(),
        ]
    );
}

#[test]
fn test_build_lightpanda_serve_args_with_proxy() {
    let args = build_lightpanda_serve_args(9333, Some("http://127.0.0.1:8080"));

    assert_eq!(
        args,
        vec![
            "serve".to_string(),
            "--host".to_string(),
            "127.0.0.1".to_string(),
            "--port".to_string(),
            "9333".to_string(),
            "--timeout".to_string(),
            resolve_lightpanda_session_timeout_secs().to_string(),
            "--http_proxy".to_string(),
            "http://127.0.0.1:8080".to_string(),
        ]
    );
}
