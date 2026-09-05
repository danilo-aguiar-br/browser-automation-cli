// SPDX-License-Identifier: MIT OR Apache-2.0
//! Inspection steps: accessibility snapshot (view) and script evaluation (eval).

use std::path::Path;

use serde_json::{json, Value};

use crate::browser::OneShotSession;
use crate::error::{CliError, ErrorKind};

use super::fields::{step_bool, step_str};

pub(super) async fn view(session: &mut OneShotSession, step: &Value) -> Result<Value, CliError> {
    let verbose = step_bool(step, "view", "verbose", false);
    let allow_empty = step_bool(step, "view", "allow_empty", false);
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
        .unwrap_or(crate::constants::ABOUT_BLANK);
    let empty = ref_count == 0
        || url_now == crate::constants::ABOUT_BLANK
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

/// The expression `eval` will read, or `None` when the step carries none.
///
/// Preflight calls this so a malformed `eval` is refused from argv alone. The
/// point is that it is the SAME reader the dispatcher uses, not a copy of the
/// rule: a second list of accepted key names would drift, and a drifted list
/// rejects a step that would have run — the failure mode this module already
/// warns about for `is_dispatchable_cmd` and `known_actions`.
pub(crate) fn eval_expression(step: &Value) -> Option<&str> {
    step_str(step, "eval", "expression")
}

/// Error for an `eval` step with no expression, worded like the dispatcher's.
pub(crate) fn eval_expression_error() -> CliError {
    CliError::new(ErrorKind::Usage, "eval requires expression")
}

pub(super) async fn eval(session: &mut OneShotSession, step: &Value) -> Result<Value, CliError> {
    let expr = eval_expression(step).ok_or_else(eval_expression_error)?;
    let args = step.get("args").map(|v| {
        if let Some(s) = v.as_str() {
            s.to_string()
        } else {
            v.to_string()
        }
    });
    let dialog_action = step_str(step, "eval", "dialog_action");
    let file_path = step_str(step, "eval", "file_path").map(Path::new);
    // GAP-035: `typed` swaps the envelope from `result` to `value` + `value_type`.
    // Declared breaking change, opt-in per step.
    let typed = step.get("typed").and_then(|v| v.as_bool()).unwrap_or(false);
    session
        .eval_ex(expr, args.as_deref(), dialog_action, file_path, typed)
        .await
}
