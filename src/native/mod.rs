// SPDX-License-Identifier: MIT OR Apache-2.0
//! Native CDP/browser stack used by one-shot MVP path.

pub mod browser;
/// Chrome DevTools Protocol transport and wire types.
pub mod cdp;
pub mod cookies;
pub mod element;
pub mod heap_snapshot;
pub mod interaction;
pub mod network;
pub mod perf_insight;
pub mod screenshot;
pub mod snapshot;
pub mod state;
pub mod stealth;
