// SPDX-License-Identifier: MIT OR Apache-2.0
//! DOM text / attribute extraction artifact handlers.

use std::path::Path;

use crate::browser::{block_on_browser_timeout, CaptureOpts, OneShotSession};
use crate::commands::common::emit_ok;
use crate::error::{CliError, ErrorKind};
use crate::lifecycle::Lifecycle;

use super::extract_llm::{handle_extract_llm, handle_extract_llm_text};

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_extract(
    life: &Lifecycle,
    target: &str,
    attr: Option<&str>,
    llm: bool,
    question: Option<&str>,
    schema_json: Option<&std::path::Path>,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    if llm {
        // GAP-015: DOM selector/ref → textContent then LLM (XDG key); http(s)/file still supported.
        if !(target.starts_with("http://")
            || target.starts_with("https://")
            || Path::new(target).is_file())
        {
            let target_owned = target.to_string();
            let attr_owned = attr.map(|s| s.to_string());
            let dom = block_on_browser_timeout(
                async move {
                    let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
                    life.record_chrome(session.chrome_pid());
                    // Selector alone needs a live page; agents should prefer run multi-step.
                    // Best-effort: extract on about:blank fails → clear usage.
                    let r = session.extract(&target_owned, attr_owned.as_deref()).await;
                    let close = session.shutdown().await;
                    life.clear_chrome();
                    close?;
                    r
                },
                timeout_secs,
            );
            match dom {
                Ok(v) => {
                    let text = v
                        .get("text")
                        .or_else(|| v.get("value"))
                        .and_then(|t| t.as_str())
                        .unwrap_or("")
                        .to_string();
                    if text.trim().is_empty() {
                        return Err(CliError::with_suggestion(
                            ErrorKind::Usage,
                            "extract --llm with selector produced empty text (need navigated page)",
                            crate::i18n::suggestion_key("run_script_multi_step", None),
                        ));
                    }
                    return handle_extract_llm_text(&text, question, schema_json, json);
                }
                Err(e) => {
                    return Err(CliError::with_suggestion(
                        ErrorKind::Usage,
                        format!("extract --llm selector path failed: {}", e.message()),
                        crate::i18n::suggestion_key("url_absolute_http", None),
                    ));
                }
            }
        }
        return handle_extract_llm(target, question, schema_json, json);
    }
    let target = target.to_string();
    let attr = attr.map(|s| s.to_string());
    let data = block_on_browser_timeout(
        async move {
            let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
            life.record_chrome(session.chrome_pid());
            let _ = session
                .goto("about:blank", crate::robots::RobotsPolicy::Honor)
                .await?;
            let r = session.extract(&target, attr.as_deref()).await;
            let blank = session
                .page_info()
                .await
                .ok()
                .and_then(|i| {
                    i.get("url")
                        .and_then(|v| v.as_str())
                        .map(|u| u == "about:blank")
                })
                .unwrap_or(false);
            let close = session.shutdown().await;
            life.clear_chrome();
            close?;
            r.map(|data| (data, blank))
        },
        timeout_secs,
    )?;
    let (data, blank) = data;
    // GAP-020: `text` and `attr` read PAGE CONTENT, so an empty answer on
    // about:blank is indistinguishable from "the caller forgot to navigate".
    // `view` already refuses this; `cookie` does not, because it reads BROWSER
    // state, where empty is a true answer. Argv is valid here, so the kind is
    // `Precondition` (75) and never `Usage` (2), which would send an agent to
    // re-read `--help` for a command line that was already correct.
    if blank && extracted_is_empty(&data) {
        return Err(CliError::with_suggestion(
            ErrorKind::Precondition,
            "extract returned no content on about:blank; refuse silent success",
            crate::i18n::suggestion_key("navigate_first", None),
        ));
    }
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!("ok extract {d}"))
    })
}

pub(crate) fn handle_attr(
    life: &Lifecycle,
    target: &str,
    name: &str,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    handle_extract(
        life,
        target,
        Some(name),
        false,
        None,
        None,
        capture,
        timeout_secs,
        json,
    )
}

pub(crate) fn handle_text(
    life: &Lifecycle,
    target: &str,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    handle_extract(
        life,
        target,
        None,
        false,
        None,
        None,
        capture,
        timeout_secs,
        json,
    )
}

/// True when an extract payload carries no usable content.
///
/// Checks the fields `session.extract` fills (`text`, `value`, `attribute`);
/// a payload with none of them non-empty is an empty read.
fn extracted_is_empty(data: &serde_json::Value) -> bool {
    ["text", "value", "attribute"]
        .iter()
        .filter_map(|k| data.get(*k))
        .filter_map(|v| v.as_str())
        .all(|s| s.trim().is_empty())
}
