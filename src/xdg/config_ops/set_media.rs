// SPDX-License-Identifier: MIT OR Apache-2.0
//! Image / video `config set` arms (split for GAP-051).

use super::super::config_model::ProductConfig;
use super::validate::{
    parse_positive_u32, parse_positive_u64, parse_quality_u8, parse_range_u8, parse_u32,
};
use crate::error::{CliError, ErrorKind};

/// Apply image/video keys. Returns `true` when `key` was handled.
///
/// # Errors
///
/// [`ErrorKind::Usage`] when a size, pixel, quality or CRF value fails its
/// validator in [`super::validate`], or when `image_default_format`,
/// `video_default_container` or `audio_default_format` names a codec/container
/// outside the accepted set, or when a default bitrate is empty. Keys of another
/// family return `Ok(false)` rather than an error.
pub(super) fn apply_media_set(
    cfg: &mut ProductConfig,
    key: &str,
    value: &str,
) -> Result<bool, CliError> {
    match key {
        "image_max_input_bytes" => {
            cfg.image_max_input_bytes = Some(parse_positive_u64(value, "image_max_input_bytes")?);
        }
        "image_max_pixels" => {
            cfg.image_max_pixels = Some(parse_positive_u64(value, "image_max_pixels")?);
        }
        "image_default_format" => {
            let fmt = value.trim().to_ascii_lowercase();
            // `avif` is accepted only as an *output* format; the encoder lives
            // behind the `image-avif` Cargo feature and fails closed when off.
            if !matches!(
                fmt.as_str(),
                "png" | "jpeg" | "jpg" | "webp" | "gif" | "avif"
            ) {
                return Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    format!("invalid image_default_format: {value}"),
                    crate::i18n::suggestion_key("use_listed_value", None),
                ));
            }
            let normalized = if fmt == "jpg" {
                "jpeg".to_string()
            } else {
                fmt
            };
            cfg.image_default_format = Some(normalized);
        }
        "image_default_quality" => {
            cfg.image_default_quality = Some(parse_quality_u8(value, "image_default_quality")?);
        }
        "image_download_max_bytes" => {
            cfg.image_download_max_bytes =
                Some(parse_positive_u64(value, "image_download_max_bytes")?);
        }
        "image_avif_speed" => {
            cfg.image_avif_speed = Some(parse_range_u8(value, "image_avif_speed", 1, 10)?);
        }
        "svg_max_bytes" => {
            cfg.svg_max_bytes = Some(parse_positive_u64(value, "svg_max_bytes")?);
        }
        "svg_max_depth" => {
            cfg.svg_max_depth = Some(parse_positive_u32(value, "svg_max_depth")?);
        }
        "svg_max_entities" => {
            // 0 is the safe value and therefore legal, unlike the other ceilings.
            cfg.svg_max_entities = Some(parse_u32(value, "svg_max_entities")?);
        }
        "gif_max_frames" => {
            cfg.gif_max_frames = Some(parse_positive_u32(value, "gif_max_frames")?);
        }
        "manifest_max_bytes" => {
            cfg.manifest_max_bytes = Some(parse_positive_u64(value, "manifest_max_bytes")?);
        }
        "manifest_max_variants" => {
            cfg.manifest_max_variants = Some(parse_positive_u32(value, "manifest_max_variants")?);
        }
        "video_max_input_bytes" => {
            cfg.video_max_input_bytes = Some(parse_positive_u64(value, "video_max_input_bytes")?);
        }
        "video_download_max_bytes" => {
            cfg.video_download_max_bytes =
                Some(parse_positive_u64(value, "video_download_max_bytes")?);
        }
        "video_default_container" => {
            let v = value.trim().to_ascii_lowercase();
            if !matches!(v.as_str(), "mp4" | "webm" | "mkv" | "mov" | "avi" | "m4v") {
                return Err(CliError::new(
                    ErrorKind::Usage,
                    "video_default_container must be mp4|webm|mkv|mov|avi|m4v",
                ));
            }
            cfg.video_default_container = Some(v);
        }
        "video_default_crf" => {
            let n = parse_positive_u64(value, "video_default_crf")?;
            if !(1..=51).contains(&n) {
                return Err(CliError::new(
                    ErrorKind::Usage,
                    "video_default_crf must be 1..=51",
                ));
            }
            cfg.video_default_crf = Some(n as u8);
        }
        "video_default_audio_bitrate" => {
            let v = value.trim();
            if v.is_empty() {
                return Err(CliError::new(
                    ErrorKind::Usage,
                    "video_default_audio_bitrate must be non-empty (e.g. 192k)",
                ));
            }
            cfg.video_default_audio_bitrate = Some(v.to_string());
        }
        "audio_max_input_bytes" => {
            cfg.audio_max_input_bytes = Some(parse_positive_u64(value, "audio_max_input_bytes")?);
        }
        "audio_download_max_bytes" => {
            cfg.audio_download_max_bytes =
                Some(parse_positive_u64(value, "audio_download_max_bytes")?);
        }
        "audio_default_format" => {
            let v = value.trim().to_ascii_lowercase();
            if !matches!(
                v.as_str(),
                "mp3" | "m4a" | "ogg" | "opus" | "flac" | "wav" | "aac"
            ) {
                return Err(CliError::new(
                    ErrorKind::Usage,
                    "audio_default_format must be mp3|m4a|ogg|opus|flac|wav|aac",
                ));
            }
            cfg.audio_default_format = Some(v);
        }
        "audio_default_bitrate" => {
            let v = value.trim();
            if v.is_empty() {
                return Err(CliError::new(
                    ErrorKind::Usage,
                    "audio_default_bitrate must be non-empty (e.g. 192k)",
                ));
            }
            cfg.audio_default_bitrate = Some(v.to_string());
        }
        _ => return Ok(false),
    }
    Ok(true)
}
