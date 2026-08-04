// SPDX-License-Identifier: MIT OR Apache-2.0
//! HLS and DASH manifest parsing (GAP-VID-WAVE-C), behind `media-manifest`.
//!
//! # Scope
//!
//! Parse and describe only. This module never downloads a segment, never
//! concatenates a stream, and never shells out to a muxer. It answers "what
//! renditions exist and where do they live", which is the question an agent
//! actually has, and leaves fetching to an explicit later step.
//!
//! # Clean stdout
//!
//! Envelopes carry variant descriptors and absolute URLs — never manifest text
//! and never media bytes. The variant list is capped by `manifest_max_variants`
//! so a pathological manifest cannot flood an agent's context.

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};

/// Which manifest dialect a body turned out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestKind {
    /// HLS master playlist (`#EXTM3U` with `EXT-X-STREAM-INF`).
    HlsMaster,
    /// HLS media playlist (`#EXTM3U` with segments).
    HlsMedia,
    /// MPEG-DASH media presentation description (XML).
    Dash,
}

impl ManifestKind {
    /// Stable lowercase wire name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HlsMaster => "hls_master",
            Self::HlsMedia => "hls_media",
            Self::Dash => "dash",
        }
    }
}

/// True when this build can parse manifests.
#[must_use]
pub const fn available() -> bool {
    cfg!(feature = "media-manifest")
}

#[cfg(not(feature = "media-manifest"))]
fn feature_off() -> CliError {
    CliError::with_suggestion(
        ErrorKind::Config,
        "manifest parsing requires the `media-manifest` Cargo feature, which is off in this build",
        crate::i18n::suggestion_key("image_feature_disabled", None),
    )
}

/// Cheap dialect probe that does not need the parser features.
#[must_use]
pub fn detect_kind(body: &[u8]) -> Option<ManifestKind> {
    let head = &body[..body.len().min(4096)];
    let text = String::from_utf8_lossy(head);
    let trimmed = text.trim_start();
    if trimmed.starts_with("#EXTM3U") {
        return Some(if text.contains("EXT-X-STREAM-INF") {
            ManifestKind::HlsMaster
        } else {
            ManifestKind::HlsMedia
        });
    }
    // An MPD may be namespaced (`<dash:MPD`), so the root element is matched by
    // suffix rather than by an exact tag name.
    if (trimmed.starts_with("<?xml") || trimmed.starts_with("<MPD"))
        && (text.contains("<MPD") || text.contains(":MPD"))
    {
        return Some(ManifestKind::Dash);
    }
    None
}

/// Reject a manifest body that exceeds `manifest_max_bytes`.
fn check_size(body: &[u8]) -> Result<(), CliError> {
    let max = crate::xdg::resolve_manifest_max_bytes();
    if body.len() > max {
        return Err(CliError::with_suggestion(
            ErrorKind::Data,
            format!(
                "manifest {} bytes exceeds manifest_max_bytes {max}",
                body.len()
            ),
            crate::i18n::suggestion_key("image_too_large", None),
        ));
    }
    Ok(())
}

/// Resolve a possibly-relative manifest URI against the manifest's own URL.
fn absolutize(base: Option<&str>, uri: &str) -> String {
    let Some(base) = base else {
        return uri.to_string();
    };
    url::Url::parse(base)
        .ok()
        .and_then(|b| b.join(uri).ok())
        .map(|u| u.to_string())
        .unwrap_or_else(|| uri.to_string())
}

/// Parse a manifest body into an agent envelope.
#[cfg(not(feature = "media-manifest"))]
pub fn parse(_body: &[u8], _base_url: Option<&str>) -> Result<Value, CliError> {
    Err(feature_off())
}

/// Parse a manifest body into an agent envelope.
///
/// `base_url` is the URL the manifest was fetched from; when supplied, every
/// emitted URI is absolute, which is what a caller needs to act on a variant.
#[cfg(feature = "media-manifest")]
pub fn parse(body: &[u8], base_url: Option<&str>) -> Result<Value, CliError> {
    check_size(body)?;
    let kind = detect_kind(body).ok_or_else(|| {
        CliError::with_suggestion(
            ErrorKind::Data,
            "body is neither an HLS playlist nor a DASH MPD",
            crate::i18n::suggestion_key("use_listed_value", None),
        )
    })?;
    match kind {
        ManifestKind::HlsMaster | ManifestKind::HlsMedia => parse_hls(body, base_url),
        ManifestKind::Dash => parse_dash(body, base_url),
    }
}

