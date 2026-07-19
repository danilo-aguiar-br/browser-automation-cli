// SPDX-License-Identifier: MIT OR Apache-2.0
//! One-shot lifecycle: INIT → EXECUTE → FINALIZE → EXIT.
//!
//! # Workload
//!
//! **Coordination (not compute fan-out):** one `CancellationToken` per process
//! invocation. Residual scavenge may use `map_cpu` when candidate paths are
//! large (see [`crate::residual`]). FINALIZE is ordered: Browser.close → residual
//! kill → flush. No multi-writer shared state.
//!
//! # Graceful shutdown model (rules_rust_encerramento_graceful_shutdown)
//!
//! This binary is a **one-shot CLI**, not a daemon. Shutdown is therefore
//! **minimal + critical for pipelines**, not a full multi-subsystem coordinator:
//!
//! 1. **Detect** — OS signals (`SIGINT`/`SIGTERM`, Windows Ctrl-C/Break) via
//!    [`crate::browser::shutdown_signal`] wired inside
//!    [`crate::browser::block_on_browser_timeout`].
//! 2. **Signal** — cooperative [`CancellationToken`] cancel (exit **130**).
//! 3. **Await / finalize** — primary `OneShotSession::shutdown` (Browser.close +
//!    wait ≤5s + kill); residual ledger kill with **SIGTERM → grace → SIGKILL**;
//!    dual stream flush; process `ExitCode`.
//!
//! ## Deadlines
//!
//! | Phase | Bound |
//! |-------|--------|
//! | Work / navigation | CLI `--timeout` / XDG / step-timeout |
//! | Browser.close wait | 5s (`finalize_browser`) |
//! | Residual child grace (SIGTERM→SIGKILL) | [`FINALIZE_CHILD_GRACE`] |
//! | Second OS signal | Forces residual [`Lifecycle::finalize`] immediately |
//!
//! ## Not applicable (product law)
//!
//! Daemon patterns: `TaskTracker` fleets, readiness probes, SIGHUP hot-reload,
//! `sd_notify`, PID files, OpenTelemetry flush, TUI terminal restore.
//!
//! # Safety
//!
//! On Unix, finalize may send `SIGTERM` then (after grace) `SIGKILL` to a
//! recorded Chrome PID as last resort when primary Browser.close reap did not
//! clear the ledger. Profile dirs and Chrome Singleton side-channels are wiped
//! **only** when recorded in this ledger (never a host-wide chrome wipe).

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio_util::sync::CancellationToken;

/// Max wait after residual `SIGTERM` before escalating to `SIGKILL` (Unix).
///
/// Kept short for one-shot CLIs (agents expect fast DIE) while still giving
/// Chrome a chance to flush profile locks before hard kill.
pub const FINALIZE_CHILD_GRACE: Duration = Duration::from_secs(2);

// Thread-local cancel + lifecycle for the active invocation (main-thread one-shot).
// Used by block_on_browser_timeout so SIGINT/SIGTERM abort work without
// threading Lifecycle through every call site.
//
// `const { RefCell::new(None) }` avoids runtime init on first access (MSRV ≥ 1.59).
// `RefCell` is correct here: TLS is not `Sync` across threads; each thread owns its cell.
thread_local! {
    static CURRENT_CANCEL: RefCell<Option<CancellationToken>> = const { RefCell::new(None) };
    static CURRENT_LIFE: RefCell<Option<Lifecycle>> = const { RefCell::new(None) };
}

/// Return the cancel token for the active invocation, or a fresh inert token.
///
/// Uses `try_borrow` so a re-entrant path that already holds the TLS `RefCell`
/// cannot panic (returns a fresh inert token instead).
pub fn current_cancel() -> CancellationToken {
    CURRENT_CANCEL.with(|c| {
        c.try_borrow()
            .ok()
            .and_then(|slot| slot.clone())
            .unwrap_or_else(CancellationToken::new)
    })
}

