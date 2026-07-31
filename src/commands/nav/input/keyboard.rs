// SPDX-License-Identifier: MIT OR Apache-2.0
//! One-shot keyboard and gesture handlers: keys, type, hover, drag.

use crate::browser::{block_on_browser_timeout, run_keys, run_type, CaptureOpts};
use crate::commands::common::emit_ok;
use crate::commands::nav::session::with_session_blank;
use crate::error::{CliError, ErrorKind};
use crate::lifecycle::Lifecycle;

pub(crate) fn handle_keys(
    life: &Lifecycle,
    key: &str,
    include_snapshot: bool,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let data =
        block_on_browser_timeout(run_keys(life, key, include_snapshot, capture), timeout_secs)?;
    emit_ok(data, json, |_| {
        crate::output::writeln_stdout(format!("ok key={key}"))
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_type(
    life: &Lifecycle,
    target: Option<&str>,
    text: &str,
    clear: bool,
    submit: Option<&str>,
    focus_only: bool,
    include_snapshot: bool,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    if target.is_none() && !focus_only {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            "type requires a target or --focus-only",
            crate::i18n::suggestion_key("target_ref_from_view", None),
        ));
    }
    let data = block_on_browser_timeout(
        run_type(
            life,
            target,
            text,
            clear,
            submit,
            focus_only,
            include_snapshot,
            capture,
        ),
        timeout_secs,
    )?;
    let label = target.unwrap_or("(focused)");
    emit_ok(data, json, |_| {
        crate::output::writeln_stdout(format!(
            "ok typed={label} len={} clear={clear} submit={submit:?} focus_only={focus_only}",
            text.len()
        ))?;
        Ok(())
    })
}

pub(crate) fn handle_hover(
    life: &Lifecycle,
    target: &str,
    include_snapshot: bool,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let target = target.to_string();
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        let v = session.hover(&target, include_snapshot).await?;
        Ok((session, v))
    })?;
    emit_ok(data, json, |_| crate::output::writeln_stdout("ok hover"))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_drag(
    life: &Lifecycle,
    from: &str,
    to: Option<&str>,
    to_x: Option<f64>,
    to_y: Option<f64>,
    anchor: &str,
    synthetic_payload: Option<&str>,
    include_snapshot: bool,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let anchor = crate::native::interaction::DropAnchor::parse(anchor).map_err(|e| {
        CliError::with_suggestion(
            ErrorKind::Usage,
            format!("drag --anchor: {e}"),
            crate::i18n::suggestion_key("use_listed_value", None),
        )
    })?;
    let synthetic_payload = match synthetic_payload {
        Some(raw) => Some(
            crate::json_util::parse_cli_json_value(raw, "drag --synthetic-payload").map_err(
                |e| {
                    CliError::with_suggestion(
                        ErrorKind::Usage,
                        format!("drag --synthetic-payload parse error: {}", e.message()),
                        r#"Pass CDP DragData: {"items":[{"mimeType":"text/plain","data":"x"}]}"#,
                    )
                },
            )?,
        ),
        None => None,
    };
    let req = crate::native::interaction::DragRequest {
        from: from.to_string(),
        to: to.map(str::to_string),
        to_x,
        to_y,
        anchor,
        synthetic_payload,
    };
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        let v = session.drag_ex(req, include_snapshot).await?;
        Ok((session, v))
    })?;
    emit_ok(data, json, |d| {
        let route = d.get("route").and_then(|v| v.as_str()).unwrap_or("unknown");
        crate::output::writeln_stdout(format!("ok drag route={route}"))
    })
}
