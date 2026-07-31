// SPDX-License-Identifier: MIT OR Apache-2.0
//! Single-URL scrape handler (HTTP and browser engines).

use serde_json::json;

use crate::browser::{block_on_browser_timeout, run_scrape, CaptureOpts};
use crate::commands::common::emit_ok;
use crate::commands::nav::post_webhook;
use crate::error::CliError;
use crate::lifecycle::Lifecycle;
use crate::robots::RobotsPolicy;

use super::formats::build_formats_map;

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_scrape(
    life: &Lifecycle,
    url: &str,
    robots: RobotsPolicy,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
    formats: &[String],
    engine: &str,
    only_main_content: bool,
    webhook_url: Option<&str>,
) -> Result<(), CliError> {
    // GAP-009: support multi-format in one invocation.
    let formats: Vec<&str> = if formats.is_empty() {
        vec!["text"]
    } else {
        formats.iter().map(String::as_str).collect()
    };
    let engine_l = engine.to_ascii_lowercase();

    if engine_l == "http" {
        if formats.len() == 1 {
            let fmt = crate::scrape_local::ScrapeFormat::parse(formats[0])?;
            let opts = crate::scrape_local::ScrapeOpts {
                format: fmt,
                only_main_content,
                engine: "http".into(),
                ..Default::default()
            };
            let data = block_on_browser_timeout(
                crate::scrape_local::scrape_http(url, robots, &opts),
                timeout_secs,
            )?;
            if let Some(wh) = webhook_url {
                post_webhook(wh, &data)?;
            }
            return emit_ok(data, json, |d| {
                let u = d.get("source_url").and_then(|v| v.as_str()).unwrap_or(url);
                crate::output::writeln_stdout(format!("ok scrape engine=http source_url={u}"))?;
                Ok(())
            });
        }
        // Multi-format HTTP: fetch once as html then derive.
        let opts_html = crate::scrape_local::ScrapeOpts {
            format: crate::scrape_local::ScrapeFormat::Html,
            only_main_content,
            engine: "http".into(),
            ..Default::default()
        };
        let base = block_on_browser_timeout(
            crate::scrape_local::scrape_http(url, robots, &opts_html),
            timeout_secs,
        )?;
        let html = base
            .get("html")
            .or_else(|| base.get("content"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let source = base
            .get("source_url")
            .and_then(|v| v.as_str())
            .unwrap_or(url)
            .to_string();
        let status = base.get("status").and_then(|v| v.as_u64()).unwrap_or(200) as u16;
        let formats_out = build_formats_map(
            &source,
            status,
            &html,
            &formats,
            only_main_content,
            "http",
            robots,
        )?;
        let data = json!({
            "source_url": source,
            "engine": "http",
            "formats": formats_out,
            "format_list": formats,
        });
        if let Some(wh) = webhook_url {
            post_webhook(wh, &data)?;
        }
        return emit_ok(data, json, |d| {
            let u = d.get("source_url").and_then(|v| v.as_str()).unwrap_or(url);
            crate::output::writeln_stdout(format!(
                "ok scrape engine=http multi-format source_url={u}"
            ))?;
            Ok(())
        });
    }

    // browser engine: CDP scrape once, derive formats from HTML.
    let data = block_on_browser_timeout(run_scrape(life, url, robots, capture), timeout_secs)?;
    let html = data
        .get("html")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let source = data
        .get("source_url")
        .and_then(|v| v.as_str())
        .unwrap_or(url)
        .to_string();
    let data = if formats.len() == 1 {
        let fmt = crate::scrape_local::ScrapeFormat::parse(formats[0])?;
        if html.is_empty() {
            let mut d = data;
            if let Some(obj) = d.as_object_mut() {
                obj.insert(
                    "format".into(),
                    serde_json::json!(format!("{fmt:?}").to_ascii_lowercase()),
                );
                obj.insert("engine".into(), serde_json::json!("browser"));
            }
            d
        } else {
            let opts = crate::scrape_local::ScrapeOpts {
                format: fmt,
                only_main_content,
                engine: "browser".into(),
                ..Default::default()
            };
            crate::scrape_local::build_scrape_payload(&source, 200, &html, &opts, robots)
        }
    } else {
        let formats_out = build_formats_map(
            &source,
            200,
            &html,
            &formats,
            only_main_content,
            "browser",
            robots,
        )?;
        json!({
            "source_url": source,
            "engine": "browser",
            "formats": formats_out,
            "format_list": formats,
            "robots_policy": robots.as_str(),
        })
    };
    if let Some(wh) = webhook_url {
        post_webhook(wh, &data)?;
    }
    emit_ok(data, json, |d| {
        let policy = d
            .get("robots_policy")
            .and_then(|v| v.as_str())
            .unwrap_or("honor");
        let u = d.get("source_url").and_then(|v| v.as_str()).unwrap_or(url);
        crate::output::writeln_stdout(format!("ok scrape source_url={u} robots_policy={policy}"))?;
        Ok(())
    })
}
