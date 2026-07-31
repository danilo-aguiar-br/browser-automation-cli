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
        "devtools3p-list" | "devtools3p"
            if step.get("action").and_then(|v| v.as_str()) == Some("list") =>
        {
            if let Some(url) = step.get("url").and_then(|v| v.as_str()) {
                if url != "about:blank" {
                    let _ = session
                        .goto(url, crate::robots::RobotsPolicy::Ignore)
                        .await?;
                }
            }
            session.devtools3p_list().await
        }
        "devtools3p-exec" | "devtools3p"
            if step.get("action").and_then(|v| v.as_str()) == Some("exec") =>
        {
            let name = step
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "devtools3p exec needs name"))?;
            let params = step.get("params").and_then(|v| v.as_str());
            if let Some(url) = step.get("url").and_then(|v| v.as_str()) {
                if url != "about:blank" {
                    let _ = session
                        .goto(url, crate::robots::RobotsPolicy::Ignore)
                        .await?;
                }
            }
            session.devtools3p_exec(name, params).await
        }
        "webmcp-list" | "webmcp" if step.get("action").and_then(|v| v.as_str()) == Some("list") => {
            if let Some(url) = step.get("url").and_then(|v| v.as_str()) {
                if url != "about:blank" {
                    let _ = session
                        .goto(url, crate::robots::RobotsPolicy::Ignore)
                        .await?;
                }
            }
            session.webmcp_list().await
        }
        "webmcp-exec" | "webmcp" if step.get("action").and_then(|v| v.as_str()) == Some("exec") => {
            let name = step
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "webmcp exec needs name"))?;
            let input = step.get("input").and_then(|v| v.as_str());
            if let Some(url) = step.get("url").and_then(|v| v.as_str()) {
                if url != "about:blank" {
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
        other => Err(CliError::new(
            ErrorKind::Usage,
            format!("internal: unexpected cmd in this family: {other}"),
        )),
    }
}
