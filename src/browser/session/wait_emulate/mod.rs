// SPDX-License-Identifier: MIT OR Apache-2.0

//! Wait / emulate / resize methods for [`super::OneShotSession`] (Pass G SRP).

mod emulate;
mod resize;
mod wait;

pub use wait::WaitRequest;
