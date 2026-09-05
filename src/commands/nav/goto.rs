// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::browser::{block_on_browser_timeout, run_view, CaptureOpts};
use crate::cli::BeforeUnloadAction;
use crate::commands::common::emit_ok;
use crate::commands::nav::session::with_session_blank;
use crate::error::{CliError, ErrorKind};
use crate::lifecycle::Lifecycle;
use crate::robots::RobotsPolicy;
use std::path::Path;

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_goto(
    life: &Lifecycle,
    url: &str,
    robots: RobotsPolicy,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
    init_script: Option<&str>,
    handle_before_unload: Option<BeforeUnloadAction>,
    navigation_timeout_ms: Option<u64>,
) -> Result<(), CliError> {
    let init = init_script.map(|s| s.to_string());
    let url_owned = url.to_string();
    let beforeunload = handle_before_unload.map(|a| a.as_str());
    let data = block_on_browser_timeout(
        crate::browser::run_goto_with_options(
            life,
            &url_owned,
            capture,
            robots,
            init.as_deref(),
            beforeunload,
            navigation_timeout_ms,
        ),
        timeout_secs,
    )?;
    emit_ok(data, json, |d| {
        let u = d.get("url").and_then(|v| v.as_str()).unwrap_or(url);
        let t = d.get("title").and_then(|v| v.as_str()).unwrap_or("");
        crate::output::writeln_stdout(format!("ok url={u} title={t}"))?;
        Ok(())
    })
}

pub(crate) fn handle_view(
    life: &Lifecycle,
    verbose: bool,
    path: Option<&Path>,
    allow_empty: bool,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let path_owned = path.map(|p| p.to_path_buf());
    let data = block_on_browser_timeout(
        async move {
            let mut data = run_view(life, verbose, capture).await?;
            let tree = data.get("tree").and_then(|v| v.as_str()).unwrap_or("");
            let empty = tree.contains("empty page")
                || data
                    .get("url")
                    .and_then(|v| v.as_str())
                    .is_some_and(|u| u == crate::constants::ABOUT_BLANK);
            if empty && !allow_empty {
                // GAP-020: argv is valid; the page state is not. `Precondition`
                // (75) tells the agent to navigate, where `Usage` (2) told it to
                // fix an argv that was already correct.
                return Err(CliError::with_suggestion(
                    ErrorKind::Precondition,
                    "view returned empty page (no content); refuse silent success",
                    crate::i18n::suggestion_key("navigate_first", None),
                ));
            }
            if let Some(obj) = data.as_object_mut() {
                obj.insert("empty".into(), serde_json::json!(empty));
            }
            if let Some(p) = path_owned.as_ref() {
                // PAR-83: tree dump off async/block_on worker (may be multi-MB).
                let tree = data
                    .get("tree")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                crate::concurrency::write_bytes_blocking(p.clone(), tree.into_bytes())
                    .await
                    .map_err(|e| CliError::new(ErrorKind::Io, format!("view --path write: {e}")))?;
                if let Some(obj) = data.as_object_mut() {
                    obj.insert(
                        "path".to_string(),
                        serde_json::Value::String(p.display().to_string()),
                    );
                }
            }
            Ok(data)
        },
        timeout_secs,
    )?;
    emit_ok(data, json, |d| {
        if let Some(p) = d.get("path").and_then(|v| v.as_str()) {
            crate::output::writeln_stdout(format!("ok view path={p}"))?;
        } else if let Some(tree) = d.get("tree").and_then(|v| v.as_str()) {
            crate::output::write_stdout(tree.as_bytes())?;
            if !tree.ends_with('\n') {
                crate::output::writeln_stdout("")?;
            }
        } else {
            crate::output::writeln_stdout("ok view")?;
        }
        Ok(())
    })
}

pub(crate) fn handle_history(
    life: &Lifecycle,
    direction: &str,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let direction = direction.to_string();
    let direction_label = direction.clone();
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        let v = match direction.as_str() {
            "back" => session.back().await?,
            "forward" => session.forward().await?,
            other => {
                return Err(CliError::new(
                    ErrorKind::Usage,
                    format!("unknown history direction: {other}"),
                ))
            }
        };
        Ok((session, v))
    })?;
    emit_ok(data, json, |_| {
        crate::output::writeln_stdout(format!("ok {direction_label}"))
    })
}

pub(crate) fn handle_reload(
    life: &Lifecycle,
    ignore_cache: bool,
    init_script: Option<&str>,
    handle_before_unload: Option<BeforeUnloadAction>,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    // Single-shot reload without a prior URL cannot apply init_script meaningfully.
    // Require multi-step `run` (session already on a document) OR reject blank-only.
    if init_script.is_some() {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            crate::i18n::suggestion_key("run_script_multi_step", None),
            crate::i18n::suggestion_key("goto_run_script", None),
        ));
    }
    let beforeunload = handle_before_unload.map(|a| a.as_str().to_string());
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        // GAP-A009/A005/A006: CDP Page.reload + dialog pump; no preventDefault inject.
        let v = session
            .reload_with_options(ignore_cache, None, beforeunload.as_deref())
            .await?;
        Ok((session, v))
    })?;
    emit_ok(data, json, |_| {
        crate::output::writeln_stdout(format!("ok reload ignore_cache={ignore_cache}"))
    })
}
