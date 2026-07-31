// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::browser::CaptureOpts;
use crate::cli::ScreencastAction;
use crate::commands::common::emit_ok;
use crate::commands::nav::with_session_blank;
use crate::error::CliError;
use crate::lifecycle::Lifecycle;

pub(crate) fn handle_screencast(
    life: &Lifecycle,
    action: ScreencastAction,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        let v = match action {
            ScreencastAction::Start { path } => session.screencast_start(path.as_deref()).await?,
            ScreencastAction::Stop { path } => session.screencast_stop(path.as_deref()).await?,
        };
        Ok((session, v))
    })?;
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!("ok screencast {d}"))
    })
}
