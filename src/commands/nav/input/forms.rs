// SPDX-License-Identifier: MIT OR Apache-2.0
//! One-shot form handlers: submit, fill-form, upload, scroll.

use crate::browser::CaptureOpts;
use crate::commands::common::emit_ok;
use crate::commands::nav::session::with_session_blank;
use crate::error::{CliError, ErrorKind};
use crate::lifecycle::Lifecycle;
use std::path::Path;

pub(crate) fn handle_submit(
    life: &Lifecycle,
    target: &str,
    timeout_ms: Option<u64>,
    include_snapshot: bool,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let target = target.to_string();
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        let v = session
            .submit(&target, timeout_ms, include_snapshot)
            .await?;
        Ok((session, v))
    })?;
    emit_ok(data, json, |d| {
        let outcome = d
            .get("outcome")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        crate::output::writeln_stdout(format!("ok submit outcome={outcome}"))
    })
}

pub(crate) fn handle_fill_form(
    life: &Lifecycle,
    fields_json: &str,
    include_snapshot: bool,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let parsed: serde_json::Value = crate::json_util::parse_cli_json_value(
        fields_json,
        "fill-form --fields-json",
    )
    .map_err(|e| {
        CliError::with_suggestion(
            ErrorKind::Usage,
            format!("fill-form --fields-json parse error: {}", e.message()),
            r##"Pass JSON array: [{"target":"input","value":"x"}] or [{"uid":"@e1","value":"x"}]"##,
        )
    })?;
    let arr = parsed.as_array().ok_or_else(|| {
        CliError::with_suggestion(
            ErrorKind::Usage,
            crate::i18n::suggestion_key("json_array_objects", None),
            r##"[{"target":"input","value":"x"}]"##,
        )
    })?;
    let mut fields = Vec::new();
    for item in arr {
        let target = item
            .get("target")
            .or_else(|| item.get("uid"))
            .or_else(|| item.get("selector"))
            .or_else(|| item.get("ref"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                CliError::new(
                    ErrorKind::Usage,
                    "fill-form field missing target/uid/selector/ref",
                )
            })?
            .to_string();
        // Normalize bare eN → @eN
        let target = if target.starts_with('e')
            && target.len() > 1
            && target[1..].chars().all(|c| c.is_ascii_digit())
        {
            format!("@{target}")
        } else {
            target
        };
        let value = item
            .get("value")
            .or_else(|| item.get("text"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| CliError::new(ErrorKind::Usage, "fill-form field missing value"))?
            .to_string();
        fields.push((target, value));
    }
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        let v = session.fill_form(&fields, include_snapshot).await?;
        Ok((session, v))
    })?;
    emit_ok(data, json, |d| {
        let n = d.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
        crate::output::writeln_stdout(format!("ok fill-form count={n}"))?;
        Ok(())
    })
}

pub(crate) fn handle_upload(
    life: &Lifecycle,
    target: &str,
    path: &Path,
    include_snapshot: bool,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let target = target.to_string();
    let path = path.to_path_buf();
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        let v = session.upload(&target, &path, include_snapshot).await?;
        Ok((session, v))
    })?;
    emit_ok(data, json, |d| {
        let p = d.get("path").and_then(|v| v.as_str()).unwrap_or("");
        crate::output::writeln_stdout(format!("ok upload path={p}"))?;
        Ok(())
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_scroll(
    life: &Lifecycle,
    target: Option<&str>,
    delta_x: f64,
    delta_y: f64,
    to_x: Option<f64>,
    to_y: Option<f64>,
    include_snapshot: bool,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let target_owned = target.map(|s| s.to_string());
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        let v = session
            .scroll_ex(
                crate::native::interaction::ScrollRequest {
                    target: target_owned.as_deref(),
                    delta_x,
                    delta_y,
                    to_x,
                    to_y,
                },
                include_snapshot,
            )
            .await?;
        Ok((session, v))
    })?;
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!("ok scroll {d}"))
    })
}
