// SPDX-License-Identifier: MIT OR Apache-2.0
//! Single-URL HTTP scrape (I/O + spawn_blocking parse).

use std::time::Duration;

use scraper::Html;
use serde_json::{json, Value};

use crate::cache::{self};
use crate::error::{CliError, ErrorKind};
use crate::robots::{shared_http_client, RobotsPolicy};

use super::html::{
    extract_branding_hints, extract_json_ld_product, extract_links, extract_main_html,
    html_to_markdown_simple, meta_content, text_of_first, visible_text,
};
use super::scheme::reject_non_http_scheme_for_http_engine;
use super::types::{ScrapeFormat, ScrapeOpts, HTTP_USER_AGENT};

/// HTTP static scrape (no Chrome).
pub async fn scrape_http(
    url: &str,
    robots: RobotsPolicy,
    opts: &ScrapeOpts,
) -> Result<Value, CliError> {
    // GAP-A004: reject non-HTTP(S) schemes early with an agent-usable suggestion.
    reject_non_http_scheme_for_http_engine(url)?;
    // Pass N: SSRF policy (XDG http_ssrf_mode; default strict).
    crate::net::assert_safe_http_url(url)?;

    crate::robots::enforce_robots(url, robots, HTTP_USER_AGENT).await?;

    // GAP-011: layered XDG cache for GET scrape (hit skips network).
    let cache_key = cache::CacheKey::http_get(url);
    if let Ok(Some(entry)) = cache::get_async(&cache_key).await {
        if let Ok(html) = String::from_utf8(entry.body) {
            let mut payload =
                build_scrape_payload_blocking(url.to_string(), 200, html, opts.clone(), robots)
                    .await?;
            if let Some(obj) = payload.as_object_mut() {
                obj.insert("cache_hit".into(), json!(true));
            }
            return Ok(payload);
        }
    }

    // Process-wide client (keep-alive + TLS session reuse across batch/crawl).
    let client = shared_http_client()?;

    // GAP-013: retry transient HTTP failures with named policy.
    let cfg = crate::retry::RetryConfig::http();
    let mut attempt = 0u32;
    let (status, final_url, bytes) = loop {
        attempt += 1;
        match client.get(url).send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let final_url = resp.url().to_string();
                // N24: re-check SSRF on post-redirect URL (literal private targets).
                crate::net::assert_safe_http_url(&final_url)?;
                match crate::net::read_body_limited(resp, opts.max_body_bytes).await {
                    Ok(bytes) => break (status, final_url, bytes),
                    Err(e) if e.kind() == ErrorKind::Data => return Err(e),
                    Err(e) => {
                        let err = e.message().to_string();
                        if attempt >= cfg.max_attempts || !crate::retry::is_retryable_message(&err)
                        {
                            return Err(CliError::new(ErrorKind::Io, err));
                        }
                    }
                }
            }
            Err(e) => {
                let err = format!("GET {url}: {e}");
                if attempt >= cfg.max_attempts || !crate::retry::is_retryable_message(&err) {
                    return Err(CliError::new(ErrorKind::Unavailable, err));
                }
            }
        }
        tokio::time::sleep(cfg.delay_for_attempt(attempt.saturating_sub(1))).await;
    };
    // Pre-reserve before UTF-8 lossy copy (rules: try_reserve on external bodies).
    let mut html = String::new();
    html.try_reserve(bytes.len()).map_err(|e| {
        CliError::new(
            ErrorKind::Unavailable,
            format!("scrape body reserve failed ({} bytes): {e}", bytes.len()),
        )
    })?;
    html.push_str(&String::from_utf8_lossy(&bytes));
    {
        let mut body = Vec::new();
        if body.try_reserve_exact(html.len()).is_ok() {
            body.extend_from_slice(html.as_bytes());
        } else {
            body = html.as_bytes().to_vec();
        }
        let _ = cache::put_async(
            &cache_key,
            cache::CacheEntry {
                body,
                content_type: Some("text/html".into()),
                expires_unix: cache::expires_after(Duration::from_secs(
                    crate::xdg::policy::policy_u64(
                        crate::xdg::policy::key::SCRAPE_HTTP_CACHE_TTL_SECS,
                    ),
                )),
            },
        )
        .await;
    }
    // CPU-bound HTML parse off the async worker (rules + docsrs spawn_blocking).
    let mut payload =
        build_scrape_payload_blocking(final_url, status, html, opts.clone(), robots).await?;
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("cache_hit".into(), json!(false));
        obj.insert("http_attempts".into(), json!(attempt));
    }
    Ok(payload)
}

