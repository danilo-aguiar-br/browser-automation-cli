// SPDX-License-Identifier: MIT OR Apache-2.0
//! Multi-URL batch scrape and crawl handlers.

use std::path::Path;

use serde_json::json;

use crate::browser::{block_on_browser_timeout, run_scrape, CaptureOpts};
use crate::commands::common::emit_ok;
use crate::error::CliError;
use crate::lifecycle::Lifecycle;
use crate::robots::RobotsPolicy;

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_batch_scrape(
    life: &Lifecycle,
    urls_file: &Path,
    robots: RobotsPolicy,
    capture: CaptureOpts,
    timeout_secs: u64,
    format: &str,
    concurrency: usize,
    engine: &str,
    json: bool,
) -> Result<(), CliError> {
    let urls = crate::scrape_local::read_urls_file(urls_file)?;
    let engine_l = engine.to_ascii_lowercase();
    if engine_l == "browser" {
        // GAP-010: one shared Chrome session — sequential navigations by product
        // law (single Page). Parallelism is HTTP engine + CDP internal fan-out.
        // Budget is still reported so agents can size HTTP batch equivalently.
        // Budget reported for agents; browser path stays sequential (single Page).
        let budget = crate::concurrency::resolve_permits(concurrency);
        let mut pages = Vec::new();
        let mut errors = Vec::new();
        for u in &urls {
            match block_on_browser_timeout(run_scrape(life, u, robots, capture), timeout_secs) {
                Ok(v) => pages.push(v),
                Err(e) => errors.push(json!({ "url": u, "error": e.message() })),
            }
        }
        let data = json!({
            "count": pages.len(),
            "pages": pages,
            "errors": errors,
            "engine": "browser",
            "format": format,
            "concurrency_budget": budget,
            "note": "browser engine is single-session sequential; use --engine http for parallel fetches",
        });
        return emit_ok(data, json, |d| {
            crate::output::writeln_stdout(format!(
                "ok batch-scrape engine=browser count={}",
                d.get("count").and_then(|v| v.as_u64()).unwrap_or(0)
            ))?;
            Ok(())
        });
    }
    let opts = crate::scrape_local::ScrapeOpts {
        format: crate::scrape_local::ScrapeFormat::parse(format)?,
        engine: "http".into(),
        ..Default::default()
    };
    let data = block_on_browser_timeout(
        crate::scrape_local::batch_scrape_http(&urls, robots, &opts, concurrency),
        0,
    )?;
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!(
            "ok batch-scrape count={}",
            d.get("count").and_then(|v| v.as_u64()).unwrap_or(0)
        ))?;
        Ok(())
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_crawl(
    life: &Lifecycle,
    url: &str,
    robots: RobotsPolicy,
    capture: CaptureOpts,
    timeout_secs: u64,
    limit: usize,
    max_depth: usize,
    format: &str,
    same_host: bool,
    engine: &str,
    json: bool,
) -> Result<(), CliError> {
    let engine_l = engine.to_ascii_lowercase();
    if engine_l == "browser" {
        // GAP-010: browser crawl = map links via browser scrape of seed then sequential goto.
        let seed = block_on_browser_timeout(run_scrape(life, url, robots, capture), timeout_secs)?;
        let mut pages = vec![seed.clone()];
        let mut seen = std::collections::BTreeSet::new();
        seen.insert(url.to_string());
        let links: Vec<String> = seed
            .get("links")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        for link in links.into_iter().take(limit.saturating_sub(1)) {
            if !seen.insert(link.clone()) {
                continue;
            }
            if same_host {
                // best-effort same host: compare host strings loosely
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
        let _ = max_depth; // depth>1 reserved; seed+1 hop for one-shot safety
        let data = json!({
            "count": pages.len(),
            "pages": pages,
            "engine": "browser",
            "format": format,
            "seed": url,
            "max_depth_applied": 1,
        });
        return emit_ok(data, json, |d| {
            crate::output::writeln_stdout(format!(
                "ok crawl engine=browser count={}",
                d.get("count").and_then(|v| v.as_u64()).unwrap_or(0)
            ))?;
            Ok(())
        });
    }
    let opts = crate::scrape_local::ScrapeOpts {
        format: crate::scrape_local::ScrapeFormat::parse(format)?,
        engine: "http".into(),
        ..Default::default()
    };
    let data = block_on_browser_timeout(
        crate::scrape_local::crawl_http(url, robots, &opts, limit, max_depth, same_host),
        0,
    )?;
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!(
            "ok crawl count={}",
            d.get("count").and_then(|v| v.as_u64()).unwrap_or(0)
        ))?;
        Ok(())
    })
}
