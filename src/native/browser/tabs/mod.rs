// SPDX-License-Identifier: MIT OR Apache-2.0
//! Multi-tab management for the one-shot process.
//!
//! # Module map (GAP-051 SRP split)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | query | ensure page, list, resolve ref, labels |
//! | create | new tab, new isolated context |
//! | mutate | switch and close, by index or stable id |

mod create;
mod mutate;
mod query;
