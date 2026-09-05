// SPDX-License-Identifier: MIT OR Apache-2.0
use std::time::Duration;

use chromiumoxide::browser::Browser;
use futures::StreamExt;

use super::types::BrowserVersionInfo;

/// Default timeout for CDP discovery HTTP requests.
fn default_discovery_timeout() -> Duration {
    crate::xdg::policy::policy_secs(crate::xdg::policy::key::DEFAULT_CDP_DISCOVERY_TIMEOUT_SECS)
}

/// Discover the CDP WebSocket URL for the given host and port.
///
/// Tries three methods in order: `/json/version`, `/json/list`, and a direct
/// WebSocket connection to `/devtools/browser`. The returned URL has its
/// host/port rewritten to match the requested target.
///
/// An optional `query` string (without the leading `?`) is appended to the
/// final WebSocket URL so that user-supplied URL parameters (e.g.
/// `?mode=Hello`) are forwarded to the remote endpoint.
///
/// # Errors
///
/// Propagates
/// [`discover_cdp_url_with_timeout`] under
/// [`DEFAULT_CDP_DISCOVERY_TIMEOUT_SECS`](crate::xdg::policy::key::DEFAULT_CDP_DISCOVERY_TIMEOUT_SECS):
/// all three discovery methods failed, and the message carries the reason for
/// each one.
pub async fn discover_cdp_url(
    host: &str,
    port: u16,
    query: Option<&str>,
) -> Result<String, String> {
    discover_cdp_url_with_timeout(host, port, query, default_discovery_timeout()).await
}

/// Like [`discover_cdp_url`] but with a custom request timeout.
///
/// # Errors
///
/// Fails only after [`crate::retry::RetryConfig::cdp`] exhausts its attempts,
/// and then carries the last `discover_cdp_url_once` message: one line joining
/// why `/json/version`, `/json/list` and the direct `/devtools/browser`
/// WebSocket each failed — endpoint unreachable, `timeout` elapsed, malformed
/// JSON, or a response with no `webSocketDebuggerUrl`.
pub async fn discover_cdp_url_with_timeout(
    host: &str,
    port: u16,
    query: Option<&str>,
    timeout: Duration,
) -> Result<String, String> {
    // GAP-013: retry the full discovery cascade on transient network/timeouts.
    let cfg = crate::retry::RetryConfig::cdp();
    crate::retry::retry_async(cfg, || async {
        discover_cdp_url_once(host, port, query, timeout).await
    })
    .await
}

/// Single-pass discovery cascade (no retry). Used by tests and by the retry wrapper.
pub(crate) async fn discover_cdp_url_once(
    host: &str,
    port: u16,
    query: Option<&str>,
    timeout: Duration,
) -> Result<String, String> {
    // Primary: /json/version (standard path)
    let version_err = match fetch_cdp_info(host, port, timeout).await {
        Ok(info) => {
            if let Some(ws_url) = info.web_socket_debugger_url {
                return Ok(append_query(&rewrite_ws_host(&ws_url, host, port), query));
            }
            format!("No webSocketDebuggerUrl in /json/version at {host}:{port}")
        }
        Err(e) => e,
    };

    // Fallback: /json/list (returns target list; look for the browser target)
    let list_err = match fetch_cdp_list(host, port, timeout).await {
        Ok(ws_url) => return Ok(append_query(&rewrite_ws_host(&ws_url, host, port), query)),
        Err(e) => e,
    };

    // Final fallback: direct WebSocket at /devtools/browser.
    // Chrome 136+ with UI-based remote debugging (chrome://inspect) exposes
    // CDP over WebSocket but does not serve HTTP discovery endpoints.
    match discover_cdp_ws(host, port, timeout).await {
        Ok(ws_url) => Ok(append_query(&ws_url, query)),
        Err(ws_err) => Err(format!(
            "All CDP discovery methods failed for {host}:{port}: /json/version: {version_err}; /json/list: {list_err}; WebSocket: {ws_err}"
        )),
    }
}

/// Bracket an IPv6 address for use in URLs. No-op for IPv4 or already-bracketed addresses.
fn bracket_ipv6(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host.to_string()
    }
}

/// Fetch `/json/version` from the given host:port and parse the response.
async fn fetch_cdp_info(
    host: &str,
    port: u16,
    timeout: Duration,
) -> Result<BrowserVersionInfo, String> {
    let url = format!("http://{}:{}/json/version", bracket_ipv6(host), port);

    let body = tokio::time::timeout(timeout, reqwest_get_string(&url))
        .await
        .map_err(|_| format!("Timeout connecting to CDP at {host}:{port}"))?
        .map_err(|e| format!("Failed to connect to CDP at {host}:{port}: {e}"))?;

    crate::json_util::from_str(&body).map_err(|e| format!("Invalid /json/version response: {e}"))
}

