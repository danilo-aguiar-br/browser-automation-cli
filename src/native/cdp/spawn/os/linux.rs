// SPDX-License-Identifier: MIT OR Apache-2.0
//! Linux parent-death binding: `PR_SET_PDEATHSIG` plus a dedicated process group.

use std::os::unix::process::CommandExt;
use std::process::Command;

use super::{ParentDeathBinding, PlatformSpawn};

/// Linux implementation of [`PlatformSpawn`].
pub(super) struct LinuxSpawn;

/// The single instance used by [`super::host`].
pub(super) const HOST: &dyn PlatformSpawn = &LinuxSpawn;

impl PlatformSpawn for LinuxSpawn {
    /// Bind the child to the spawning **thread** and give it its own group.
    ///
    /// # Async-signal-safety
    ///
    /// The closure passed to [`CommandExt::pre_exec`] runs in the child after
    /// `fork` and before `exec`. In that window the child holds a copy of every
    /// mutex the parent held at fork time, so allocating, locking, formatting a
    /// string, or logging can deadlock the child forever. Only async-signal-safe
    /// syscalls are allowed. Both calls below qualify: `prctl` and `setpgid` are
    /// plain syscalls with no allocation and no locking, and the closure touches
    /// nothing else.
    ///
    /// # Thread scope of `PR_SET_PDEATHSIG`
    ///
    /// The signal fires when the child's **parent thread** exits, not when the
    /// parent process exits. A child forked from a Tokio worker or from a
    /// `spawn_blocking` thread (which retires after an idle timeout) would be
    /// killed mid-session. The caller must therefore spawn from the perennial
    /// thread owned by [`crate::native::cdp::spawn::guard`], which lives as long
    /// as the CLI process does.
    fn bind_child(&self, command: &mut Command) {
        // Defined outside the `pre_exec` block so each syscall keeps its own
        // `unsafe` scope and justification; nesting them would collapse three
        // distinct operations into one unaudited block.
        let bind = || {
            // SAFETY:
            // - Contract: bind this child's lifetime to the spawning thread.
            // - Invariant: `prctl` is a plain syscall — no allocation, no lock —
            //   so it is legal between fork and exec.
            // - Failure is ignored on purpose: a child without the death signal
            //   is still reaped by FINALIZE and by residual GC, so aborting here
            //   would trade a degraded guarantee for no browser at all.
            // - See: `man 2 prctl` (PR_SET_PDEATHSIG).
            unsafe {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL);
            }
            // SAFETY:
            // - Contract: put the child in its own process group so FINALIZE can
            //   signal the whole browser tree without touching ours.
            // - Invariant: `setpgid` is async-signal-safe and allocates nothing.
            // - Failure is ignored: the child then shares our group, which only
            //   costs the group-kill optimisation (the caller refuses to signal
            //   its own group).
            // - See: `man 2 setpgid`.
            unsafe {
                libc::setpgid(0, 0);
            }
            Ok(())
        };
        // SAFETY:
        // - Contract: `pre_exec` requires an async-signal-safe closure.
        // - Invariant: `bind` runs only the two syscalls above; it allocates
        //   nothing, takes no lock, and calls nothing else.
        // - See: `std::os::unix::process::CommandExt::pre_exec`, `man 7 signal-safety`.
        unsafe {
            command.pre_exec(bind);
        }
    }

    fn process_group_of(&self, pid: u32) -> Option<i32> {
        // SAFETY:
        // - Contract: read the process group of a pid this process spawned.
        // - Invariant: `getpgid` has no preconditions; it returns -1 and sets
        //   errno for an unknown pid, which is mapped to `None` below.
        // - See: `man 2 getpgid`.
        let pgid = unsafe { libc::getpgid(pid as libc::pid_t) };
        (pgid > 0).then_some(pgid)
    }

    fn binding(&self) -> ParentDeathBinding {
        ParentDeathBinding::Kernel
    }
}
