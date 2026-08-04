// SPDX-License-Identifier: MIT OR Apache-2.0
//! Windows parent-death binding: Job Object with kill-on-job-close.

use std::process::Command;

use super::{ParentDeathBinding, PlatformSpawn};

/// Windows implementation of [`PlatformSpawn`].
pub(super) struct WindowsSpawn;

/// The single instance used by [`super::host`].
pub(super) const HOST: &dyn PlatformSpawn = &WindowsSpawn;

impl PlatformSpawn for WindowsSpawn {
    /// No pre-spawn hook: the binding is applied *after* spawn.
    ///
    /// Windows has no `fork`, so there is no pre-exec window to use. The child
    /// is instead assigned to a Job Object carrying
    /// `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, which the kernel enforces when the
    /// last handle to the job closes — including when this process is terminated.
    /// That assignment already happens in
    /// [`crate::win_job::create_and_assign`], driven from the lifecycle ledger,
    /// so binding here would create a second job for the same pid.
    fn bind_child(&self, _command: &mut Command) {}

    /// Windows models job objects rather than POSIX process groups.
    fn process_group_of(&self, _pid: u32) -> Option<i32> {
        None
    }

    fn binding(&self) -> ParentDeathBinding {
        ParentDeathBinding::Kernel
    }
}
