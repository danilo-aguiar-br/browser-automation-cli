// SPDX-License-Identifier: MIT OR Apache-2.0
//! Lightpanda child process RAII (kill + wait on Drop).
use std::process::Child;
use std::time::Duration;

use crate::constants::{
    LIGHTPANDA_DISCOVERY_TIMEOUT_MS, LIGHTPANDA_POLL_INTERVAL_MS, LIGHTPANDA_READY_SLICE_MS,
};
use crate::xdg::resolve_lightpanda_startup_timeout_secs;

pub(crate) fn lightpanda_startup_timeout() -> Duration {
    Duration::from_secs(resolve_lightpanda_startup_timeout_secs())
}

pub(crate) fn lightpanda_poll_interval() -> Duration {
    Duration::from_millis(LIGHTPANDA_POLL_INTERVAL_MS)
}

pub(crate) fn lightpanda_discovery_timeout() -> Duration {
    Duration::from_millis(LIGHTPANDA_DISCOVERY_TIMEOUT_MS)
}

pub(crate) fn lightpanda_ready_slice() -> Duration {
    Duration::from_millis(LIGHTPANDA_READY_SLICE_MS)
}

/// Owned Lightpanda child process (RAII: kill + `wait` on Drop).
///
/// # Drop
///
/// [`Drop`] is idempotent: after an explicit [`Self::kill`], the inner
/// [`Child`] is taken so Drop is a no-op (no double-wait). Every kill path
/// always pairs `kill` with `wait` to avoid Unix zombies (rules: spawn→wait).
/// Log drainers are joined after the child is reaped so pipes EOF cleanly.
pub struct LightpandaProcess {
    /// `None` after successful reap via [`Self::kill`].
    pub(crate) child: Option<Child>,
    /// DevTools websocket URL the spawned process is serving on.
    pub ws_url: String,
    pub(crate) log_drainers: Vec<std::thread::JoinHandle<()>>,
}

impl LightpandaProcess {
    /// Kill and reap the child (idempotent). Safe to call from Drop.
    pub fn kill(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.join_log_drainers();
    }

    /// Wait up to `timeout` for exit after Browser.close, then kill+reap.
    pub fn wait_or_kill(&mut self, timeout: std::time::Duration) {
        if let Some(mut child) = self.child.take() {
            crate::platform::wait_child_or_kill(&mut child, timeout);
        }
        self.join_log_drainers();
    }

    fn join_log_drainers(&mut self) {
        for handle in std::mem::take(&mut self.log_drainers) {
            let _ = handle.join();
        }
    }

    /// Non-blocking exit probe (`try_wait`). `true` when already reaped or exited.
    pub fn has_exited(&mut self) -> bool {
        match self.child.as_mut() {
            None => true,
            Some(child) => match child.try_wait() {
                Ok(Some(_)) => {
                    // Process exited; drop the Child to reap fully.
                    let _ = self.child.take().map(|mut c| c.wait());
                    true
                }
                Ok(None) => false,
                Err(_) => false,
            },
        }
    }

    /// OS pid while the child is still owned; `None` after reap.
    pub fn id(&self) -> Option<u32> {
        self.child.as_ref().map(|c| c.id())
    }
}

impl Drop for LightpandaProcess {
    fn drop(&mut self) {
        // Significant drop: external process resource. Keep short, no panic.
        self.kill();
    }
}

/// Best-effort kill + wait (rules: every spawn path reaps the child).
pub(crate) fn kill_and_reap(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}
