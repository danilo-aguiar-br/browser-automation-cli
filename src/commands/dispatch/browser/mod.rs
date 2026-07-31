// SPDX-License-Identifier: MIT OR Apache-2.0
//! Browser command family (nav + input + page state).

mod input_actions;
mod nav_actions;
mod page;

pub(crate) use input_actions::*;
pub(crate) use nav_actions::*;
pub(crate) use page::*;
