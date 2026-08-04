// SPDX-License-Identifier: MIT OR Apache-2.0
//! Magic-byte video container detection (never trust file extensions).

use crate::constants::VIDEO_MAGIC_PROBE_BYTES;
use crate::error::{CliError, ErrorKind};

/// Containers this pipeline can identify from headers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedContainer {
    /// ISO BMFF brand suggests MP4 (`isom`/`mp41`/`mp42`/…).
    Mp4,
    /// QuickTime brand `qt  `.
    Mov,
    /// Apple M4V brand.
    M4v,
    /// ISO BMFF with unrecognized brand (still ftyp).
    IsoBmffUnknown,
    /// RIFF AVI (`RIFF….AVI `).
    Avi,
    /// Matroska / WebM EBML header (disambiguate with extension or probe).
    MatroskaOrWebm,
    /// MPEG Program Stream pack start.
    MpegPs,
    /// ASF / WMV GUID header.
    Asf,
}

impl DetectedContainer {
    /// Stable lowercase wire name for envelopes.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Mov => "mov",
            Self::M4v => "m4v",
            Self::IsoBmffUnknown => "isom",
            Self::Avi => "avi",
            Self::MatroskaOrWebm => "mkv_or_webm",
            Self::MpegPs => "mpeg",
            Self::Asf => "wmv",
        }
    }

    /// True when this build treats the container as a supported video envelope.
    #[must_use]
    pub fn is_video_container(self) -> bool {
        true
    }
}

/// Probe the first bytes of a media file for container magic.
pub fn detect_container(bytes: &[u8]) -> Result<DetectedContainer, CliError> {
    if bytes.len() < 4 {
        return Err(CliError::with_suggestion(
            ErrorKind::Data,
            format!(
                "media too short for magic probe ({} bytes; need ≥ {VIDEO_MAGIC_PROBE_BYTES})",
                bytes.len()
            ),
            crate::i18n::suggestion_key("video_magic_invalid", None),
        ));
    }

    // AVI: RIFF....AVI
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"AVI " {
        return Ok(DetectedContainer::Avi);
    }

    // EBML (MKV/WebM)
    if bytes.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
        return Ok(DetectedContainer::MatroskaOrWebm);
    }

    // MPEG-PS pack start
    if bytes.starts_with(&[0x00, 0x00, 0x01, 0xBA]) {
        return Ok(DetectedContainer::MpegPs);
    }

    // ASF header object GUID (partial)
    if bytes.len() >= 8 && bytes.starts_with(&[0x30, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11]) {
        return Ok(DetectedContainer::Asf);
    }

    // ISO BMFF: ....ftyp
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        let brand = &bytes[8..12];
        // Image HEIF brands are not video containers for this pipeline.
        if brand == b"avif"
            || brand == b"avis"
            || brand == b"heic"
            || brand == b"heix"
            || brand == b"mif1"
            || brand == b"heif"
        {
            return Err(CliError::with_suggestion(
                ErrorKind::Data,
                format!(
                    "HEIF/AVIF brand '{}' is not a video container for video ops",
                    String::from_utf8_lossy(brand)
                ),
                crate::i18n::suggestion_key("video_magic_invalid", None),
            ));
        }
        if brand == b"qt  " {
            return Ok(DetectedContainer::Mov);
        }
        if brand == b"M4V " || brand == b"M4VH" || brand == b"M4VP" {
            return Ok(DetectedContainer::M4v);
        }
        if brand == b"isom"
            || brand == b"iso2"
            || brand == b"iso3"
            || brand == b"iso4"
            || brand == b"iso5"
            || brand == b"iso6"
            || brand == b"mp41"
            || brand == b"mp42"
            || brand == b"mp71"
            || brand == b"avc1"
            || brand == b"dash"
            || brand == b"msdh"
            || brand == b"mmp4"
            || brand == b"3gp4"
            || brand == b"3gp5"
            || brand == b"3gp6"
        {
            return Ok(DetectedContainer::Mp4);
        }
        return Ok(DetectedContainer::IsoBmffUnknown);
    }

    Err(CliError::with_suggestion(
        ErrorKind::Data,
        "unrecognized media magic (not a known video container)",
        crate::i18n::suggestion_key("video_magic_invalid", None),
    ))
}

/// Map filesystem path I/O errors to agent-native CliError (suggestion on permission).
///
/// `op` is a short English verb for the message (`open`, `stat`, `mkdir`, `rename`, …).
pub(crate) fn io_path_err(path: &std::path::Path, op: &str, e: &std::io::Error) -> CliError {
    let msg = format!("video {op} {}: {e}", path.display());
    let lower = e.to_string().to_ascii_lowercase();
    if e.kind() == std::io::ErrorKind::PermissionDenied || lower.contains("permission") {
        CliError::with_suggestion(
            ErrorKind::Io,
            msg,
            crate::i18n::suggestion_key("ffmpeg_io_failed", None),
        )
    } else {
        CliError::new(ErrorKind::Io, msg)
    }
}

/// Map filesystem open/read errors (`op = "open"`).
pub(crate) fn io_open_err(path: &std::path::Path, e: &std::io::Error) -> CliError {
    io_path_err(path, "open", e)
}

/// Read up to [`VIDEO_MAGIC_PROBE_BYTES`] from a path without loading the whole file.
pub fn probe_path_magic(path: &std::path::Path) -> Result<DetectedContainer, CliError> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).map_err(|e| io_open_err(path, &e))?;
    let mut buf = vec![0u8; VIDEO_MAGIC_PROBE_BYTES];
    let n = f.read(&mut buf).map_err(|e| io_open_err(path, &e))?;
    buf.truncate(n);
    detect_container(&buf)
}
