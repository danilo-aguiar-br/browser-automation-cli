// SPDX-License-Identifier: MIT OR Apache-2.0
//! One-shot JavaScript evaluation artifact handler.

use std::path::Path;

use crate::browser::{block_on_browser_timeout, CaptureOpts, OneShotSession};
use crate::commands::common::emit_ok;
use crate::error::CliError;
use crate::lifecycle::Lifecycle;

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_eval(
    life: &Lifecycle,
    expression: &str,
    args: Option<&str>,
    dialog_action: Option<&str>,
    file_path: Option<&Path>,
    service_worker_id: Option<&str>,
    typed: bool,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let expr = expression.to_string();
    let args_owned = args.map(|s| s.to_string());
    let dialog_owned = dialog_action.map(|s| s.to_string());
    let path_owned = file_path.map(|p| p.to_path_buf());
    let sw_owned = service_worker_id.map(|s| s.to_string());
    let data = block_on_browser_timeout(
        async move {
            let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
            life.with_ledger_mut(|ledger| {
                ledger.chrome_launched = true;
                ledger.chrome_pid = session.chrome_pid();
                if let Some(dir) = session.temp_user_data_dir() {
                    ledger.profile_dir = Some(dir);
                }
            });
            let r = if let Some(ref sw) = sw_owned {
                session.eval_service_worker(sw, &expr).await
            } else {
                let _ = session
                    .goto("about:blank", crate::robots::RobotsPolicy::Honor)
                    .await?;
                session
                    .eval_ex(
                        &expr,
                        args_owned.as_deref(),
                        dialog_owned.as_deref(),
                        path_owned.as_deref(),
                        typed,
                    )
                    .await
            };
            let close = session.shutdown().await;
            life.clear_chrome();
            close?;
            r
        },
        timeout_secs,
    )?;
    emit_ok(data, json, |d| {
        // Typed mode moves the payload to `value`; keep both readable in text mode.
        let shown = d
            .get("result")
            .or_else(|| d.get("value"))
            .unwrap_or(&serde_json::Value::Null);
        crate::output::writeln_stdout(format!("ok eval={shown}"))?;
        Ok(())
    })
}
