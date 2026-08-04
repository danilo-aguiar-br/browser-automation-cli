// SPDX-License-Identifier: MIT OR Apache-2.0
//! Diagnosing a `video download` aimed at something that is not a media file.
//!
//! # Why this exists
//!
//! `video download` fetches a **direct media URL**. Point it at a site player
//! page and the body is HTML, so the magic probe fails with "not a supported
//! video container" — true, but useless: the agent cannot tell whether it typed
//! the URL wrong or asked for a capability this build refuses to have.
//!
//! # The rule being cited
//!
//! Site-player extraction — the yt-dlp class of feature — is **rejected by
//! rule**, not deferred. It means shipping and maintaining per-site signature
//! scrapers that break whenever a provider rotates its player, and it exists to
//! defeat the access controls those providers put in place. That is a permanent
//! product boundary, so the error says so instead of implying a future release
//! will cover it.
//!
//! A manifest URL is a different case: HLS and DASH *are* supported, just by
//! `video_local::manifest` rather than by the byte-for-byte downloader, so that
//! diagnosis points at the right tool.

use crate::error::{CliError, ErrorKind};

/// What a non-media response body turned out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonMediaBody {
    /// An HTML document — almost always a site player or a consent wall.
    HtmlPage,
    /// An HLS playlist or DASH MPD: supported, but by the manifest parser.
    Manifest,
}

/// Classify a downloaded body that failed container detection.
///
/// Returns `None` when the body is neither HTML nor a manifest, so the caller
/// keeps its existing generic magic error.
#[must_use]
pub fn classify_non_media(bytes: &[u8]) -> Option<NonMediaBody> {
    if crate::video_local::detect_manifest_kind(bytes).is_some() {
        return Some(NonMediaBody::Manifest);
    }
    // Only the head matters: an HTML document declares itself in the first
    // bytes, and scanning a capped body further would just cost time.
    let head = &bytes[..bytes.len().min(1024)];
    let lower = String::from_utf8_lossy(head)
        .trim_start()
        .to_ascii_lowercase();
    if lower.starts_with("<!doctype html")
        || lower.starts_with("<html")
        || lower.starts_with("<?xml") && lower.contains("<html")
    {
        return Some(NonMediaBody::HtmlPage);
    }
    None
}

/// Build the agent-facing error for a non-media body.
pub fn non_media_error(kind: NonMediaBody, url: &str) -> CliError {
    match kind {
        NonMediaBody::HtmlPage => CliError::with_suggestion(
            ErrorKind::Data,
            format!(
                "{url} returned an HTML page, not a media file. Extracting a stream from a \
                 site player is rejected by rule: it requires per-site scrapers that break on \
                 every player change and exist to bypass provider access control. Pass the \
                 direct media URL instead."
            ),
            crate::i18n::suggestion_key("video_site_extraction_rejected", None),
        ),
        NonMediaBody::Manifest => CliError::with_suggestion(
            ErrorKind::Data,
            format!(
                "{url} returned an HLS/DASH manifest, not a self-contained media file. \
                 Manifests are parsed, not downloaded as one blob — this build describes \
                 variants and leaves segment fetching to an explicit step."
            ),
            crate::i18n::suggestion_key("video_manifest_not_a_file", None),
        ),
    }
}