/// Rewrite the host and port in a WebSocket URL to match the target we
/// actually connected to. Chrome's `/json/version` always returns
/// `ws://127.0.0.1:<local-port>/...` which is unreachable when the
/// browser is on a remote machine or behind a port-forward.
fn rewrite_ws_host(ws_url: &str, host: &str, port: u16) -> String {
    if let Ok(mut parsed) = url::Url::parse(ws_url) {
        let _ = parsed.set_host(Some(&bracket_ipv6(host)));
        let _ = parsed.set_port(Some(port));
        parsed.to_string()
    } else {
        ws_url.to_string()
    }
}

/// Append a query string to a URL, preserving any existing query parameters.
fn append_query(url: &str, query: Option<&str>) -> String {
    match query {
        Some(q) if !q.is_empty() => {
            if let Ok(mut parsed) = url::Url::parse(url) {
                {
                    let mut pairs = parsed.query_pairs_mut();
                    pairs.extend_pairs(url::form_urlencoded::parse(q.as_bytes()));
                }
                parsed.to_string()
            } else {
                // Fallback: raw string append
                if url.contains('?') {
                    format!("{url}&{q}")
                } else {
                    format!("{url}?{q}")
                }
            }
        }
        _ => url.to_string(),
    }
}

/// Fetch `/json/list` and extract the `webSocketDebuggerUrl` from the first
/// target with `type == "browser"`, or the first target if none has that type.
async fn fetch_cdp_list(host: &str, port: u16, timeout: Duration) -> Result<String, String> {
    let url = format!("http://{}:{}/json/list", bracket_ipv6(host), port);

    let body = tokio::time::timeout(timeout, reqwest_get_string(&url))
        .await
        .map_err(|_| format!("Timeout connecting to /json/list at {host}:{port}"))?
        .map_err(|e| format!("Failed to connect to /json/list at {host}:{port}: {e}"))?;

    let targets: Vec<serde_json::Value> = crate::json_util::from_str(&body)
        .map_err(|e| format!("Invalid /json/list response: {e}"))?;

    // Prefer targets with type "browser", fall back to first target with a ws URL
    let browser_target = targets
        .iter()
        .find(|t| t.get("type").and_then(|v| v.as_str()) == Some("browser"));

    let target = browser_target.or_else(|| targets.first());

    target
        .and_then(|t| t.get("webSocketDebuggerUrl"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "No webSocketDebuggerUrl found in /json/list targets".to_string())
}

/// Discover a CDP endpoint by connecting via chromiumoxide `Browser::connect`
/// and verifying `Browser.getVersion` / version() succeeds.
/// Returns the WebSocket URL on success.
async fn discover_cdp_ws(host: &str, port: u16, timeout: Duration) -> Result<String, String> {
    let ws_url = format!("ws://{}:{}/devtools/browser", bracket_ipv6(host), port);

    tokio::time::timeout(timeout, async {
        let (mut browser, mut handler) = Browser::connect(&ws_url)
            .await
            .map_err(|e| format!("WebSocket connect failed at {ws_url}: {e}"))?;

        let handle = tokio::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });

        let version = browser
            .version()
            .await
            .map_err(|e| format!("Browser.getVersion failed: {e}"))?;
        let _ = version.product;
        let _ = browser.close().await;
        let _ = handle.await;
        Ok::<(), String>(())
    })
    .await
    .map_err(|_| format!("Timeout connecting to WebSocket at {ws_url}"))?
    .map(|()| ws_url)
}

async fn reqwest_get_string(url: &str) -> Result<String, String> {
    // Always reuse the process-wide OnceLock client (rules: create reqwest once;
    // never fall back to `reqwest::get` which builds a throwaway Client + pool).
    // Body capped (Pass N) — CDP discovery JSON is small.
    let client = crate::robots::shared_http_client().map_err(|e| e.to_string())?;
    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    let bytes = crate::net::read_body_limited(
        resp,
        crate::xdg::policy::policy_usize(crate::xdg::policy::key::CDP_DISCOVERY_MAX_BODY_BYTES),
    )
    .await
    .map_err(|e| e.message().to_string())?;
    String::from_utf8(bytes).map_err(|e| format!("cdp discovery utf8: {e}"))
}

#[cfg(test)]
mod tests;
