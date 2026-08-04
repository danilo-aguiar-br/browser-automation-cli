// SPDX-License-Identifier: MIT OR Apache-2.0
//! Single-URL scrape handler (HTTP and browser engines).

use serde_json::json;

use crate::browser::{block_on_browser_timeout, run_scrape_wait, CaptureOpts};
use crate::commands::common::emit_ok;
use crate::commands::nav::post_webhook;
use crate::error::CliError;
use crate::lifecycle::Lifecycle;
use crate::robots::RobotsPolicy;
use crate::scrape_local::finalize_scrape_value;

use super::formats::build_formats_map;

fn resolve_max_text(cli: Option<usize>) -> usize {
    match cli {
        Some(0) | None => crate::xdg::resolve_scrape_max_text_chars(),
        Some(n) => n,
    }
}

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
    select: Option<&str>,
    max_text_chars: Option<usize>,
    include_selector: &[String],
    exclude_selector: &[String],
    redact_pii: bool,
    with_content_hash: bool,
    schema_json: Option<&std::path::Path>,
    question: Option<&str>,
    header: &[String],
    wait_ms: u64,
) -> Result<(), CliError> {
    let formats: Vec<&str> = if formats.is_empty() {
        vec!["text"]
    } else {
        formats.iter().map(String::as_str).collect()
    };
    let engine_l = engine.to_ascii_lowercase();
    let max_text = resolve_max_text(max_text_chars);
    let extra_headers = parse_header_flags(header);

    if engine_l == "http" {
        if formats.len() == 1 {
            let fmt = crate::scrape_local::ScrapeFormat::parse(formats[0])?;
            let opts = crate::scrape_local::ScrapeOpts {
                format: fmt,
                only_main_content,
                engine: "http".into(),
                max_text_chars: max_text,
                include_selectors: include_selector.to_vec(),
                exclude_selectors: exclude_selector.to_vec(),
                redact_pii,
                with_content_hash,
                extra_headers,
                ..Default::default()
            };
            let mut data = block_on_browser_timeout(
                crate::scrape_local::scrape_http(url, robots, &opts),
                timeout_secs,
            )?;
            if matches!(fmt, crate::scrape_local::ScrapeFormat::Json) {
                data = maybe_llm_json(data, schema_json, question)?;
            }
            let data = finalize_scrape_value(data, select, None, Some(max_text));
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
            max_text_chars: max_text,
            include_selectors: include_selector.to_vec(),
            exclude_selectors: exclude_selector.to_vec(),
            redact_pii,
            with_content_hash,
            extra_headers: parse_header_flags(header),
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
        let status = base
            .get("status_code")
            .or_else(|| base.get("status"))
            .and_then(|v| v.as_u64())
            .unwrap_or(200) as u16;
        let formats_out =
            build_formats_map(&source, status, &html, &formats, &opts_html, "http", robots)?;
        let data = json!({
            "source_url": source,
            "engine": "http",
            "formats": formats_out,
            "format_list": formats,
        });
        let data = finalize_scrape_value(data, select, None, Some(max_text));
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
    let data = block_on_browser_timeout(
        run_scrape_wait(life, url, robots, capture, wait_ms),
        timeout_secs,
    )?;
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
                max_text_chars: max_text,
                include_selectors: include_selector.to_vec(),
                exclude_selectors: exclude_selector.to_vec(),
                redact_pii,
                with_content_hash,
                ..Default::default()
            };
            crate::scrape_local::build_scrape_payload(&source, 200, &html, &opts, robots)
        }
    } else {
        let opts_br = crate::scrape_local::ScrapeOpts {
            format: crate::scrape_local::ScrapeFormat::Html,
            only_main_content,
            engine: "browser".into(),
            max_text_chars: max_text,
            include_selectors: include_selector.to_vec(),
            exclude_selectors: exclude_selector.to_vec(),
            redact_pii,
            with_content_hash,
            ..Default::default()
        };
        let formats_out =
            build_formats_map(&source, 200, &html, &formats, &opts_br, "browser", robots)?;
        json!({
            "source_url": source,
            "engine": "browser",
            "formats": formats_out,
            "format_list": formats,
            "robots_policy": robots.as_str(),
        })
    };
    let data = finalize_scrape_value(data, select, None, Some(max_text));
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

fn parse_header_flags(headers: &[String]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for h in headers {
        if let Some((n, v)) = h.split_once(':') {
            let n = n.trim();
            let v = v.trim();
            if !n.is_empty() {
                out.push((n.to_string(), v.to_string()));
            }
        }
    }
    out
}

fn maybe_llm_json(
    mut data: serde_json::Value,
    schema_json: Option<&std::path::Path>,
    question: Option<&str>,
) -> Result<serde_json::Value, CliError> {
    if schema_json.is_none() && question.map(|q| q.trim().is_empty()).unwrap_or(true) {
        return Ok(data);
    }
    let key = crate::xdg::openrouter_api_key();
    if key.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true) {
        return Err(CliError::with_suggestion(
            crate::error::ErrorKind::Usage,
            "format json requires XDG openrouter_api_key via config set",
            crate::i18n::suggestion_key("use_listed_value", None),
        ));
    }
    // Source text: prefer markdown/text from payload or nested json placeholder.
    let source_text = data
        .get("text")
        .and_then(|v| v.as_str())
        .or_else(|| data.get("markdown").and_then(|v| v.as_str()))
        .or_else(|| {
            data.get("json")
                .and_then(|j| j.get("text").or_else(|| j.get("markdown")))
                .and_then(|v| v.as_str())
        })
        .unwrap_or("")
        .to_string();
    if source_text.trim().is_empty() {
        return Err(CliError::new(
            crate::error::ErrorKind::Data,
            "format json: empty page text for LLM extract",
        ));
    }
    let schema_body = match schema_json {
        Some(p) => Some(std::fs::read_to_string(p).map_err(|e| {
            CliError::new(
                crate::error::ErrorKind::Io,
                format!("read schema-json {}: {e}", p.display()),
            )
        })?),
        None => None,
    };
    let llm = crate::llm_local::extract_with_llm(&source_text, question, schema_body.as_deref())?;
    if let Some(obj) = data.as_object_mut() {
        obj.insert("json".into(), llm);
        obj.insert("llm_extracted".into(), serde_json::json!(true));
    }
    Ok(data)
}
