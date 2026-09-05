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
            ..Default::default()
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
            crate::i18n::suggestion_key("extract_llm_usage", None),
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
    // Ceiling AND allowed-roots, not a bare read: `--schema-json` names a path
    // the caller controls, exactly like `run --script`'s include arm, which
    // calls `ensure_read_allowed` before reading through this same helper.
    //
    // Only half of that symmetry was here. The byte ceiling was applied, so the
    // allocation was bounded; the roots check was not, so the PATH was not — the
    // flag could read a file the workspace policy refuses to any other read in
    // the product. A ceiling answers "how much", and the roots check answers
    // "which file"; they are different questions and this one went unasked.
    let schema_body = match schema_json {
        Some(p) => {
            crate::fs_roots::ensure_read_allowed(p)?;
            Some(crate::json_util::read_text_file_limited(
                p,
                crate::xdg::resolve_max_json_file_bytes(),
            )?)
        }
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
