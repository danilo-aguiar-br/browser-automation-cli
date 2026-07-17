//! `run --script` — multi-step NDJSON, one launch, fail-fast (layers A+B).

use std::path::Path;

use serde_json::{json, Value};

use crate::browser::{CaptureOpts, OneShotSession};
use crate::error::{CliError, ErrorKind};
use crate::lifecycle::Lifecycle;
use crate::robots::RobotsPolicy;

/// Feature flags for multi-step `run` (mirrors global CLI gates).
#[derive(Debug, Clone, Copy, Default)]
pub struct RunFlags {
    pub experimental_vision: bool,
    pub experimental_screencast: bool,
    pub category_memory: bool,
    pub category_extensions: bool,
    pub category_third_party: bool,
    pub category_webmcp: bool,
}

impl RunFlags {
    /// Project CLI global gates into the multi-step dispatcher.
    pub fn from_globals(
        experimental_vision: bool,
        experimental_screencast: bool,
        category_memory: bool,
        category_extensions: bool,
        category_third_party: bool,
        category_webmcp: bool,
    ) -> Self {
        Self {
            experimental_vision,
            experimental_screencast,
            category_memory,
            category_extensions,
            category_third_party,
            category_webmcp,
        }
    }
}

/// Execute NDJSON script with feature gates (vision/screencast/memory).
pub async fn run_script_with_flags(
    life: &Lifecycle,
    script_path: &Path,
    robots: RobotsPolicy,
    capture: CaptureOpts,
    flags: RunFlags,
) -> Result<Value, CliError> {
    let text = std::fs::read_to_string(script_path).map_err(|e| {
        CliError::with_suggestion(
            ErrorKind::NoInput,
            format!("cannot read script {}: {e}", script_path.display()),
            "Pass an existing NDJSON/JSONL file to --script",
        )
    })?;

    let mut steps: Vec<Value> = Vec::new();
    for (lineno, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let v: Value = serde_json::from_str(line).map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Data,
                format!("script line {}: invalid JSON: {e}", lineno + 1),
                "Each non-empty line must be one JSON object with \"cmd\"",
            )
        })?;
        steps.push(v);
    }

    if steps.is_empty() {
        return Err(CliError::with_suggestion(
            ErrorKind::Data,
            "script has no steps",
            "Add at least one NDJSON line with a cmd field",
        ));
    }

    let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
    if let Ok(mut ledger) = life.ledger.lock() {
        ledger.chrome_launched = true;
        ledger.chrome_pid = session.chrome_pid();
    }

    let mut results: Vec<Value> = Vec::new();
    for (idx, step) in steps.iter().enumerate() {
        let cmd = step
            .get("cmd")
            .or_else(|| step.get("action"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let step_res = execute_step(&mut session, cmd, step, robots, flags).await;
        match step_res {
            Ok(data) => {
                results.push(json!({
                    "index": idx,
                    "cmd": cmd,
                    "ok": true,
                    "data": data,
                }));
            }
            Err(e) => {
                let _ = session.shutdown().await;
                if let Ok(mut ledger) = life.ledger.lock() {
                    ledger.chrome_launched = false;
                    ledger.chrome_pid = None;
                }
                // Fail-fast keeps partial steps so agents retain context (GAP-006/016).
                results.push(json!({
                    "index": idx,
                    "cmd": cmd,
                    "ok": false,
                    "error": {
                        "kind": e.kind().as_str(),
                        "message": e.message(),
                        "suggestion": e.suggestion(),
                    }
                }));
                return Ok(json!({
                    "total": steps.len(),
                    "failed_index": idx,
                    "failed_cmd": cmd,
                    "steps": results,
                    "ok": false,
                    "error": {
                        "kind": e.kind().as_str(),
                        "message": format!("run fail-fast at step {idx} cmd={cmd}: {e}"),
                        "suggestion": crate::i18n::suggestion_key("run_fail_fast", None),
                        "exit_code": e.exit_code(),
                    }
                }));
            }
        }
    }

    let close = session.shutdown().await;
    if let Ok(mut ledger) = life.ledger.lock() {
        ledger.chrome_launched = false;
        ledger.chrome_pid = None;
    }
    close?;

    Ok(json!({
        "total": results.len(),
        "steps": results,
    }))
}

/// Reject unknown fields that look like silent discards (agent-first).
fn reject_unknown_step_fields(cmd: &str, step: &Value) -> Result<(), CliError> {
    let Some(obj) = step.as_object() else {
        return Ok(());
    };
    let allowed: &[&str] = match cmd {
        "scroll" => &[
            "cmd", "action", "target", "selector", "delta_x", "delta_y", "deltaX", "deltaY", "dx",
            "dy",
        ],
        "goto" => &[
            "cmd",
            "action",
            "url",
            "init_script",
            "initScript",
            "handle_before_unload",
            "handleBeforeUnload",
            "navigation_timeout_ms",
            "navigationTimeoutMs",
            "timeout_ms",
            "timeoutMs",
        ],
        _ => return Ok(()),
    };
    for key in obj.keys() {
        if !allowed.iter().any(|a| a == key) {
            return Err(CliError::with_suggestion(
                ErrorKind::Usage,
                format!("unknown field `{key}` on step cmd={cmd}"),
                format!("Allowed fields for {cmd}: {}", allowed.join(", ")),
            ));
        }
    }
    Ok(())
}

async fn execute_step(
    session: &mut OneShotSession,
    cmd: &str,
    step: &Value,
    robots: RobotsPolicy,
    flags: RunFlags,
) -> Result<Value, CliError> {
    reject_unknown_step_fields(cmd, step)?;
    match cmd {
        "goto" => {
            let url = step
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "goto requires url"))?;
            session.goto(url, robots).await
        }
        "wait" => {
            let ms = step.get("ms").and_then(|v| v.as_u64());
            let texts: Vec<String> = match step.get("text") {
                Some(Value::Array(arr)) => arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect(),
                Some(Value::String(s)) => vec![s.clone()],
                _ => Vec::new(),
            };
            let selector = step
                .get("selector")
                .or_else(|| step.get("sel"))
                .and_then(|v| v.as_str());
            let state = step.get("state").and_then(|v| v.as_str());
            let include_snapshot = step
                .get("include_snapshot")
                .or_else(|| step.get("includeSnapshot"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if texts.is_empty() && selector.is_none() && state.is_none() && !include_snapshot {
                session.wait_ms(ms.unwrap_or(0)).await
            } else {
                session
                    .wait_for_any(ms, &texts, selector, state, include_snapshot)
                    .await
            }
        }
        "hover" => {
            let target = step
                .get("target")
                .or_else(|| step.get("ref"))
                .or_else(|| step.get("selector"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "hover requires target"))?;
            let include_snapshot = step
                .get("include_snapshot")
                .or_else(|| step.get("includeSnapshot"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            session.hover(target, include_snapshot).await
        }
        "drag" => {
            let from = step
                .get("from")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "drag requires --from"))?;
            let to = step
                .get("to")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "drag requires --to"))?;
            let include_snapshot = step
                .get("include_snapshot")
                .or_else(|| step.get("includeSnapshot"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            session.drag(from, to, include_snapshot).await
        }
        "fill-form" | "fill_form" => {
            let arr = step
                .get("fields")
                .or_else(|| step.get("json"))
                .ok_or_else(|| {
                    CliError::new(ErrorKind::Usage, "fill-form requires fields array or json")
                })?;
            let items = if let Some(s) = arr.as_str() {
                serde_json::from_str::<Value>(s).map_err(|e| {
                    CliError::new(ErrorKind::Usage, format!("fill-form json: {e}"))
                })?
            } else {
                arr.clone()
            };
            let list = items.as_array().ok_or_else(|| {
                CliError::new(ErrorKind::Usage, "fill-form fields must be an array")
            })?;
            let mut fields = Vec::new();
            for item in list {
                let target = item
                    .get("target")
                    .or_else(|| item.get("uid"))
                    .or_else(|| item.get("selector"))
                    .or_else(|| item.get("ref"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        CliError::new(ErrorKind::Usage, "fill-form field missing target/uid")
                    })?
                    .to_string();
                let value = item
                    .get("value")
                    .or_else(|| item.get("text"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        CliError::new(ErrorKind::Usage, "fill-form field missing value")
                    })?
                    .to_string();
                fields.push((target, value));
            }
            let include_snapshot = step
                .get("include_snapshot")
                .or_else(|| step.get("includeSnapshot"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            session.fill_form(&fields, include_snapshot).await
        }
        "upload" => {
            let target = step
                .get("target")
                .or_else(|| step.get("ref"))
                .or_else(|| step.get("selector"))
                .or_else(|| step.get("uid"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "upload requires target"))?;
            let path = step
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "upload requires path"))?;
            let include_snapshot = step
                .get("include_snapshot")
                .or_else(|| step.get("includeSnapshot"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            session
                .upload(target, Path::new(path), include_snapshot)
                .await
        }
        "back" => session.back().await,
        "forward" => session.forward().await,
        "reload" => {
            let ignore_cache = step
                .get("ignore_cache")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            session.reload(ignore_cache).await
        }
        "view" => {
            let verbose = step
                .get("verbose")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            session.view(verbose).await
        }
        "press" | "click" => {
            let target = step
                .get("target")
                .or_else(|| step.get("ref"))
                .or_else(|| step.get("selector"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "press requires target"))?;
            let dbl = step
                .get("dblclick")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let include_snapshot = step
                .get("include_snapshot")
                .or_else(|| step.get("includeSnapshot"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            session.press(target, dbl, include_snapshot).await
        }
        "write" | "fill" => {
            let target = step
                .get("target")
                .or_else(|| step.get("ref"))
                .or_else(|| step.get("selector"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "write requires target"))?;
            let value = step
                .get("value")
                .or_else(|| step.get("text"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "write requires value"))?;
            let include_snapshot = step
                .get("include_snapshot")
                .or_else(|| step.get("includeSnapshot"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            session.write(target, value, include_snapshot).await
        }
        "keys" => {
            let key = step
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "keys requires key"))?;
            let include_snapshot = step
                .get("include_snapshot")
                .or_else(|| step.get("includeSnapshot"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            session.keys(key, include_snapshot).await
        }
        "type" => {
            let focus_only = step
                .get("focus_only")
                .or_else(|| step.get("focusOnly"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let target = step
                .get("target")
                .or_else(|| step.get("ref"))
                .or_else(|| step.get("selector"))
                .or_else(|| step.get("uid"))
                .and_then(|v| v.as_str());
            if target.is_none() && !focus_only {
                return Err(CliError::new(
                    ErrorKind::Usage,
                    "type requires target or focus_only",
                ));
            }
            let text = step
                .get("text")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "type requires text"))?;
            let clear = step
                .get("clear")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let submit = step.get("submit").and_then(|v| v.as_str());
            session
                .type_text(target, text, clear, submit, focus_only)
                .await
        }
        "click-at" | "click_at" => {
            if !flags.experimental_vision {
                return Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    "click-at requires --experimental-vision",
                    "Pass --experimental-vision on the same invocation",
                ));
            }
            let x = step
                .get("x")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "click-at requires x"))?;
            let y = step
                .get("y")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "click-at requires y"))?;
            let dblclick = step
                .get("dblclick")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let include_snapshot = step
                .get("include_snapshot")
                .or_else(|| step.get("includeSnapshot"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            session.click_at(x, y, dblclick, include_snapshot).await
        }
        "eval" => {
            let expr = step
                .get("expression")
                .or_else(|| step.get("function"))
                .or_else(|| step.get("js"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "eval requires expression"))?;
            let args = step.get("args").map(|v| {
                if let Some(s) = v.as_str() {
                    s.to_string()
                } else {
                    v.to_string()
                }
            });
            let dialog_action = step
                .get("dialog_action")
                .or_else(|| step.get("dialogAction"))
                .and_then(|v| v.as_str());
            let file_path = step
                .get("file_path")
                .or_else(|| step.get("filePath"))
                .and_then(|v| v.as_str())
                .map(Path::new);
            session
                .eval(expr, args.as_deref(), dialog_action, file_path)
                .await
        }
        "grab" | "screenshot" => {
            let path = step
                .get("path")
                .and_then(|v| v.as_str())
                .map(std::path::PathBuf::from);
            let format = step
                .get("format")
                .and_then(|v| v.as_str())
                .unwrap_or("png");
            let full_page = step
                .get("full_page")
                .or_else(|| step.get("fullPage"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let quality = step
                .get("quality")
                .and_then(|v| v.as_i64())
                .map(|n| n as i32);
            let element = step
                .get("element")
                .or_else(|| step.get("selector"))
                .or_else(|| step.get("ref"))
                .and_then(|v| v.as_str());
            session
                .grab(path.as_deref(), format, full_page, quality, element)
                .await
        }
        "extract" => {
            let target = step
                .get("ref")
                .or_else(|| step.get("target"))
                .or_else(|| step.get("selector"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    CliError::new(ErrorKind::Usage, "extract requires ref, target, or selector")
                })?;
            let attr = step.get("attr").and_then(|v| v.as_str());
            session.extract(target, attr).await
        }
        "text" => {
            let target = step
                .get("ref")
                .or_else(|| step.get("target"))
                .or_else(|| step.get("selector"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    CliError::new(ErrorKind::Usage, "text requires ref, target, or selector")
                })?;
            session.text(target).await
        }
        "scroll" => {
            let target = step
                .get("ref")
                .or_else(|| step.get("target"))
                .or_else(|| step.get("selector"))
                .and_then(|v| v.as_str());
            let delta_x = step
                .get("delta_x")
                .or_else(|| step.get("deltaX"))
                .or_else(|| step.get("dx"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let delta_y = step
                .get("delta_y")
                .or_else(|| step.get("deltaY"))
                .or_else(|| step.get("dy"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            session.scroll(target, delta_x, delta_y).await
        }
        "cookie" => {
            let action = step
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("list");
            match action {
                "list" => {
                    let url = step.get("url").and_then(|v| v.as_str());
                    session.cookie_list(url).await
                }
                "set" => {
                    let body = step
                        .get("json")
                        .or_else(|| step.get("cookies"))
                        .map(|v| {
                            if let Some(s) = v.as_str() {
                                s.to_string()
                            } else {
                                v.to_string()
                            }
                        })
                        .ok_or_else(|| {
                            CliError::new(ErrorKind::Usage, "cookie set requires json/cookies")
                        })?;
                    session.cookie_set(&body).await
                }
                "clear" => session.cookie_clear().await,
                other => Err(CliError::new(
                    ErrorKind::Usage,
                    format!("unknown cookie action: {other}"),
                )),
            }
        }
        "attr" => {
            let target = step
                .get("ref")
                .or_else(|| step.get("target"))
                .or_else(|| step.get("selector"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    CliError::new(ErrorKind::Usage, "attr requires ref, target, or selector")
                })?;
            let name = step
                .get("name")
                .or_else(|| step.get("attr"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "attr requires name"))?;
            session.attr(target, name).await
        }
        "assert" => execute_assert(session, step).await,
        "console" => {
            let action = step
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("list");
            match action {
                "list" => {
                    let page_idx = step
                        .get("page_idx")
                        .or_else(|| step.get("pageIdx"))
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize);
                    let page_size = step
                        .get("page_size")
                        .or_else(|| step.get("pageSize"))
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize);
                    let types = step.get("types").and_then(|v| v.as_str());
                    let include_preserved = step
                        .get("include_preserved")
                        .or_else(|| step.get("includePreservedMessages"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let sw = step
                        .get("service_worker_id")
                        .or_else(|| step.get("serviceWorkerId"))
                        .and_then(|v| v.as_str());
                    session.console_list(page_idx, page_size, types, include_preserved, sw)
                }
                "get" => {
                    let id = step
                        .get("id")
                        .or_else(|| step.get("msgid"))
                        .or_else(|| step.get("index"))
                        .and_then(|v| v.as_u64())
                        .ok_or_else(|| {
                            CliError::new(
                                ErrorKind::Usage,
                                "console get requires id|msgid|index (0-based)",
                            )
                        })? as usize;
                    session.console_get(id)
                }
                "clear" => session.console_clear(),
                "dump" => {
                    let path = step
                        .get("path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            CliError::new(ErrorKind::Usage, "console dump requires path")
                        })?;
                    session.console_dump(Path::new(path))
                }
                other => Err(CliError::new(
                    ErrorKind::Usage,
                    format!("unknown console action: {other}"),
                )),
            }
        }
        "net" => {
            let action = step
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("list");
            match action {
                "list" => {
                    let page_idx = step
                        .get("page_idx")
                        .or_else(|| step.get("pageIdx"))
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize);
                    let page_size = step
                        .get("page_size")
                        .or_else(|| step.get("pageSize"))
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize);
                    let resource_types = step
                        .get("resource_types")
                        .or_else(|| step.get("resourceTypes"))
                        .and_then(|v| v.as_str());
                    let include_preserved = step
                        .get("include_preserved")
                        .or_else(|| step.get("includePreservedRequests"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    session.net_list(page_idx, page_size, resource_types, include_preserved)
                }
                "get" => {
                    let id = step
                        .get("id")
                        .map(|v| {
                            if let Some(s) = v.as_str() {
                                s.to_string()
                            } else if let Some(n) = v.as_u64() {
                                n.to_string()
                            } else {
                                String::new()
                            }
                        })
                        .filter(|s| !s.is_empty())
                        .ok_or_else(|| {
                            CliError::new(ErrorKind::Usage, "net get requires id (index or requestId)")
                        })?;
                    let request_path = step
                        .get("request_path")
                        .and_then(|v| v.as_str())
                        .map(Path::new);
                    let response_path = step
                        .get("response_path")
                        .and_then(|v| v.as_str())
                        .map(Path::new);
                    session.net_get(&id, request_path, response_path)
                }
                other => Err(CliError::new(
                    ErrorKind::Usage,
                    format!("unknown net action: {other}"),
                )),
            }
        }
        "page" => {
            let action = step
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("info");
            match action {
                "info" | "" => session.page_info().await,
                "list" => session.page_list().await,
                "new" => {
                    let url = step.get("url").and_then(|v| v.as_str());
                    let background = step
                        .get("background")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let isolated = step
                        .get("isolated_context")
                        .or_else(|| step.get("isolatedContext"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    session.page_new(url, background, isolated).await
                }
                "select" => {
                    // Prefer 0-based index; pageId tool-ref alias; tab_id 1-based from page list.
                    let index = if let Some(i) = step
                        .get("index")
                        .or_else(|| step.get("page_id"))
                        .or_else(|| step.get("pageId"))
                        .and_then(|v| v.as_u64())
                    {
                        i as usize
                    } else if let Some(tab_id) = step.get("tab_id").and_then(|v| v.as_u64()) {
                        if tab_id == 0 {
                            return Err(CliError::new(
                                ErrorKind::Usage,
                                "page select tab_id is 1-based (got 0)",
                            ));
                        }
                        (tab_id - 1) as usize
                    } else {
                        return Err(CliError::new(
                            ErrorKind::Usage,
                            "page select requires index/pageId (0-based) or tab_id (1-based)",
                        ));
                    };
                    let bring_to_front = step
                        .get("bring_to_front")
                        .or_else(|| step.get("bringToFront"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);
                    session.page_select(index, bring_to_front).await
                }
                "close" => {
                    let index = step
                        .get("index")
                        .or_else(|| step.get("page_id"))
                        .or_else(|| step.get("pageId"))
                        .and_then(|v| v.as_u64())
                        .map(|i| i as usize);
                    session.page_close(index).await
                }
                other => Err(CliError::new(
                    ErrorKind::Usage,
                    format!("unknown page action: {other}"),
                )),
            }
        }
        "dialog" => {
            let action = step
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("accept");
            let text = step.get("text").and_then(|v| v.as_str());
            match action {
                "accept" => session.dialog(true, text).await,
                "dismiss" => session.dialog(false, None).await,
                other => Err(CliError::new(
                    ErrorKind::Usage,
                    format!("unknown dialog action: {other}"),
                )),
            }
        }
        "emulate" => {
            let headers_owned = step.get("extra_headers").map(|v| {
                if let Some(s) = v.as_str() {
                    s.to_string()
                } else {
                    v.to_string()
                }
            });
            session
                .emulate(
                    step.get("user_agent").and_then(|v| v.as_str()),
                    step.get("locale").and_then(|v| v.as_str()),
                    step.get("timezone").and_then(|v| v.as_str()),
                    step.get("offline")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    step.get("latitude").and_then(|v| v.as_f64()),
                    step.get("longitude").and_then(|v| v.as_f64()),
                    step.get("media").and_then(|v| v.as_str()),
                    step.get("network_conditions").and_then(|v| v.as_str()),
                    step.get("cpu_throttling_rate").and_then(|v| v.as_f64()),
                    step.get("color_scheme").and_then(|v| v.as_str()),
                    headers_owned.as_deref(),
                    step.get("viewport").and_then(|v| v.as_str()),
                )
                .await
        }
        "resize" => {
            let width = step
                .get("width")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "resize requires width"))?
                as i32;
            let height = step
                .get("height")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "resize requires height"))?
                as i32;
            let scale = step.get("scale").and_then(|v| v.as_f64()).unwrap_or(1.0);
            let mobile = step
                .get("mobile")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            session.resize(width, height, scale, mobile).await
        }
        "perf" => {
            let action = step
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("insight");
            match action {
                "start" => {
                    let path = step.get("path").and_then(|v| v.as_str()).map(Path::new);
                    let auto_stop = step
                        .get("auto_stop")
                        .or_else(|| step.get("autoStop"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let reload = step
                        .get("reload")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    session.perf_start(path, reload, auto_stop).await
                }
                "stop" => {
                    let path = step.get("path").and_then(|v| v.as_str()).map(Path::new);
                    session.perf_stop(path).await
                }
                "insight" => {
                    let name = step
                        .get("name")
                        .or_else(|| step.get("insight_name"))
                        .or_else(|| step.get("insightName"))
                        .and_then(|v| v.as_str());
                    let set_id = step
                        .get("insight_set_id")
                        .or_else(|| step.get("insightSetId"))
                        .and_then(|v| v.as_str());
                    session.perf_insight(name, set_id).await
                }
                other => Err(CliError::new(
                    ErrorKind::Usage,
                    format!("unknown perf action: {other}"),
                )),
            }
        }
        "screencast" => {
            if !flags.experimental_screencast {
                return Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    "screencast requires --experimental-screencast",
                    "Pass --experimental-screencast on the same invocation",
                ));
            }
            let action = step
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("start");
            match action {
                "start" => {
                    let path = step
                        .get("path")
                        .or_else(|| step.get("dir"))
                        .and_then(|v| v.as_str())
                        .map(Path::new);
                    session.screencast_start(path).await
                }
                "stop" => {
                    let path = step.get("path").and_then(|v| v.as_str()).map(Path::new);
                    session.screencast_stop(path).await
                }
                other => Err(CliError::new(
                    ErrorKind::Usage,
                    format!("unknown screencast action: {other}"),
                )),
            }
        }
        "heap" => {
            let action = step
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("take");
            let deep = !matches!(action, "take" | "summary" | "close");
            if deep && !flags.category_memory {
                return Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    "deep heap tools require --category-memory",
                    "Pass --category-memory (heap take/summary/close work without deep graph ops)",
                ));
            }
            match action {
                "take" => {
                    let path = step
                        .get("path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| CliError::new(ErrorKind::Usage, "heap take requires path"))?;
                    session.heap_take(Path::new(path)).await
                }
                "summary" => {
                    let path = step
                        .get("path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| CliError::new(ErrorKind::Usage, "heap path required"))?;
                    OneShotSession::heap_file_summary(Path::new(path))
                }
                "close" => {
                    let path = step
                        .get("path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| CliError::new(ErrorKind::Usage, "heap path required"))?;
                    OneShotSession::heap_close(Path::new(path))
                }
                "details" => {
                    let path = step
                        .get("path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| CliError::new(ErrorKind::Usage, "heap path required"))?;
                    OneShotSession::heap_details(Path::new(path))
                }
                "dup-strings" | "dup_strings" => {
                    let path = step
                        .get("path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| CliError::new(ErrorKind::Usage, "heap path required"))?;
                    OneShotSession::heap_dup_strings(Path::new(path))
                }
                "compare" => {
                    let base = step
                        .get("base")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| CliError::new(ErrorKind::Usage, "heap compare needs base"))?;
                    let current = step.get("current").and_then(|v| v.as_str()).ok_or_else(|| {
                        CliError::new(ErrorKind::Usage, "heap compare needs current")
                    })?;
                    OneShotSession::heap_compare(Path::new(base), Path::new(current))
                }
                "class-nodes" => {
                    let path = step
                        .get("path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| CliError::new(ErrorKind::Usage, "heap class-nodes needs path"))?;
                    let id = step
                        .get("id")
                        .and_then(|v| v.as_u64())
                        .ok_or_else(|| CliError::new(ErrorKind::Usage, "heap class-nodes needs id"))?;
                    OneShotSession::heap_class_nodes(Path::new(path), id)
                }
                "dominators" | "edges" | "retainers" | "paths" => {
                    let path = step
                        .get("path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| CliError::new(ErrorKind::Usage, "heap node op needs path"))?;
                    let node = step
                        .get("node")
                        .and_then(|v| v.as_u64())
                        .ok_or_else(|| CliError::new(ErrorKind::Usage, "heap node op needs node"))?;
                    OneShotSession::heap_node_op(Path::new(path), node, action)
                }
                "object-details" | "object_details" => {
                    let path = step
                        .get("path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            CliError::new(ErrorKind::Usage, "heap object-details needs path")
                        })?;
                    let node = step
                        .get("node")
                        .and_then(|v| v.as_u64())
                        .ok_or_else(|| {
                            CliError::new(ErrorKind::Usage, "heap object-details needs node")
                        })?;
                    OneShotSession::heap_object_details(Path::new(path), node)
                }
                other => Err(CliError::new(
                    ErrorKind::Usage,
                    format!("unknown heap action: {other}"),
                )),
            }
        }
        "devtools3p-list" | "devtools3p" if step.get("action").and_then(|v| v.as_str()) == Some("list") => {
            if !flags.category_third_party {
                return Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    "devtools3p tools require --category-third-party",
                    "Pass --category-third-party on the same invocation",
                ));
            }
            if let Some(url) = step.get("url").and_then(|v| v.as_str()) {
                if url != "about:blank" {
                    let _ = session
                        .goto(url, crate::robots::RobotsPolicy::Ignore)
                        .await?;
                }
            }
            session.devtools3p_list().await
        }
        "devtools3p-exec" | "devtools3p" if step.get("action").and_then(|v| v.as_str()) == Some("exec") => {
            if !flags.category_third_party {
                return Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    "devtools3p tools require --category-third-party",
                    "Pass --category-third-party on the same invocation",
                ));
            }
            let name = step
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "devtools3p exec needs name"))?;
            let params = step.get("params").and_then(|v| v.as_str());
            if let Some(url) = step.get("url").and_then(|v| v.as_str()) {
                if url != "about:blank" {
                    let _ = session
                        .goto(url, crate::robots::RobotsPolicy::Ignore)
                        .await?;
                }
            }
            session.devtools3p_exec(name, params).await
        }
        "webmcp-list" | "webmcp" if step.get("action").and_then(|v| v.as_str()) == Some("list") => {
            if !flags.category_webmcp {
                return Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    "webmcp tools require --category-webmcp",
                    "Pass --category-webmcp on the same invocation",
                ));
            }
            if let Some(url) = step.get("url").and_then(|v| v.as_str()) {
                if url != "about:blank" {
                    let _ = session
                        .goto(url, crate::robots::RobotsPolicy::Ignore)
                        .await?;
                }
            }
            session.webmcp_list().await
        }
        "webmcp-exec" | "webmcp" if step.get("action").and_then(|v| v.as_str()) == Some("exec") => {
            if !flags.category_webmcp {
                return Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    "webmcp tools require --category-webmcp",
                    "Pass --category-webmcp on the same invocation",
                ));
            }
            let name = step
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "webmcp exec needs name"))?;
            let input = step.get("input").and_then(|v| v.as_str());
            if let Some(url) = step.get("url").and_then(|v| v.as_str()) {
                if url != "about:blank" {
                    let _ = session
                        .goto(url, crate::robots::RobotsPolicy::Ignore)
                        .await?;
                }
            }
            session.webmcp_exec(name, input).await
        }
        "scrape" => {
            let url = step
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "scrape requires url"))?;
            // Prefer browser engine inside `run` (session already live).
            session.scrape(url, robots).await
        }
        "lighthouse" => {
            let url = step
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "lighthouse requires url"))?;
            let out_dir = step
                .get("out_dir")
                .or_else(|| step.get("outDir"))
                .and_then(|v| v.as_str())
                .map(Path::new);
            let device = step
                .get("device")
                .and_then(|v| v.as_str())
                .unwrap_or("desktop");
            let mode = step
                .get("mode")
                .and_then(|v| v.as_str())
                .unwrap_or("navigation");
            let lighthouse_path = step
                .get("lighthouse_path")
                .or_else(|| step.get("lighthousePath"))
                .and_then(|v| v.as_str())
                .map(Path::new);
            // External binary; run off the browser session but same process.
            super::lighthouse_to_value(url, out_dir, device, mode, lighthouse_path)
        }
        "extension" => {
            if !flags.category_extensions {
                return Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    "extension tools require --category-extensions",
                    "Pass --category-extensions on the same invocation",
                ));
            }
            let action = step
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("list");
            match action {
                "list" => session.extension_list().await,
                "reload" => {
                    let id = step
                        .get("id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            CliError::new(ErrorKind::Usage, "extension reload requires id")
                        })?;
                    session.extension_reload(id).await
                }
                "trigger" => {
                    let id = step
                        .get("id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            CliError::new(ErrorKind::Usage, "extension trigger requires id")
                        })?;
                    session.extension_trigger(id).await
                }
                other => Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    format!("unsupported extension action in run: {other}"),
                    "Use list|reload|trigger in run; install/uninstall are top-level one-shot launches",
                )),
            }
        }
        "" => Err(CliError::new(
            ErrorKind::Usage,
            "step missing cmd/action field",
        )),
        other => Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("unknown script cmd: {other}"),
            "Supported: goto wait hover drag fill-form upload back forward reload view press write keys type click-at eval grab extract text scroll cookie attr assert console net page dialog scrape emulate resize perf lighthouse screencast heap extension devtools3p-list devtools3p-exec webmcp-list webmcp-exec",
        )),
    }
}

