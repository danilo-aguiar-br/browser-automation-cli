// SPDX-License-Identifier: MIT OR Apache-2.0
//! ffmpeg path→path ops for local audio.

mod atomic;
mod convert;
mod trim;
mod types;

pub use convert::convert_path;
pub use trim::trim_path;
pub use types::{ConvertOpts, ConvertResult};
