// SPDX-License-Identifier: MIT OR Apache-2.0
//! Browser navigation and interaction command handlers.
//!
//! # Module map (Pass 32 SRP-04)
//! session | goto | input | wait | capture | page | scripts

mod capture;
mod goto;
mod input;
mod page;
mod scripts;
mod session;
mod storage;
mod wait;

pub(crate) use capture::*;
pub(crate) use goto::*;
pub(crate) use input::*;
pub(crate) use page::*;
pub(crate) use scripts::*;
pub(crate) use session::*;
pub(crate) use storage::handle_storage;
pub(crate) use wait::*;
