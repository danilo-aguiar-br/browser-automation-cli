// SPDX-License-Identifier: MIT OR Apache-2.0
//! Element resolution, hit-testing, and ref-based node helpers.
//!
//! **I/O-bound single-act** (N-138). Multi-ref fan-out lives in snapshot/grab.
//!
//! # Module map (Pass 32 SRP-06): refs | js | resolve | queries

mod js;
pub mod locator;
mod queries;
mod refs;
mod resolve;

#[cfg(test)]
mod tests;

pub use locator::{assign_locators, DurableLocator};
pub use queries::*;
pub use refs::{parse_ref, RefEntry, RefMap};
pub use resolve::{resolve_element_center, resolve_element_object_id};

pub(crate) use resolve::resolve_ax_session;
