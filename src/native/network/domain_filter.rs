// SPDX-License-Identifier: MIT OR Apache-2.0
//! Domain allow-list filter (CDP script + Fetch enable).
use serde_json::{json, Value};

use crate::native::cdp::client::CdpClient;

use super::domain_script::domain_filter_script;

/// Host allow-list applied to page navigation and subresource requests.
///
/// An EMPTY list means "allow everything", not "deny everything": the filter is
/// opt-in, so an unconfigured run behaves as if it were not there.
#[derive(Debug, Clone)]
pub struct DomainFilter {
    /// Lower-cased patterns. `*.example.com` matches the apex and any subdomain;
    /// anything else must match the host exactly.
    pub allowed_domains: Vec<String>,
}

impl DomainFilter {
    /// Build from a comma-separated list, trimming and lower-casing each entry.
    pub fn new(domains: &str) -> Self {
        let allowed = parse_domain_list(domains);
        Self {
            allowed_domains: allowed,
        }
    }

    /// Whether a bare hostname passes the list.
    ///
    /// Suffix matching requires the dot (`.example.com`), so `notexample.com`
    /// does not slip through a `*.example.com` rule.
    pub fn is_allowed(&self, hostname: &str) -> bool {
        if self.allowed_domains.is_empty() {
            return true;
        }
        let hostname = hostname.to_lowercase();
        for pattern in &self.allowed_domains {
            if let Some(suffix) = pattern.strip_prefix("*.") {
                if hostname == suffix || hostname.ends_with(&format!(".{suffix}")) {
                    return true;
                }
            } else if hostname == *pattern {
                return true;
            }
        }
        false
    }

    /// [`is_allowed`](Self::is_allowed) against a full URL, as a `Result`.
    ///
    /// A URL that does not parse, or that carries no host, is REFUSED rather
    /// than waved through: an unparseable target cannot be shown to be allowed.
    ///
    /// # Errors
    ///
    /// Returns `Ok(())` unconditionally while `allowed_domains` is empty.
    /// Otherwise fails with `"Invalid URL: <url>"` when the string does not
    /// parse, `"No hostname in URL: <url>"` for a host-less scheme such as
    /// `data:` or `about:`, and `"Domain '<host>' is not in the allowed
    /// domains list"` when the host matches no pattern.
    pub fn check_url(&self, url: &str) -> Result<(), String> {
        if self.allowed_domains.is_empty() {
            return Ok(());
        }
        let parsed = url::Url::parse(url).map_err(|_| format!("Invalid URL: {url}"))?;
        let hostname = parsed
            .host_str()
            .ok_or_else(|| format!("No hostname in URL: {url}"))?;
        if self.is_allowed(hostname) {
            Ok(())
        } else {
            Err(format!(
                "Domain '{hostname}' is not in the allowed domains list"
            ))
        }
    }
}

pub(crate) fn parse_domain_list(input: &str) -> Vec<String> {
    // ASCII fold, not Unicode: DNS is case-insensitive over ASCII only
    // (RFC 4343), and an internationalised name reaches us already punycoded.
    crate::agent_ops::path::split_csv_lower(input)
}

/// Navigate any already-open page that violates the filter to `about:blank`.
///
/// Pages can exist before the filter is installed, and the install only governs
/// what happens NEXT. Without this, a disallowed page stays loaded and readable.
pub async fn sanitize_existing_pages(
    client: &CdpClient,
    pages: &[crate::native::browser::PageInfo],
    filter: &DomainFilter,
) {
    // Multi-page navigate is I/O-bound CDP — fan-out with join_bounded (Semaphore).
    let to_blank: Vec<&crate::native::browser::PageInfo> = pages
        .iter()
        .filter(|page| {
            if page.url.is_empty() || page.url == crate::constants::ABOUT_BLANK {
                return false;
            }
            url::Url::parse(&page.url)
                .ok()
                .and_then(|u| u.host_str().map(|h| h.to_string()))
                .is_some_and(|hostname| !filter.is_allowed(&hostname))
        })
        .collect();
    if to_blank.is_empty() {
        return;
    }
    let cdp_limit = crate::concurrency::effective_limit_capped(crate::concurrency::CDP_FANOUT_CAP);
    let futs: Vec<_> = to_blank
        .iter()
        .map(|page| {
            let sid = page.session_id.as_str();
            async move {
                let _ = client
                    .send_command(
                        "Page.navigate",
                        Some(json!({ "url": crate::constants::ABOUT_BLANK })),
                        Some(sid),
                    )
                    .await;
            }
        })
        .collect();
    let _ = crate::concurrency::join_bounded(futs, cdp_limit).await;
}

