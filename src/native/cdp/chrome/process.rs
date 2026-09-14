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
    /// The private X server this Chrome draws into, when one was started.
    ///
    /// Owned here so the ordering is the only one that is safe: this struct
    /// reaps Chrome, and the field drops afterwards, so the display always
    /// outlives the browser drawing into it. Killing the server first would
    /// take Chrome down with it and turn an orderly teardown into a crash.
    ///
    /// It also means the display cannot leak by being forgotten at a call
    /// site: whoever owns the browser owns the screen it renders on.
    xvfb: Option<crate::native::cdp::xvfb::XvfbGuard>,
    /// The DevTools pipe bridge, when Chrome was launched on the pipe.
    ///
    /// Shut down only after the child is reaped: its reader thread ends on the
    /// end-of-file a dead Chrome produces, so joining it earlier would wait on
    /// a browser that is still alive.
    transport: Option<crate::native::cdp::pipe::PipeBridge>,
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
            xvfb: None,
            transport: None,
        }
    }

    /// Hand the DevTools pipe bridge to the process whose pipe it relays.
    #[must_use]
    pub fn with_transport(mut self, bridge: crate::native::cdp::pipe::PipeBridge) -> Self {
        self.transport = Some(bridge);
        self
    }

    /// The bridge, for the readiness wait before the client connects.
    pub fn transport_mut(&mut self) -> Option<&mut crate::native::cdp::pipe::PipeBridge> {
        self.transport.as_mut()
    }

    /// Hand the private display to the process that renders into it.
    ///
    /// Separate from [`Self::new`] so the twelve call sites that never start a
    /// display do not have to name the field.
    #[must_use]
    pub fn with_private_display(
        mut self,
        xvfb: Option<crate::native::cdp::xvfb::XvfbGuard>,
    ) -> Self {
        self.xvfb = xvfb;
        self
    }

    /// Kill and reap the child (idempotent). Safe to call from Drop.
    pub fn kill(&mut self) {
        if self.child.is_some() {
            // The forced path: SIGKILL at once, as `child.kill()` always did. A
            // grace here would always run out, because the unreaped leader still
            // counts as a live member of its group.
            self.kill_group(Duration::ZERO);
        }
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.shutdown_transport();
        self.join_log_drainers();
    }

    /// Wait up to `timeout` for a cooperative exit, then kill and reap.
    pub fn wait_or_kill(&mut self, timeout: Duration) {
        if let Some(mut child) = self.child.take() {
            crate::platform::wait_child_or_kill(&mut child, timeout);
            // Stragglers only: the leader exited or was killed just above, so
            // they get SIGTERM and a short grace before SIGKILL.
            self.kill_group(Duration::from_millis(
                crate::constants::CHROME_GROUP_STRAGGLER_GRACE_MS,
            ));
        }
        self.shutdown_transport();
        self.join_log_drainers();
    }

    /// Kill every member of Chrome's process group, not only its leader.
    ///
    /// Killing the pid alone let a descendant that stayed in the group outlive
    /// the CLI: measured after a failed launch, where the lifecycle ledger
    /// never learned the pgid and FINALIZE therefore signalled nothing. Such a
    /// descendant also holds the DevTools pipe and the output pipes open, which
    /// is what the bounded joins below would otherwise have to wait out.
    fn kill_group(&self, grace: Duration) {
        #[cfg(unix)]
        if let Some(pgid) = self.pgid {
            crate::lifecycle::kill_unix_group_graceful(pgid, grace);
        }
        #[cfg(not(unix))]
        let _ = grace;
    }

    fn shutdown_transport(&mut self) {
        if let Some(mut bridge) = self.transport.take() {
            bridge.shutdown();
        }
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

    /// The live child for a readiness probe, or `None` after reap.
    ///
    /// Exists so the launcher can build this owner BEFORE its first wait and
    /// still probe the child through it: owning first is what makes a
    /// cancelled launch kill Chrome instead of forgetting it.
    pub fn child_mut(&mut self) -> Option<&mut Child> {
        self.child.as_mut()
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
        crate::native::cdp::spawn::logs::join_drainers_within(
            std::mem::take(&mut self.log_drainers),
            std::time::Duration::from_millis(crate::constants::LOG_DRAINER_JOIN_GRACE_MS),
        );
    }
}

impl Drop for ChromeProcess {
    fn drop(&mut self) {
        // Significant drop: external process resource. Keep short, never panic.
        self.kill();
    }
}