/// Return a clone of the active [`Lifecycle`], if one was registered.
///
/// Used by the second-signal force path to run residual finalize without
/// threading `Lifecycle` through every browser call site.
///
/// Uses `try_borrow` so re-entrancy cannot panic on the TLS `RefCell`.
pub fn current_lifecycle() -> Option<Lifecycle> {
    CURRENT_LIFE.with(|c| c.try_borrow().ok().and_then(|slot| slot.clone()))
}

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
        CURRENT_CANCEL.with(|c| {
            if let Ok(mut slot) = c.try_borrow_mut() {
                *slot = Some(cancel.clone());
            }
        });
        let lc = Self {
            cancel,
            finalize_done: Arc::new(AtomicBool::new(false)),
            ledger: Arc::new(std::sync::Mutex::new(ResourceLedger::default())),
        };
        CURRENT_LIFE.with(|c| {
            if let Ok(mut slot) = c.try_borrow_mut() {
                *slot = Some(lc.clone());
            }
        });
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
        lc
    }

    /// Mutate the resource ledger under the mutex.
    ///
    /// # Poison policy
    ///
    /// Recovers via `PoisonError::into_inner` so a prior panic cannot prevent
    /// residual kill/wipe (rules: treat `PoisonError`, never silent-skip FINALIZE).
    pub fn with_ledger_mut<R>(&self, f: impl FnOnce(&mut ResourceLedger) -> R) -> R {
        let mut guard = self.ledger.lock().unwrap_or_else(|poisoned| {
            // Best-effort residual cleanup must not abort because of poison.
            tracing::debug!("lifecycle ledger mutex poisoned; recovering via into_inner");
            poisoned.into_inner()
        });
        f(&mut guard)
    }

    /// Record that this invocation launched Chrome (residual kill target).
    pub fn record_chrome(&self, pid: Option<u32>) {
        self.with_ledger_mut(|ledger| {
            ledger.chrome_launched = true;
            ledger.chrome_pid = pid;
        });
    }

    /// Clear residual Chrome ownership after primary close reaped the child.
    pub fn clear_chrome(&self) {
        self.with_ledger_mut(|ledger| {
            ledger.chrome_launched = false;
            ledger.chrome_pid = None;
        });
    }

    /// Clear Chrome + profile ledger after primary session shutdown.
    pub fn clear_chrome_and_profile(&self) {
        self.with_ledger_mut(|ledger| {
            ledger.chrome_launched = false;
            ledger.chrome_pid = None;
            ledger.profile_dir = None;
            ledger.side_channels.clear();
        });
    }

    /// Whether cooperative cancel was requested (signal or finalize).
    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    /// Idempotent FINALIZE: last-resort residual kill if chrome still flagged.
    ///
    /// Primary reap is `OneShotSession::shutdown` (Browser.close + wait_or_kill).
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
        self.with_ledger_mut(|ledger| {
            // RES-01: copy pid BEFORE take so invocation-window scavenge can
            // still attribute Chromium side-channels under /tmp.
            let pid_for_scavenge = ledger.chrome_pid;
            if ledger.chrome_launched {
                if let Some(pid) = ledger.chrome_pid.take() {
                    residual_kill_child(pid, ledger.windows_job_handle);
                    #[cfg(windows)]
                    {
                        ledger.windows_job_handle = 0;
                    }
                }
            }
            ledger.chrome_launched = false;
            let profile = ledger.profile_dir.take();
            // RES-05: re-scan side-channels while profile/pid are still known,
            // then wipe ledger paths + discovery hits.
            let not_before = ledger.started_at.unwrap_or_else(std::time::SystemTime::now);
            let extras = crate::residual::discover_owned_chromium_tmp_side_channels(
                profile.as_deref(),
                pid_for_scavenge,
                not_before,
            );
            for p in extras {
                if !ledger.side_channels.iter().any(|e| e == &p) {
                    ledger.side_channels.push(p);
                }
            }
            if let Some(ref dir) = profile {
                wipe_owned_path(dir);
            }
            let sides = std::mem::take(&mut ledger.side_channels);
            for p in sides {
                wipe_owned_path(&p);
            }
            // GAP-A002: scavenge owned Chromium tmp orphans for this invocation window.
            let _ = crate::residual::scavenge_owned_chromium_tmp_orphans(
                profile.as_deref(),
                pid_for_scavenge,
                not_before,
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
            #[cfg(windows)]
            if ledger.windows_job_handle != 0 {
                crate::win_job::close_job(ledger.windows_job_handle);
                ledger.windows_job_handle = 0;
            }
        });
    }
}