/// Install the page-side guard that blocks disallowed `fetch`/`XHR` targets.
///
/// Runs on every new document, so it also covers pages created after this call.
/// A no-op when the list is empty.
///
/// # Errors
///
/// Returns `Ok(())` immediately when `allowed_domains` is empty. Otherwise
/// fails with the CDP error raised by
/// `Page.addScriptToEvaluateOnNewDocument` or by the `Runtime.evaluate` that
/// applies the same guard to the CURRENT document, and with
/// `"Failed to apply domain filter to the current execution context: …"` when
/// that evaluation reports a JavaScript exception. The already-loaded page is
/// the reason the second install exists: the new-document hook alone would
/// leave it unguarded.
pub async fn install_domain_filter_script(
    client: &CdpClient,
    session_id: &str,
    allowed_domains: &[String],
) -> Result<(), String> {
    if allowed_domains.is_empty() {
        return Ok(());
    }

    let script = domain_filter_script(allowed_domains);

    client
        .send_command(
            "Page.addScriptToEvaluateOnNewDocument",
            Some(json!({ "source": &script })),
            Some(session_id),
        )
        .await?;

    install_domain_filter_runtime_script(client, session_id, allowed_domains).await?;

    Ok(())
}

async fn install_domain_filter_runtime_script(
    client: &CdpClient,
    session_id: &str,
    allowed_domains: &[String],
) -> Result<(), String> {
    if allowed_domains.is_empty() {
        return Ok(());
    }

    let script = domain_filter_script(allowed_domains);
    let evaluation = client
        .send_command(
            "Runtime.evaluate",
            Some(json!({ "expression": &script })),
            Some(session_id),
        )
        .await?;
    if let Some(details) = evaluation.get("exceptionDetails") {
        let message = details
            .get("exception")
            .and_then(|exception| exception.get("description"))
            .and_then(Value::as_str)
            .or_else(|| details.get("text").and_then(Value::as_str))
            .unwrap_or("unknown JavaScript error");
        return Err(format!(
            "Failed to apply domain filter to the current execution context: {message}"
        ));
    }

    Ok(())
}

/// Enable the CDP `Fetch` domain so requests can be judged before they leave.
///
/// This is the network-side half of the filter: the page-side script cannot see
/// requests the page did not make through JavaScript.
///
/// # Errors
///
/// Fails with the CDP error raised by `Fetch.enable` — an engine that does not
/// implement the `Fetch` domain, which is how Lightpanda refuses this half of
/// the filter.
pub async fn install_domain_filter_fetch(
    client: &CdpClient,
    session_id: &str,
    handle_auth_requests: bool,
) -> Result<(), String> {
    let mut params = json!({
        "patterns": [{ "urlPattern": "*" }]
    });
    if handle_auth_requests {
        params["handleAuthRequests"] = json!(true);
    }
    client
        .send_command("Fetch.enable", Some(params), Some(session_id))
        .await?;
    Ok(())
}

/// Install both layers of domain filtering on a session:
/// 1. Fetch-based network interception
/// 2. JS patching for APIs outside Fetch interception, including workers,
///    WebSocket, EventSource, sendBeacon, and RTCPeerConnection.
///
/// # Errors
///
/// Propagates [`install_domain_filter_fetch`] first, then
/// [`install_domain_filter_script`]. The order matters for failure too: a
/// refusal of the second layer leaves `Fetch.enable` armed, so requests are
/// still intercepted while the JavaScript APIs it does not cover are not.
pub async fn install_domain_filter(
    client: &CdpClient,
    session_id: &str,
    allowed_domains: &[String],
    handle_auth_requests: bool,
) -> Result<(), String> {
    install_domain_filter_fetch(client, session_id, handle_auth_requests).await?;
    install_domain_filter_script(client, session_id, allowed_domains).await?;
    Ok(())
}
