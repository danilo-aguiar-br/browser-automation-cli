//! One-shot lifecycle: INIT → … → FINALIZE → EXIT (PRD Camada M).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

/// Ledger of resources owned by this invocation (browser, profile, temp dirs).
#[derive(Debug, Default)]
pub struct ResourceLedger {
    pub chrome_launched: bool,
    pub chrome_pid: Option<u32>,
    pub profile_dir: Option<std::path::PathBuf>,
}

/// Runtime token for cooperative cancel and FINALIZE.
#[derive(Clone)]
pub struct Lifecycle {
    pub cancel: CancellationToken,
    pub finalize_done: Arc<AtomicBool>,
    pub ledger: Arc<std::sync::Mutex<ResourceLedger>>,
}

impl Lifecycle {
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
