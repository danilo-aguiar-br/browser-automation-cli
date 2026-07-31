// SPDX-License-Identifier: MIT OR Apache-2.0
//! Capture/extract steps for run/exec.

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
        "grab" | "screenshot" => {
            let path = step
                .get("path")
                .and_then(|v| v.as_str())
                .map(std::path::PathBuf::from);
            let format = step.get("format").and_then(|v| v.as_str()).unwrap_or("png");
            let full_page = step
                .get("full_page")
                .or_else(|| step.get("fullPage"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let quality = step
                .get("quality")
                .and_then(|v| v.as_i64())
                .map(|n| n as i32);
            let element = step
                .get("element")
                .or_else(|| step.get("selector"))
                .or_else(|| step.get("ref"))
                .and_then(|v| v.as_str());
            session
                .grab(path.as_deref(), format, full_page, quality, element)
                .await
        }
        "extract" => {
            let target = step
                .get("ref")
                .or_else(|| step.get("target"))
                .or_else(|| step.get("selector"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    CliError::new(
                        ErrorKind::Usage,
                        "extract requires ref, target, or selector",
                    )
                })?;
            let attr = step.get("attr").and_then(|v| v.as_str());
            session.extract(target, attr).await
        }
        "text" => {
            let target = step
                .get("ref")
                .or_else(|| step.get("target"))
                .or_else(|| step.get("selector"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    CliError::new(ErrorKind::Usage, "text requires ref, target, or selector")
                })?;
            session.text(target).await
        }
        "scroll" => {
            let target = step
                .get("ref")
                .or_else(|| step.get("target"))
                .or_else(|| step.get("selector"))
                .and_then(|v| v.as_str());
            let delta_x = step
                .get("delta_x")
                .or_else(|| step.get("deltaX"))
                .or_else(|| step.get("dx"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let delta_y = step
                .get("delta_y")
                .or_else(|| step.get("deltaY"))
                .or_else(|| step.get("dy"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let include_snapshot = step
                .get("include_snapshot")
                .or_else(|| step.get("includeSnapshot"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let to_x = step
                .get("to_x")
                .or_else(|| step.get("toX"))
                .and_then(|v| v.as_f64());
            let to_y = step
                .get("to_y")
                .or_else(|| step.get("toY"))
                .and_then(|v| v.as_f64());
            session
                .scroll_ex(
                    crate::native::interaction::ScrollRequest {
                        target,
                        delta_x,
                        delta_y,
                        to_x,
                        to_y,
                    },
                    include_snapshot,
                )
                .await
        }
        other => Err(CliError::new(
            ErrorKind::Usage,
            format!("internal: unexpected cmd in this family: {other}"),
        )),
    }
}
