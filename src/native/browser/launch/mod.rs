// SPDX-License-Identifier: MIT OR Apache-2.0
//! Browser launch, CDP attach, and domain enablement.
//!
//! # Module map (GAP-051 SRP split)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | chrome | Engine dispatch and one-shot launch |
//! | targets | Target discovery and attach |
//! | domains | Domain enablement and debugger resume |
//! | lightpanda | Lightpanda attach retry and startup deadline |
//!
//! Free functions and constants are re-exported flat so existing paths keep
//! working.

mod chrome;
mod domains;
mod lightpanda;
mod targets;

// Test-only re-export: `chrome` imports what it needs straight from the
// `lightpanda` module; only `browser::tests` reaches these through this path.
#[cfg(test)]
pub(crate) use lightpanda::*;
