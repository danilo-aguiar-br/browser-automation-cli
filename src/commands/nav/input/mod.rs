// SPDX-License-Identifier: MIT OR Apache-2.0
//! One-shot input handlers.
//!
//! # Module map (GAP-051 SRP split)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | pointer | press, write, click-at |
//! | keyboard | keys, type, hover, drag |
//! | forms | submit, fill-form, upload, scroll |

mod forms;
mod keyboard;
mod pointer;

pub(crate) use forms::*;
pub(crate) use keyboard::*;
pub(crate) use pointer::*;
