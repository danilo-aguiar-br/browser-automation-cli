//! One-shot lifecycle: INIT → EXECUTE → FINALIZE → EXIT.
//!
//! # Flow
//!
//! 1. `Lifecycle::new` creates cancel token and resource ledger
//! 2. Commands may mark Chrome PID / profile in `ResourceLedger`
//! 3. `Lifecycle::finalize` is idempotent and safe to call multiple times
//! 4. `Drop` also calls finalize as a safety net
//!
//! # Safety
//!
//! On Unix, finalize may send `SIGTERM`/`SIGKILL` to a recorded Chrome PID as
//! last resort when primary Browser.close reap did not clear the ledger.
//! Profile dirs and Chrome Singleton side-channels are wiped **only** when
//! recorded in this ledger (never a host-wide chrome wipe).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

/// Ledger of resources owned by this invocation (browser, profile, temp dirs).
#[derive(Debug, Default)]
pub struct ResourceLedger {
    /// True when this process launched Chrome and still owns the residual flag.
    pub chrome_launched: bool,
    /// Optional OS process id of launched Chrome for last-resort kill.
    pub chrome_pid: Option<u32>,
    /// Temporary profile directory created for this invocation, if any.
    pub profile_dir: Option<PathBuf>,
    /// Side-channel paths owned by this launch (e.g. SingletonLock under /tmp).
    pub side_channels: Vec<PathBuf>,
    /// Windows Job Object handle as usize (cfg windows); zero when unused.
    pub windows_job_handle: usize,
    /// Wall-clock start of this invocation (for FINALIZE scavenger window).
    pub started_at: Option<std::time::SystemTime>,
}

/// Runtime token for cooperative cancel and FINALIZE.
#[derive(Clone)]
pub struct Lifecycle {
    /// Cancellation token shared with async tasks.
    pub cancel: CancellationToken,
    /// Whether FINALIZE already completed.
    pub finalize_done: Arc<AtomicBool>,
    /// Owned resources for residual cleanup.
    pub ledger: Arc<std::sync::Mutex<ResourceLedger>>,
}

impl Lifecycle {
    /// Create a fresh lifecycle for one process invocation.
    pub fn new() -> Self {
        let lc = Self {
            cancel: CancellationToken::new(),
            finalize_done: Arc::new(AtomicBool::new(false)),
            ledger: Arc::new(std::sync::Mutex::new(ResourceLedger::default())),
        };
        if let Ok(mut ledger) = lc.ledger.lock() {
            ledger.started_at = Some(std::time::SystemTime::now());
        }
        lc
    }

    /// Idempotent FINALIZE: last-resort residual kill if chrome still flagged.
    ///
    /// Primary reap is `OneShotSession::shutdown` (Browser.close + wait_or_kill).
    pub fn finalize(&self) {
        if self
            .finalize_done
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }
        self.cancel.cancel();
        if let Ok(mut ledger) = self.ledger.lock() {
            if ledger.chrome_launched {
                if let Some(pid) = ledger.chrome_pid.take() {
                    #[cfg(unix)]
                    unsafe {
                        let _ = libc::kill(pid as i32, libc::SIGTERM);
                        let _ = libc::kill(pid as i32, libc::SIGKILL);
                    }
                    #[cfg(windows)]
                    {
                        // Prefer Job Object kill when available (GAP-009).
                        if ledger.windows_job_handle != 0 {
                            crate::win_job::terminate_job(ledger.windows_job_handle);
                            ledger.windows_job_handle = 0;
                        } else {
                            crate::win_job::terminate_pid(pid);
                        }
                    }
                }
            }
            ledger.chrome_launched = false;
            let profile = ledger.profile_dir.take();
            if let Some(ref dir) = profile {
                wipe_owned_path(dir);
            }
            let sides = std::mem::take(&mut ledger.side_channels);
            for p in sides {
                wipe_owned_path(&p);
            }
            // GAP-A002: scavenge owned Chromium tmp orphans for this invocation window.
            let not_before = ledger.started_at.unwrap_or_else(std::time::SystemTime::now);
            let chrome_pid = ledger.chrome_pid; // already taken above; None here is fine
            let _ = crate::residual::scavenge_owned_chromium_tmp_orphans(
                profile.as_deref(),
                chrome_pid,
                not_before,
            );
            #[cfg(windows)]
            if ledger.windows_job_handle != 0 {
                crate::win_job::close_job(ledger.windows_job_handle);
                ledger.windows_job_handle = 0;
            }
        }
    }
}

/// Remove a file or directory tree owned by this process only.
fn wipe_owned_path(path: &std::path::Path) {
    if !path.exists() {
        return;
    }
    if path.is_dir() {
        let _ = std::fs::remove_dir_all(path);
    } else {
        let _ = std::fs::remove_file(path);
    }
}

/// Record a temporary Chrome profile directory on the ledger for FINALIZE wipe.
pub fn mark_profile_dir(life: &Lifecycle, dir: Option<PathBuf>) {
    if let Some(dir) = dir {
        if let Ok(mut ledger) = life.ledger.lock() {
            ledger.profile_dir = Some(dir);
        }
    }
}

/// Record a side-channel path (SingletonLock, etc.) owned by this launch.
pub fn mark_side_channel(life: &Lifecycle, path: PathBuf) {
    if let Ok(mut ledger) = life.ledger.lock() {
        if !ledger.side_channels.iter().any(|p| p == &path) {
            ledger.side_channels.push(path);
        }
    }
}

impl Default for Lifecycle {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Lifecycle {
    fn drop(&mut self) {
        self.finalize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finalize_is_idempotent() {
        let lc = Lifecycle::new();
        lc.finalize();
        lc.finalize();
        assert!(lc.finalize_done.load(Ordering::SeqCst));
    }
}
