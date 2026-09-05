// SPDX-License-Identifier: MIT OR Apache-2.0
//! Image / video XDG resolve helpers (split from resolve.rs for GAP-051).

use super::config_io::load_config;

/// Max bytes for local image decode / convert / resize input.
pub fn resolve_image_max_input_bytes() -> usize {
    load_config()
        .ok()
        .and_then(|c| c.image_max_input_bytes)
        .filter(|&n| n > 0)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(crate::constants::DEFAULT_IMAGE_MAX_INPUT_BYTES as usize)
}

/// Max decoded pixel count for image ops.
pub fn resolve_image_max_pixels() -> u64 {
    load_config()
        .ok()
        .and_then(|c| c.image_max_pixels)
        .filter(|&n| n > 0)
        .unwrap_or(crate::constants::DEFAULT_IMAGE_MAX_PIXELS)
}

/// Default `image convert` format (`png`|`jpeg`|`webp`|`gif`).
pub fn resolve_image_default_format() -> String {
    load_config()
        .ok()
        .and_then(|c| c.image_default_format)
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| matches!(s.as_str(), "png" | "jpeg" | "jpg" | "webp" | "gif"))
        .map(|s| if s == "jpg" { "jpeg".into() } else { s })
        .unwrap_or_else(|| crate::constants::DEFAULT_IMAGE_FORMAT.to_string())
}

/// Default lossy quality for image convert/resize (1..=100).
pub fn resolve_image_default_quality() -> u8 {
    load_config()
        .ok()
        .and_then(|c| c.image_default_quality)
        .filter(|&n| (1..=100).contains(&n))
        .unwrap_or(crate::constants::DEFAULT_IMAGE_QUALITY)
}

/// Max HTTP body bytes for `image download`.
pub fn resolve_image_download_max_bytes() -> usize {
    load_config()
        .ok()
        .and_then(|c| c.image_download_max_bytes)
        .filter(|&n| n > 0)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or_else(|| {
            usize::try_from(crate::constants::DEFAULT_IMAGE_DOWNLOAD_MAX_BYTES)
                .unwrap_or(32_000_000)
        })
}

/// AVIF encoder speed 1..=10 (1 slowest / best quality).
pub fn resolve_image_avif_speed() -> u8 {
    load_config()
        .ok()
        .and_then(|c| c.image_avif_speed)
        .filter(|&n| (1..=10).contains(&n))
        .unwrap_or(crate::constants::DEFAULT_IMAGE_AVIF_SPEED)
}

/// Max bytes accepted for an SVG source before rasterisation.
pub fn resolve_svg_max_bytes() -> usize {
    load_config()
        .ok()
        .and_then(|c| c.svg_max_bytes)
        .filter(|&n| n > 0)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(crate::constants::DEFAULT_SVG_MAX_BYTES as usize)
}

/// Max XML nesting depth accepted in an SVG source.
pub fn resolve_svg_max_depth() -> u32 {
    load_config()
        .ok()
        .and_then(|c| c.svg_max_depth)
        .filter(|&n| n > 0)
        .unwrap_or(crate::constants::DEFAULT_SVG_MAX_DEPTH)
}

/// Max `<!ENTITY>` declarations tolerated in an SVG DTD (0 rejects any).
///
/// Unlike the other knobs this one accepts `0`, because zero is the safe value.
pub fn resolve_svg_max_entities() -> u32 {
    load_config()
        .ok()
        .and_then(|c| c.svg_max_entities)
        .unwrap_or(crate::constants::DEFAULT_SVG_MAX_ENTITIES)
}

/// Max animation frames decoded from a GIF.
pub fn resolve_gif_max_frames() -> u32 {
    load_config()
        .ok()
        .and_then(|c| c.gif_max_frames)
        .filter(|&n| n > 0)
        .unwrap_or(crate::constants::DEFAULT_GIF_MAX_FRAMES)
}

/// Max bytes accepted for an HLS / DASH manifest body.
pub fn resolve_manifest_max_bytes() -> usize {
    load_config()
        .ok()
        .and_then(|c| c.manifest_max_bytes)
        .filter(|&n| n > 0)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(crate::constants::DEFAULT_MANIFEST_MAX_BYTES as usize)
}

