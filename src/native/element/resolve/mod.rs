// SPDX-License-Identifier: MIT OR Apache-2.0
//! Element center/object/AX resolution over CDP (Pass H SRP split).
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`helpers`] | Frame owner, scroll, selector, box center |
//! | [`center`] | Click center resolution |
//! | [`object_id`] | Remote object id resolution |
//! | [`ax`] | Accessibility session helpers |

mod ax;
mod center;
mod helpers;
mod object_id;

pub(crate) use ax::resolve_ax_session;
pub use center::resolve_element_center;
pub use object_id::resolve_element_object_id;

// Test-only re-export: production callers (`center`, `object_id`) import it
// straight from `helpers`; only `element::tests` reaches it through this path.
#[cfg(test)]
pub(super) use helpers::resolve_frame_session;

#[cfg(test)]
pub(super) use helpers::box_model_center;
