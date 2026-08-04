// SPDX-License-Identifier: MIT OR Apache-2.0
//! Mark profile / side-channel paths on the lifecycle ledger; Default + Drop.

use std::path::PathBuf;

use super::ledger::Lifecycle;

/// Record a side-channel path (SingletonLock, etc.) owned by this launch.
pub fn mark_side_channel(life: &Lifecycle, path: PathBuf) {
    life.with_ledger_mut(|ledger| {
        if !ledger.side_channels.iter().any(|p| p == &path) {
            ledger.side_channels.push(path);
        }
    });
}

impl Default for Lifecycle {
    fn default() -> Self {
        Self::new()
    }
}

/// Safety net: if the call site forgets explicit `Lifecycle::finalize`, Drop still
/// runs residual cleanup (sync only: kill/wipe; no async I/O).
///
/// # Drop contract
///
/// - **Idempotent** via `finalize_done` (`SeqCst` CAS).
/// - **Bounded**: residual Unix path may sleep up to `FINALIZE_CHILD_GRACE` (not
///   indefinite; rules forbid unbounded block in Drop).
/// - **No panic**: kill/wipe errors are ignored (best-effort).
/// - Clones share the same Arcs; the first Drop that wins the CAS performs work.
impl Drop for Lifecycle {
    fn drop(&mut self) {
        self.finalize();
    }
}
