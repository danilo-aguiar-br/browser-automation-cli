// SPDX-License-Identifier: MIT OR Apache-2.0
//! Platform facade for parent-death binding of self-spawned browser children.
//!
//! Every `#[cfg(target_os = …)]` in the self-spawn path lives in this module
//! tree. Business code depends on the [`PlatformSpawn`] trait and on
//! [`host`], never on a conditional compilation attribute.
//!
//! # Guarantee matrix
//!
//! | Host | Mechanism | Guarantee on hard-kill of the CLI |
//! |------|-----------|-----------------------------------|
//! | Linux | `prctl(PR_SET_PDEATHSIG, SIGKILL)` + `setpgid(0, 0)` | kernel-enforced: the child dies with the spawning thread |
//! | Windows | Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` | kernel-enforced: the tree dies when the last job handle closes |
//! | macOS | new process group only | **degraded**: no parent-death signal exists; reaping relies on FINALIZE and on cross-run residual GC |
//! | other | none | **degraded**: same as macOS |
//!
//! The degraded rows are why residual GC is not optional: on those hosts a
//! `SIGKILL` of the CLI leaves the browser group alive until a later invocation
//! collects it.

use std::process::Command;

#[cfg_attr(docsrs, doc(cfg(target_os = "linux")))]
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as imp;

#[cfg_attr(docsrs, doc(cfg(target_os = "macos")))]
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as imp;

#[cfg_attr(docsrs, doc(cfg(windows)))]
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as imp;

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod other;
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
use other as imp;

/// How strongly the host binds a spawned child to the lifetime of this process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParentDeathBinding {
    /// The kernel kills the child when the spawning thread or job goes away.
    Kernel,
    /// Only cooperative reaping is available; residual GC is the safety net.
    Degraded,
}

impl ParentDeathBinding {
    /// Stable token for doctor output and tests.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Kernel => "kernel",
            Self::Degraded => "degraded",
        }
    }
}

/// Host hooks applied around a self-spawned browser child.
pub trait PlatformSpawn {
    /// Install the pre-exec hooks that bind the child to this process.
    ///
    /// Called once per [`Command`] before `spawn`.
    fn bind_child(&self, command: &mut Command);

    /// Process group id to signal for a whole-tree kill, when the host has one.
    fn process_group_of(&self, pid: u32) -> Option<i32>;

    /// Strength of the parent-death guarantee on this host.
    fn binding(&self) -> ParentDeathBinding;
}

/// The single [`PlatformSpawn`] implementation compiled for this target.
#[must_use]
pub fn host() -> &'static dyn PlatformSpawn {
    imp::HOST
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_token_is_stable() {
        assert_eq!(ParentDeathBinding::Kernel.as_str(), "kernel");
        assert_eq!(ParentDeathBinding::Degraded.as_str(), "degraded");
    }

    #[test]
    fn host_binding_matches_documented_matrix() {
        let expected = if cfg!(target_os = "linux") || cfg!(windows) {
            ParentDeathBinding::Kernel
        } else {
            ParentDeathBinding::Degraded
        };
        assert_eq!(host().binding(), expected);
    }

    #[test]
    fn process_group_of_self_is_never_zero() {
        // A pid that exists must resolve to a positive group, or to None on
        // hosts that do not model process groups at all.
        if let Some(pgid) = host().process_group_of(std::process::id()) {
            assert!(pgid > 0, "process group must be positive, got {pgid}");
        }
    }
}
