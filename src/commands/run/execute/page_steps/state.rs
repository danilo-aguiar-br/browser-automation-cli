// SPDX-License-Identifier: MIT OR Apache-2.0
//! Cookie jar and attribute steps.
#![allow(missing_docs, unused_imports)]

use std::path::Path;

use serde_json::{json, Value};

use crate::browser::OneShotSession;
use crate::error::{CliError, ErrorKind};
use crate::robots::RobotsPolicy;

use super::super::super::RunFlags;
use super::super::helpers::step_beforeunload_action;
pub(super) async fn handle(
    session: &mut OneShotSession,
    cmd: &str,
    step: &Value,
) -> Result<Value, CliError> {
    match cmd {
        "cookie" => {
            let action = step
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("list");
            match action {
                "list" => {
                    let url = step.get("url").and_then(|v| v.as_str());
                    session.cookie_list(url).await
                }
                "set" => {
                    let body = step
                        .get("json")
                        .or_else(|| step.get("cookies"))
                        .map(|v| {
                            if let Some(s) = v.as_str() {
                                s.to_string()
                            } else {
                                v.to_string()
                            }
                        })
                        .ok_or_else(|| {
                            CliError::new(ErrorKind::Usage, "cookie set requires json/cookies")
                        })?;
                    session.cookie_set(&body).await
                }
                "clear" => session.cookie_clear().await,
                other => Err(CliError::new(
                    ErrorKind::Usage,
                    format!("unknown cookie action: {other}"),
                )),
            }
        }
        "attr" => {
            let target = step
                .get("ref")
                .or_else(|| step.get("target"))
                .or_else(|| step.get("selector"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    CliError::new(ErrorKind::Usage, "attr requires ref, target, or selector")
                })?;
            let name = step
                .get("name")
                .or_else(|| step.get("attr"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "attr requires name"))?;
            session.attr(target, name).await
        }
        other => Err(CliError::new(
            ErrorKind::Usage,
            format!("internal: unexpected cmd in this family: {other}"),
        )),
    }
}
