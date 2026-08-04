// SPDX-License-Identifier: MIT OR Apache-2.0
//! Residual child process kill (Unix SIGTERM→SIGKILL; Windows job).
//!
//! # Why the unit of kill is a group, not a pid
//!
//! Chrome is a process tree: the launcher forks renderer, GPU, zygote and
//! utility children. Signalling only the launcher pid leaves every one of those
//! children alive and reparented to init, which is exactly the residue the
//! doctor kept reporting. The self-spawn path therefore puts Chrome in its own
//! process group (see [`crate::native::cdp::spawn::os`]) so FINALIZE can signal
//! the whole tree at once.
//!
//! When no group is available — a host without process groups, or the legacy
//! `chromiumoxide::Browser::launch` fallback — the escalation degrades to a
//! pid-tree walk over the `sysinfo` snapshot instead of silently reverting to a
//! single-pid kill.

use std::time::{Duration, Instant};

use super::FINALIZE_CHILD_GRACE;

pub(crate) fn residual_kill_child(pid: u32, pgid: Option<i32>, windows_job_handle: usize) {
    #[cfg(unix)]
    {
        let _ = windows_job_handle;
        kill_unix_tree(pid, pgid, FINALIZE_CHILD_GRACE);
    }
    #[cfg(windows)]
    {
        let _ = pgid;
        if windows_job_handle != 0 {
            crate::win_job::terminate_job(windows_job_handle);
            crate::win_job::close_job(windows_job_handle);
        } else {
            crate::win_job::terminate_pid(pid);
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (pid, pgid, windows_job_handle);
    }
}

/// Kill the whole browser tree: process group when known, else pid descendants.
///
/// `pgid` comes from the spawn guard. `Some` is the good path and costs one
/// syscall; `None` forces the `sysinfo` walk, which is correct but pays a full
/// process-table snapshot.
#[cfg(unix)]
pub fn kill_unix_tree(pid: u32, pgid: Option<i32>, grace: Duration) {
    match pgid {
        Some(pgid) if pgid > 1 => kill_unix_group_graceful(pgid, grace),
        _ => {
            // Deepest-first: a parent is never signalled before its children, so
            // an intermediate process cannot fork a replacement mid-walk.
            for target in descendant_pids(pid) {
                kill_unix_graceful(target, Duration::ZERO);
            }
            kill_unix_graceful(pid, grace);
        }
    }
}

/// Every live descendant of `root`, deepest first, excluding `root` itself.
///
/// Uses the shared `sysinfo` index rather than a second process-table
/// implementation. Returns empty when the host process table cannot be read:
/// signalling pids that cannot be attributed risks killing a process this
/// invocation never spawned.
#[cfg(unix)]
#[must_use]
pub fn descendant_pids(root: u32) -> Vec<u32> {
    let Some(index) = crate::residual::index_live_processes() else {
        tracing::warn!(
            root,
            "residual tree kill skipped: live process table unavailable"
        );
        return Vec::new();
    };
    descendants_in_index(root, &index)
}

/// [`descendant_pids`] against an explicit index (unit tests, alternate probes).
#[cfg(unix)]
#[must_use]
pub fn descendants_in_index(root: u32, index: &crate::residual::LiveProcessIndex) -> Vec<u32> {
    let mut out = Vec::new();
    let mut frontier = vec![root];
    // `seen` bounds the walk by the number of live processes: every pid is
    // enqueued at most once, so a cycle in a corrupted ppid table cannot loop.
    let mut seen: std::collections::HashSet<u32> = std::collections::HashSet::from([root]);
    while let Some(parent) = frontier.pop() {
        for child in index.children_of(parent) {
            if seen.insert(child) {
                out.push(child);
                frontier.push(child);
            }
        }
    }
    // Breadth order is shallow-first; reversing yields deepest-first.
    out.reverse();
    out
}

/// Send SIGTERM to a whole process group, wait out `grace`, then SIGKILL it.
///
/// A negative pid argument to `kill` addresses the process group, which is the
/// only way to reach the renderer and GPU children in one call.
#[cfg(unix)]
pub fn kill_unix_group_graceful(pgid: i32, grace: Duration) {
    if pgid <= 1 {
        // Group 0 means "the caller's own group" and 1 is init. Either would
        // turn a residual reap into self-destruction, so refuse both.
        return;
    }
    // SAFETY: `getpgrp` has no preconditions and returns this process's group.
    // See: `man 2 getpgrp`.
    let own = unsafe { libc::getpgrp() };
    if pgid == own {
        // Defence in depth: the caller is expected to have filtered this out, but
        // signalling our own group kills this CLI mid-FINALIZE.
        tracing::warn!(pgid, "refusing group kill: target group is our own");
        return;
    }
    // SAFETY:
    // - Contract: FINALIZE SIGTERM of the browser group this process created.
    // - Invariant: `pgid` was read from a child we spawned into its own group,
    //   so the negation addresses that group and never ours (guarded above).
    // - Failure is ignored (best-effort residual reap); ESRCH means it is gone.
    // - See: `man 2 kill` (a negative pid addresses a process group).
    unsafe {
        let _ = libc::kill(-pgid, libc::SIGTERM);
    }

    if wait_until_group_gone(pgid, grace) {
        return;
    }

    // SAFETY: same group ownership as the SIGTERM above; escalate to SIGKILL.
    unsafe {
        let _ = libc::kill(-pgid, libc::SIGKILL);
    }
}

/// Poll until the group has no members left, or `grace` expires.
#[cfg(unix)]
fn wait_until_group_gone(pgid: i32, grace: Duration) -> bool {
    let deadline = Instant::now() + grace;
    while Instant::now() < deadline {
        // SAFETY: signal 0 probes deliverability without delivering anything.
        // See: `man 2 kill`; ESRCH means no process remains in the group.
        let alive = unsafe { libc::kill(-pgid, 0) == 0 };
        if !alive {
            return true;
        }
        std::thread::sleep(Duration::from_millis(
            crate::constants::PLATFORM_CHILD_POLL_MS,
        ));
    }
    false
}

/// Send SIGTERM, wait up to `grace` while the pid exists, then SIGKILL if needed.
///
/// The single-pid primitive: [`kill_unix_tree`] builds the fallback path on it,
/// and it stays public because the previous FINALIZE contract exposed it.
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
