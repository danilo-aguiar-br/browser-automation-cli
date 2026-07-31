// SPDX-License-Identifier: MIT OR Apache-2.0
//! Launch/finalize marks for residual-zero ledger ownership.

use crate::error::CliError;
use crate::lifecycle::Lifecycle;
use serde_json::Value;

use super::session::{CaptureOpts, OneShotSession};

fn mark_launched(life: &Lifecycle, pid: Option<u32>, profile: Option<std::path::PathBuf>) {
    let launch_t = std::time::SystemTime::now();
    // Deliberate PathBuf clone: ledger takes ownership of `profile`, while residual
    // discovery after the settle sleep still needs the same path (rules: name clones).
    let profile_scan = profile.clone();
    // Poison-recovering ledger API (rules: never silent-skip residual ownership).
    life.with_ledger_mut(|ledger| {
        ledger.chrome_launched = true;
        ledger.chrome_pid = pid;
        // Move profile into the ledger — caller transfers ownership of the Option.
        if let Some(dir) = profile {
            // Track Singleton side-channel under the profile when present.
            for name in ["SingletonLock", "SingletonCookie", "SingletonSocket"] {
                let p = dir.join(name);
                if p.exists() {
                    ledger.side_channels.push(p);
                }
            }
            ledger.profile_dir = Some(dir);
        }
        #[cfg(windows)]
        if let Some(p) = pid {
            if ledger.windows_job_handle == 0 {
                ledger.windows_job_handle = crate::win_job::create_and_assign(p);
            }
        }
    });
    // GAP-020: discover owned /tmp/org.chromium.* created for this launch only.
    // Brief settle so Chrome can create Singleton side-channels.
    // Sync path (FINALIZE / post-launch ledger) — not a Tokio async worker
    // (rules: std::thread::sleep forbidden only inside async fn).
    std::thread::sleep(std::time::Duration::from_millis(
        crate::xdg::policy::policy_u64(crate::xdg::policy::key::DEFAULT_SUPPORT_SETTLE_MS),
    ));
    let extras = crate::residual::discover_owned_chromium_tmp_side_channels(
        profile_scan.as_deref(),
        pid,
        launch_t,
    );
    for p in extras {
        crate::lifecycle::mark_side_channel(life, p);
    }
    // Re-scan profile Singleton* after settle.
    if let Some(ref dir) = profile_scan {
        for name in ["SingletonLock", "SingletonCookie", "SingletonSocket"] {
            let p = dir.join(name);
            if p.exists() {
                crate::lifecycle::mark_side_channel(life, p);
            }
        }
    }
}

fn mark_closed(life: &Lifecycle) {
    // Primary close already wiped profile via BrowserManager::close; clear ledger.
    life.clear_chrome_and_profile();
}

pub(crate) async fn launch_marked(
    life: &Lifecycle,
    capture: CaptureOpts,
) -> Result<OneShotSession, CliError> {
    let session = OneShotSession::launch_headless_with_capture(capture).await?;
    mark_launched(life, session.chrome_pid(), session.temp_user_data_dir());
    Ok(session)
}

pub(crate) async fn finish(
    life: &Lifecycle,
    session: OneShotSession,
    work_res: Result<Value, CliError>,
) -> Result<Value, CliError> {
    // GAP-039: the session is still alive here; this is the only point where
    // captured console/network evidence can still be read on the failure path.
    let mut session = session;
    let work_res = match work_res {
        Err(e) => Err(attach_failure_dump(&mut session, e)),
        ok => ok,
    };
    let close_res = session.shutdown().await;
    mark_closed(life);
    match (work_res, close_res) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), _) => Err(e),
        (Ok(_), Err(e)) => Err(e),
    }
}

/// Write console/network evidence for a still-live session (GAP-039).
///
/// Returns the artifact path, or `None` when the dump is disabled, no capture
/// was active, or the write failed: diagnosing the original failure always wins.
pub(crate) fn dump_failure_evidence(
    session: &mut OneShotSession,
    err: &CliError,
) -> Option<std::path::PathBuf> {
    if !crate::failure_dump::enabled() {
        return None;
    }
    let console = session
        .console_list(None, None, None, true, None)
        .unwrap_or(Value::Null);
    let network = session
        .net_list(None, None, None, true)
        .unwrap_or(Value::Null);
    if console.is_null() && network.is_null() {
        // Neither capture flag was active: nothing to preserve.
        return None;
    }
    let payload = serde_json::json!({
        "error": { "kind": err.kind().as_str(), "message": err.message() },
        "console": console,
        "network": network,
    });
    crate::failure_dump::write_dump(&payload)
}

/// Record the dump artifact path on the error envelope.
fn attach_failure_dump(session: &mut OneShotSession, err: CliError) -> CliError {
    let Some(path) = dump_failure_evidence(session, &err) else {
        return err;
    };
    let mut data = err.data().cloned().unwrap_or_else(|| serde_json::json!({}));
    if let Some(obj) = data.as_object_mut() {
        obj.insert(
            "failure_dump_path".to_string(),
            serde_json::json!(path.display().to_string()),
        );
    }
    err.with_data(data)
}