/// Map flat `exec` argv into a single NDJSON step object for the shared dispatcher.
pub fn argv_to_step(args: &[String]) -> Result<Value, CliError> {
    if args.is_empty() {
        return Err(CliError::new(ErrorKind::Usage, "exec argv empty"));
    }
    let cmd = args[0].as_str();
    let mut step = json!({ "cmd": cmd });
    let obj = step.as_object_mut().ok_or_else(|| {
        CliError::new(ErrorKind::Software, "exec step object construction failed")
    })?;
    let mut i = 1;
    while i < args.len() {
        let a = args[i].as_str();
        if let Some(key) = a.strip_prefix("--") {
            let key = key.replace('-', "_");
            if i + 1 < args.len() && !args[i + 1].starts_with("--") {
                let val = &args[i + 1];
                if val == "true" || val == "false" {
                    obj.insert(key, json!(val == "true"));
                } else if let Ok(n) = val.parse::<u64>() {
                    obj.insert(key, json!(n));
                } else if let Ok(n) = val.parse::<f64>() {
                    obj.insert(key, json!(n));
                } else {
                    obj.insert(key, json!(val));
                }
                i += 2;
            } else {
                obj.insert(key, json!(true));
                i += 1;
            }
        } else {
            // positional fallbacks by cmd
            match cmd {
                "goto" if !obj.contains_key("url") => {
                    obj.insert("url".into(), json!(a));
                }
                "press" | "write" | "hover" | "type" | "extract" | "attr" | "upload"
                    if !obj.contains_key("target") =>
                {
                    obj.insert("target".into(), json!(a));
                }
                "write" | "type" if obj.contains_key("target") && !obj.contains_key("value") => {
                    obj.insert("value".into(), json!(a));
                    obj.insert("text".into(), json!(a));
                }
                "keys" if !obj.contains_key("key") => {
                    obj.insert("key".into(), json!(a));
                }
                "eval" if !obj.contains_key("expression") => {
                    obj.insert("expression".into(), json!(a));
                }
                "wait" if !obj.contains_key("ms") => {
                    if let Ok(n) = a.parse::<u64>() {
                        obj.insert("ms".into(), json!(n));
                    }
                }
                "view" if a == "verbose" => {
                    obj.insert("verbose".into(), json!(true));
                }
                "net" | "page" | "console" | "dialog" | "perf" | "heap" | "extension"
                | "devtools3p" | "webmcp" | "screencast"
                    if !obj.contains_key("action") =>
                {
                    obj.insert("action".into(), json!(a));
                }
                "net"
                    if obj.get("action").and_then(|v| v.as_str()) == Some("get")
                        && !obj.contains_key("id") =>
                {
                    obj.insert("id".into(), json!(a));
                }
                _ => {
                    return Err(CliError::with_suggestion(
                        ErrorKind::Usage,
                        format!("unrecognized exec argument: {a}"),
                        "Use --flags or run --script NDJSON for complex steps",
                    ));
                }
            }
            i += 1;
        }
    }
    Ok(step)
}

