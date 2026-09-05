// SPDX-License-Identifier: MIT OR Apache-2.0
//! PDF print artifact handler.

use std::path::Path;

use crate::browser::{block_on_browser_timeout, CaptureOpts};
use crate::commands::common::emit_ok;
use crate::error::{CliError, ErrorKind};
use crate::etd::{with_target, TargetSource};
use crate::lifecycle::Lifecycle;
use crate::robots::RobotsPolicy;

pub(crate) fn handle_print_pdf(
    life: &Lifecycle,
    path: Option<&Path>,
    url: Option<&str>,
    robots: RobotsPolicy,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    // GAP-013: refuse blank PDF without an explicit URL (agent-first).
    if url.map(str::trim).filter(|u| !u.is_empty()).is_none() {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            "print-pdf requires --url (blank about:blank PDF refused)",
            crate::i18n::suggestion_key("navigate_first", None),
        ));
    }
    // Without `--path` the PDF lands wherever the session decides, which argv
    // never named.
    let etd = path.map_or_else(
        || ("(session default)".to_string(), TargetSource::Ambient),
        |p| (p.display().to_string(), TargetSource::Argv),
    );
    let path = path.map(|p| p.to_path_buf());
    let url = url.map(|s| s.to_string());
    let data = block_on_browser_timeout(
        async {
            let mut session =
                crate::browser::OneShotSession::launch_headless_with_capture(capture).await?;
            life.record_chrome(session.chrome_pid());
            if let Some(u) = url.as_deref() {
                let _ = session.goto(u, robots).await?;
            }
            let out = session.print_pdf(path.as_deref(), false).await;
            let _ = session.shutdown().await;
            life.clear_chrome();
            out
        },
        timeout_secs,
    )?;
    let data = with_target(data, &etd.0, etd.1);
    emit_ok(data, json, |d| {
        let p = d.get("path").and_then(|v| v.as_str()).unwrap_or("");
        crate::output::writeln_stdout(format!("ok print-pdf path={p}"))?;
        Ok(())
    })
}
