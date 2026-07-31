// SPDX-License-Identifier: MIT OR Apache-2.0
//! Screenshot (`grab`) artifact handler.

use std::path::Path;

use crate::browser::{block_on_browser_timeout, CaptureOpts, OneShotSession};
use crate::cli::GrabFormat;
use crate::commands::common::emit_ok;
use crate::error::{CliError, ErrorKind};
use crate::lifecycle::Lifecycle;

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_grab(
    life: &Lifecycle,
    path: Option<&Path>,
    format: GrabFormat,
    full_page: bool,
    quality: Option<i32>,
    element: Option<&str>,
    artifacts: Option<&Path>,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let fmt = match format {
        GrabFormat::Png => "png",
        GrabFormat::Jpeg => "jpeg",
        GrabFormat::Webp => "webp",
    };
    if let Some(a) = artifacts {
        std::fs::create_dir_all(a)
            .map_err(|e| CliError::new(ErrorKind::Io, format!("artifacts-dir mkdir: {e}")))?;
    }
    let path_owned = path.map(|p| p.to_path_buf()).or_else(|| {
        artifacts.map(|a| {
            a.join(format!(
                "grab-{}.{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0),
                fmt
            ))
        })
    });
    if let Some(ref p) = path_owned {
        if let Some(parent) = p.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| CliError::new(ErrorKind::Io, format!("grab path mkdir: {e}")))?;
            }
        }
    }
    let element_owned = element.map(|s| s.to_string());
    let data = block_on_browser_timeout(
        async move {
            let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
            life.record_chrome(session.chrome_pid());
            let _ = session
                .goto("about:blank", crate::robots::RobotsPolicy::Honor)
                .await?;
            let r = session
                .grab(
                    path_owned.as_deref(),
                    fmt,
                    full_page,
                    quality,
                    element_owned.as_deref(),
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
        let p = d.get("path").and_then(|v| v.as_str()).unwrap_or("");
        crate::output::writeln_stdout(format!("ok grab path={p}"))?;
        Ok(())
    })
}
