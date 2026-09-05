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

/// Actions the `cookie` arm below accepts.
///
/// Preflight consults this slice BEFORE BORN so a typo costs an exit 2 instead
/// of a whole browser. The `match` below consumes the same names, and the slice
/// sits against it on purpose: an arm added without a name added here makes
/// preflight reject a step that would have run. Every action here is exercised
/// by `tests/cookie_jar_gate.rs`, so that divergence fails the suite rather
/// than shipping.
pub(crate) const COOKIE_ACTIONS: &[&str] = &["list", "set", "clear"];

/// Read the `cookie set` payload, accepting either spelling.
///
/// Shared by preflight and the dispatcher so the accepted key names have ONE
/// definition. A second reader in the preflight would drift the moment a third
/// spelling is accepted here.
///
/// `cookies_json` is the third spelling, and it exists because
/// `--cookies-json` is the name on the CLI surface: an author who learned the
/// flag first reaches for it in a step. Declaring it in `STEP_KEY_SYNONYMS`
/// alone was not enough — measured 2026-09-01, that made the field pass
/// validation and then fail here with "requires json/cookies", turning an
/// honest "unknown field" into a confusing one. A spelling is accepted only
/// when BOTH readers accept it.
pub(crate) fn cookie_set_payload(step: &Value) -> Option<String> {
    step.get("json")
        .or_else(|| step.get("cookies"))
        .or_else(|| step.get("cookies_json"))
        .map(|v| {
            if let Some(s) = v.as_str() {
                s.to_string()
            } else {
                v.to_string()
            }
        })
}

/// The error a missing `cookie set` payload produces, wherever it is detected.
pub(crate) fn cookie_set_payload_error() -> CliError {
    CliError::new(
        ErrorKind::Usage,
        "cookie set requires json/cookies/cookies_json",
    )
}

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
                    let body = cookie_set_payload(step).ok_or_else(cookie_set_payload_error)?;
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
