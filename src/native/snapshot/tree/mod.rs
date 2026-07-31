// SPDX-License-Identifier: MIT OR Apache-2.0
//! AX tree construction and text rendering.
//!
//! # Module map (GAP-051 SRP split)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | types | `TreeNode`, `HiddenInputKind`, `RoleNameTracker` |
//! | build | AX nodes to tree, depth assignment |
//! | render | tree to indented text, compaction |
//!
//! Re-exported at `pub(super)` so `snapshot` sees the same surface as before.

mod build;
mod render;
mod types;

pub(super) use build::build_tree;
// `count_indent` is consumed only by `snapshot::tests`; keep it on the module
// surface so the test does not reach into a private submodule path.
#[cfg(test)]
pub(super) use render::count_indent;
pub(super) use render::{compact_tree, render_tree};
pub(super) use types::*;
