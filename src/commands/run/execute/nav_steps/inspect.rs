// SPDX-License-Identifier: MIT OR Apache-2.0
//! Inspection steps: accessibility snapshot (view) and script evaluation (eval).

use std::path::Path;

use serde_json::{json, Value};

use crate::browser::OneShotSession;
use crate::error::{CliError, ErrorKind};

use super::fields::{first_bool, first_str};

pub(super) async fn view(session: &mut OneShotSession, step: &Value) -> Result<Value, CliError> {
    let verbose = first_bool(step, &["verbose", "detailed"], false);
    let allow_empty = first_bool(step, &["allow_empty", "allowEmpty"], false);
    let mut data = session.view(verbose).await?;
    let ref_count = data
        .get("ref_count")
        .or_else(|| data.pointer("/snapshot/ref_count"))
        .and_then(|v| v.as_u64())
        .unwrap_or_else(|| {
            data.get("tree")
                .and_then(|v| v.as_str())
                .map(|t| if t.contains("(empty") { 0 } else { 1 })
                .unwrap_or(1)
        });
    let info = session.page_info().await.unwrap_or_else(|_| json!({}));
    let url_now = info
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("about:blank");
    let empty = ref_count == 0
        || url_now == "about:blank"
        || data
            .get("tree")
            .and_then(|v| v.as_str())
            .is_some_and(|t| t.contains("empty page"));
    if empty && !allow_empty {
        // GAP-020: state precondition, not an argv error.
        return Err(CliError::with_suggestion(
            ErrorKind::Precondition,
            "view returned empty page (no refs); refuse silent success",
            crate::i18n::suggestion_key("navigate_first", None),
        ));
    }
    if let Some(obj) = data.as_object_mut() {
        obj.insert("empty".into(), json!(empty));
    }
    Ok(data)
}

pub(super) async fn eval(session: &mut OneShotSession, step: &Value) -> Result<Value, CliError> {
    let expr = first_str(step, &["expression", "function", "js"])
        .ok_or_else(|| CliError::new(ErrorKind::Usage, "eval requires expression"))?;
    let args = step.get("args").map(|v| {
        if let Some(s) = v.as_str() {
            s.to_string()
        } else {
            v.to_string()
        }
    });
    let dialog_action = first_str(step, &["dialog_action", "dialogAction"]);
    let file_path = first_str(step, &["file_path", "filePath"]).map(Path::new);
    // GAP-035: `typed` swaps the envelope from `result` to `value` + `value_type`.
    // Declared breaking change, opt-in per step.
    let typed = step.get("typed").and_then(|v| v.as_bool()).unwrap_or(false);
    session
        .eval_ex(expr, args.as_deref(), dialog_action, file_path, typed)
        .await
}