/// Last-resort kill of a child process we launched (Chrome).
///
/// Unix: `SIGTERM`, poll until dead or [`FINALIZE_CHILD_GRACE`], then `SIGKILL`.
/// Windows: Job Object terminate + close when handle is set, else pid terminate.
fn residual_kill_child(pid: u32, windows_job_handle: usize) {
    #[cfg(unix)]
    {
        let _ = windows_job_handle;
        kill_unix_graceful(pid, FINALIZE_CHILD_GRACE);
    }
    #[cfg(windows)]
    {
        if windows_job_handle != 0 {
            crate::win_job::terminate_job(windows_job_handle);
            crate::win_job::close_job(windows_job_handle);
        } else {
            crate::win_job::terminate_pid(pid);
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (pid, windows_job_handle);
    }
}

/// Send SIGTERM, wait up to `grace` while the pid exists, then SIGKILL if needed.
///
/// Extracted for unit tests (does not require a live child when grace is zero
/// and pid is invalid — kill returns ESRCH).
#[cfg(unix)]
pub fn kill_unix_graceful(pid: u32, grace: Duration) {
    // SAFETY:
    // - Contract: last-resort FINALIZE SIGTERM of Chrome launched by this process.
    // - Invariant: `pid` was recorded when we spawned Chrome; cast fits on Unix.
    // - Caller guarantees ownership of the child tree; failure is ignored (best-effort).
    // - See: `man 2 kill`; product prefers Browser.close before this fallback.
    unsafe {
        let _ = libc::kill(pid as i32, libc::SIGTERM);
    }

    let deadline = Instant::now() + grace;
    while Instant::now() < deadline {
        // kill(pid, 0) probes existence without delivering a signal.
        // SAFETY: same ownership as SIGTERM; ESRCH means process is gone.
        let alive = unsafe { libc::kill(pid as i32, 0) == 0 };
        if !alive {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    // SAFETY: same pid ownership as SIGTERM; escalate to SIGKILL if still alive.
    unsafe {
        let _ = libc::kill(pid as i32, libc::SIGKILL);
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
        life.with_ledger_mut(|ledger| {
            ledger.profile_dir = Some(dir);
        });
    }
}

/// Record a side-channel path (SingletonLock, etc.) owned by this launch.
pub fn mark_side_channel(life: &Lifecycle, path: PathBuf) {
    life.with_ledger_mut(|ledger| {
        if !ledger.side_channels.iter().any(|p| p == &path) {
            ledger.side_channels.push(path);
        }
    });
}

impl Default for Lifecycle {
    fn default() -> Self {
        Self::new()
    }
}

/// Safety net: if the call site forgets explicit [`Lifecycle::finalize`], Drop still
/// runs residual cleanup (sync only: kill/wipe; no async I/O).
///
/// # Drop contract
///
/// - **Idempotent** via `finalize_done` (`SeqCst` CAS).
/// - **Bounded**: residual Unix path may sleep up to [`FINALIZE_CHILD_GRACE`] (not
///   indefinite; rules forbid unbounded block in Drop).
/// - **No panic**: kill/wipe errors are ignored (best-effort).
/// - Clones share the same Arcs; the first Drop that wins the CAS performs work.
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

    #[test]
    fn with_ledger_mut_recovers_from_poison() {
        let lc = Lifecycle::new();
        // Poison the mutex deliberately (simulates a prior panic while holding it).
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = lc.ledger.lock().unwrap();
            panic!("poison ledger for test");
        }));
        assert!(lc.ledger.is_poisoned());
        // Recovery must still apply residual ownership updates.
        lc.record_chrome(Some(42));
        lc.with_ledger_mut(|ledger| {
            assert!(ledger.chrome_launched);
            assert_eq!(ledger.chrome_pid, Some(42));
        });
        lc.clear_chrome();
        lc.with_ledger_mut(|ledger| {
            assert!(!ledger.chrome_launched);
            assert!(ledger.chrome_pid.is_none());
        });
    }

    #[test]
    fn current_cancel_tracks_active_lifecycle() {
        let lc = Lifecycle::new();
        assert!(!current_cancel().is_cancelled());
        lc.cancel.cancel();
        assert!(current_cancel().is_cancelled());
        assert!(lc.is_cancelled());
    }

    #[test]
    fn current_lifecycle_is_registered() {
        let lc = Lifecycle::new();
        let cur = current_lifecycle().expect("registered");
        assert!(!cur.finalize_done.load(Ordering::SeqCst));
        cur.cancel.cancel();
        assert!(lc.is_cancelled());
    }

    #[test]
    fn finalize_child_grace_is_bounded() {
        // One-shot residual must stay short (agents expect fast DIE).
        assert!(FINALIZE_CHILD_GRACE <= Duration::from_secs(5));
        assert!(FINALIZE_CHILD_GRACE >= Duration::from_millis(100));
    }

    #[cfg(unix)]
    #[test]
    fn kill_unix_graceful_on_missing_pid_returns_quickly() {
        // PID 1 is init and not ours; we only assert the helper does not hang
        // when grace is tiny. Using an extremely unlikely high pid that is dead.
        let start = Instant::now();
        // First free pid-ish: use a large number that should not exist.
        kill_unix_graceful(4_294_967_294, Duration::from_millis(0));
        assert!(start.elapsed() < Duration::from_secs(1));
    }
}
