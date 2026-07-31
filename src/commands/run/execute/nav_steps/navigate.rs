// SPDX-License-Identifier: MIT OR Apache-2.0
//! Navigation steps: goto, back, forward, reload.

use serde_json::Value;

use crate::browser::OneShotSession;
use crate::error::{CliError, ErrorKind};
use crate::robots::RobotsPolicy;

use super::super::helpers::step_beforeunload_action;
use super::fields::{first_bool, first_present, first_str};

pub(super) async fn goto(
    session: &mut OneShotSession,
    step: &Value,
    robots: RobotsPolicy,
) -> Result<Value, CliError> {
    let url = step
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CliError::new(ErrorKind::Usage, "goto requires url"))?;
    let init = first_str(step, &["init_script", "initScript"]);
    let beforeunload = step_beforeunload_action(step);
    let nav_timeout_ms =
        first_present(step, &["navigation_timeout_ms", "timeout"]).and_then(|v| v.as_u64());
    session
        .goto_with_options(url, robots, init, beforeunload, nav_timeout_ms)
        .await
}

pub(super) async fn back(session: &mut OneShotSession) -> Result<Value, CliError> {
    session.back().await
}

pub(super) async fn forward(session: &mut OneShotSession) -> Result<Value, CliError> {
    session.forward().await
}

pub(super) async fn reload(session: &mut OneShotSession, step: &Value) -> Result<Value, CliError> {
    let ignore_cache = first_bool(step, &["ignore_cache", "ignoreCache"], false);
    let init = first_str(step, &["init_script", "initScript"]);
    // GAP-A009: never inject preventDefault; CDP dialog pump handles beforeunload.
    let beforeunload = step_beforeunload_action(step);
    session
        .reload_with_options(ignore_cache, init, beforeunload)
        .await
}
