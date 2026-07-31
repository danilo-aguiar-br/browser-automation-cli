// SPDX-License-Identifier: MIT OR Apache-2.0
//! Element property queries over CDP.
//!
//! # Module map (GAP-051 SRP split)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | call | Shared resolve-then-`callFunctionOn` helper |
//! | text | textContent / innerText / innerHTML |
//! | attributes | Attribute and property reads, form value writes |
//! | state | Visible / enabled / checked predicates |
//! | geometry | Bounding box and computed styles |
//! | counting | Selector match counting |
//!
//! All query functions are re-exported flat so existing paths keep working.

mod attributes;
mod call;
mod counting;
mod geometry;
mod state;
mod text;

pub use attributes::*;
pub use counting::*;
pub use geometry::*;
pub use state::*;
pub use text::*;
