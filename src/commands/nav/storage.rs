// SPDX-License-Identifier: MIT OR Apache-2.0
//! `storage export|import` handlers (GAP-034 pillar 1).
//!
//! Both actions accept an optional `--url` so the whole round trip fits in one
//! invocation: navigate, then export; or import, then navigate so the restored
//! cookies apply to the first real request.

use crate::browser::CaptureOpts;
use crate::cli::StorageAction;
use crate::commands::common::emit_ok;
use crate::error::CliError;
use crate::lifecycle::Lifecycle;
use crate::robots::RobotsPolicy;

use super::with_session_blank;

pub(crate) fn handle_storage(
    life: &Lifecycle,
    action: StorageAction,
    robots: RobotsPolicy,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let data = match action {
        StorageAction::Export { path, url } => {
            with_session_blank(life, capture, timeout_secs, move |mut session| async move {
                if let Some(url) = url.as_deref() {
                    session.goto(url, robots).await?;
                }
                let value = session.storage_export(&path).await?;
                Ok((session, value))
            })?
        }
        StorageAction::Import { path, url } => {
            with_session_blank(life, capture, timeout_secs, move |mut session| async move {
                let mut value = session.storage_import(&path).await?;
                // Navigate after the import so the restored cookies are sent on
                // the first request rather than on a later one.
                if let Some(url) = url.as_deref() {
                    session.goto(url, robots).await?;
                    // Report what actually landed, so the caller does not have
                    // to trust that a successful write means a successful load.
                    let live = session.storage_live_counts().await?;
                    if let Some(obj) = value.as_object_mut() {
                        obj.insert("restored".into(), live);
                    }
                }
                Ok((session, value))
            })?
        }
    };
    emit_ok(data, json, |d| {
        let cookies = d.get("cookies").and_then(|v| v.as_u64()).unwrap_or(0);
        let origins = d.get("origins").and_then(|v| v.as_u64()).unwrap_or(0);
        let path = d.get("path").and_then(|v| v.as_str()).unwrap_or("");
        crate::output::writeln_stdout(format!(
            "ok storage cookies={cookies} origins={origins} path={path}"
        ))
    })
}
