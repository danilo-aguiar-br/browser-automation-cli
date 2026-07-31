// SPDX-License-Identifier: MIT OR Apache-2.0
//! OneShotSession media methods (perf, screencast, heap).
//!
//! Split by domain for SRP (Pass F). Each submodule owns an `impl OneShotSession` block.

mod heap;
mod perf;
mod screencast;