#[cfg(feature = "media-manifest")]
fn parse_hls(body: &[u8], base_url: Option<&str>) -> Result<Value, CliError> {
    let max_variants = crate::xdg::resolve_manifest_max_variants();
    let playlist = m3u8_rs::parse_playlist_res(body)
        .map_err(|e| CliError::new(ErrorKind::Data, format!("hls parse: {e}")))?;

    match playlist {
        m3u8_rs::Playlist::MasterPlaylist(master) => {
            let total = master.variants.len();
            let variants: Vec<Value> = master
                .variants
                .iter()
                .take(max_variants)
                .enumerate()
                .map(|(i, v)| {
                    json!({
                        "index": i,
                        "uri": absolutize(base_url, &v.uri),
                        "bandwidth": v.bandwidth,
                        "average_bandwidth": v.average_bandwidth,
                        "codecs": v.codecs,
                        "resolution": v.resolution.map(|r| format!("{}x{}", r.width, r.height)),
                        "frame_rate": v.frame_rate,
                    })
                })
                .collect();
            let audio: Vec<Value> = master
                .alternatives
                .iter()
                .take(max_variants)
                .map(|a| {
                    json!({
                        "type": format!("{:?}", a.media_type).to_ascii_lowercase(),
                        "group_id": a.group_id,
                        "name": a.name,
                        "language": a.language,
                        "default": a.default,
                        "uri": a.uri.as_deref().map(|u| absolutize(base_url, u)),
                    })
                })
                .collect();
            Ok(json!({
                "action": "manifest",
                "kind": ManifestKind::HlsMaster.as_str(),
                "engine": "m3u8-rs",
                "bytes": body.len(),
                "variant_count": total,
                "variants_truncated": total > max_variants,
                "variants": variants,
                "alternatives": audio,
                "independent_segments": master.independent_segments,
            }))
        }
        m3u8_rs::Playlist::MediaPlaylist(media) => {
            let total = media.segments.len();
            // Segment URIs are intentionally not listed: a long VOD playlist has
            // thousands, and dumping them defeats the clean-stdout contract.
            let duration: f32 = media.segments.iter().map(|s| s.duration).sum();
            Ok(json!({
                "action": "manifest",
                "kind": ManifestKind::HlsMedia.as_str(),
                "engine": "m3u8-rs",
                "bytes": body.len(),
                "segment_count": total,
                "target_duration": media.target_duration,
                "total_duration_secs": duration,
                "media_sequence": media.media_sequence,
                "end_list": media.end_list,
                "playlist_type": media.playlist_type.as_ref().map(|t| format!("{t:?}")),
                "first_segment_uri": media
                    .segments
                    .first()
                    .map(|s| absolutize(base_url, &s.uri)),
            }))
        }
    }
}

#[cfg(feature = "media-manifest")]
fn parse_dash(body: &[u8], base_url: Option<&str>) -> Result<Value, CliError> {
    let max_variants = crate::xdg::resolve_manifest_max_variants();
    let text = std::str::from_utf8(body)
        .map_err(|e| CliError::new(ErrorKind::Data, format!("dash mpd is not UTF-8: {e}")))?;
    let mpd = dash_mpd::parse(text)
        .map_err(|e| CliError::new(ErrorKind::Data, format!("dash parse: {e}")))?;

    let mut representations = Vec::new();
    let mut total = 0usize;
    for (p_idx, period) in mpd.periods.iter().enumerate() {
        for (a_idx, adaptation) in period.adaptations.iter().enumerate() {
            for rep in &adaptation.representations {
                total += 1;
                if representations.len() >= max_variants {
                    continue;
                }
                representations.push(json!({
                    "period": p_idx,
                    "adaptation_set": a_idx,
                    "id": rep.id,
                    "bandwidth": rep.bandwidth,
                    "width": rep.width,
                    "height": rep.height,
                    "codecs": rep.codecs.clone().or_else(|| adaptation.codecs.clone()),
                    "mime_type": rep.mimeType.clone().or_else(|| adaptation.mimeType.clone()),
                    "frame_rate": rep.frameRate.clone(),
                    "base_url": rep
                        .BaseURL
                        .first()
                        .map(|b| absolutize(base_url, &b.base)),
                }));
            }
        }
    }
    Ok(json!({
        "action": "manifest",
        "kind": ManifestKind::Dash.as_str(),
        "engine": "dash-mpd",
        "bytes": body.len(),
        "period_count": mpd.periods.len(),
        "representation_count": total,
        "representations_truncated": total > max_variants,
        "representations": representations,
        "mpd_type": mpd.mpdtype,
        "min_buffer_time_secs": mpd.minBufferTime.map(|d| d.as_secs_f64()),
    }))
}
