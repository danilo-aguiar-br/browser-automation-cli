// SPDX-License-Identifier: MIT OR Apache-2.0
//! High-level video info / convert / to-mp3 / trim / thumbnail (agent envelopes).

mod common;
mod info;
mod source;
mod transform;

pub(crate) use common::project_fields;
pub use info::info;
pub use source::VideoSource;
pub use transform::{convert, thumbnail, to_mp3, trim};
