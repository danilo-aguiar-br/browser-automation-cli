// SPDX-License-Identifier: MIT OR Apache-2.0
//! Screenshot, screencast, and local image pipeline defaults.
//!
//! Operator overrides live in XDG (`config set …`); these constants are the
//! named compile-time fallbacks only (never product env vars).

/// Default JPEG quality when `grab`/`screenshot` omits `--quality` (1..=100).
///
/// Operator override: XDG `config set default_jpeg_quality <n>`.
pub const DEFAULT_JPEG_QUALITY: u8 = 80;

/// Default screencast CDP JPEG quality (1..=100).
///
/// Operator override: XDG `config set screencast_jpeg_quality <n>`.
pub const DEFAULT_SCREENCAST_JPEG_QUALITY: u8 = 60;

/// Screencast start: immediate pump iterations after Page.startScreencast.
pub const DEFAULT_SCREENCAST_START_PUMP_ITERS: u32 = 15;

/// Screencast stop: drain pump iterations before stopScreencast.
pub const DEFAULT_SCREENCAST_STOP_PUMP_ITERS: u32 = 40;

/// Max bytes accepted for local image decode / convert / resize input.
///
/// Operator override: XDG `config set image_max_input_bytes <n>`.
pub const DEFAULT_IMAGE_MAX_INPUT_BYTES: u64 = 32_000_000;

/// Max decoded pixel count (`width * height`) before reject (anti-bomb).
///
/// Operator override: XDG `config set image_max_pixels <n>`.
pub const DEFAULT_IMAGE_MAX_PIXELS: u64 = 64_000_000;

/// Default output format for `image convert` when `--format` is omitted.
///
/// Operator override: XDG `config set image_default_format <png|jpeg|webp|gif>`.
pub const DEFAULT_IMAGE_FORMAT: &str = "png";

/// Default quality for lossy `image convert` / `image resize` encode (1..=100).
///
/// Operator override: XDG `config set image_default_quality <n>`.
pub const DEFAULT_IMAGE_QUALITY: u8 = 85;

/// Max bytes for `image download` HTTP body (defaults equal input cap).
///
/// Operator override: XDG `config set image_download_max_bytes <n>`.
pub const DEFAULT_IMAGE_DOWNLOAD_MAX_BYTES: u64 = DEFAULT_IMAGE_MAX_INPUT_BYTES;

/// Minimum header bytes to probe image magic (covers WebP RIFF+WEBP).
pub const IMAGE_MAGIC_PROBE_BYTES: usize = 12;

/// AVIF encoder speed for `ravif` (1 = slowest/best, 10 = fastest/worst).
///
/// 6 keeps a 4K frame under a few seconds on a laptop while staying visually
/// close to the slow presets. Operator override: `config set image_avif_speed <1..=10>`.
pub const DEFAULT_IMAGE_AVIF_SPEED: u8 = 6;

/// Max bytes accepted for an SVG source before rasterisation.
///
/// Deliberately far below [`DEFAULT_IMAGE_MAX_INPUT_BYTES`]: SVG is a *program*,
/// not a pixel buffer, so a small file can still expand into gigabytes of work.
/// Operator override: `config set svg_max_bytes <n>`.
pub const DEFAULT_SVG_MAX_BYTES: u64 = 4_000_000;

/// Max XML element nesting depth accepted in an SVG source.
///
/// Guards the recursive-descent parser against stack exhaustion from a
/// pathologically nested document. Operator override: `config set svg_max_depth <n>`.
pub const DEFAULT_SVG_MAX_DEPTH: u32 = 128;

/// Max `<!ENTITY>` declarations tolerated in an SVG DTD before reject.
///
/// The billion-laughs class of attack needs only a handful of nested entity
/// definitions, so the honest ceiling is zero-ish. Operator override:
/// `config set svg_max_entities <n>`.
pub const DEFAULT_SVG_MAX_ENTITIES: u32 = 0;

/// Max animation frames decoded from a GIF before reject.
///
/// Operator override: `config set gif_max_frames <n>`.
pub const DEFAULT_GIF_MAX_FRAMES: u32 = 2_000;

/// Max bytes accepted for an HLS or DASH manifest body.
///
/// Manifests are text; a multi-megabyte one is a bug or an attack.
/// Operator override: `config set manifest_max_bytes <n>`.
pub const DEFAULT_MANIFEST_MAX_BYTES: u64 = 8_000_000;

/// Max variant / representation entries emitted from one manifest envelope.
///
/// Operator override: `config set manifest_max_variants <n>`.
pub const DEFAULT_MANIFEST_MAX_VARIANTS: u32 = 500;

/// Max bytes accepted for local video input materialization (stdin / pre-check).
///
/// Operator override: XDG `config set video_max_input_bytes <n>`.
pub const DEFAULT_VIDEO_MAX_INPUT_BYTES: u64 = 512_000_000;

