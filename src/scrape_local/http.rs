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
use super::types::ScrapeOpts;

mod request_url;

use request_url::canonical_request_url;

/// HTTP static scrape (no Chrome).
pub async fn scrape_http(
    url: &str,
    robots: RobotsPolicy,
    opts: &ScrapeOpts,
) -> Result<Value, CliError> {
    // GAP-A004: reject non-HTTP(S) schemes early with an agent-usable suggestion.
    reject_non_http_scheme_for_http_engine(url)?;
    // One spelling of the URL, chosen BEFORE the cache/network branch. See
    // `canonical_request_url` for the defect this closes.
    let canonical = canonical_request_url(url);
    let url: &str = &canonical;
    // Pass N: SSRF policy (XDG http_ssrf_mode; default strict).
    crate::net::assert_safe_http_url(url)?;

    // The token robots is matched against is the one the SERVER saw, which
    // stealth may have rewritten. See `robots::robots_user_agent`.
    crate::robots::enforce_robots(url, robots, &crate::robots::robots_user_agent()).await?;
    // Politeness: Crawl-delay + XDG floor (per origin).
    let crawl_delay_secs = crate::robots::wait_origin(url).await;

    // GAP-005 (`--warmup`): land on the origin root first so the client reaches
    // the target with cookies, the way a browser would. Opt-in; no-op when off.
    super::warmup::warm_origin(url).await;

    // GAP-011: layered XDG cache for GET scrape (hit skips network).
    //
    // The key carries the egress route, not just the address. Keyed on the URL
    // alone, a cached entry was served through a dead `--proxy` as `ok: true`
    // with `cache_hit: true`, so the isolation the operator paid for silently
    // did not happen.
    let cache_ctx = cache::CacheContext {
        proxy: crate::browser_policy::proxy(),
        // `resolved()` and not the raw profile: `auto` means a different
        // identity on a different host, and it is the identity that decides
        // which headers went out.
        stealth_profile: crate::browser_policy::stealth_cache_token(),
        extra_headers: &opts.extra_headers,
    };
    let cache_key = cache::CacheKey::http_get(url, &cache_ctx);
    let mut cache_poisoned = false;
    // A READ bypass, and only a read bypass.
    //
    // `monitor check` hashes whatever body it receives. Served from cache, it
    // compared a stored page against itself and reported `changed: false` for a
    // page that had changed — a false negative carrying `ok: true`, exit 0 and
    // `diff_available: true`, which is the worst shape a wrong answer can take.
    //
    // The write below still runs, so a bypassing caller leaves the entry FRESH
    // for every other command instead of leaving it stale. Skipping the write
    // too would trade one silent staleness for another.
    let cached = if opts.no_cache {
        None
    } else {
        cache::get_async(&cache_key).await.ok().flatten()
    };
    if let Some(entry) = cached {
        // Report where the body actually came from, exactly as the fresh path
        // does from `final_url`. An entry written before the field existed has
        // `None` and falls back to the requested URL, which is precisely the
        // behaviour every cache hit had until now.
        let reported_url = entry.final_url.as_deref().unwrap_or(url);
        // Same G12 branch as the network path, plus one thing the network path
        // does not need: the damage G12 already did is PERSISTENT.
        //
        // Every release up to 0.1.7 ran `decode_html_body` over the raw response
        // and stored the LOSSY UTF-8 result. For a PDF that string is not the
        // document — replacement characters overwrote the binary — and it was
        // filed under `content_type: application/pdf`. So `classify` is right
        // that the entry is a PDF, and `lopdf` is right that it will not load:
        //   measured on arxiv.org/pdf/1706.03762v7, "pdf load failed: couldn't
        //   parse input: invalid file trailer" against a cache entry, while the
        //   same URL with a fresh key returned 15 pages and 32769 characters.
        //
        // Failing here would hand the agent an error it cannot act on, about a
        // file that is perfectly fine at the origin. A cache is an optimisation,
        // never a source of truth: an entry that cannot be decoded is treated as
        // a MISS and the network answers instead.
        let cached_kind = super::content_kind::classify(entry.content_type.as_deref(), &entry.body);
        if !cached_kind.is_html() {
            match super::content_kind::extract(cached_kind, &entry.body) {
                Ok(extracted) => {
                    let mut payload = super::content_kind::build_payload(
                        reported_url,
                        200,
                        cached_kind,
                        entry.content_type.as_deref(),
                        &extracted,
                        opts,
                        robots,
                    );
                    if let Some(obj) = payload.as_object_mut() {
                        obj.insert("cache_hit".into(), json!(true));
                        obj.insert("change_status".into(), json!("unchanged"));
                    }
                    return Ok(payload);
                }
                Err(e) => {
                    tracing::warn!(
                        url = %url,
                        kind = cached_kind.as_str(),
                        error = %e.message(),
                        "cached body does not decode as its own content kind; \
                         re-fetching (entry written before content-type routing)"
                    );
                    cache_poisoned = true;
                }
            }
        }
        if !cache_poisoned {
            // Re-decode cached bytes (BUG-CACHE-UTF8: never assume UTF-8 only).
            let decoded = decode_html_body(&entry.body, entry.content_type.as_deref());
            // A block page cached before detection existed would otherwise be
            // served as content forever: the cache has no headers to re-check, so
            // the body is the only evidence left and must be re-examined on every
            // hit.
            if let Some(hit) = super::block_detect::detect_in_body(&decoded.text) {
                return Err(CliError::with_suggestion(
                    ErrorKind::Blocked,
                    format!(
                        "{} bot check served from cache for {url} (signal {} in {})",
                        hit.waf,
                        hit.signal,
                        hit.phase.as_str()
                    ),
                    hit.suggestion(),
                )
                // Fase 1 of the roadmap asks for three things together: exit 6,
                // `error.kind: blocked` and `data.block_detection`. The first two
                // shipped and the third did not, so the only machine-readable
                // form of WHICH vendor and WHICH signal fired lived in prose
                // inside `message`. An agent had to regex an error string to
                // branch on the WAF, which is the parse-the-prose failure this
                // product refuses everywhere else.
                .with_data(json!({ "block_detection": hit.to_json() })));
            }
            let mut payload = build_scrape_payload_blocking(
                reported_url.to_string(),
                200,
                decoded.text,
                opts.clone(),
                robots,
            )
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
    }

    // Process-wide client (keep-alive + TLS session reuse across batch/crawl).
    let client = shared_http_client()?;

    // GAP-013: retry transient HTTP failures with named policy.
    let cfg = crate::retry::RetryConfig::http();
    let mut attempt = 0u32;
    let (status, final_url, bytes, content_type, x_robots, block_headers, block_cookies) = loop {
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
                // WAF fingerprints must be read while the response is alive:
                // `read_body_limited` consumes it, and the body alone cannot tell
                // a challenge apart from a page that merely discusses CAPTCHAs.
                //
                // Captured OWNED, and the detector runs once later with the body
                // in hand. Calling it here with an empty body made the vendor
                // attribution in `detect` structurally unreachable: with no body
                // only a mitigation header can match, and the later body-only
                // call had no headers left to attribute with. A `cf-ray` next to
                // a `captcha-form` therefore reported `waf: "generic"` while the
                // vendor was sitting in the response the whole time.
                let block_headers: Vec<String> = resp
                    .headers()
                    .keys()
                    .map(|k| k.as_str().to_string())
                    .collect();
                let block_cookies: Vec<String> = resp
                    .headers()
                    .get_all(reqwest::header::SET_COOKIE)
                    .iter()
                    .filter_map(|v| v.to_str().ok())
                    .map(str::to_string)
                    .collect();
                // N24: re-check SSRF on post-redirect URL (literal private targets).
                crate::net::assert_safe_http_url(&final_url)?;
                match crate::net::read_body_limited(resp, opts.max_body_bytes).await {
                    Ok(bytes) => {
                        break (
                            status,
                            final_url,
                            bytes,
                            content_type,
                            x_robots,
                            block_headers,
                            block_cookies,
                        )
                    }
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

    // G12: branch on the type RECEIVED before anything assumes markup.
    //
    // Everything below this point treats the body as HTML, and `decode_html_body`
    // is where that assumption becomes irreversible: a PDF decoded as text yields
    // `%PDF-1.5 %âãÏÓ ... FlateDecode` and travels the rest of the pipeline as
    // "page content", with `exit 0` and `ok: true`. `--only-main-content` is a
    // DOM-selector pass, so it is a silent no-op over binary and cannot catch it.
    //
    // The decision has to be made from the response, never from `--format`: the
    // caller asks for the SHAPE it wants back, which says nothing about what the
    // server actually sent.
    let kind = super::content_kind::classify(content_type.as_deref(), &bytes);
    if !kind.is_html() {
        // `Opaque` returns Err here with a suggestion naming the command that CAN
        // read the file. Refusing is the point: raw bytes under a `text` key is
        // the single outcome this branch exists to prevent.
        let extracted = super::content_kind::extract(kind, &bytes)?;
        return Ok(super::content_kind::build_payload(
            &final_url,
            status,
            kind,
            content_type.as_deref(),
            &extracted,
            opts,
            robots,
        ));
    }

    let decoded = decode_html_body(&bytes, content_type.as_deref());
    let html = decoded.text;

    // A bot check is HTTP 200 with valid HTML, so it survived every transport
    // check above. Without this branch the CAPTCHA is returned as content with
    // exit 0, and the agent quotes the wall as if it were the page.
    // ONE call, headers and body together, so a generic body challenge can still
    // name the vendor standing in front of the origin.
    let header_pairs: Vec<(&str, &str)> = block_headers.iter().map(|n| (n.as_str(), "")).collect();
    let cookie_refs: Vec<&str> = block_cookies.iter().map(String::as_str).collect();
    if let Some(hit) = super::block_detect::detect(header_pairs, cookie_refs, &html) {
        return Err(CliError::with_suggestion(
            ErrorKind::Blocked,
            format!(
                "{} served a bot check for {final_url} (signal {} in {})",
                hit.waf,
                hit.signal,
                hit.phase.as_str()
            ),
            hit.suggestion(),
        )
        .with_data(json!({ "block_detection": hit.to_json() })));
    }

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
                // Store where the body was really served from, so a later hit
                // reports the same origin this fetch is about to report.
                final_url: Some(final_url.clone()),
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
