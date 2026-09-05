// SPDX-License-Identifier: MIT OR Apache-2.0
//! Navigation steps: goto, back, forward, reload.

use serde_json::Value;

use crate::browser::OneShotSession;
use crate::error::{CliError, ErrorKind};
use crate::robots::RobotsPolicy;

use super::super::helpers::step_beforeunload_action;
use super::fields::{step_bool, step_present, step_str};

/// The URL `goto` will navigate to, or `None` when the step carries none.
///
/// Preflight calls this so a `goto` with no URL is refused from argv alone,
/// before a browser is paid for. It is the SAME reader the dispatcher uses, by
/// design: a parallel copy of the accepted key would drift and start rejecting
/// steps that would have run.
pub(crate) fn goto_url(step: &Value) -> Option<&str> {
    step.get("url").and_then(|v| v.as_str())
}

/// Error for a `goto` step with no URL, worded like the dispatcher's.
pub(crate) fn goto_url_error() -> CliError {
    CliError::new(ErrorKind::Usage, "goto requires url")
}

pub(super) async fn goto(
    session: &mut OneShotSession,
    step: &Value,
    robots: RobotsPolicy,
) -> Result<Value, CliError> {
    let url = goto_url(step).ok_or_else(goto_url_error)?;
    let init = step_str(step, "goto", "init_script");
    let beforeunload = step_beforeunload_action(step);
    // `navigationTimeoutMs` and `timeoutMs` were on the old reject-list and in
    // no reader: the validator accepted them and the handler dropped them, so a
    // step asking for a longer navigation timeout silently got the default.
    // Both spellings now come from the same table the validator reads.
    let nav_timeout_ms =
        step_present(step, "goto", "navigation_timeout_ms").and_then(|v| v.as_u64());
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
    let ignore_cache = step_bool(step, "reload", "ignore_cache", false);
    let init = step_str(step, "reload", "init_script");
    // GAP-A009: never inject preventDefault; CDP dialog pump handles beforeunload.
    let beforeunload = step_beforeunload_action(step);
    session
        .reload_with_options(ignore_cache, init, beforeunload)
        .await
}
