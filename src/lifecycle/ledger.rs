// SPDX-License-Identifier: MIT OR Apache-2.0
//! Resource ledger and Lifecycle construction / mutation helpers.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use super::tls::{register_cancel, register_lifecycle};

/// Owned residual resources for one one-shot invocation (Chrome pid, profile, side-channels).
#[derive(Debug, Default)]
pub struct ResourceLedger {
    /// True when this process launched Chrome and still owns the residual flag.
    pub chrome_launched: bool,
    /// Optional OS process id of launched Chrome for last-resort kill.
    pub chrome_pid: Option<u32>,
    /// POSIX process group of launched Chrome, when the host models one.
    ///
    /// `Some` lets FINALIZE signal the whole browser tree with `kill(-pgid, …)`.
    /// `None` (legacy chromiumoxide launch, or a host without process groups)
    /// forces the pid-tree fallback in [`super::kill::kill_unix_tree`].
    pub chrome_pgid: Option<i32>,
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
///
/// # Ownership
///
/// Clones share the same `Arc` ledger and cancel token. Dropping any clone that
/// wins the finalize CAS performs residual cleanup — treat as a resource owner
/// (`#[must_use]`).
#[derive(Clone)]
#[must_use = "Lifecycle owns residual Chrome/profile cleanup; call finalize or drop"]
pub struct Lifecycle {
    /// Cancellation token shared with async tasks.
    pub cancel: CancellationToken,
    /// Whether FINALIZE already completed.
    ///
    /// Updated with `Ordering::SeqCst` so a single total order is visible to
    /// Drop, signal handlers, and async tasks racing on idempotent finalize.
    pub finalize_done: Arc<AtomicBool>,
    /// Owned resources for residual cleanup.
    ///
    /// `std::sync::Mutex` (not `tokio::sync::Mutex`): critical sections are short
    /// and never held across `.await`. Poison is always recovered via
    /// [`Self::with_ledger_mut`] so residual FINALIZE cannot sticky-fail.
    pub ledger: Arc<std::sync::Mutex<ResourceLedger>>,
}

impl Lifecycle {
    /// Create a fresh lifecycle for one process invocation.
    pub fn new() -> Self {
        let cancel = CancellationToken::new();
        // try_borrow_mut: never panic if a re-entrant signal path already holds TLS.
        register_cancel(cancel.clone());
        let lc = Self {
            cancel,
            finalize_done: Arc::new(AtomicBool::new(false)),
            ledger: Arc::new(std::sync::Mutex::new(ResourceLedger::default())),
        };
        register_lifecycle(lc.clone());
        lc.with_ledger_mut(|ledger| {
            ledger.started_at = Some(std::time::SystemTime::now());
        });
        // RES-06: BORN cross-run GC — wipe stale Singleton-only /tmp orphans
        // left by prior one-shot invocations (disk hygiene, not process kill).
        // Best-effort; never panics; never touches host Flatpak Chrome profiles.
        let wiped = crate::residual::scavenge_stale_singleton_orphans();
        if !wiped.is_empty() {
            tracing::debug!(
                count = wiped.len(),
                "lifecycle BORN scavenge_stale_singleton_orphans"
            );
        }
        // Reap browser trees whose owning CLI already died. The scavenger above
        // cannot do this: it refuses any path with a live holder, and an orphaned
        // Chrome *is* a live holder of its own profile, so the two pin each other
        // forever. A live owner pid is never touched, so a concurrent invocation
        // is safe (see `crate::residual::reconcile`).
        let reconciled = crate::residual::reconcile_abandoned_profiles();
        if !reconciled.is_empty() {
            tracing::debug!(
                killed = reconciled.killed_pids.len(),
                wiped = reconciled.wiped_paths.len(),
                "lifecycle BORN reconcile_abandoned_profiles"
            );
        }
        lc
    }

    /// Mutate the resource ledger under the mutex.
    ///
    /// # Poison policy
    ///
    /// Recovers via `PoisonError::into_inner` so a prior panic cannot prevent
    /// residual kill/wipe (rules: treat `PoisonError`, never silent-skip FINALIZE).
    pub fn with_ledger_mut<R>(&self, f: impl FnOnce(&mut ResourceLedger) -> R) -> R {
        // Best-effort residual cleanup must not abort because of poison.
        let mut guard = crate::sync_util::lock_recover(&self.ledger);
        f(&mut guard)
    }

    /// Record that this invocation launched Chrome (residual kill target).
    pub fn record_chrome(&self, pid: Option<u32>) {
        self.record_chrome_group(pid, None);
    }

    /// Record the launched Chrome together with its process group.
    ///
    /// Prefer this over [`Self::record_chrome`]: without the group, FINALIZE can
    /// only signal the launcher pid and every renderer child survives.
    pub fn record_chrome_group(&self, pid: Option<u32>, pgid: Option<i32>) {
        self.with_ledger_mut(|ledger| {
            ledger.chrome_launched = true;
            ledger.chrome_pid = pid;
            ledger.chrome_pgid = pgid;
        });
    }

    /// Clear residual Chrome ownership after primary close reaped the child.
    pub fn clear_chrome(&self) {
        self.with_ledger_mut(|ledger| {
            ledger.chrome_launched = false;
            ledger.chrome_pid = None;
            ledger.chrome_pgid = None;
        });
    }

    /// Clear Chrome + profile ledger after primary session shutdown.
    pub fn clear_chrome_and_profile(&self) {
        self.with_ledger_mut(|ledger| {
            ledger.chrome_launched = false;
            ledger.chrome_pid = None;
            ledger.chrome_pgid = None;
            ledger.profile_dir = None;
            ledger.side_channels.clear();
        });
    }

    /// Whether cooperative cancel was requested (signal or finalize).
    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }
}
