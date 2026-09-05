// SPDX-License-Identifier: MIT OR Apache-2.0
//! Form steps: fill-form, select-option/pick, upload, submit, write, keys, type.

use std::path::Path;

use serde_json::Value;

use crate::browser::OneShotSession;
use crate::error::{CliError, ErrorKind};
use crate::etd::{with_target, TargetSource};

use super::fields::{include_snapshot, step_bool, step_present, step_str};

/// Table row for the objects inside `fill-form.fields`.
///
/// Not a command: `is_dispatchable_cmd` is consulted before the field tables,
/// so a step naming it is refused as an unknown command.
const FIELDS_ITEM: &str = "fill-form.fields[]";

pub(super) async fn fill_form(
    session: &mut OneShotSession,
    step: &Value,
) -> Result<Value, CliError> {
    // GAP-053, second instance: the published step schema advertises three keys
    // (`fields`, `json`, `fields_json`), but `fields_json` — the CLI long name —
    // was not read here. A step carrying only it failed with "requires fields
    // array or json", telling the caller to pass something it had already passed
    // under a documented name.
    let arr = step_present(step, "fill-form", "fields").ok_or_else(|| {
        CliError::new(ErrorKind::Usage, "fill-form requires fields array or json")
    })?;
    let items = if let Some(s) = arr.as_str() {
        crate::json_util::value_from_str(s)
            .map_err(|e| CliError::new(ErrorKind::Usage, format!("fill-form json: {e}")))?
    } else {
        arr.clone()
    };
    let list = items
        .as_array()
        .ok_or_else(|| CliError::new(ErrorKind::Usage, "fill-form fields must be an array"))?;
    let mut fields = Vec::new();
    for item in list {
        // The spellings inside the array belong to the ITEM, not to the step,
        // so they resolve through the `fill-form.fields[]` row rather than
        // through `fill-form`. That row is what `reject_unknown_item_fields`
        // validates against: while these lists lived inline here, the reader
        // and the validator could disagree, which is the defect this release
        // exists to remove — `uid`, `ref` and `text` were read here and allowed
        // by nothing.
        let target = step_str(item, FIELDS_ITEM, "target")
            .ok_or_else(|| CliError::new(ErrorKind::Usage, "fill-form field missing target/uid"))?
            .to_string();
        let value = step_str(item, FIELDS_ITEM, "value")
            .ok_or_else(|| CliError::new(ErrorKind::Usage, "fill-form field missing value"))?
            .to_string();
        fields.push((target, value));
    }
    let resolved = fields
        .iter()
        .map(|(t, _)| t.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let out = session
        .fill_form(&fields, include_snapshot(step, "fill-form"))
        .await?;
    Ok(with_target(out, &resolved, TargetSource::Step))
}

pub(super) async fn pick_option(
    session: &mut OneShotSession,
    step: &Value,
) -> Result<Value, CliError> {
    // GAP-023: custom select / badge / popover / role=option.
    let target = step_str(step, "select-option", "target").ok_or_else(|| {
        CliError::new(
            ErrorKind::Usage,
            "select-option/pick requires target (trigger)",
        )
    })?;
    let option = step_str(step, "select-option", "option").ok_or_else(|| {
        CliError::new(
            ErrorKind::Usage,
            "select-option/pick requires option (text, selector, or role label)",
        )
    })?;
    let out = session
        .pick_option(target, option, include_snapshot(step, "select-option"))
        .await?;
    Ok(with_target(out, target, TargetSource::Step))
}

pub(super) async fn upload(session: &mut OneShotSession, step: &Value) -> Result<Value, CliError> {
    let target = step_str(step, "upload", "target")
        .ok_or_else(|| CliError::new(ErrorKind::Usage, "upload requires target"))?;
    let path = step
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CliError::new(ErrorKind::Usage, "upload requires path"))?;
    let out = session
        .upload(target, Path::new(path), include_snapshot(step, "upload"))
        .await?;
    Ok(with_target(out, target, TargetSource::Step))
}

pub(super) async fn write(session: &mut OneShotSession, step: &Value) -> Result<Value, CliError> {
    let target = step_str(step, "write", "target")
        .ok_or_else(|| CliError::new(ErrorKind::Usage, "write requires target"))?;
    let value = step_str(step, "write", "value")
        .ok_or_else(|| CliError::new(ErrorKind::Usage, "write requires value"))?;
    let out = session
        .write(target, value, include_snapshot(step, "write"))
        .await?;
    Ok(with_target(out, target, TargetSource::Step))
}

/// Send one named key, optionally focusing an element first.
///
/// # Why `target` exists here
///
/// `keys` is the only step that dispatches a real keystroke, and until now it
/// had no way to say WHERE. The key landed on whatever the page happened to
/// have focused, which is ambient state a script cannot see — so the working
/// recipe was a `press` on the field followed by `keys`, and a caller who
/// instead wrote `key` on the `press` step got `ok: true` and no keystroke at
/// all. With a `target` the pairing is one step and the destination is stated.
pub(super) async fn keys(session: &mut OneShotSession, step: &Value) -> Result<Value, CliError> {
    let key = step
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CliError::new(ErrorKind::Usage, "keys requires key"))?;
    let target = step_str(step, "keys", "target");
    let out = session
        .keys_ex(key, target, include_snapshot(step, "keys"))
        .await?;
    Ok(match target {
        Some(t) => with_target(out, t, TargetSource::Step),
        None => with_target(out, "(focused element)", TargetSource::Ambient),
    })
}

pub(super) async fn type_text(
    session: &mut OneShotSession,
    step: &Value,
) -> Result<Value, CliError> {
    let focus_only = step_bool(step, "type", "focus_only", false);
    let target = step_str(step, "type", "target");
    if target.is_none() && !focus_only {
        return Err(CliError::new(
            ErrorKind::Usage,
            "type requires target or focus_only",
        ));
    }
    // `exec type <target> <text>` writes the positional into BOTH `value` and
    // `text` (see `argv_to_step`), and this read used to see only `text`.
    let text = step_str(step, "type", "text")
        .ok_or_else(|| CliError::new(ErrorKind::Usage, "type requires text"))?;
    let clear = step.get("clear").and_then(|v| v.as_bool()).unwrap_or(false);
    let submit = step.get("submit").and_then(|v| v.as_str());
    let out = session
        .type_text(
            target,
            text,
            clear,
            submit,
            focus_only,
            include_snapshot(step, "type"),
        )
        .await?;
    Ok(match target {
        Some(t) => with_target(out, t, TargetSource::Step),
        None => with_target(out, "(focused element)", TargetSource::Ambient),
    })
}

/// GAP-036: submit a form by the `<form>` itself or by any field inside it.
pub(super) async fn submit(session: &mut OneShotSession, step: &Value) -> Result<Value, CliError> {
    let target = step_str(step, "submit", "target")
        .ok_or_else(|| CliError::new(ErrorKind::Usage, "submit requires target"))?;
    let timeout_ms = step_present(step, "submit", "timeout_ms").and_then(|v| v.as_u64());
    let out = session
        .submit(target, timeout_ms, include_snapshot(step, "submit"))
        .await?;
    Ok(with_target(out, target, TargetSource::Step))
}
