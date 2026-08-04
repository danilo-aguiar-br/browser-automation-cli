// SPDX-License-Identifier: MIT OR Apache-2.0
//! Owned Chrome child process (RAII: kill + wait on Drop).

use std::process::Child;
use std::time::Duration;

/// Chrome spawned by this invocation, owned end to end.
///
/// # Drop
///
/// [`Drop`] is idempotent: after an explicit [`Self::kill`] the inner [`Child`]
/// has been taken, so Drop is a no-op and never double-waits. Every kill path
/// pairs `kill` with `wait` so no zombie survives. Log drainers are joined after
/// the child is reaped, so the pipes reach EOF instead of blocking the readers.
pub struct ChromeProcess {
    /// `None` after a successful reap.
    child: Option<Child>,
    /// POSIX process group, when the host models one (see
    /// [`crate::native::cdp::spawn::os`]).
    pgid: Option<i32>,
    /// Join handles of the stdout/stderr drainer threads.
    log_drainers: Vec<std::thread::JoinHandle<()>>,
}

impl ChromeProcess {
    /// Take ownership of a spawned Chrome and its drainer threads.
    #[must_use]
    pub fn new(
        child: Child,
        pgid: Option<i32>,
        log_drainers: Vec<std::thread::JoinHandle<()>>,
    ) -> Self {
        Self {
            child: Some(child),
            pgid,
            log_drainers,
        }
    }

    /// Kill and reap the child (idempotent). Safe to call from Drop.
    pub fn kill(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.join_log_drainers();
    }

    /// Wait up to `timeout` for a cooperative exit, then kill and reap.
    pub fn wait_or_kill(&mut self, timeout: Duration) {
        if let Some(mut child) = self.child.take() {
            crate::platform::wait_child_or_kill(&mut child, timeout);
        }
        self.join_log_drainers();
    }

    /// Non-blocking exit probe. `true` once the child exited or was reaped.
    pub fn has_exited(&mut self) -> bool {
        match self.child.as_mut() {
            None => true,
            Some(child) => match child.try_wait() {
                Ok(Some(_)) => {
                    // Exited: drop the Child so the slot is fully reaped.
                    let _ = self.child.take().map(|mut c| c.wait());
                    true
                }
                Ok(None) | Err(_) => false,
            },
        }
    }

    /// OS pid while the child is still owned; `None` after reap.
    #[must_use]
    pub fn id(&self) -> Option<u32> {
        self.child.as_ref().map(Child::id)
    }

    /// Process group to signal for a whole-tree kill, when the host has one.
    #[must_use]
    pub fn pgid(&self) -> Option<i32> {
        self.pgid
    }

    fn join_log_drainers(&mut self) {
        for handle in std::mem::take(&mut self.log_drainers) {
            let _ = handle.join();
        }
    }
}

impl Drop for ChromeProcess {
    fn drop(&mut self) {
        // Significant drop: external process resource. Keep short, never panic.
        self.kill();
    }
}
