// SPDX-License-Identifier: MIT OR Apache-2.0
//! One-shot local image pipeline (no Chrome): probe, convert, resize, download,
//! and EXIF.
//!
//! # Workload
//!
//! **CPU-bound** decode/encode runs on the caller's thread (CLI one-shot) or
//! via `spawn_blocking` when invoked from async browser paths. Single-file
//! ops stay sequential: Rayon spawn cost exceeds gain for one image
//! (rules_rust_paralelismo).
//!
//! # Agent-native contract
//!
//! Envelopes carry path, dimensions, format, hashes, and EXIF maps. Pixel
//! base64 is never emitted unless a caller opts in explicitly.
//!
//! Text recognition is deliberately absent. The agent consuming this CLI reads
//! images natively, so an in-process OCR stage would be redundant middleware
//! that spends tokens and drags an external C binary into a rust-native tool.
//!
//! # Module map (Tier-3 SRP)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `magic` | Magic-byte format probe (extension never trusted) |
//! | `limits` | XDG-backed input/pixel ceilings |
//! | `atomic` | tmp + fsync + rename byte writes |
//! | `decode` | ImageReader + limits + magic-first load |
//! | `encode` | Format encode + quality + atomic write |
//! | `ops` | info / convert / resize |
//! | `download` | HTTP fetch + SSRF + body cap + magic |
//! | `exif` | kamadak-exif read (GPS gated) |
//! | `iptc` | IPTC IIM read from the Photoshop 8BIM APP13 block |
//! | `xmp` | XMP packet extract + flatten (JPEG/PNG/ISOBMFF) |
//! | `avif` | AVIF encode via ravif (decode impossible without C — see module) |
//! | `heic` | HEIC decode via heif-oxide (encode impossible — no Rust HEVC encoder) |
//! | `svg_sanitize` | SVG threat gate: bytes, entities, depth, href, script |
//! | `svg` | SVG rasterise via resvg/tiny-skia |
//! | `gif_frames` | GIF animation probe, frame extract, reassemble |
//! | `resize_simd` | SIMD resize via fast_image_resize (falls back to imageops) |

mod atomic;
mod avif;
mod decode;
mod download;
mod encode;
mod exif;
mod gif_frames;
mod heic;
mod iptc;
mod limits;
mod magic;
mod ops;
mod resize_simd;
mod svg;
mod svg_sanitize;
mod xmp;

#[cfg(test)]
mod tests;

pub use atomic::write_bytes_atomic;
pub use decode::{decode_bytes, decode_path};
pub use download::download_image;
pub use encode::{encode_to_path, encode_to_vec, parse_output_format, OutputFormat};
pub use exif::read_exif_map;
pub use limits::ImageLimits;
pub use magic::{detect_format, verify_format_name, DetectedFormat};
pub(crate) use ops::project_fields;
pub use ops::{convert, decode_path_for_qr, decode_path_for_qr_with, info, resize, ImageSource};

pub use avif::{
    decode_available as avif_decode_available, encode_available as avif_encode_available,
    encode_to_path as encode_avif_to_path, encode_to_vec as encode_avif_to_vec,
};
pub use gif_frames::{extract_frames, frame_count, reassemble, write_frame_png, GifFrame};
pub use heic::{
    decode_available as heic_decode_available, encode_available as heic_encode_available,
};
pub use iptc::read_iptc_map;
pub use resize_simd::{backend as resize_backend, resize_exact};
pub use svg::{looks_like_svg, raster_available as svg_raster_available, rasterize, RasterizedSvg};
pub use svg_sanitize::{sanitize as sanitize_svg, SvgReport};
pub use xmp::{extract_packet as extract_xmp_packet, read_xmp_map};
