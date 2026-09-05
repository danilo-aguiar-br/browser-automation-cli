// SPDX-License-Identifier: MIT OR Apache-2.0
//! Tab management and dialog steps.
#![allow(missing_docs, unused_imports)]

use std::path::Path;

use serde_json::{json, Value};

use crate::browser::OneShotSession;
use crate::error::{CliError, ErrorKind};
use crate::robots::RobotsPolicy;

use super::super::super::RunFlags;
use super::super::helpers::step_beforeunload_action;

/// Actions the `page` arm below accepts, aliases included.
///
/// The empty string is a real member: the dispatcher treats `"action": ""` as
/// `info`, so omitting it from the slice would make preflight reject a step the
/// dispatcher runs. See [`COOKIE_ACTIONS`](super::state::COOKIE_ACTIONS) for
/// why the slice lives beside the `match` instead of in the preflight.
pub(crate) const PAGE_ACTIONS: &[&str] = &[
    "info",
    "",
    "list",
    "new",
    "select",
    "close",
    "tab-id",
    "tab_id",
    "get_tab_id",
];

/// Actions the `dialog` arm below accepts.
pub(crate) const DIALOG_ACTIONS: &[&str] = &["accept", "dismiss"];

pub(super) async fn handle(
    session: &mut OneShotSession,
    cmd: &str,
    step: &Value,
) -> Result<Value, CliError> {
    match cmd {
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
                    // GAP-004: tool-ref isolatedContext is a name string; bool true → auto name.
                    let isolated_name: Option<String> = match step
                        .get("isolated_context")
                        .or_else(|| step.get("isolatedContext"))
                    {
                        Some(Value::String(s)) if !s.trim().is_empty() => Some(s.clone()),
                        Some(Value::Bool(true)) => Some("default-isolated".into()),
                        _ => None,
                    };
                    session
                        .page_new(url, background, isolated_name.as_deref())
                        .await
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
                "tab-id" | "tab_id" | "get_tab_id" => {
                    let tab = session.active_tab_id_string().ok_or_else(|| {
                        CliError::with_suggestion(
                            ErrorKind::Browser,
                            "no active tab id",
                            crate::i18n::suggestion_key("navigate_first", None),
                        )
                    })?;
                    Ok(serde_json::json!({
                        "tab_id": tab,
                        "tool": "get_tab_id",
                    }))
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
            let if_present = step
                .get("if_present")
                .or_else(|| step.get("ifPresent"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let result =
                match action {
                    "accept" => session.dialog(true, text).await,
                    // Refuse the pair rather than drop half of it.
                    //
                    // `text` is the prompt response, and CDP only carries it when
                    // the dialog is ACCEPTED: dismissing cancels the prompt, so
                    // there is nothing to submit. Passing `None` here is therefore
                    // the right BEHAVIOUR, but it used to be silent — a step
                    // reading `{"action":"dismiss","text":"..."}` answered
                    // `ok: true` with the text gone and no word about it.
                    //
                    // Same shape as `perf insight --path` beside
                    // `--insight-set-id`: two keys of one action that cannot both
                    // be honoured, where answering the half you can serve looks
                    // like success for a request nobody made.
                    "dismiss" if text.is_some() => return Err(CliError::new(
                        ErrorKind::Usage,
                        "dialog dismiss cancels the prompt and sends no text; drop `text`, or use \
                         action `accept` to submit it",
                    )),
                    "dismiss" => session.dialog(false, None).await,
                    other => {
                        return Err(CliError::new(
                            ErrorKind::Usage,
                            format!("unknown dialog action: {other}"),
                        ))
                    }
                };
            match result {
                Ok(v) => Ok(v),
                Err(e) if if_present => {
                    let msg = e.message().to_ascii_lowercase();
                    if msg.contains("no dialog")
                        || msg.contains("not showing")
                        || msg.contains("-32602")
                        || msg.contains("dialog failed")
                    {
                        Ok(json!({
                            "dialog": action,
                            "dialog_shown": false,
                            "if_present": true,
                            "ok": true,
                        }))
                    } else {
                        Err(e)
                    }
                }
                Err(e) => Err(e),
            }
        }
        other => Err(CliError::new(
            ErrorKind::Usage,
            format!("internal: unexpected cmd in this family: {other}"),
        )),
    }
}
