// SPDX-License-Identifier: MIT OR Apache-2.0
//! Cookie read/write helpers over CDP Network domain.
//!
//! # Workload
//!
//! **I/O-bound single session:** list/set/clear are one CDP command each
//! (batch set via `Network.setCookies`). No multi-item fan-out; cookie vectors
//! are owned by one browser session (sequential_justified N-138).
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::cdp::client::CdpClient;

/// One cookie, in the shape CDP `Network.Cookie` uses.
///
/// Both read and written: it deserializes browser state and serializes the
/// `cookie set --cookies-json` payload, which is why every field carries a
/// `#[serde(default)]` — a caller setting a cookie supplies a name, a value and
/// little else.
///
/// The `value` is a credential. It is never logged.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cookie {
    /// Cookie name.
    pub name: String,
    /// Cookie value. Treated as a secret: never written to logs or traces.
    pub value: String,
    /// Domain the cookie is scoped to. A leading dot includes subdomains.
    pub domain: String,
    /// Path prefix the cookie is sent for.
    pub path: String,
    /// Expiry as a Unix timestamp in seconds. `-1` means a session cookie.
    #[serde(default)]
    pub expires: f64,
    /// Size in bytes as the browser accounts it. Read-only; ignored on set.
    #[serde(default)]
    pub size: i64,
    /// Hidden from `document.cookie`, so script cannot read it.
    #[serde(default)]
    pub http_only: bool,
    /// Sent over HTTPS only.
    #[serde(default)]
    pub secure: bool,
    /// Dies with the browser session rather than at `expires`.
    #[serde(default)]
    pub session: bool,
    /// `Strict`, `Lax` or `None`. Absent means the browser default applies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub same_site: Option<String>,
}

/// Every cookie in the browser profile, across all origins.
///
/// Not limited to the current page: use [`get_cookies`] when the caller only
/// cares about the site it is on.
///
/// # Errors
///
/// Fails with the CDP error raised by `Network.getAllCookies` on
/// `session_id` — the `Network` domain was never enabled, or the session is
/// gone. A response whose `cookies` array is missing or does not deserialize
/// is **not** an error: it yields an empty vector, so a shape change in the
/// protocol reads as "no cookies" rather than as a failure.
pub async fn get_all_cookies(client: &CdpClient, session_id: &str) -> Result<Vec<Cookie>, String> {
    let result = client
        .send_command_no_params("Network.getAllCookies", Some(session_id))
        .await?;

    let cookies: Vec<Cookie> = result
        .get("cookies")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    Ok(cookies)
}

/// Cookies that would be sent to the given URLs.
///
/// `None` or an empty list falls back to the current page's URL, which is what
/// makes the no-argument form useful.
///
/// # Errors
///
/// Fails with the CDP error raised by `Network.getCookies` on `session_id` —
/// the `Network` domain was never enabled, the session is gone, or an entry in
/// `urls` is not a valid URL. As in [`get_all_cookies`], a `cookies` array
/// that is missing or does not deserialize yields an empty vector instead of
/// an error.
pub async fn get_cookies(
    client: &CdpClient,
    session_id: &str,
    urls: Option<Vec<String>>,
) -> Result<Vec<Cookie>, String> {
    let params = match urls {
        Some(ref u) if !u.is_empty() => json!({ "urls": u }),
        _ => json!({}),
    };

    let result = client
        .send_command("Network.getCookies", Some(params), Some(session_id))
        .await?;

    let cookies: Vec<Cookie> = result
        .get("cookies")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    Ok(cookies)
}

/// Write cookies in one CDP call.
///
/// Batched on purpose: `Network.setCookies` takes the whole vector, so a
/// per-cookie loop would pay a round-trip each and still not be atomic.
///
/// # Errors
///
/// Fails with the CDP error raised by `Network.setCookies`, which rejects the
/// **whole batch** when any entry is malformed — no `name`, or neither `url`
/// nor `domain` once the `current_url` auto-fill has run. A cookie that
/// carries neither and arrives with `current_url` as `None` therefore fails
/// here rather than being silently dropped.
pub async fn set_cookies(
    client: &CdpClient,
    session_id: &str,
    cookies: Vec<Value>,
    current_url: Option<&str>,
) -> Result<(), String> {
    let cookies: Vec<Value> = cookies
        .into_iter()
        .map(|mut c| {
            // Auto-fill url if no domain/path/url provided
            if c.get("url").is_none() && c.get("domain").is_none() {
                if let Some(url) = current_url {
                    if let Some(m) = c.as_object_mut() {
                        m.insert("url".to_string(), Value::String(url.to_string()));
                    }
                }
            }
            c
        })
        .collect();

    client
        .send_command(
            "Network.setCookies",
            Some(json!({ "cookies": cookies })),
            Some(session_id),
        )
        .await?;

    Ok(())
}

/// Remove every cookie from the browser profile.
///
/// # Errors
///
/// Fails with the CDP error raised by `Network.clearBrowserCookies` on
/// `session_id` — the `Network` domain was never enabled, or the session is
/// gone.
pub async fn clear_cookies(client: &CdpClient, session_id: &str) -> Result<(), String> {
    client
        .send_command_no_params("Network.clearBrowserCookies", Some(session_id))
        .await?;
    Ok(())
}
