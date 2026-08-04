// SPDX-License-Identifier: MIT OR Apache-2.0
//! macOS parent-death binding: process group only (documented degradation).

use std::os::unix::process::CommandExt;
use std::process::Command;

use super::{ParentDeathBinding, PlatformSpawn};

/// macOS implementation of [`PlatformSpawn`].
pub(super) struct MacosSpawn;

/// The single instance used by [`super::host`].
pub(super) const HOST: &dyn PlatformSpawn = &MacosSpawn;

impl PlatformSpawn for MacosSpawn {
    /// Give the child its own process group; no parent-death signal exists.
    ///
    /// # Why the guarantee is degraded here
    ///
    /// Darwin has no `PR_SET_PDEATHSIG`. The usual replacement is a watchdog
    /// thread that registers `kqueue`/`EVFILT_PROC` with `NOTE_EXIT` on the
    /// parent pid — but the watchdog lives *inside the CLI*, so it is killed by
    /// the same `SIGKILL` it is supposed to react to. Making it survive means
    /// spawning a second helper process per invocation, which contradicts the
    /// one-shot BORN → DIE contract and doubles the residue surface.
    ///
    /// The accepted trade-off is therefore: cooperative FINALIZE reaps the group
    /// on every normal and signalled exit, and a hard `SIGKILL` of the CLI leaves
    /// the group alive until cross-run residual GC collects the profile. This is
    /// recorded as [`ParentDeathBinding::Degraded`] and surfaced in the matrix in
    /// [`super`].
    ///
    /// # Async-signal-safety
    ///
    /// `setpgid` is async-signal-safe and is the only call made between `fork`
    /// and `exec`; the closure allocates nothing and takes no lock.
    fn bind_child(&self, command: &mut Command) {
        // SAFETY:
        // - Contract: run only async-signal-safe syscalls between fork and exec.
        // - Invariant: `setpgid(0, 0)` allocates nothing and takes no lock.
        // - Failure is ignored: the child then shares our group, which only
        //   weakens group-kill; FINALIZE still reaps it by pid.
        // - See: `man 2 setpgid`, `man 7 signal-safety`.
        // Defined outside the `pre_exec` block so the syscall keeps its own
        // `unsafe` scope and justification.
        let bind = || {
            // SAFETY: `setpgid` is async-signal-safe, allocates nothing, and
            // takes no lock, so it is legal between fork and exec. Failure is
            // ignored: the child then shares our group, which only weakens
            // group-kill. See `man 2 setpgid`.
            unsafe {
                libc::setpgid(0, 0);
            }
            Ok(())
        };
        // SAFETY: `pre_exec` requires an async-signal-safe closure; `bind` runs
        // only the syscall above. See `man 7 signal-safety`.
        unsafe {
            command.pre_exec(bind);
        }
    }

    fn process_group_of(&self, pid: u32) -> Option<i32> {
        // SAFETY:
        // - Contract: read the process group of a pid this process spawned.
        // - Invariant: `getpgid` returns -1 for an unknown pid, mapped to `None`.
        // - See: `man 2 getpgid`.
        let pgid = unsafe { libc::getpgid(pid as libc::pid_t) };
        (pgid > 0).then_some(pgid)
    }

    fn binding(&self) -> ParentDeathBinding {
        ParentDeathBinding::Degraded
    }
}
