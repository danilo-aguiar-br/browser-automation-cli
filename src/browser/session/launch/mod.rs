// SPDX-License-Identifier: MIT OR Apache-2.0
//! `OneShotSession` construction and the state it carries afterwards.
//!
//! # Module map (GAP-051 SRP split)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | spawn | launch paths (headless, proxy, extensions) |
//! | state | capture domains, dialog tracking, CDP event pump |

mod spawn;
mod state;
mod stealth;
