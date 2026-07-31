// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::browser::CaptureOpts;
use crate::cli::WebmcpAction;
use crate::commands::common::emit_ok;
use crate::commands::nav::with_session_blank;
use crate::error::CliError;
use crate::lifecycle::Lifecycle;

pub(crate) fn handle_webmcp(
    life: &Lifecycle,
    action: WebmcpAction,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    match action {
        WebmcpAction::List { url } => {
            let url = url.unwrap_or_else(|| "about:blank".into());
            let data =
                with_session_blank(life, capture, timeout_secs, move |mut session| async move {
                    if url != "about:blank" {
                        let _ = session
                            .goto(&url, crate::robots::RobotsPolicy::Ignore)
                            .await?;
                    }
                    let v = session.webmcp_list().await?;
                    Ok((session, v))
                })?;
            emit_ok(data, json, |d| {
                crate::output::writeln_stdout(format!("ok webmcp list {d}"))
            })
        }
        WebmcpAction::Exec { name, input, url } => {
            // Move owned match bindings into the async block (no redundant clone).
            let url = url.unwrap_or_else(|| "about:blank".into());
            let data =
                with_session_blank(life, capture, timeout_secs, move |mut session| async move {
                    if url != "about:blank" {
                        let _ = session
                            .goto(&url, crate::robots::RobotsPolicy::Ignore)
                            .await?;
                    }
                    let v = session.webmcp_exec(&name, input.as_deref()).await?;
                    Ok((session, v))
                })?;
            emit_ok(data, json, |d| {
                crate::output::writeln_stdout(format!("ok webmcp exec {d}"))
            })
        }
    }
}
