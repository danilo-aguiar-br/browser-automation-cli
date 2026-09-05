// SPDX-License-Identifier: MIT OR Apache-2.0
//! Extension/devtools3p/webmcp steps for run/exec.

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
    flags: RunFlags,
) -> Result<Value, CliError> {
    match cmd {
        "devtools3p-list" | "devtools3p" if selects(cmd, step, "devtools3p-list", "list") => {
            if let Some(url) = step.get("url").and_then(|v| v.as_str()) {
                if url != crate::constants::ABOUT_BLANK {
                    let _ = session
                        .goto(url, crate::robots::RobotsPolicy::Ignore)
                        .await?;
                }
            }
            session.devtools3p_list().await
        }
        "devtools3p-exec" | "devtools3p" if selects(cmd, step, "devtools3p-exec", "exec") => {
            let name = step
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "devtools3p exec needs name"))?;
            let params = step.get("params").and_then(|v| v.as_str());
            if let Some(url) = step.get("url").and_then(|v| v.as_str()) {
                if url != crate::constants::ABOUT_BLANK {
                    let _ = session
                        .goto(url, crate::robots::RobotsPolicy::Ignore)
                        .await?;
                }
            }
            session.devtools3p_exec(name, params).await
        }
        "webmcp-list" | "webmcp" if selects(cmd, step, "webmcp-list", "list") => {
            if let Some(url) = step.get("url").and_then(|v| v.as_str()) {
                if url != crate::constants::ABOUT_BLANK {
                    let _ = session
                        .goto(url, crate::robots::RobotsPolicy::Ignore)
                        .await?;
                }
            }
            session.webmcp_list().await
        }
        "webmcp-exec" | "webmcp" if selects(cmd, step, "webmcp-exec", "exec") => {
            let name = step
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "webmcp exec needs name"))?;
            let input = step.get("input").and_then(|v| v.as_str());
            if let Some(url) = step.get("url").and_then(|v| v.as_str()) {
                if url != crate::constants::ABOUT_BLANK {
                    let _ = session
                        .goto(url, crate::robots::RobotsPolicy::Ignore)
                        .await?;
                }
            }
            session.webmcp_exec(name, input).await
        }
        "extension" => {
            if !flags.category_extensions {
                return Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    "extension tools require --category-extensions",
                    crate::i18n::suggestion_key("category_extensions", None),
                ));
            }
            let action = step
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("list");
            match action {
                "list" => session.extension_list().await,
                "reload" => {
                    let id = step.get("id").and_then(|v| v.as_str()).ok_or_else(|| {
                        CliError::new(ErrorKind::Usage, "extension reload requires id")
                    })?;
                    session.extension_reload(id).await
                }
                "trigger" => {
                    let id = step.get("id").and_then(|v| v.as_str()).ok_or_else(|| {
                        CliError::new(ErrorKind::Usage, "extension trigger requires id")
                    })?;
                    session.extension_trigger(id).await
                }
                other => Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    format!("unsupported extension action in run: {other}"),
                    crate::i18n::suggestion_key("use_listed_value", None),
                )),
            }
        }
        // Reached when the bare family command carries an `action` no arm
        // above accepts. Without these two arms the step fell to the catch-all
        // below, which is written for a routing bug and says so: an operator
        // who mistyped the ACTION was told `internal: unexpected cmd in this
        // family: webmcp`, blaming the command and inviting a bug report.
        "devtools3p" => Err(unknown_family_action("devtools3p", step)),
        "webmcp" => Err(unknown_family_action("webmcp", step)),
        other => Err(CliError::new(
            ErrorKind::Usage,
            format!("internal: unexpected cmd in this family: {other}"),
        )),
    }
}

/// Whether this arm owns the step: the dedicated command spelling, or the bare
/// family command carrying the matching `action`.
///
/// The guards used to test the action ALONE, so `devtools3p-list`,
/// `devtools3p-exec`, `webmcp-list` and `webmcp-exec` — four spellings
/// `RUN_DISPATCHED_CMDS` advertises and `canonical_step_cmd` maps — could not
/// dispatch without ALSO repeating the action they already name. Measured
/// 2026-09-01: `{"cmd":"webmcp-list","url":"..."}` answered `internal:
/// unexpected cmd in this family: webmcp-list`, so an advertised command was
/// unreachable.
///
/// That is the same phantom this module's history records being closed on
/// 2026-08-31, surviving under a different message: the fix then renamed four
/// underscore spellings to hyphens and listed them, and the guards were never
/// taught about them.
fn selects(cmd: &str, step: &Value, dedicated: &str, action: &str) -> bool {
    cmd == dedicated || step.get("action").and_then(|v| v.as_str()) == Some(action)
}

/// The refusal for a family command whose `action` no arm accepts.
fn unknown_family_action(cmd: &str, step: &Value) -> CliError {
    let action = step
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("<missing>");
    CliError::with_suggestion(
        ErrorKind::Usage,
        format!("unsupported {cmd} action in run: {action} (expected `list` or `exec`)"),
        crate::i18n::suggestion_key("use_listed_value", None),
    )
}
