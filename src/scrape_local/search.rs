// SPDX-License-Identifier: MIT OR Apache-2.0
//! Local HTTP search (public HTML SERP; no SaaS key).

use serde_json::{json, Value};
use url::Url;

use crate::error::CliError;
use crate::robots::RobotsPolicy;

use super::crawl::map_http;
use super::http::scrape_http;
use super::types::{ScrapeFormat, ScrapeOpts};

/// Local search: fetch a public HTML search page or treat query as URL list seed.
/// MVP: if query looks like URL, map it; else use DuckDuckGo HTML (optional network).
pub async fn search_http(
    query: &str,
    robots: RobotsPolicy,
    limit: usize,
) -> Result<Value, CliError> {
    let limit = limit.clamp(
        1,
        crate::xdg::policy::policy_usize(crate::xdg::policy::key::SCRAPE_SEARCH_LIMIT_MAX),
    );
    let q = query.trim();
    if q.starts_with("http://") || q.starts_with("https://") {
        return map_http(q, robots, limit, 1).await;
    }
    // Endpoint from XDG `search_base_url` or named const DEFAULT_SEARCH_BASE_URL.
    let base = crate::xdg::search_base_url();
    let base = base.trim_end_matches('/');
    let search_url = format!("{base}/?q={}", urlencoding::encode(q));
    let opts = ScrapeOpts {
        format: ScrapeFormat::Links,
        engine: "http".into(),
        ..ScrapeOpts::default()
    };
    let page = scrape_http(&search_url, robots, &opts).await?;
    let mut results = Vec::new();
    if let Some(links) = page.get("links").and_then(|v| v.as_array()) {
        for l in links {
            let raw = l
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let text = l
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let clean = clean_serp_url(&raw);
            // Drop same-host SERP chrome and empty destinations.
            if clean.is_empty() {
                continue;
            }
            if clean.contains("duckduckgo.com/html") || clean.ends_with("duckduckgo.com/") {
                continue;
            }
            results.push(json!({ "text": text, "url": clean }));
            if results.len() >= limit {
                break;
            }
        }
    }
    Ok(json!({
        "query": q,
        "count": results.len(),
        "results": results,
        "source_url": search_url,
        "robots_policy": robots.as_str(),
        "engine": "http",
        "note": "local HTTP search via public HTML SERP; no SaaS API key",
    }))
}

/// Unwrap SERP redirect wrappers (e.g. uddg=) into destination URLs.
pub(crate) fn clean_serp_url(raw: &str) -> String {
    let raw = raw.trim();
    if raw.is_empty() {
        return String::new();
    }
    if let Ok(u) = Url::parse(raw) {
        // duckduckgo /l/?uddg=https%3A%2F%2F...
        for (k, v) in u.query_pairs() {
            if k == "uddg" || k == "u" || k == "url" {
                let decoded = urlencoding::decode(&v).unwrap_or_else(|_| v.clone());
                let s = decoded.into_owned();
                if s.starts_with("http://") || s.starts_with("https://") {
                    return s;
                }
            }
        }
        // already clean
        if u.host_str()
            .map(|h| !h.contains("duckduckgo.com"))
            .unwrap_or(true)
        {
            return raw.to_string();
        }
    }
    raw.to_string()
}
