// SPDX-License-Identifier: MIT OR Apache-2.0

//! Browser storage state save/load (XDG sessions; optional encryption).
//!
//! # Workload
//!
//! **I/O-bound** CDP collection + **CPU** crypto. Disk reads use blocking helpers
//! off async when required. Paths via [`crate::xdg`] only (no product env).

mod collect;
mod crypto;
mod dispatch;
mod fs_ops;
mod save_load;
mod types;

#[cfg(test)]
mod tests;

pub use dispatch::dispatch_state_command;
pub use fs_ops::{
    get_sessions_dir, get_state_dir, state_clean, state_clear, state_list, state_rename, state_show,
};
pub use save_load::{load_state, save_state};
pub use types::{OriginStorage, StorageEntry, StorageState};
