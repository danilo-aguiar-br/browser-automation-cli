// SPDX-License-Identifier: MIT OR Apache-2.0
//! Emulate/resize steps for run/exec.

use serde_json::Value;

use crate::browser::OneShotSession;
use crate::error::{CliError, ErrorKind};
use crate::robots::RobotsPolicy;

use super::super::RunFlags;

pub(super) async fn handle(
    session: &mut OneShotSession,
    cmd: &str,
    step: &Value,
    _robots: RobotsPolicy,
    _flags: RunFlags,
) -> Result<Value, CliError> {
    match cmd {
        "emulate" => {
            apply_step_screen(step)?;
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
            apply_step_screen(step)?;
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
        other => Err(CliError::new(
            ErrorKind::Usage,
            format!("internal: unexpected cmd in this family: {other}"),
        )),
    }
}

fn apply_step_screen(step: &Value) -> Result<(), CliError> {
    let Some(raw) = step.get("screen").and_then(|v| v.as_str()) else {
        return Ok(());
    };
    let size = crate::native::stealth::parse_screen_spec(raw).map_err(|e| {
        CliError::with_suggestion(
            ErrorKind::Usage,
            e,
            crate::i18n::suggestion_key("screen_spec_format", None),
        )
    })?;
    crate::native::stealth::set_screen_override(
        Some(size),
        crate::native::stealth::ScreenSource::Step,
    );
    Ok(())
}
