// SPDX-License-Identifier: MIT OR Apache-2.0
//! Browser automation one-shot session and command entry points.
//!
//! Lifecycle: launch Chrome via CDP → execute → FINALIZE → DIE.
//!
//! ## Module map (A-05 componentization)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `helpers` | pure helpers (eval normalize, image magic) |
//! | `session` | `OneShotSession` (SRP submods: launch/nav/content/assert_net/interact/wait_emulate/media/extensions) |
//! | `support` | launch marks + finish |
//! | `commands` | `run_*` + `block_on_browser*` |
//! | `shutdown` | [`ShutdownTrigger`](crate::browser::ShutdownTrigger), [`shutdown_signal`](crate::browser::shutdown_signal) |

mod commands;
mod helpers;
mod session;
mod shutdown;
mod support;

pub use commands::*;
pub use helpers::tree_to_at_refs;
pub use session::*;
pub(crate) use shutdown::cancelled_error;
pub use shutdown::{shutdown_signal, ShutdownTrigger};
pub(crate) use support::dump_failure_evidence;
