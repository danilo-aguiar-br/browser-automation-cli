// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::browser::{block_on_browser_timeout, CaptureOpts, OneShotSession};
use crate::cli::{AssertKind, DialogAction};
use crate::commands::common::emit_ok;
use crate::error::CliError;
use crate::lifecycle::Lifecycle;

/// Resolve a `0`-or-absent millisecond window to its built-in default.
///
/// The CLI uses `0` as the bare-flag sentinel so the default lives in
/// `constants` instead of being duplicated in a clap attribute literal.
fn window_or_default(requested: Option<u64>, default_ms: u64) -> Option<u64> {
    requested.map(|v| if v == 0 { default_ms } else { v })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_wait(
    life: &Lifecycle,
    ms: u64,
    texts: &[String],
    selector: Option<&str>,
    state: Option<&str>,
    wait_timeout_ms: Option<u64>,
    network_idle_ms: Option<u64>,
    min_count: Option<u64>,
    dom_stable_ms: Option<u64>,
    include_snapshot: bool,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let texts_owned = texts.to_vec();
    let selector_owned = selector.map(|s| s.to_string());
    let state_owned = state.map(|s| s.to_string());
    // Prefer explicit wait_timeout_ms for text/selector; fall back to ms
    let wait_ms = wait_timeout_ms.or(if ms == 0 { None } else { Some(ms) });
    let network_idle_ms = window_or_default(
        network_idle_ms,
        crate::constants::DEFAULT_NETWORK_IDLE_WINDOW_MS,
    );
    let dom_stable_ms = window_or_default(
        dom_stable_ms,
        crate::constants::DEFAULT_DOM_STABLE_WINDOW_MS,
    );
    let data = block_on_browser_timeout(
        async move {
            let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
            life.record_chrome(session.chrome_pid());
            // OR semantics across every condition, evaluated under one deadline
            // (GAP-032). The previous per-text loop restarted the full deadline
            // for each candidate, so N texts could burn N * wait_ms.
            let r = session
                .wait_for_conditions(
                    crate::browser::WaitRequest {
                        ms: wait_ms,
                        texts: &texts_owned,
                        selector: selector_owned.as_deref(),
                        selectors: &[],
                        state: state_owned.as_deref(),
                        url_exact: None,
                        url_contains: None,
                        navigation: false,
                        network_idle_ms,
                        min_count,
                        dom_stable_ms,
                    },
                    include_snapshot,
                )
                .await;
            let close = session.shutdown().await;
            life.clear_chrome();
            close?;
            r
        },
        timeout_secs,
    )?;
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!("ok wait {d}"))?;
        Ok(())
    })
}

pub(crate) fn handle_assert(
    life: &Lifecycle,
    kind: AssertKind,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let data = block_on_browser_timeout(
        async move {
            let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
            life.record_chrome(session.chrome_pid());
            let _ = session
                .goto("about:blank", crate::robots::RobotsPolicy::Honor)
                .await?;
            let r = match kind {
                AssertKind::Url { value, contains } => session.assert_url(&value, contains).await,
                AssertKind::Text { value, target } => {
                    session.assert_text(&value, target.as_deref()).await
                }
                AssertKind::Console { level, max } => session.assert_console(&level, max).await,
                AssertKind::ConsoleEmpty => session.assert_console_empty().await,
                AssertKind::ConsoleNoMatch { pattern } => {
                    session.assert_console_no_match(&pattern).await
                }
            };
            let close = session.shutdown().await;
            life.clear_chrome();
            close?;
            r
        },
        timeout_secs,
    )?;
    emit_ok(data, json, |_| crate::output::writeln_stdout("ok assert"))
}

pub(crate) fn handle_dialog(
    life: &Lifecycle,
    action: DialogAction,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let data = block_on_browser_timeout(
        async move {
            let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
            life.record_chrome(session.chrome_pid());
            let _ = session
                .goto("about:blank", crate::robots::RobotsPolicy::Honor)
                .await?;
            let (r, if_present) = match action {
                DialogAction::Accept { text, if_present } => {
                    (session.dialog(true, text.as_deref()).await, if_present)
                }
                DialogAction::Dismiss { if_present } => {
                    (session.dialog(false, None).await, if_present)
                }
            };
            let r = match r {
                Ok(v) => Ok(v),
                Err(e) if if_present => {
                    let msg = e.message().to_ascii_lowercase();
                    if msg.contains("no dialog")
                        || msg.contains("not showing")
                        || msg.contains("-32602")
                        || msg.contains("dialog failed")
                    {
                        Ok(serde_json::json!({
                            "dialog_shown": false,
                            "if_present": true,
                            "ok": true,
                        }))
                    } else {
                        Err(e)
                    }
                }
                Err(e) => Err(e),
            };
            let close = session.shutdown().await;
            life.clear_chrome();
            close?;
            r
        },
        timeout_secs,
    )?;
    emit_ok(data, json, |_| crate::output::writeln_stdout("ok dialog"))
}
