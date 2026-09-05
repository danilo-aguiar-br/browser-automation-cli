// SPDX-License-Identifier: MIT OR Apache-2.0
//! One-shot local video pipeline (no Chrome): probe, download, convert, extract audio, trim, thumbnail.
//!
//! # Workload
//!
//! Container magic is pure-Rust. Stream probe and encode/remux use optional OS
//! binaries (`ffprobe` / `ffmpeg`) resolved via XDG `ffmpeg_path` then PATH —
//! never product env vars and never a linked libav build dependency.
//!
//! Single-file ops stay sequential (`sequential_justified`): Rayon spawn cost
//! exceeds gain for one media file (rules_rust_paralelismo).
//!
//! # Agent-native contract
//!
//! Envelopes carry paths, container, stream codecs, duration, and hashes.
//! Raw media bytes and frame base64 are never emitted on stdout.
//!
//! # Module map (Tier-3 SRP)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `magic` | Magic-byte container probe (extension never trusted) |
//! | `limits` | XDG-backed input/download ceilings |
//! | `download` | HTTP fetch + SSRF + body cap + magic |
//! | `probe` | ffprobe JSON metadata (path-only; no full decode) |
//! | `container_matrix` | container×codec compatibility (spec facts, no policy) |
//! | `encoder_policy` | smart copy / re-encode plan and encoder defaults |
//! | `ffmpeg_bin` | resolve ffmpeg/ffprobe binaries |
//! | `ffmpeg_ops` | convert / to-mp3 / trim / thumbnail argv + atomic out |
//! | `ops` | public info / convert / to_mp3 / trim / thumbnail / download |

mod container_matrix;
mod download;
mod encoder_policy;
mod ffmpeg_bin;
mod ffmpeg_ops;
mod limits;
mod magic;
mod manifest;
mod ops;
mod probe;
mod site_extraction;

#[cfg(test)]
mod tests;

pub use container_matrix::{parse_output_container, OutputContainer};
pub use download::download_video;
pub use ffmpeg_bin::{require_ffmpeg, require_ffprobe, resolve_ffmpeg_bin, resolve_ffprobe_bin};
pub use ffmpeg_ops::ConvertOpts;
pub use limits::VideoLimits;
pub use magic::{detect_container, DetectedContainer};
pub use manifest::{
    available as manifest_available, detect_kind as detect_manifest_kind, parse as parse_manifest,
    ManifestKind,
};
pub(crate) use ops::project_fields;
pub use ops::{convert, info, thumbnail, to_mp3, trim, VideoSource};
pub use probe::{
    compact_streams, duration_secs, primary_audio_codec, primary_video_codec, probe_path,
};
pub use site_extraction::{classify_non_media, non_media_error, NonMediaBody};
