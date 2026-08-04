// SPDX-License-Identifier: MIT OR Apache-2.0
//! Engine-agnostic self-spawn of a CDP browser process.
//!
//! Both engines follow the same shape: reserve a loopback port, fork the binary
//! from the perennial guard thread, drain its output, and poll the CDP endpoint
//! until it answers. Only the argv and the readiness budget differ.
//!
//! # Why the product forks instead of delegating
//!
//! `chromiumoxide::Browser::launch` forks Chrome from whichever thread happens
//! to call it and hands back only a `Child` it owns privately, so the product
//! never learns the pid. With no pid there is no residual kill target and no
//! process group, which is exactly why FINALIZE could not reap a hard-killed
//! Chrome. Forking here restores both, and the connection is then made with
//! `Browser::connect_with_config`.
//!
//! # Module map
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`guard`] | perennial spawn thread that keeps `PR_SET_PDEATHSIG` valid |
//! | [`os`] | the only place a platform `#[cfg]` appears on this path |
//! | [`logs`] | bounded stdout/stderr rings for launch diagnostics |
//! | [`ready`] | ephemeral port plus CDP readiness polling |

pub mod guard;
pub mod logs;
pub mod os;
pub mod ready;

pub use guard::{spawn_guarded, GuardedChild, SpawnRequest};
pub use logs::LaunchLogBuffer;
pub use os::{host, ParentDeathBinding, PlatformSpawn};
pub use ready::{reserve_loopback_port, wait_for_cdp_ready, ReadinessBudget};

pub(crate) use logs::start_log_drainers;

/// Test-only re-export for the Lightpanda error-shape assertions.
#[cfg(test)]
pub(crate) use logs::launch_error;
