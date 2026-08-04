// SPDX-License-Identifier: MIT OR Apache-2.0
//! Single-URL HTTP fetch: robots, SSRF policy, cache, retry and charset decode.
//!
//! Payload shaping lives in [`super::payload`]; this module only produces bytes
//! and hands the decoded HTML over for envelope construction.

use std::time::Duration;

use serde_json::{json, Value};

use crate::cache::{self};
use crate::error::{CliError, ErrorKind};
use crate::robots::{shared_http_client, RobotsPolicy};

use super::directives::{merge_robots, parse_meta_robots, parse_x_robots_tag};
use super::encoding::decode_html_body;
use super::payload::build_scrape_payload_blocking;
use super::scheme::reject_non_http_scheme_for_http_engine;
use super::types::{ScrapeOpts, HTTP_USER_AGENT};

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
    // Politeness: Crawl-delay + XDG floor (per origin).
    let crawl_delay_secs = crate::robots::wait_origin(url).await;

    // GAP-011: layered XDG cache for GET scrape (hit skips network).
    let cache_key = cache::CacheKey::http_get(url);
    if let Ok(Some(entry)) = cache::get_async(&cache_key).await {
        // Re-decode cached bytes (BUG-CACHE-UTF8: never assume UTF-8 only).
        let decoded = decode_html_body(&entry.body, entry.content_type.as_deref());
        let mut payload =
            build_scrape_payload_blocking(url.to_string(), 200, decoded.text, opts.clone(), robots)
                .await?;
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("cache_hit".into(), json!(true));
            obj.insert("change_status".into(), json!("unchanged"));
            obj.insert("charset".into(), json!(decoded.charset));
            if decoded.had_errors {
                obj.insert("charset_had_errors".into(), json!(true));
            }
            if crawl_delay_secs > 0.0 {
                obj.insert("crawl_delay_secs".into(), json!(crawl_delay_secs));
            }
        }
        return Ok(payload);
    }

    // Process-wide client (keep-alive + TLS session reuse across batch/crawl).
    let client = shared_http_client()?;

    // GAP-013: retry transient HTTP failures with named policy.
    let cfg = crate::retry::RetryConfig::http();
    let mut attempt = 0u32;
    let (status, final_url, bytes, content_type, x_robots) = loop {
        attempt += 1;
        let mut req = client.get(url);
        for (name, value) in &opts.extra_headers {
            if let (Ok(n), Ok(v)) = (
                reqwest::header::HeaderName::from_bytes(name.as_bytes()),
                reqwest::header::HeaderValue::from_str(value),
            ) {
                req = req.header(n, v);
            }
        }
        match req.send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let final_url = resp.url().to_string();
                let content_type = resp
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());
                let x_robots = resp
                    .headers()
                    .get("x-robots-tag")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());
                // N24: re-check SSRF on post-redirect URL (literal private targets).
                crate::net::assert_safe_http_url(&final_url)?;
                match crate::net::read_body_limited(resp, opts.max_body_bytes).await {
                    Ok(bytes) => break (status, final_url, bytes, content_type, x_robots),
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

    // NC-SCRAPE-HTTP-STATUS: 4xx/5xx are structured errors (never silent success text).
    if status >= 400 {
        return Err(CliError::with_suggestion(
            if status >= 500 {
                ErrorKind::Unavailable
            } else {
                ErrorKind::Data
            },
            format!("HTTP {status} for {final_url}"),
            crate::i18n::suggestion_key("http_status_scrape", None),
        ));
    }

    let decoded = decode_html_body(&bytes, content_type.as_deref());
    let html = decoded.text;

    // Meta / X-Robots noindex under honor policy.
    if opts.honor_meta_robots && matches!(robots, RobotsPolicy::Honor) {
        let header = parse_x_robots_tag(x_robots.as_deref());
        let meta = parse_meta_robots(&html);
        let merged = merge_robots(header, meta);
        if merged.noindex {
            return Err(CliError::with_suggestion(
                ErrorKind::Data,
                format!(
                    "blocked by page robots noindex ({}): {final_url}",
                    merged.source
                ),
                crate::i18n::suggestion_key("meta_robots_noindex", None),
            ));
        }
    }

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
                content_type: content_type.clone().or_else(|| Some("text/html".into())),
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
        obj.insert("change_status".into(), json!("fresh"));
        obj.insert("http_attempts".into(), json!(attempt));
        obj.insert("charset".into(), json!(decoded.charset));
        if decoded.had_errors {
            obj.insert("charset_had_errors".into(), json!(true));
        }
        if crawl_delay_secs > 0.0 {
            obj.insert("crawl_delay_secs".into(), json!(crawl_delay_secs));
        }
    }
    Ok(payload)
}
