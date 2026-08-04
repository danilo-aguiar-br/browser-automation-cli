// SPDX-License-Identifier: MIT OR Apache-2.0
//! One-shot local audio pipeline (no Chrome): probe, download, convert, trim.
//!
//! # Workload
//!
//! Container magic is pure-Rust. Stream probe and encode/remux use optional OS
//! binaries (`ffprobe` / `ffmpeg`) resolved via XDG `ffmpeg_path` then PATH —
//! never product env vars and never a linked libav build dependency.
//!
//! Single-file ops stay sequential (`sequential_justified`).
//!
//! # Agent-native contract
//!
//! Envelopes carry paths, container, codec, duration, and hashes.
//! Raw PCM / base64 audio is never emitted on stdout.

mod download;
mod ffmpeg_ops;
mod limits;
mod magic;
mod ops;
mod validate;

#[cfg(test)]
mod tests;

pub use ffmpeg_ops::ConvertOpts;
pub use limits::AudioLimits;
pub use magic::{detect_container, DetectedAudio};
#[allow(unused_imports)] // unit tests + parity with video_local surface
pub(crate) use ops::project_fields;
pub use ops::{convert, download, info, trim, AudioSource};
pub use validate::{parse_output_format, OutputFormat};
