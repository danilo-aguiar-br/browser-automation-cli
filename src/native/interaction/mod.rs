// SPDX-License-Identifier: MIT OR Apache-2.0
//! Pointer, keyboard, form, and dialog interaction over CDP.
//!
//! **I/O-bound ordered** (N-135/N-141). No JoinSet fan-out.
//!
//! # Module map: types | kinematics | pointer | keyboard | form | scroll | element_ops | dispatch | keys | drag_html5
//!
//! `kinematics` is pure geometry and timing; every other module here owns wire
//! dispatch. Keeping the split means the trajectory maths is unit-tested without
//! a browser, and no dispatch path can quietly grow its own pacing rules.

mod dispatch;
mod drag_html5;
mod element_ops;
mod form;
mod keyboard;
mod keys;
mod kinematics;
mod pointer;
mod scroll;
mod types;
mod wheel;

#[cfg(test)]
mod tests;

pub use drag_html5::*;
pub use element_ops::*;
pub use form::*;
pub use keyboard::*;
pub use kinematics::{
    active as active_kinematics, active_profile, set_input_profile, set_input_seed, InputProfile,
    Jitter, Kinematics,
};
pub use pointer::*;
pub use scroll::*;
pub use types::{ClickResult, PendingRelease};
