// SPDX-License-Identifier: MIT OR Apache-2.0

//! Snapshot graph load + dominator algorithms (CPU-bound; dominators sequential N-142).
//!
//! # Module map (Pass G SRP)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `types` | `NodeRec`, `EdgeRec`, `SnapshotGraph` storage |
//! | `load` | parse heapsnapshot file into graph |
//! | `metrics` | BFS distances, retained sizes, resolve |
//! | `json` | node/edge/object JSON views |
//! | `dominators` | idom tree + dominator chain |
//! | `paths` | retaining paths |

mod dominators;
mod json;
mod load;
mod metrics;
mod paths;
mod types;
pub(crate) use types::SnapshotGraph;
