// SPDX-License-Identifier: MIT OR Apache-2.0
//! Multi-URL batch scrape handler.

use std::path::Path;

use serde_json::json;

use crate::browser::{block_on_browser_timeout, run_scrape, CaptureOpts};
use crate::commands::common::emit_ok;
use crate::error::CliError;
use crate::lifecycle::Lifecycle;
use crate::robots::RobotsPolicy;
use crate::scrape_local::{dedup_similar_pages_envelope, finalize_scrape_value_ex};

use super::formats::build_formats_map;
use super::options::{
    emit_collection, resolve_dedup_similar, resolve_dedup_similar_distance, resolve_max_text,
};

/// Scrape every URL of `urls_file` and emit the aggregated page collection.
#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_batch_scrape(
    life: &Lifecycle,
    urls_file: &Path,
    robots: RobotsPolicy,
    capture: CaptureOpts,
    timeout_secs: u64,
    formats: &[String],
    concurrency: usize,
    engine: &str,
    json: bool,
    select: Option<&str>,
    max_text_chars: Option<usize>,
    filter: Option<&str>,
    output_mode: &str,
    sort: Option<&str>,
    dedup_key: Option<&str>,
    dedup_similar: Option<bool>,
    include_selector: &[String],
    exclude_selector: &[String],
    redact_pii: bool,
    with_content_hash: bool,
) -> Result<(), CliError> {
    let urls = crate::scrape_local::read_urls_file(urls_file)?;
    let engine_l = engine.to_ascii_lowercase();
    let max_text = resolve_max_text(max_text_chars);
    // Near-duplicate collapsing is opt-in (XDG `scrape_dedup_similar`, default
    // off) because it changes how many rows the envelope emits.
    let dedup_similar = resolve_dedup_similar(dedup_similar);
    let dedup_distance = resolve_dedup_similar_distance();
    let formats: Vec<&str> = if formats.is_empty() {
        vec!["text"]
    } else {
        formats.iter().map(String::as_str).collect()
    };

    if engine_l == "browser" {
        // The browser engine drives one CDP session, so this loop is sequential
        // by construction. Reporting the computed permit budget here announced a
        // parallelism that never happens: an agent reading `concurrency_budget:
        // 8` next to a serial loop plans for throughput it will not get. Emit
        // the effective value instead — the `note` already explains why.
        let budget = 1;
        let mut pages = Vec::new();
        let mut errors = Vec::new();
        for u in &urls {
            match block_on_browser_timeout(run_scrape(life, u, robots, capture), timeout_secs) {
                Ok(v) => pages.push(v),
                Err(e) => errors.push(json!({
                    "url": u,
                    "error": e.message(),
                    "http_error": true
                })),
            }
        }
        let data = json!({
            "count": pages.len(),
            "pages": pages,
            "errors": errors,
            "engine": "browser",
            "format": formats,
            "concurrency_budget": budget,
            "note": "browser engine is single-session sequential; use --engine http for parallel fetches",
        });
        let data = dedup_similar_pages_envelope(data, dedup_similar, dedup_distance);
        let data = finalize_scrape_value_ex(data, select, filter, Some(max_text), sort, dedup_key);
        if emit_collection(&data, "pages", output_mode, json, "")? {
            return Ok(());
        }
        return emit_ok(data, json, |d| {
            crate::output::writeln_stdout(format!(
                "ok batch-scrape engine=browser count={}",
                d.get("count").and_then(|v| v.as_u64()).unwrap_or(0)
            ))?;
            Ok(())
        });
    }

    // HTTP: single format uses batch_scrape_http; multi-format fetches html once per URL.
    if formats.len() == 1 {
        let opts = crate::scrape_local::ScrapeOpts {
            format: crate::scrape_local::ScrapeFormat::parse(formats[0])?,
            engine: "http".into(),
            max_text_chars: max_text,
            include_selectors: include_selector.to_vec(),
            exclude_selectors: exclude_selector.to_vec(),
            redact_pii,
            with_content_hash,
            ..Default::default()
        };
        let data = block_on_browser_timeout(
            crate::scrape_local::batch_scrape_http(&urls, robots, &opts, concurrency),
            0,
        )?;
        // Normalize results→pages alias for filter/project
        let mut data = data;
        if let Some(obj) = data.as_object_mut() {
            if obj.get("pages").is_none() {
                if let Some(results) = obj.get("results").cloned() {
                    obj.insert("pages".into(), results);
                }
            }
        }
        let data = dedup_similar_pages_envelope(data, dedup_similar, dedup_distance);
        let data = finalize_scrape_value_ex(data, select, filter, Some(max_text), sort, dedup_key);
        let key = if data.get("pages").is_some() {
            "pages"
        } else {
            "results"
        };
        if emit_collection(&data, key, output_mode, json, "")? {
            return Ok(());
        }
        return emit_ok(data, json, |d| {
            crate::output::writeln_stdout(format!(
                "ok batch-scrape count={}",
                d.get("count").and_then(|v| v.as_u64()).unwrap_or(0)
            ))?;
            Ok(())
        });
    }

    // Multi-format HTTP: sequential per URL (bounded by concurrency via batch of html then derive).
    let opts_html = crate::scrape_local::ScrapeOpts {
        format: crate::scrape_local::ScrapeFormat::Html,
        engine: "http".into(),
        max_text_chars: max_text,
        include_selectors: include_selector.to_vec(),
        exclude_selectors: exclude_selector.to_vec(),
        redact_pii,
        with_content_hash,
        ..Default::default()
    };
    let base = block_on_browser_timeout(
        crate::scrape_local::batch_scrape_http(&urls, robots, &opts_html, concurrency),
        0,
    )?;
    let mut pages = Vec::new();
    let results = base
        .get("results")
        .or_else(|| base.get("pages"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    for page in results {
        let html = page
            .get("html")
            .or_else(|| page.get("content"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let source = page
            .get("source_url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let status = page
            .get("status_code")
            .or_else(|| page.get("status"))
            .and_then(|v| v.as_u64())
            .unwrap_or(200) as u16;
        if html.is_empty() {
            pages.push(page);
            continue;
        }
        let formats_out =
            build_formats_map(&source, status, &html, &formats, &opts_html, "http", robots)?;
        pages.push(json!({
            "source_url": source,
            "engine": "http",
            "formats": formats_out,
            "format_list": formats,
            "status_code": status,
        }));
    }
    let data = json!({
        "ok": base.get("ok").cloned().unwrap_or(json!(true)),
        "count": pages.len(),
        "pages": pages,
        "errors": base.get("errors").cloned().unwrap_or(json!([])),
        "engine": "http",
        "format": formats,
        "multi_format": true,
    });
    let data = dedup_similar_pages_envelope(data, dedup_similar, dedup_distance);
    let data = finalize_scrape_value_ex(data, select, filter, Some(max_text), sort, dedup_key);
    if emit_collection(&data, "pages", output_mode, json, "")? {
        return Ok(());
    }
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!(
            "ok batch-scrape multi-format count={}",
            d.get("count").and_then(|v| v.as_u64()).unwrap_or(0)
        ))?;
        Ok(())
    })
}
