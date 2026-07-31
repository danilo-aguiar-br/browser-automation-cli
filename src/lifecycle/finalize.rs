// SPDX-License-Identifier: MIT OR Apache-2.0
//! Idempotent FINALIZE path (residual kill + wipe + scavenge).

use std::path::PathBuf;
use std::sync::atomic::Ordering;

use super::kill::{residual_kill_child, wipe_owned_path};
use super::ledger::Lifecycle;
use super::tls::clear_tls_registration;

impl Lifecycle {
    /// Idempotent FINALIZE: last-resort residual kill if chrome still flagged.
    ///
    /// Primary reap is `OneShotSession::shutdown` (Browser.close + wait_or_kill).
    ///
    /// # Interior mutability (copy-out)
    ///
    /// The ledger mutex is held only long enough to **snapshot and clear**
    /// ownership fields. Residual kill (may sleep up to `FINALIZE_CHILD_GRACE`),
    /// filesystem wipe, and scavengers run **outside** the critical section
    /// (rules: short sections; never hold `Mutex` across external I/O or sleep).
    pub fn finalize(&self) {
        // SeqCst CAS: total order across Drop, second-signal path, and explicit finalize.
        if self
            .finalize_done
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }
        self.cancel.cancel();
        // Drop TLS registration for this invocation so second-signal / re-entry
        // cannot observe a finalized Lifecycle as "current" (RefCell hygiene).
        clear_tls_registration(self);

        // RES-01: copy pid under lock so invocation-window scavenge can still
        // attribute Chromium side-channels under /tmp after take().
        let snap = self.with_ledger_mut(|ledger| {
            let pid_for_scavenge = ledger.chrome_pid;
            let kill_pid = if ledger.chrome_launched {
                ledger.chrome_pid.take()
            } else {
                None
            };
            let job_handle = ledger.windows_job_handle;
            // Ownership of the job moves out with the snapshot; zero ledger so a
            // concurrent with_ledger_mut cannot double-close.
            ledger.windows_job_handle = 0;
            ledger.chrome_launched = false;
            let profile = ledger.profile_dir.take();
            let not_before = ledger.started_at.unwrap_or_else(std::time::SystemTime::now);
            let side_channels = std::mem::take(&mut ledger.side_channels);
            FinalizeSnapshot {
                kill_pid,
                job_handle,
                pid_for_scavenge,
                profile,
                not_before,
                side_channels,
            }
        });

        // --- outside lock: kill / discover / wipe / scavenge ---
        if let Some(pid) = snap.kill_pid {
            residual_kill_child(pid, snap.job_handle);
        } else {
            // No pid kill path: still close a recorded Windows job handle.
            #[cfg(windows)]
            if snap.job_handle != 0 {
                crate::win_job::close_job(snap.job_handle);
            }
        }

        // RES-05: re-scan side-channels while profile/pid are still known,
        // then wipe ledger paths + discovery hits.
        let mut sides = snap.side_channels;
        let extras = crate::residual::discover_owned_chromium_tmp_side_channels(
            snap.profile.as_deref(),
            snap.pid_for_scavenge,
            snap.not_before,
        );
        for p in extras {
            if !sides.iter().any(|e| e == &p) {
                sides.push(p);
            }
        }
        if let Some(ref dir) = snap.profile {
            wipe_owned_path(dir);
        }
        for p in sides {
            wipe_owned_path(&p);
        }
        // GAP-A002: scavenge owned Chromium tmp orphans for this invocation window.
        let _ = crate::residual::scavenge_owned_chromium_tmp_orphans(
            snap.profile.as_deref(),
            snap.pid_for_scavenge,
            snap.not_before,
        );
        // RES-02/RES-06: second pass cross-run GC catches late Singleton
        // side-channels that never referenced profile/pid.
        let wiped = crate::residual::scavenge_stale_singleton_orphans();
        if !wiped.is_empty() {
            tracing::debug!(
                count = wiped.len(),
                "lifecycle FINALIZE scavenge_stale_singleton_orphans"
            );
        }
    }
}

/// Ownership snapshot extracted under the ledger mutex for residual work.
///
/// Lives only on the finalize stack; never shared across threads.
struct FinalizeSnapshot {
    kill_pid: Option<u32>,
    job_handle: usize,
    pid_for_scavenge: Option<u32>,
    profile: Option<PathBuf>,
    not_before: std::time::SystemTime,
    side_channels: Vec<PathBuf>,
}
