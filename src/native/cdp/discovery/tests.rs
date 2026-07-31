// SPDX-License-Identifier: MIT OR Apache-2.0
//! Unit tests for CDP endpoint discovery.

use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const HTTP_404: &str = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

fn http_200(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\nContent-Type: application/json\r\n\r\n{}",
        body.len(), body
    )
}

async fn accept_http(listener: &TcpListener, response: &str) {
    let (mut s, _) = listener.accept().await.unwrap();
    let mut buf = [0u8; 1024];
    let _ = s.read(&mut buf).await;
    s.write_all(response.as_bytes()).await.unwrap();
}

#[tokio::test]
async fn discovers_ws_url_from_json_version() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        accept_http(
            &listener,
            &http_200(r#"{"webSocketDebuggerUrl":"ws://127.0.0.1:1234/"}"#),
        )
        .await;
    });

    let ws_url = discover_cdp_url("127.0.0.1", port, None).await.unwrap();
    assert_eq!(ws_url, format!("ws://127.0.0.1:{port}/"));
    server.await.unwrap();
}

#[tokio::test]
async fn returns_error_when_version_returns_invalid_json() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        accept_http(&listener, &http_200("not-json")).await;
        // /json/list and ws fallback both fail (server closes)
    });

    // Use single-pass discovery: outer retry would re-hit a one-shot mock server.
    let err = discover_cdp_url_once("127.0.0.1", port, None, Duration::from_secs(2))
        .await
        .unwrap_err();
    assert!(
        err.contains("Invalid /json/version response") || err.contains("/json/version"),
        "unexpected discovery error: {err}"
    );
    server.await.unwrap();
}

#[tokio::test]
async fn falls_back_to_json_list_on_version_404() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        accept_http(&listener, HTTP_404).await;
        accept_http(
            &listener,
            &http_200(r#"[{"type":"browser","webSocketDebuggerUrl":"ws://127.0.0.1:1234/devtools/browser/abc"}]"#),
        ).await;
    });

    let ws_url = discover_cdp_url("127.0.0.1", port, None).await.unwrap();
    assert!(ws_url.contains("/devtools/browser/abc"));
    assert!(ws_url.contains(&port.to_string()));
    server.await.unwrap();
}

#[tokio::test]
async fn all_discovery_methods_fail_when_http_404_and_no_browser() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        // /json/version -> 404, /json/list -> 404; WS connect has no CDP browser
        accept_http(&listener, HTTP_404).await;
        accept_http(&listener, HTTP_404).await;
    });

    let err = discover_cdp_url("127.0.0.1", port, None).await.unwrap_err();
    assert!(
        err.contains("All CDP discovery methods failed"),
        "unexpected error: {err}"
    );
    let _ = server.await;
}

#[test]
fn rewrite_ws_host_replaces_host_and_port() {
    let original = "ws://127.0.0.1:9222/devtools/browser/abc";
    let rewritten = rewrite_ws_host(original, "10.211.55.12", 9223);
    assert_eq!(rewritten, "ws://10.211.55.12:9223/devtools/browser/abc");
}

#[test]
fn rewrite_ws_host_handles_ipv6() {
    let original = "ws://127.0.0.1:9222/devtools/browser/abc";
    let rewritten = rewrite_ws_host(original, "::1", 9222);
    assert_eq!(rewritten, "ws://[::1]:9222/devtools/browser/abc");
}

#[test]
fn append_query_adds_params_to_url_without_query() {
    let url = "ws://127.0.0.1:9222/devtools/browser/abc";
    let result = append_query(url, Some("mode=Hello"));
    assert_eq!(
        result,
        "ws://127.0.0.1:9222/devtools/browser/abc?mode=Hello"
    );
}

#[test]
fn append_query_merges_with_existing_query() {
    let url = "ws://127.0.0.1:9222/devtools/browser/abc?token=xyz";
    let result = append_query(url, Some("mode=Hello"));
    assert_eq!(
        result,
        "ws://127.0.0.1:9222/devtools/browser/abc?token=xyz&mode=Hello"
    );
}

#[test]
fn append_query_noop_for_none() {
    let url = "ws://127.0.0.1:9222/devtools/browser/abc";
    let result = append_query(url, None);
    assert_eq!(result, url);
}

#[test]
fn append_query_noop_for_empty() {
    let url = "ws://127.0.0.1:9222/devtools/browser/abc";
    let result = append_query(url, Some(""));
    assert_eq!(result, url);
}

#[test]
fn append_query_handles_multiple_params() {
    let url = "ws://127.0.0.1:9222/devtools/browser/abc";
    let result = append_query(url, Some("mode=Hello&token=abc"));
    assert_eq!(
        result,
        "ws://127.0.0.1:9222/devtools/browser/abc?mode=Hello&token=abc"
    );
}

#[tokio::test]
async fn discover_preserves_query_params() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        accept_http(
            &listener,
            &http_200(r#"{"webSocketDebuggerUrl":"ws://127.0.0.1:1234/"}"#),
        )
        .await;
    });

    let ws_url = discover_cdp_url("127.0.0.1", port, Some("mode=Hello"))
        .await
        .unwrap();
    assert_eq!(ws_url, format!("ws://127.0.0.1:{port}/?mode=Hello"));
    server.await.unwrap();
}