/// Run [`build_scrape_payload`] on Tokio's blocking pool (CPU-bound HTML parse).
///
/// Bound by the caller's Semaphore permit when used from batch/crawl tasks so
/// `max_blocking_threads` cannot be saturated independently of I/O admits.
async fn build_scrape_payload_blocking(
    source_url: String,
    status: u16,
    html: String,
    opts: ScrapeOpts,
    robots: RobotsPolicy,
) -> Result<Value, CliError> {
    tokio::task::spawn_blocking(move || {
        build_scrape_payload(&source_url, status, &html, &opts, robots)
    })
    .await
    .map_err(|e| {
        if e.is_panic() {
            CliError::new(ErrorKind::Software, "scrape HTML parse task panicked")
        } else {
            CliError::new(ErrorKind::Software, format!("scrape HTML parse join: {e}"))
        }
    })
}

/// Build agent envelope data from HTML.
pub fn build_scrape_payload(
    source_url: &str,
    status: u16,
    html: &str,
    opts: &ScrapeOpts,
    robots: RobotsPolicy,
) -> Value {
    let document = Html::parse_document(html);
    let title = text_of_first(&document, "title");
    let description = meta_content(&document, "description")
        .or_else(|| meta_content(&document, "og:description"))
        .unwrap_or_default();
    let body_html = if opts.only_main_content {
        extract_main_html(&document).unwrap_or_else(|| html.to_string())
    } else {
        html.to_string()
    };
    let body_doc = Html::parse_document(&body_html);
    let text = visible_text(&body_doc);
    let markdown = html_to_markdown_simple(&body_html, &title);
    let links = extract_links(source_url, &document);

    let mut data = json!({
        "source_url": source_url,
        "status_code": status,
        "title": title,
        "robots_policy": robots.as_str(),
        "engine": opts.engine,
        "format": format!("{:?}", opts.format).to_ascii_lowercase(),
    });

    match opts.format {
        ScrapeFormat::Text => {
            data["text"] = json!(text);
        }
        ScrapeFormat::Markdown => {
            data["markdown"] = json!(markdown);
            data["text"] = json!(text);
        }
        ScrapeFormat::Html => {
            data["html"] = json!(body_html);
        }
        ScrapeFormat::Links => {
            data["links"] = json!(links);
        }
        ScrapeFormat::Metadata => {
            data["metadata"] = json!({
                "title": title,
                "description": description,
                "status_code": status,
                "source_url": source_url,
                "link_count": links.len(),
            });
        }
        ScrapeFormat::Screenshot => {
            // Browser path attaches path after grab; HTTP engine notes unsupported.
            data["text"] = json!(text);
            data["screenshot"] = json!({
                "note": "screenshot format requires --engine browser; use grab for explicit capture",
                "path": null,
            });
        }
        ScrapeFormat::Summary => {
            let summary = if text.len() > 400 {
                format!("{}…", text.chars().take(400).collect::<String>())
            } else {
                text.clone()
            };
            data["summary"] = json!(summary);
            data["text"] = json!(text);
            data["llm_required_for_full"] = json!(true);
        }
        ScrapeFormat::Product => {
            data["product"] = extract_json_ld_product(html);
            data["text"] = json!(text);
        }
        ScrapeFormat::Branding => {
            data["branding"] = extract_branding_hints(html, &title);
            data["text"] = json!(text);
        }
    }
    data
}
