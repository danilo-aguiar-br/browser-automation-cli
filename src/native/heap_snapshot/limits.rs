// SPDX-License-Identifier: MIT OR Apache-2.0

//! Heap snapshot infra ceilings, resolved through the policy layer.
//!
//! These were plain `pub use` re-exports of the compile-time constants until
//! the 0.1.9 audit measured what that cost: `config set heap_max_retainers 500`
//! returned `ok`, `config get` echoed 500, and the runtime kept using 200. The
//! key was published by `policy_knobs!` and read by nobody, so the whole XDG
//! surface for heap was decoration.
//!
//! Every accessor below goes through [`crate::xdg::policy::policy_usize`], which
//! reads the XDG snapshot once per process and falls back to the same constant
//! that used to be re-exported. The constants stay the defaults; they stop being
//! the only answer.
//!
//! `tests/phantom_flag_gate.rs` now scans the knob table for exactly this shape,
//! so a future refactor that inlines a constant again fails the gate instead of
//! shipping a lie.

use crate::xdg::policy::{key, policy_u64, policy_usize};

/// Offline heap snapshot file size ceiling, in bytes.
#[must_use]
pub fn max_heap_snapshot_bytes() -> u64 {
    policy_u64(key::HEAP_SNAPSHOT_MAX_BYTES)
}

/// Max retainers returned by a heap node operation.
#[must_use]
pub fn default_max_retainers() -> usize {
    policy_usize(key::HEAP_DEFAULT_MAX_RETAINERS)
}

/// Max edges returned by a heap node operation.
#[must_use]
pub fn default_max_edges() -> usize {
    policy_usize(key::HEAP_DEFAULT_MAX_EDGES)
}

/// Max paths enumerated by `heap paths`.
#[must_use]
pub fn default_max_paths() -> usize {
    policy_usize(key::HEAP_DEFAULT_MAX_PATHS)
}

/// Max depth walked by `heap paths`.
#[must_use]
pub fn default_max_path_depth() -> usize {
    policy_usize(key::HEAP_DEFAULT_MAX_PATH_DEPTH)
}

/// Cap on the `heap class-nodes` list.
#[must_use]
pub fn default_max_class_nodes() -> usize {
    policy_usize(key::HEAP_DEFAULT_MAX_CLASS_NODES)
}

/// Visited-state ceiling for the dominator walk (anti-pathological graphs).
#[must_use]
pub fn max_states() -> usize {
    policy_usize(key::HEAP_DOMINATOR_MAX_STATES)
}
