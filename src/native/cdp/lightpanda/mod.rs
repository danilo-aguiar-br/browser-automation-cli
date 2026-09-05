// SPDX-License-Identifier: MIT OR Apache-2.0
//! Lightpanda browser backend (process + launch) — Pass H SRP split.

mod launch;
mod process;

#[cfg(test)]
mod tests;

pub use launch::{find_lightpanda, launch_lightpanda, LightpandaLaunchOptions};
pub use process::LightpandaProcess;

#[cfg(test)]
pub(crate) use process::lightpanda_startup_timeout;

#[cfg(test)]
pub(crate) use crate::native::cdp::spawn::LaunchLogBuffer;

#[cfg(test)]
pub(crate) use launch::{
    build_lightpanda_serve_args_with, lightpanda_launch_error, start_log_drainers,
    wait_for_lightpanda_ready,
};
