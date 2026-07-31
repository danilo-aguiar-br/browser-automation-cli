// SPDX-License-Identifier: MIT OR Apache-2.0

//! Offline V8 `.heapsnapshot` analysis for `browser-automation-cli heap *`.
//!
//! # Workload / parallelism
//!
//! **CPU-bound** graph algorithms. Dominator phases stay sequential (N-142 / N-152).
//! File size hard-capped by [`limits::MAX_HEAP_SNAPSHOT_BYTES`].

mod graph;
mod limits;
mod ops;
mod parse;

#[cfg(test)]
mod tests;

pub use ops::{
    class_nodes, close_snapshot, compare, details, duplicate_strings, node_op, node_op_with_limits,
    object_details, summarize,
};
