// SPDX-License-Identifier: MIT OR Apache-2.0
//! One-shot pointer handlers: press, write, click-at.

use crate::browser::OneShotSession;
use crate::browser::{block_on_browser_timeout, run_press, run_write, CaptureOpts};
use crate::commands::common::emit_ok;
use crate::error::CliError;
use crate::etd::{with_target, TargetSource};
use crate::lifecycle::Lifecycle;

pub(crate) fn handle_press(
    life: &Lifecycle,
    target: &str,
    dblclick: bool,
    include_snapshot: bool,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let data = block_on_browser_timeout(
        run_press(life, target, dblclick, include_snapshot, capture),
        timeout_secs,
    )?;
    let data = with_target(data, target, TargetSource::Argv);
    emit_ok(data, json, |_| {
        crate::output::writeln_stdout(format!("ok pressed={target} dblclick={dblclick}"))?;
        Ok(())
    })
}

pub(crate) fn handle_write(
    life: &Lifecycle,
    target: &str,
    value: &str,
    include_snapshot: bool,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let data = block_on_browser_timeout(
        run_write(life, target, value, include_snapshot, capture),
        timeout_secs,
    )?;
    let data = with_target(data, target, TargetSource::Argv);
    emit_ok(data, json, |_| {
        crate::output::writeln_stdout(format!("ok written={target} len={}", value.len()))?;
        Ok(())
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_click_at(
    life: &Lifecycle,
    x: f64,
    y: f64,
    dblclick: bool,
    include_snapshot: bool,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let data = block_on_browser_timeout(
        async move {
            let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
            life.record_chrome(session.chrome_pid());
            let _ = session
                .goto(
                    crate::constants::ABOUT_BLANK,
                    crate::robots::RobotsPolicy::Honor,
                )
                .await?;
            let r = session.click_at(x, y, dblclick, include_snapshot).await;
            let close = session.shutdown().await;
            life.clear_chrome();
            close?;
            r
        },
        timeout_secs,
    )?;
    let data = with_target(data, &format!("{x},{y}"), TargetSource::Argv);
    emit_ok(data, json, |_| {
        crate::output::writeln_stdout(format!("ok click-at x={x} y={y} dbl={dblclick}"))
    })
}
