// SPDX-License-Identifier: MIT OR Apache-2.0
//! `wait` surface for `OneShotSession`.
//!
//! # Module map (GAP-051 SRP split)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | request | `WaitRequest`, the condition set a wait carries |
//! | conditions | the OR algebra that resolves them |
//! | entry | `wait_for`, `wait_for_any`, `wait_for_any_ex` argument shaping |
//! | pick | `pick_option`, which shares the polling helpers |
//!
//! Every method stays inherent on `OneShotSession`; only the defining file moved.

mod conditions;
mod entry;
mod pick;
mod probes;
mod request;

pub use request::WaitRequest;