/// Run a single step object in one browser process (exec parity with run).
pub async fn run_one_step(
    life: &Lifecycle,
    step: Value,
    robots: RobotsPolicy,
    capture: CaptureOpts,
    flags: RunFlags,
) -> Result<Value, CliError> {
    let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
    if let Ok(mut ledger) = life.ledger.lock() {
        ledger.chrome_launched = true;
        ledger.chrome_pid = session.chrome_pid();
    }
    let cmd = step
        .get("cmd")
        .or_else(|| step.get("action"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let step_res = execute_step(&mut session, &cmd, &step, robots, flags).await;
    let close = session.shutdown().await;
    if let Ok(mut ledger) = life.ledger.lock() {
        ledger.chrome_launched = false;
        ledger.chrome_pid = None;
    }
    close?;
    step_res.map(|data| {
        json!({
            "cmd": cmd,
            "ok": true,
            "data": data,
        })
    })
}

async fn execute_assert(session: &mut OneShotSession, step: &Value) -> Result<Value, CliError> {
    // Forms:
    // {"cmd":"assert","kind":"url","value":"...","contains":true}
    // {"cmd":"assert","kind":"url","url_contains":"..."}
    // {"cmd":"assert","kind":"text","value":"...","ref":"@e1"}
    // {"cmd":"assert","kind":"console","level":"error","max":0}
    // {"cmd":"assert","url":"..."} / {"cmd":"assert","text":"..."}
    // {"cmd":"assert","url_contains":"..."} / {"cmd":"assert","text_contains":"..."}
    if let Some(kind) = step.get("kind").and_then(|v| v.as_str()) {
        match kind {
            "url" => {
                let value = step
                    .get("value")
                    .or_else(|| step.get("url_contains"))
                    .or_else(|| step.get("url"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        CliError::with_suggestion(
                            ErrorKind::Usage,
                            "assert url requires value",
                            "Use {\"cmd\":\"assert\",\"kind\":\"url\",\"value\":\"example.com\"} or url_contains",
                        )
                    })?;
                let contains = step
                    .get("contains")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                return session.assert_url(value, contains).await;
            }
            "text" => {
                let value = step
                    .get("value")
                    .or_else(|| step.get("text_contains"))
                    .or_else(|| step.get("text"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        CliError::with_suggestion(
                            ErrorKind::Usage,
                            "assert text requires value",
                            "Use {\"cmd\":\"assert\",\"kind\":\"text\",\"value\":\"Hello\"}",
                        )
                    })?;
                let target = step
                    .get("ref")
                    .or_else(|| step.get("target"))
                    .and_then(|v| v.as_str());
                return session.assert_text(value, target).await;
            }
            "console" => {
                let level = step
                    .get("level")
                    .and_then(|v| v.as_str())
                    .unwrap_or("error");
                let max = step.get("max").and_then(|v| v.as_u64()).unwrap_or(0);
                return session.assert_console(level, max).await;
            }
            other => {
                return Err(CliError::new(
                    ErrorKind::Usage,
                    format!("unknown assert kind: {other}"),
                ));
            }
        }
    }
    if let Some(url) = step
        .get("url_contains")
        .or_else(|| step.get("url"))
        .and_then(|v| v.as_str())
    {
        let contains = step
            .get("contains")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        return session.assert_url(url, contains).await;
    }
    if let Some(text) = step
        .get("text_contains")
        .or_else(|| step.get("text"))
        .and_then(|v| v.as_str())
    {
        let target = step
            .get("ref")
            .or_else(|| step.get("target"))
            .and_then(|v| v.as_str());
        return session.assert_text(text, target).await;
    }
    Err(CliError::with_suggestion(
        ErrorKind::Usage,
        "assert requires kind=url|text|console or url/text/url_contains fields",
        "Example: {\"cmd\":\"assert\",\"kind\":\"url\",\"value\":\"example.com\"}",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argv_to_step_goto_and_wait_flags() {
        let step = argv_to_step(&[
            "wait".into(),
            "--ms".into(),
            "250".into(),
            "--state".into(),
            "networkidle".into(),
            "--include-snapshot".into(),
        ])
        .unwrap();
        assert_eq!(step["cmd"], "wait");
        assert_eq!(step["ms"], 250);
        assert_eq!(step["state"], "networkidle");
        assert_eq!(step["include_snapshot"], true);
    }

    #[test]
    fn argv_to_step_net_get_id() {
        let step = argv_to_step(&["net".into(), "get".into(), "req-abc".into()]).unwrap();
        assert_eq!(step["cmd"], "net");
        assert_eq!(step["action"], "get");
        assert_eq!(step["id"], "req-abc");
    }

    #[test]
    fn argv_to_step_press_target() {
        let step = argv_to_step(&[
            "press".into(),
            "#btn".into(),
            "--dblclick".into(),
            "--include-snapshot".into(),
        ])
        .unwrap();
        assert_eq!(step["target"], "#btn");
        assert_eq!(step["dblclick"], true);
        assert_eq!(step["include_snapshot"], true);
    }
}