/// Max variant / representation entries emitted per manifest envelope.
///
/// The stored knob is a `u32`, so the cast to `usize` is lossless on every
/// target this crate builds for, where a pointer is at least 32 bits wide. The
/// sibling byte knobs above store a `u64` and cannot do this — they narrow
/// through `usize::try_from` because a 32-bit target would truncate them.
pub fn resolve_manifest_max_variants() -> usize {
    load_config()
        .ok()
        .and_then(|c| c.manifest_max_variants)
        .filter(|&n| n > 0)
        .map_or(
            crate::constants::DEFAULT_MANIFEST_MAX_VARIANTS as usize,
            |n| n as usize,
        )
}

/// Max bytes for video stdin materialization / path pre-check.
pub fn resolve_video_max_input_bytes() -> usize {
    load_config()
        .ok()
        .and_then(|c| c.video_max_input_bytes)
        .filter(|&n| n > 0)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or_else(|| {
            usize::try_from(crate::constants::DEFAULT_VIDEO_MAX_INPUT_BYTES).unwrap_or(512_000_000)
        })
}

/// Max HTTP body bytes for `video download`.
pub fn resolve_video_download_max_bytes() -> usize {
    load_config()
        .ok()
        .and_then(|c| c.video_download_max_bytes)
        .filter(|&n| n > 0)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or_else(|| {
            usize::try_from(crate::constants::DEFAULT_VIDEO_DOWNLOAD_MAX_BYTES)
                .unwrap_or(512_000_000)
        })
}

/// Default `video convert` container.
pub fn resolve_video_default_container() -> String {
    load_config()
        .ok()
        .and_then(|c| c.video_default_container)
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| matches!(s.as_str(), "mp4" | "webm" | "mkv" | "mov" | "avi" | "m4v"))
        .unwrap_or_else(|| crate::constants::DEFAULT_VIDEO_CONTAINER.to_string())
}

/// Default CRF for lossy video re-encode (1..=51).
pub fn resolve_video_default_crf() -> u8 {
    load_config()
        .ok()
        .and_then(|c| c.video_default_crf)
        .filter(|&n| (1..=51).contains(&n))
        .unwrap_or(crate::constants::DEFAULT_VIDEO_CRF)
}

/// Default audio bitrate for `video to-mp3`.
pub fn resolve_video_default_audio_bitrate() -> String {
    load_config()
        .ok()
        .and_then(|c| c.video_default_audio_bitrate)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| crate::constants::DEFAULT_VIDEO_AUDIO_BITRATE.to_string())
}

/// Max bytes for audio stdin materialization / path pre-check.
pub fn resolve_audio_max_input_bytes() -> usize {
    load_config()
        .ok()
        .and_then(|c| c.audio_max_input_bytes)
        .filter(|&n| n > 0)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or_else(|| {
            usize::try_from(crate::constants::DEFAULT_AUDIO_MAX_INPUT_BYTES).unwrap_or(256_000_000)
        })
}

/// Max HTTP body bytes for `audio download`.
pub fn resolve_audio_download_max_bytes() -> usize {
    load_config()
        .ok()
        .and_then(|c| c.audio_download_max_bytes)
        .filter(|&n| n > 0)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or_else(|| {
            usize::try_from(crate::constants::DEFAULT_AUDIO_DOWNLOAD_MAX_BYTES)
                .unwrap_or(256_000_000)
        })
}

/// Default `audio convert` format.
pub fn resolve_audio_default_format() -> String {
    load_config()
        .ok()
        .and_then(|c| c.audio_default_format)
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| {
            matches!(
                s.as_str(),
                "mp3" | "m4a" | "ogg" | "opus" | "flac" | "wav" | "aac"
            )
        })
        .unwrap_or_else(|| crate::constants::DEFAULT_AUDIO_FORMAT.to_string())
}

/// Default bitrate for lossy audio encode.
pub fn resolve_audio_default_bitrate() -> String {
    load_config()
        .ok()
        .and_then(|c| c.audio_default_bitrate)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| crate::constants::DEFAULT_AUDIO_BITRATE.to_string())
}
