// SPDX-License-Identifier: MIT OR Apache-2.0
//! Crawl handler (browser seed+1 hop, or HTTP BFS with multi-format derivation).

use serde_json::json;

use crate::browser::{block_on_browser_timeout, run_scrape, CaptureOpts};
use crate::commands::common::emit_ok;
use crate::error::CliError;
use crate::lifecycle::Lifecycle;
use crate::robots::RobotsPolicy;
use crate::scrape_local::{dedup_similar_pages_envelope, finalize_scrape_value_ex, PathFilter};

use super::formats::build_formats_map;
use super::options::{
    emit_collection, resolve_dedup_similar, resolve_dedup_similar_distance,
    resolve_follow_rel_next, resolve_max_text, resolve_use_sitemap,
};

/// Crawl a seed URL and emit the aggregated page collection.
#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_crawl(
    life: &Lifecycle,
    url: &str,
    robots: RobotsPolicy,
    capture: CaptureOpts,
    timeout_secs: u64,
    limit: usize,
    max_depth: usize,
    formats: &[String],
    same_host: bool,
    engine: &str,
    json: bool,
    select: Option<&str>,
    max_text_chars: Option<usize>,
    filter: Option<&str>,
    output_mode: &str,
    include_path: &[String],
    exclude_path: &[String],
    use_sitemap: Option<bool>,
    ignore_query_params: bool,
    follow_rel_next: Option<bool>,
    dedup_similar: Option<bool>,
    sort: Option<&str>,
    dedup_key: Option<&str>,
    redact_pii: bool,
    with_content_hash: bool,
    include_selector: &[String],
    exclude_selector: &[String],
    dry_run: bool,
) -> Result<(), CliError> {
    let engine_l = engine.to_ascii_lowercase();
    let max_text = resolve_max_text(max_text_chars);
    let path_filter = PathFilter::from_lists(include_path, exclude_path);
    // Near-duplicate collapsing is opt-in (XDG `scrape_dedup_similar`, default
    // off) because it changes how many rows the envelope emits.
    let dedup_similar = resolve_dedup_similar(dedup_similar);
    let dedup_distance = resolve_dedup_similar_distance();
    let use_sm = resolve_use_sitemap(use_sitemap);

    if dry_run {
        // Every knob above is already resolved: flags merged over XDG over
        // defaults. Emitting here is the only point where the effective plan
        // exists and nothing has been fetched yet, which is the whole value —
        // an agent can audit a 500-page crawl before paying for it.
        return emit_ok(
            json!({
                "action": "crawl",
                "dry_run": true,
                "seed": url,
                "engine": engine_l,
                "limit": limit,
                "max_depth": max_depth,
                "same_host": same_host,
                "formats": formats,
                "output_mode": output_mode,
                "robots": format!("{robots:?}").to_ascii_lowercase(),
                "max_text_chars": max_text,
                "use_sitemap": use_sm,
                "ignore_query_params": ignore_query_params,
                "dedup_similar": dedup_similar,
                "dedup_similar_distance": dedup_distance,
                "include_path": include_path,
                "exclude_path": exclude_path,
                "include_selector": include_selector,
                "exclude_selector": exclude_selector,
                "redact_pii": redact_pii,
                "with_content_hash": with_content_hash,
                "timeout_secs": timeout_secs,
                "requests_planned_max": limit,
            }),
            json,
            |d| {
                let seed = d.get("seed").and_then(|v| v.as_str()).unwrap_or("");
                let limit = d
                    .get("limit")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let depth = d
                    .get("max_depth")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let engine = d.get("engine").and_then(|v| v.as_str()).unwrap_or("");
                crate::output::writeln_stdout(format!(
                    "plan crawl seed={seed} engine={engine} limit={limit} max_depth={depth} requests=0"
                ))
            },
        );
    }

    if engine_l == "browser" {
        let seed = block_on_browser_timeout(run_scrape(life, url, robots, capture), timeout_secs)?;
        let mut pages = vec![seed.clone()];
        let mut seen = std::collections::BTreeSet::new();
        seen.insert(url.to_string());
        let links: Vec<String> = seed
            .get("links")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| {
                        x.get("url")
                            .and_then(|v| v.as_str())
                            .or_else(|| x.as_str())
                            .map(|s| s.to_string())
                    })
                    .collect()
            })
            .unwrap_or_default();
        for link in links.into_iter().take(limit.saturating_sub(1)) {
            if !path_filter.allows_url(&link) {
                continue;
            }
            if !seen.insert(link.clone()) {
                continue;
            }
            if same_host {
                if let (Ok(seed_u), Ok(link_u)) = (url::Url::parse(url), url::Url::parse(&link)) {
                    if seed_u.host_str() != link_u.host_str() {
                        continue;
                    }
                }
            }
            match block_on_browser_timeout(run_scrape(life, &link, robots, capture), timeout_secs) {
                Ok(v) => pages.push(v),
                Err(_) => continue,
            }
            if pages.len() >= limit {
                break;
            }
        }
        let _ = max_depth;
        let data = json!({
            "count": pages.len(),
            "pages": pages,
            "engine": "browser",
            "format": formats,
            "seed": url,
            "max_depth_applied": 1,
            "note": "browser crawl is one-shot seed+1 hop; use --engine http for multi-depth BFS",
            "use_sitemap": false,
        });
        let data = dedup_similar_pages_envelope(data, dedup_similar, dedup_distance);
        let data = finalize_scrape_value_ex(data, select, filter, Some(max_text), sort, dedup_key);
        if emit_collection(&data, "pages", output_mode, json, url)? {
            return Ok(());
        }
        return emit_ok(data, json, |d| {
            crate::output::writeln_stdout(format!(
                "ok crawl engine=browser count={}",
                d.get("count").and_then(|v| v.as_u64()).unwrap_or(0)
            ))?;
            Ok(())
        });
    }
    let fmt0 = formats.first().map(|s| s.as_str()).unwrap_or("text");
    let multi = formats.len() > 1;
    let opts = crate::scrape_local::ScrapeOpts {
        format: if multi {
            crate::scrape_local::ScrapeFormat::Html
        } else {
            crate::scrape_local::ScrapeFormat::parse(fmt0)?
        },
        engine: "http".into(),
        max_text_chars: max_text,
        redact_pii,
        with_content_hash,
        include_selectors: include_selector.to_vec(),
        exclude_selectors: exclude_selector.to_vec(),
        follow_rel_next: resolve_follow_rel_next(follow_rel_next),
        ..Default::default()
    };
    let data = block_on_browser_timeout(
        crate::scrape_local::crawl_http(
            url,
            robots,
            &opts,
            limit,
            max_depth,
            same_host,
            &path_filter,
            use_sm,
            ignore_query_params,
        ),
        0,
    )?;
    let mut data = data;
    if multi {
        if let Some(pages) = data.get_mut("pages").and_then(|p| p.as_array_mut()) {
            for page in pages.iter_mut() {
                let html = page
                    .get("html")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let source = page
                    .get("source_url")
                    .and_then(|v| v.as_str())
                    .unwrap_or(url)
                    .to_string();
                let status = page
                    .get("status_code")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(200) as u16;
                if html.is_empty() {
                    continue;
                }
                let fmts: Vec<&str> = formats.iter().map(String::as_str).collect();
                if let Ok(map) =
                    build_formats_map(&source, status, &html, &fmts, &opts, "http", robots)
                {
                    if let Some(obj) = page.as_object_mut() {
                        obj.insert("formats".into(), serde_json::Value::Object(map));
                        obj.insert("format_list".into(), serde_json::json!(formats));
                    }
                }
            }
        }
    }
    let data = dedup_similar_pages_envelope(data, dedup_similar, dedup_distance);
    let data = finalize_scrape_value_ex(data, select, filter, Some(max_text), sort, dedup_key);
    if emit_collection(&data, "pages", output_mode, json, url)? {
        return Ok(());
    }
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!(
            "ok crawl count={}",
            d.get("count").and_then(|v| v.as_u64()).unwrap_or(0)
        ))?;
        Ok(())
    })
}
