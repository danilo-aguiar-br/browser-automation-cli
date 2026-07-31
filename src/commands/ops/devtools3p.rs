// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::browser::CaptureOpts;
use crate::cli::Devtools3pAction;
use crate::commands::common::emit_ok;
use crate::commands::nav::with_session_blank;
use crate::error::CliError;
use crate::lifecycle::Lifecycle;

pub(crate) fn handle_devtools3p(
    life: &Lifecycle,
    action: Devtools3pAction,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    match action {
        Devtools3pAction::List { url } => {
            let url = url.unwrap_or_else(|| "about:blank".into());
            let data =
                with_session_blank(life, capture, timeout_secs, move |mut session| async move {
                    if url != "about:blank" {
                        let _ = session
                            .goto(&url, crate::robots::RobotsPolicy::Ignore)
                            .await?;
                    }
                    let v = session.devtools3p_list().await?;
                    Ok((session, v))
                })?;
            emit_ok(data, json, |d| {
                crate::output::writeln_stdout(format!("ok devtools3p list {d}"))
            })
        }
        Devtools3pAction::Exec { name, params, url } => {
            // `name`/`params` already owned by the match binding — move, do not clone.
            let url = url.unwrap_or_else(|| "about:blank".into());
            let data =
                with_session_blank(life, capture, timeout_secs, move |mut session| async move {
                    if url != "about:blank" {
                        let _ = session
                            .goto(&url, crate::robots::RobotsPolicy::Ignore)
                            .await?;
                    }
                    let v = session.devtools3p_exec(&name, params.as_deref()).await?;
                    Ok((session, v))
                })?;
            emit_ok(data, json, |d| {
                crate::output::writeln_stdout(format!("ok devtools3p exec {d}"))
            })
        }
    }
}
