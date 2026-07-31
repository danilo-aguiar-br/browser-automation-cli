// SPDX-License-Identifier: MIT OR Apache-2.0
//! Pointer steps: hover, drag, press/click, click-at.

use serde_json::Value;

use crate::browser::OneShotSession;
use crate::error::{CliError, ErrorKind};

use super::super::super::RunFlags;
use super::fields::{first_str, include_snapshot};

pub(super) async fn hover(session: &mut OneShotSession, step: &Value) -> Result<Value, CliError> {
    let target = first_str(step, &["target", "ref", "selector"])
        .ok_or_else(|| CliError::new(ErrorKind::Usage, "hover requires target"))?;
    session.hover(target, include_snapshot(step)).await
}

pub(super) async fn drag(session: &mut OneShotSession, step: &Value) -> Result<Value, CliError> {
    use crate::native::interaction::{DragRequest, DropAnchor};

    let from = step
        .get("from")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CliError::new(ErrorKind::Usage, "drag requires --from"))?
        .to_string();
    let to = step.get("to").and_then(|v| v.as_str()).map(str::to_string);
    let to_x = step
        .get("to_x")
        .or_else(|| step.get("toX"))
        .and_then(|v| v.as_f64());
    let to_y = step
        .get("to_y")
        .or_else(|| step.get("toY"))
        .and_then(|v| v.as_f64());
    if to.is_none() && to_x.is_none() && to_y.is_none() {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            "drag requires a destination",
            crate::i18n::suggestion_key("drag_destination_required", None),
        ));
    }
    let anchor = match step.get("anchor").and_then(|v| v.as_str()) {
        Some(a) => DropAnchor::parse(a).map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Usage,
                format!("drag anchor invalid: {e}"),
                crate::i18n::suggestion_key("use_listed_value", None),
            )
        })?,
        None => DropAnchor::parse("center").expect("center is a valid anchor"),
    };
    let synthetic_payload = step
        .get("synthetic_payload")
        .or_else(|| step.get("syntheticPayload"))
        .cloned();
    session
        .drag_ex(
            DragRequest {
                from,
                to,
                to_x,
                to_y,
                anchor,
                synthetic_payload,
            },
            include_snapshot(step),
        )
        .await
}

pub(super) async fn press(session: &mut OneShotSession, step: &Value) -> Result<Value, CliError> {
    let target = first_str(step, &["target", "ref", "selector"])
        .ok_or_else(|| CliError::new(ErrorKind::Usage, "press requires target"))?;
    let dbl = step
        .get("dblclick")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    session.press(target, dbl, include_snapshot(step)).await
}

pub(super) async fn click_at(
    session: &mut OneShotSession,
    step: &Value,
    // Gates are enforced centrally in execute_step from the capability table.
    _flags: RunFlags,
) -> Result<Value, CliError> {
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
    session
        .click_at(x, y, dblclick, include_snapshot(step))
        .await
}
