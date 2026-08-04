// SPDX-License-Identifier: MIT OR Apache-2.0
//! ffmpeg convert / extract-audio / trim / thumbnail via safe subprocess (path → path).

mod atomic;
mod clip;
mod convert;
mod types;

pub use clip::{thumbnail_path, to_mp3_path, trim_path};
pub use convert::convert_path;
pub use types::{ConvertOpts, ConvertResult};
