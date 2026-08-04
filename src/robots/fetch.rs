// SPDX-License-Identifier: MIT OR Apache-2.0
//! robots.txt fetch and enforcement over the shared HTTP client.

use std::time::Duration;

use url::Url;

use crate::error::{CliError, ErrorKind};

use super::{robots_exemption, shared_http_client, url_allowed_by_robots_body, RobotsPolicy};
use crate::constants::ROBOTS_MAX_BODY_BYTES;

/// Fetch origin robots.txt and enforce honor policy (async, one-shot).
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

    // Reuse process-wide client; short per-request timeout for robots only.
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

    let body_bytes = crate::net::read_body_limited(resp, ROBOTS_MAX_BODY_BYTES).await?;
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
