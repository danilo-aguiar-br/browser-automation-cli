// SPDX-License-Identifier: MIT OR Apache-2.0
//! Offline (non-browser) workflow step execution.

use std::path::Path;

use serde_json::{json, Value};

use super::types::WorkflowStep;
use crate::error::{CliError, ErrorKind};

/// The only engine an offline step can run, because it launches no browser.
const OFFLINE_ENGINE: &str = "http";

/// Field names each offline step reads, or `None` for a step that reads all.
///
/// `noop` and `echo` are absent DELIBERATELY: they echo `args` back verbatim,
/// so every key they receive is read by definition and an allowlist for them
/// would be a list of everything.
fn offline_allowed_fields(cmd: &str) -> Option<&'static [&'static str]> {
    match cmd {
        "parse" => Some(&["path"]),
        "scrape" => Some(&["url", "format", "engine", "only_main_content"]),
        "batch-scrape" | "batch_scrape" => Some(&["urls_file", "urls-file", "format", "engine"]),
        _ => None,
    }
}

/// Refuse a manifest key no offline handler reads.
///
/// # Why the whole family needed this
///
/// Every key not on the list above was accepted and dropped without a word, so
/// a manifest could ask for something the runner never did and still report
/// `ok: true` — the one failure shape a caller cannot detect by reading the
/// envelope, because nothing in it disagrees with the request.
fn reject_unknown_offline_fields(step: &WorkflowStep) -> Result<(), CliError> {
    let Some(allowed) = offline_allowed_fields(step.cmd.as_str()) else {
        return Ok(());
    };
    let Some(obj) = step.args.as_object() else {
        return Ok(());
    };
    let cmd = &step.cmd;
    for key in obj.keys() {
        if allowed.contains(&key.as_str()) {
            continue;
        }
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("unknown field `{key}` in args of offline workflow step cmd={cmd}"),
            format!("Allowed fields for {cmd}: {}", allowed.join(", ")),
        ));
    }
    Ok(())
}

/// Accept `engine` only when it names the engine the step can actually run.
///
/// The published manifest in `docs/COOKBOOK.md` carries `"engine": "http"`, and
/// that value AGREES with what this module does, so refusing the key outright
/// would break a documented example to fix nothing. What had to stop was the
/// other half: `"engine": "browser"` was accepted and silently downgraded to
/// HTTP, so the answer came from a different engine than the one requested and
/// still said `ok: true`.
fn check_offline_engine(step: &WorkflowStep) -> Result<(), CliError> {
    // Steps with no allowlist are the echoing ones, and they take `args`
    // verbatim by contract. Reading a key of theirs as a directive would
    // invent a meaning the step never had.
    if offline_allowed_fields(step.cmd.as_str()).is_none() {
        return Ok(());
    }
    let Some(asked) = step.args.get("engine").and_then(|v| v.as_str()) else {
        return Ok(());
    };
    if asked == OFFLINE_ENGINE {
        return Ok(());
    }
    Err(CliError::with_suggestion(
        ErrorKind::Usage,
        format!(
            "workflow offline step cannot use engine `{asked}`: offline steps launch no browser, \
             so `{OFFLINE_ENGINE}` is the only engine available here"
        ),
        crate::i18n::suggestion_key("scrape_engine_choice", None),
    ))
}

/// Execute one offline step on the runtime the CALLER owns.
///
/// # Why the runtime is a parameter and not a local
///
/// The `scrape` and `batch-scrape` arms below used [`crate::runtime_util::block_on_io`],
/// which builds AND tears down a whole Tokio runtime per call. Called from the
/// step loops in [`super::run`], that meant one runtime per manifest step.
/// The caller now builds a single runtime outside the loop and passes it here.
///
/// Sharing one runtime across calls is only safe because `block_on_with_shutdown`
/// binds its signal task to an `AbortOnDrop` guard: the task dies with the call
/// instead of with the runtime, so N steps do not leave N tasks parked in
/// `shutdown_signal()`.
pub(crate) fn execute_offline_step(
    rt: &tokio::runtime::Runtime,
    step: &WorkflowStep,
) -> Result<Value, CliError> {
    reject_unknown_offline_fields(step)?;
    check_offline_engine(step)?;
    match step.cmd.as_str() {
        "noop" | "echo" => Ok(json!({
            "cmd": step.cmd,
            "args": step.args,
            "ok": true,
        })),
        "parse" => {
            let path = step
                .args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "parse step needs args.path"))?;
            crate::scrape_local::parse_file(Path::new(path))
        }
        "scrape" => {
            // Offline workflow cannot launch browser without lifecycle; require engine=http.
            let url = step
                .args
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "scrape step needs args.url"))?;
            let fmt = step
                .args
                .get("format")
                .and_then(|v| v.as_str())
                .unwrap_or("text");
            let opts = crate::scrape_local::ScrapeOpts {
                format: crate::scrape_local::ScrapeFormat::parse(fmt)?,
                engine: OFFLINE_ENGINE.into(),
                only_main_content: step
                    .args
                    .get("only_main_content")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                ..Default::default()
            };
            // Driven on the caller's shared runtime, not on a per-call one.
            let robots = crate::robots::RobotsPolicy::Honor;
            crate::runtime_util::block_on_with_shutdown(
                rt,
                crate::scrape_local::scrape_http(url, robots, &opts),
                0,
            )
        }
        "batch-scrape" | "batch_scrape" => {
            let path = step
                .args
                .get("urls_file")
                .or_else(|| step.args.get("urls-file"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    CliError::new(ErrorKind::Usage, "batch-scrape needs args.urls_file")
                })?;
            let urls = crate::scrape_local::read_urls_file(Path::new(path))?;
            // `format` is honoured here for the same reason the `scrape` arm
            // above honours it. It used to be pinned to `Text`, so a manifest
            // asking for `html` got text back under `ok: true` — the two arms
            // sat in one file reading the same key and only one obeyed it.
            let fmt = step
                .args
                .get("format")
                .and_then(|v| v.as_str())
                .unwrap_or("text");
            let opts = crate::scrape_local::ScrapeOpts {
                format: crate::scrape_local::ScrapeFormat::parse(fmt)?,
                engine: OFFLINE_ENGINE.into(),
                ..Default::default()
            };
            crate::runtime_util::block_on_with_shutdown(
                rt,
                crate::scrape_local::batch_scrape_http(
                    &urls,
                    crate::robots::RobotsPolicy::Honor,
                    &opts,
                    2,
                ),
                0,
            )
        }
        other => Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("workflow offline step unsupported cmd: {other}"),
            crate::i18n::suggestion_key("use_listed_value", None),
        )),
    }
}
