// SPDX-License-Identifier: MIT OR Apache-2.0
//! LLM-backed extraction artifact handlers (opt-in, XDG key required).

use std::path::Path;

use crate::commands::common::emit_ok;
use crate::error::{CliError, ErrorKind};

pub(crate) fn handle_extract_llm(
    target: &str,
    question: Option<&str>,
    schema_json: Option<&std::path::Path>,
    json: bool,
) -> Result<(), CliError> {
    let source_text = if target.starts_with("http://") || target.starts_with("https://") {
        let opts = crate::scrape_local::ScrapeOpts {
            format: crate::scrape_local::ScrapeFormat::Text,
            only_main_content: true,
            engine: "http".into(),
            max_body_bytes: crate::xdg::policy::policy_usize(
                crate::xdg::policy::key::DEFAULT_BROWSER_SCRAPE_MAX_BODY_BYTES,
            ),
        };
        // HTTP-only: current_thread runtime (rules_rust_latencia — no unbounded
        // multi_thread workers for a one-shot scrape before LLM extract).
        let data = crate::runtime_util::block_on_io(crate::scrape_local::scrape_http(
            target,
            crate::robots::RobotsPolicy::Honor,
            &opts,
        ))?;
        data.get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string()
    } else if Path::new(target).is_file() {
        let parsed = crate::scrape_local::parse_file(Path::new(target))?;
        parsed
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string()
    } else {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            "extract --llm target must be http(s) URL or local file path",
            "Example: browser-automation-cli --json extract --llm --question 'sum' https://example.com",
        ));
    };
    if source_text.trim().is_empty() {
        return Err(CliError::new(
            ErrorKind::Data,
            "extract --llm: empty source text",
        ));
    }
    handle_extract_llm_text(&source_text, question, schema_json, json)
}

/// GAP-015: run LLM extract on already-fetched text (DOM or HTTP).
pub(crate) fn handle_extract_llm_text(
    source_text: &str,
    question: Option<&str>,
    schema_json: Option<&std::path::Path>,
    json: bool,
) -> Result<(), CliError> {
    let schema_body = match schema_json {
        Some(p) => Some(std::fs::read_to_string(p).map_err(|e| {
            CliError::new(
                ErrorKind::Io,
                format!("read schema-json {}: {e}", p.display()),
            )
        })?),
        None => None,
    };
    if source_text.trim().is_empty() {
        return Err(CliError::new(
            ErrorKind::Data,
            "extract --llm: empty source text",
        ));
    }
    let data = crate::llm_local::extract_with_llm(source_text, question, schema_body.as_deref())?;
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!("ok extract-llm {d}"))
    })
}
