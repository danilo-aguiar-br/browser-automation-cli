// SPDX-License-Identifier: MIT OR Apache-2.0
//! Form steps: fill-form, select-option/pick, upload, write, keys, type.

use std::path::Path;

use serde_json::Value;

use crate::browser::OneShotSession;
use crate::error::{CliError, ErrorKind};

use super::fields::{first_bool, first_present, first_str, include_snapshot};

pub(super) async fn fill_form(
    session: &mut OneShotSession,
    step: &Value,
) -> Result<Value, CliError> {
    // GAP-053, second instance: the published step schema advertises three keys
    // (`fields`, `json`, `fields_json`), but `fields_json` — the CLI long name —
    // was not read here. A step carrying only it failed with "requires fields
    // array or json", telling the caller to pass something it had already passed
    // under a documented name.
    let arr =
        first_present(step, &["fields", "json", "fields_json", "fieldsJson"]).ok_or_else(|| {
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
        let target = first_str(item, &["target", "uid", "selector", "ref"])
            .ok_or_else(|| CliError::new(ErrorKind::Usage, "fill-form field missing target/uid"))?
            .to_string();
        let value = first_str(item, &["value", "text"])
            .ok_or_else(|| CliError::new(ErrorKind::Usage, "fill-form field missing value"))?
            .to_string();
        fields.push((target, value));
    }
    session.fill_form(&fields, include_snapshot(step)).await
}

pub(super) async fn pick_option(
    session: &mut OneShotSession,
    step: &Value,
) -> Result<Value, CliError> {
    // GAP-023: custom select / badge / popover / role=option.
    let target = first_str(step, &["target", "ref", "selector", "trigger"]).ok_or_else(|| {
        CliError::new(
            ErrorKind::Usage,
            "select-option/pick requires target (trigger)",
        )
    })?;
    let option = first_str(step, &["option", "value", "text"]).ok_or_else(|| {
        CliError::new(
            ErrorKind::Usage,
            "select-option/pick requires option (text, selector, or role label)",
        )
    })?;
    session
        .pick_option(target, option, include_snapshot(step))
        .await
}

pub(super) async fn upload(session: &mut OneShotSession, step: &Value) -> Result<Value, CliError> {
    let target = first_str(step, &["target", "ref", "selector", "uid"])
        .ok_or_else(|| CliError::new(ErrorKind::Usage, "upload requires target"))?;
    let path = step
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CliError::new(ErrorKind::Usage, "upload requires path"))?;
    session
        .upload(target, Path::new(path), include_snapshot(step))
        .await
}

pub(super) async fn write(session: &mut OneShotSession, step: &Value) -> Result<Value, CliError> {
    let target = first_str(step, &["target", "ref", "selector"])
        .ok_or_else(|| CliError::new(ErrorKind::Usage, "write requires target"))?;
    let value = first_str(step, &["value", "text"])
        .ok_or_else(|| CliError::new(ErrorKind::Usage, "write requires value"))?;
    session.write(target, value, include_snapshot(step)).await
}

pub(super) async fn keys(session: &mut OneShotSession, step: &Value) -> Result<Value, CliError> {
    let key = step
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CliError::new(ErrorKind::Usage, "keys requires key"))?;
    session.keys(key, include_snapshot(step)).await
}

pub(super) async fn type_text(
    session: &mut OneShotSession,
    step: &Value,
) -> Result<Value, CliError> {
    let focus_only = first_bool(step, &["focus_only", "focusOnly"], false);
    let target = first_str(step, &["target", "ref", "selector", "uid"]);
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
    let clear = step.get("clear").and_then(|v| v.as_bool()).unwrap_or(false);
    let submit = step.get("submit").and_then(|v| v.as_str());
    session
        .type_text(
            target,
            text,
            clear,
            submit,
            focus_only,
            include_snapshot(step),
        )
        .await
}

/// GAP-036: submit a form by the `<form>` itself or by any field inside it.
pub(super) async fn submit(session: &mut OneShotSession, step: &Value) -> Result<Value, CliError> {
    let target = first_str(step, &["target", "ref", "selector"])
        .ok_or_else(|| CliError::new(ErrorKind::Usage, "submit requires target"))?;
    let timeout_ms = step
        .get("timeout_ms")
        .or_else(|| step.get("timeoutMs"))
        .and_then(|v| v.as_u64());
    session
        .submit(target, timeout_ms, include_snapshot(step))
        .await
}
