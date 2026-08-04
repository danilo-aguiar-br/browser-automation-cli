// SPDX-License-Identifier: MIT OR Apache-2.0
//! Fallback parent-death binding for hosts with neither PDEATHSIG nor jobs.

use std::process::Command;

use super::{ParentDeathBinding, PlatformSpawn};

/// Fallback implementation of [`PlatformSpawn`] for unsupported targets.
pub(super) struct FallbackSpawn;

/// The single instance used by [`super::host`].
pub(super) const HOST: &dyn PlatformSpawn = &FallbackSpawn;

impl PlatformSpawn for FallbackSpawn {
    /// No binding is available; the child is reaped cooperatively only.
    fn bind_child(&self, _command: &mut Command) {}

    /// No process-group model is assumed on an unknown host.
    fn process_group_of(&self, _pid: u32) -> Option<i32> {
        None
    }

    fn binding(&self) -> ParentDeathBinding {
        ParentDeathBinding::Degraded
    }
}
