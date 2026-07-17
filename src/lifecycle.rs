//! One-shot lifecycle: INIT → EXECUTA → FINALIZE → EXIT.
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
    pub profile_dir: Option<std::path::PathBuf>,
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
        Self {
            cancel: CancellationToken::new(),
            finalize_done: Arc::new(AtomicBool::new(false)),
            ledger: Arc::new(std::sync::Mutex::new(ResourceLedger::default())),
        }
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
                        let _ = pid;
                    }
                }
            }
            ledger.chrome_launched = false;
            ledger.profile_dir = None;
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
