// SPDX-License-Identifier: MIT OR Apache-2.0
//! Screenshot annotation: collect, project and draw element callouts.
//!
//! # Module map (GAP-051 SRP split)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | collect | annotation collection and rect lookup |
//! | geometry | rect parsing, overlap filtering, rounding |
//! | overlay | in-page overlay injection and removal |
//! | project | scroll offsets and viewport projection |

mod collect;
mod geometry;
mod overlay;
mod project;

pub(crate) use collect::*;
pub(crate) use geometry::*;
pub(crate) use overlay::*;
pub(crate) use project::*;
