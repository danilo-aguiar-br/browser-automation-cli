// SPDX-License-Identifier: MIT OR Apache-2.0
//! One-shot lifecycle: INIT → EXECUTE → FINALIZE → EXIT.
//!
//! # Workload
//!
//! **Coordination (not compute fan-out):** one `CancellationToken` per process
//! invocation. Residual scavenge may use `map_cpu` when candidate paths are
//! large (see [`crate::residual`]). FINALIZE is ordered: Browser.close → residual
//! kill → flush. No multi-writer shared state.
//!
//! # Modules
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `tls` | Thread-local cancel / lifecycle registration |
//! | `ledger` | `ResourceLedger` + `Lifecycle` construction |
//! | `finalize` | Idempotent FINALIZE residual path |
//! | `kill` | Child SIGTERM→SIGKILL / wipe helpers |
//! | `mark` | Profile / side-channel markers; Drop safety net |

use std::time::Duration;

mod finalize;
mod kill;
mod ledger;
mod mark;
mod tls;

#[cfg(test)]
mod tests;

/// Max wait after residual `SIGTERM` before escalating to `SIGKILL` (Unix).
///
/// Kept short for one-shot CLIs (agents expect fast DIE) while still giving
/// Chrome a chance to flush profile locks before hard kill. Value lives in
/// [`crate::constants::FINALIZE_CHILD_GRACE_SECS`] (no magic literal).
pub const FINALIZE_CHILD_GRACE: Duration =
    Duration::from_secs(crate::constants::FINALIZE_CHILD_GRACE_SECS);

#[cfg(unix)]
pub use kill::{
    descendant_pids, descendants_in_index, kill_unix_graceful, kill_unix_group_graceful,
    kill_unix_tree,
};
pub use ledger::{Lifecycle, ResourceLedger};
pub use mark::mark_side_channel;
pub use tls::{current_cancel, current_lifecycle};