/// Max HTTP body bytes for `video download`.
///
/// Operator override: XDG `config set video_download_max_bytes <n>`.
pub const DEFAULT_VIDEO_DOWNLOAD_MAX_BYTES: u64 = DEFAULT_VIDEO_MAX_INPUT_BYTES;

/// Default output container for `video convert` when `--format` is omitted.
///
/// Operator override: XDG `config set video_default_container <mp4|webm|mkv|mov|avi|m4v>`.
pub const DEFAULT_VIDEO_CONTAINER: &str = "mp4";

/// Default CRF for lossy video re-encode (ffmpeg scale; lower = higher quality).
///
/// Operator override: XDG `config set video_default_crf <n>`.
pub const DEFAULT_VIDEO_CRF: u8 = 23;

/// Default audio bitrate for `video to-mp3`.
///
/// Operator override: XDG `config set video_default_audio_bitrate <rate>`.
pub const DEFAULT_VIDEO_AUDIO_BITRATE: &str = "192k";

/// Minimum header bytes to probe video container magic.
pub const VIDEO_MAGIC_PROBE_BYTES: usize = 64;

/// Max bytes accepted for local audio input materialization (stdin / pre-check).
///
/// Operator override: XDG `config set audio_max_input_bytes <n>`.
pub const DEFAULT_AUDIO_MAX_INPUT_BYTES: u64 = 256_000_000;

/// Max HTTP body bytes for `audio download`.
///
/// Operator override: XDG `config set audio_download_max_bytes <n>`.
pub const DEFAULT_AUDIO_DOWNLOAD_MAX_BYTES: u64 = DEFAULT_AUDIO_MAX_INPUT_BYTES;

/// Default output format for `audio convert` when `--format` is omitted.
///
/// Operator override: XDG `config set audio_default_format <mp3|m4a|ogg|opus|flac|wav|aac>`.
pub const DEFAULT_AUDIO_FORMAT: &str = "mp3";

/// Default audio bitrate for lossy `audio convert` / trim re-encode.
///
/// Operator override: XDG `config set audio_default_bitrate <rate>`.
pub const DEFAULT_AUDIO_BITRATE: &str = "192k";

/// Default libvorbis quality (`-q:a`) for `audio convert --format ogg`.
///
/// Prefer quality mode over `-b:a` so low sample-rate inputs (e.g. 8 kHz) encode reliably.
pub const DEFAULT_VORBIS_QUALITY: &str = "4";

/// Minimum header bytes to probe audio container magic.
pub const AUDIO_MAGIC_PROBE_BYTES: usize = 64;

const _: () = assert!(DEFAULT_IMAGE_MAX_INPUT_BYTES > 0);
const _: () = assert!(DEFAULT_VIDEO_MAX_INPUT_BYTES > 0);
const _: () = assert!(DEFAULT_VIDEO_DOWNLOAD_MAX_BYTES > 0);
const _: () = assert!(DEFAULT_VIDEO_CRF >= 1 && DEFAULT_VIDEO_CRF <= 51);
const _: () = assert!(!DEFAULT_VIDEO_CONTAINER.is_empty());
const _: () = assert!(!DEFAULT_VIDEO_AUDIO_BITRATE.is_empty());
const _: () = assert!(VIDEO_MAGIC_PROBE_BYTES >= 12);
const _: () = assert!(DEFAULT_AUDIO_MAX_INPUT_BYTES > 0);
const _: () = assert!(DEFAULT_AUDIO_DOWNLOAD_MAX_BYTES > 0);
const _: () = assert!(!DEFAULT_AUDIO_FORMAT.is_empty());
const _: () = assert!(!DEFAULT_AUDIO_BITRATE.is_empty());
const _: () = assert!(!DEFAULT_VORBIS_QUALITY.is_empty());
const _: () = assert!(AUDIO_MAGIC_PROBE_BYTES >= 12);
const _: () = assert!(DEFAULT_IMAGE_MAX_PIXELS > 0);
const _: () = assert!(DEFAULT_IMAGE_QUALITY >= 1 && DEFAULT_IMAGE_QUALITY <= 100);
const _: () = assert!(!DEFAULT_IMAGE_FORMAT.is_empty());
const _: () = assert!(DEFAULT_IMAGE_DOWNLOAD_MAX_BYTES > 0);
const _: () = assert!(IMAGE_MAGIC_PROBE_BYTES >= 12);
const _: () = assert!(DEFAULT_IMAGE_AVIF_SPEED >= 1 && DEFAULT_IMAGE_AVIF_SPEED <= 10);
const _: () = assert!(DEFAULT_SVG_MAX_BYTES > 0);
const _: () = assert!(DEFAULT_SVG_MAX_BYTES <= DEFAULT_IMAGE_MAX_INPUT_BYTES);
const _: () = assert!(DEFAULT_SVG_MAX_DEPTH > 0);
const _: () = assert!(DEFAULT_GIF_MAX_FRAMES > 0);
const _: () = assert!(DEFAULT_MANIFEST_MAX_BYTES > 0);
const _: () = assert!(DEFAULT_MANIFEST_MAX_VARIANTS > 0);
