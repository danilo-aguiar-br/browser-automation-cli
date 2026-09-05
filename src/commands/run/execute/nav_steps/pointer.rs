// SPDX-License-Identifier: MIT OR Apache-2.0
//! Pointer steps: hover, drag, press/click, click-at.

use serde_json::Value;

use crate::browser::OneShotSession;
use crate::error::{CliError, ErrorKind};
use crate::etd::{with_target, TargetSource};

use super::super::super::RunFlags;
use super::fields::{include_snapshot, step_present, step_str};

pub(super) async fn hover(session: &mut OneShotSession, step: &Value) -> Result<Value, CliError> {
    let target = step_str(step, "hover", "target")
        .ok_or_else(|| CliError::new(ErrorKind::Usage, "hover requires target"))?;
    let out = session
        .hover(target, include_snapshot(step, "hover"))
        .await?;
    Ok(with_target(out, target, TargetSource::Step))
}

pub(super) async fn drag(session: &mut OneShotSession, step: &Value) -> Result<Value, CliError> {
    use crate::native::interaction::{DragRequest, DropAnchor};

    let from = step
        .get("from")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CliError::new(ErrorKind::Usage, "drag requires --from"))?
        .to_string();
    let to = step.get("to").and_then(|v| v.as_str()).map(str::to_string);
    let to_x = step_present(step, "drag", "to_x").and_then(|v| v.as_f64());
    let to_y = step_present(step, "drag", "to_y").and_then(|v| v.as_f64());
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
    let synthetic_payload = step_present(step, "drag", "synthetic_payload").cloned();
    let resolved = match (to.as_deref(), to_x, to_y) {
        (Some(t), _, _) => format!("{from}->{t}"),
        (None, Some(x), Some(y)) => format!("{from}->{x},{y}"),
        _ => from.clone(),
    };
    let out = session
        .drag_ex(
            DragRequest {
                from,
                to,
                to_x,
                to_y,
                anchor,
                synthetic_payload,
            },
            include_snapshot(step, "drag"),
        )
        .await?;
    Ok(with_target(out, &resolved, TargetSource::Step))
}

/// Click an element. `cmd` is `press` or its alias `click`.
///
/// The name is the trap: `press` sounds like a keystroke and is a mouse click.
/// A `key` field on this step is refused by `reject_unknown_step_fields` with a
/// message naming `keys`, because the silent discard that preceded it answered
/// `ok: true` for a step that did nothing.
pub(super) async fn press(session: &mut OneShotSession, step: &Value) -> Result<Value, CliError> {
    let target = step_str(step, "press", "target")
        .ok_or_else(|| CliError::new(ErrorKind::Usage, "press requires target"))?;
    let dbl = step_bool_dblclick(step);
    let out = session
        .press(target, dbl, include_snapshot(step, "press"))
        .await?;
    Ok(with_target(out, target, TargetSource::Step))
}

/// `dblclick` is spelled the same on `press` and `click-at`, so one reader.
fn step_bool_dblclick(step: &Value) -> bool {
    step.get("dblclick")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
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
    let dblclick = step_bool_dblclick(step);
    let out = session
        .click_at(x, y, dblclick, include_snapshot(step, "click-at"))
        .await?;
    Ok(with_target(out, &format!("{x},{y}"), TargetSource::Step))
}
