// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::browser::CaptureOpts;
use crate::cli::PerfAction;
use crate::commands::common::emit_ok;
use crate::commands::nav::with_session_blank;
use crate::error::CliError;
use crate::lifecycle::Lifecycle;

pub(crate) fn handle_perf(
    life: &Lifecycle,
    action: PerfAction,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        let v = match action {
            PerfAction::Start {
                path,
                reload,
                auto_stop,
            } => {
                session
                    .perf_start(path.as_deref(), reload, auto_stop)
                    .await?
            }
            PerfAction::Stop { path } => session.perf_stop(path.as_deref()).await?,
            PerfAction::Insight {
                name,
                insight_set_id,
                insight_name,
            } => {
                let resolved = insight_name.or(name);
                session
                    .perf_insight(resolved.as_deref(), insight_set_id.as_deref())
                    .await?
            }
        };
        Ok((session, v))
    })?;
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!("ok perf {d}"))
    })
}
