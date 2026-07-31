// SPDX-License-Identifier: MIT OR Apache-2.0

//! Form fill / select / check / focus (Pass G SRP).

mod check;
mod fill;
mod focus;
mod select;
mod select_events;

pub use check::{check, uncheck};
pub use fill::{fill, fill_smart};
pub use focus::{clear, focus};
pub use select::select_option;
/// Shared native `<select>` event dispatch (GAP-055); used by fill-form and pick.
pub use select_events::DISPATCH_INPUT_AND_CHANGE;
