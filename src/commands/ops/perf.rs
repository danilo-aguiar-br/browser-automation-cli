// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::browser::{CaptureOpts, OneShotSession};
use crate::cli::PerfAction;
use crate::commands::common::emit_ok_summary;
use crate::commands::nav::with_session_blank;
use crate::error::{CliError, ErrorKind};
use crate::lifecycle::Lifecycle;

pub(crate) fn handle_perf(
    life: &Lifecycle,
    action: PerfAction,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    // `perf insight --path` answers from a trace already on disk, so it
    // short-circuits BEFORE `with_session_blank`. Launching Chrome to read a
    // JSON file is a cost with no upside, and routing it through the session
    // would also make offline analysis fail on a host with no browser at all.
    //
    // This is what connects `OneShotSession::perf_insight_file` to argv. That
    // function shipped with a correct root check and zero callers in the whole
    // tree, which made the read axis look covered where nothing was executable.
    if let PerfAction::Insight {
        path: Some(trace),
        name,
        insight_name,
        insight_set_id,
    } = &action
    {
        // Refuse the contradictory pair instead of dropping half of it.
        //
        // `insight_set_id` selects among the sets a LIVE trace session
        // published in `perf stop`'s `available_insight_sets`. A trace read
        // from disk has no such session and `perf_insight_file` takes no set
        // argument, so the flag had nowhere to go and the `..` in this pattern
        // discarded it in silence: the caller asked for one set, got the
        // whole file analysed, and read `ok: true` over it.
        if insight_set_id.is_some() {
            return Err(CliError::new(
                ErrorKind::Usage,
                "perf insight --path analyses a trace file offline and has no insight sets; \
                 drop --insight-set-id, or drop --path to select a set from the live session",
            ));
        }
        let resolved = insight_name.clone().or_else(|| name.clone());
        let data = OneShotSession::perf_insight_file(trace, resolved.as_deref())?;
        return emit_ok_summary(data, json, "perf");
    }
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
                // `Some` was handled offline above; only `None` reaches the session.
                path: _,
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
    emit_ok_summary(data, json, "perf")
}
