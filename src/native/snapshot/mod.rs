// SPDX-License-Identifier: MIT OR Apache-2.0
//! Accessibility tree and DOM snapshot builders for agents.
//!
//! **Mista:** multi-ref CDP resolve uses join_bounded (I/O). Tree build sequential (N-145).
//!
//! # Module map (Pass 32 SRP-07): options | take | tree | cursor | ax

mod ax;
mod cursor;
mod options;
mod take;
mod tree;

#[cfg(test)]
mod tests;

pub use options::SnapshotOptions;
pub use take::take_snapshot;
