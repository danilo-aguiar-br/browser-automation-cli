// SPDX-License-Identifier: MIT OR Apache-2.0
//! Residual child process kill (Unix SIGTERM→SIGKILL; Windows job).

use std::time::{Duration, Instant};

use super::FINALIZE_CHILD_GRACE;

pub(crate) fn residual_kill_child(pid: u32, windows_job_handle: usize) {
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
        // Poll slice shares the platform child-wait budget (DRY with PLATFORM_CHILD_POLL_MS).
        std::thread::sleep(Duration::from_millis(
            crate::constants::PLATFORM_CHILD_POLL_MS,
        ));
    }

    // SAFETY: same pid ownership as SIGTERM; escalate to SIGKILL if still alive.
    unsafe {
        let _ = libc::kill(pid as i32, libc::SIGKILL);
    }
}

/// Remove a file or directory tree owned by this process only.
pub(crate) fn wipe_owned_path(path: &std::path::Path) {
    if !path.exists() {
        return;
    }
    if path.is_dir() {
        let _ = std::fs::remove_dir_all(path);
    } else {
        let _ = std::fs::remove_file(path);
    }
}
