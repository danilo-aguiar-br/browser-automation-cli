// SPDX-License-Identifier: MIT OR Apache-2.0
//! Pointer, keyboard, form, and dialog interaction over CDP.
//!
//! **I/O-bound ordered** (N-135/N-141). No JoinSet fan-out.
//!
//! # Module map (Pass 32 SRP-05): types | pointer | keyboard | form | scroll | dispatch | keys | drag_html5

mod dispatch;
mod drag_html5;
mod form;
mod keyboard;
mod keys;
mod pointer;
mod scroll;
mod types;

#[cfg(test)]
mod tests;

pub use drag_html5::*;
pub use form::*;
pub use keyboard::*;
pub use pointer::*;
pub use scroll::*;
pub use types::{ClickResult, PendingRelease};
