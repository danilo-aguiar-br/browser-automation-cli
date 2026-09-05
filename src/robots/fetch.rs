// SPDX-License-Identifier: MIT OR Apache-2.0
//! robots.txt fetch and enforcement over the shared HTTP client.

use std::time::Duration;

use url::Url;

use crate::error::{CliError, ErrorKind};

use super::{robots_exemption, shared_http_client, url_allowed_by_robots_body, RobotsPolicy};
use crate::xdg::policy::{key, policy_usize};

/// Fetch origin robots.txt and enforce honor policy (async, one-shot).
///
/// # Errors
///
/// [`ErrorKind::Usage`] when `url` does not parse as a URL, carrying the
/// `url_absolute_http` suggestion.
/// [`ErrorKind::Data`] when robots.txt exists and disallows `url` for
/// `user_agent`; the message names the policy explicitly so a blocked navigation
/// never reads as a network failure (GAP-033).
/// [`ErrorKind::Software`] or [`ErrorKind::Usage`] propagated from
/// [`shared_http_client`] when the process-wide client cannot be built.
/// Errors propagated from [`crate::net::read_body_limited`] when the robots body
/// exceeds `robots_max_body_bytes` or the stream fails mid-read.
///
/// A failed fetch, a non-2xx status, and a non-HTTP scheme are all `Ok(())`: a
/// missing or unreachable robots.txt allows, with a warning on the tracing
/// target.
pub async fn enforce_robots(
    url: &str,
    policy: RobotsPolicy,
    user_agent: &str,
) -> Result<(), CliError> {
    if let Some(exemption) = robots_exemption(url, policy) {
        tracing::debug!(
            target: "browser_automation_cli::robots",
            url = %url,
            exemption = exemption.as_str(),
            "robots.txt not consulted"
        );
        return Ok(());
    }

    let parsed = Url::parse(url).map_err(|e| {
        CliError::with_suggestion(
            ErrorKind::Usage,
            format!("invalid URL for robots check: {e}"),
            crate::i18n::suggestion_key("url_absolute_http", None),
        )
    })?;

    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Ok(());
    }

    let origin = parsed.origin().ascii_serialization();
    let robots_url = format!("{origin}/robots.txt");

    // Reuse process-wide client; short per-request budget for robots only.
    //
    // This is the ONLY robots request in the product, and it is a GET. A second
    // key, `robots_fetch_timeout_secs`, was published alongside this one and
    // read by nobody; it was removed in 0.1.9 rather than wired, because wiring
    // it would have raised this budget from 5s to 30s on the critical path of
    // every scrape to fix a naming problem.
    let client = shared_http_client()?;
    let resp = match client
        .get(&robots_url)
        .header(reqwest::header::USER_AGENT, user_agent)
        .timeout(Duration::from_secs(crate::xdg::policy::policy_u64(
            crate::xdg::policy::key::ROBOTS_PROBE_TIMEOUT_SECS,
        )))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            // PRD: fetch failure → allow with warning (tracing → stderr / optional file).
            tracing::warn!(
                target: "browser_automation_cli::robots",
                robots_url = %robots_url,
                error = %e,
                "robots fetch failed; treating as allow"
            );
            return Ok(());
        }
    };

    if !resp.status().is_success() {
        // Missing robots → allow
        return Ok(());
    }

    let body_bytes =
        crate::net::read_body_limited(resp, policy_usize(key::ROBOTS_MAX_BODY_BYTES)).await?;
    let body = String::from_utf8_lossy(&body_bytes);

    // Remember Crawl-delay for this origin (politeness; non-standard but common).
    if let Some(delay) = super::parse_crawl_delay_secs(&body, user_agent) {
        super::remember_crawl_delay(&origin, delay);
    }

    if url_allowed_by_robots_body(&body, user_agent, url) {
        return Ok(());
    }

    Err(CliError::with_suggestion(
        ErrorKind::Data,
        // Name the policy explicitly: a blocked navigation must never read as a
        // network failure (GAP-033).
        format!("blocked by robots.txt policy (not a network error): {url}"),
        crate::i18n::suggestion_key("robots_dual", None),
    ))
}
