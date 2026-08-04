// SPDX-License-Identifier: MIT OR Apache-2.0
//! Magic-byte audio container detection (never trust file extensions).

use crate::constants::AUDIO_MAGIC_PROBE_BYTES;
use crate::error::{CliError, ErrorKind};

/// Containers this pipeline can identify from headers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedAudio {
    /// RIFF WAVE.
    Wav,
    /// OGG container (Vorbis/Opus/FLAC — codec via probe).
    Ogg,
    /// Bare FLAC.
    Flac,
    /// MP3 (ID3 or frame sync).
    Mp3,
    /// ISO BMFF audio (M4A/MP4 brands).
    M4a,
    /// AIFF / AIFC.
    Aiff,
    /// AAC ADTS stream.
    Adts,
}

impl DetectedAudio {
    /// Stable lowercase wire name for envelopes.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wav => "wav",
            Self::Ogg => "ogg",
            Self::Flac => "flac",
            Self::Mp3 => "mp3",
            Self::M4a => "m4a",
            Self::Aiff => "aiff",
            Self::Adts => "aac",
        }
    }

    /// True when treated as a supported audio envelope for fail-closed download.
    #[must_use]
    pub fn is_audio_container(self) -> bool {
        true
    }
}

/// Probe the first bytes of a media file for audio container magic.
pub fn detect_container(bytes: &[u8]) -> Result<DetectedAudio, CliError> {
    if bytes.len() < 4 {
        return Err(CliError::with_suggestion(
            ErrorKind::Data,
            format!(
                "media too short for audio magic probe ({} bytes; need ≥ {AUDIO_MAGIC_PROBE_BYTES})",
                bytes.len()
            ),
            crate::i18n::suggestion_key("audio_magic_invalid", None),
        ));
    }

    // WAV: RIFF....WAVE (not AVI)
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
        return Ok(DetectedAudio::Wav);
    }

    // OGG
    if bytes.starts_with(b"OggS") {
        return Ok(DetectedAudio::Ogg);
    }

    // FLAC
    if bytes.starts_with(b"fLaC") {
        return Ok(DetectedAudio::Flac);
    }

    // MP3 with ID3
    if bytes.starts_with(b"ID3") {
        return Ok(DetectedAudio::Mp3);
    }

    // AIFF
    if bytes.len() >= 12 && &bytes[0..4] == b"FORM" {
        let form = &bytes[8..12];
        if form == b"AIFF" || form == b"AIFC" {
            return Ok(DetectedAudio::Aiff);
        }
    }

    // ISO BMFF ftyp (M4A / MP4 audio brands)
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        let brand = &bytes[8..12];
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
                    "HEIF/AVIF brand '{}' is not an audio container",
                    String::from_utf8_lossy(brand)
                ),
                crate::i18n::suggestion_key("audio_magic_invalid", None),
            ));
        }
        // Audio-friendly and common ISO brands (codec confirmed via ffprobe).
        if brand == b"M4A "
            || brand == b"M4B "
            || brand == b"mp41"
            || brand == b"mp42"
            || brand == b"isom"
            || brand == b"iso2"
            || brand == b"iso5"
            || brand == b"iso6"
            || brand == b"dash"
            || brand == b"MSNV"
            || brand == b"M4P "
        {
            return Ok(DetectedAudio::M4a);
        }
        // Other ftyp may still hold audio tracks (e.g. mp4 with audio only).
        return Ok(DetectedAudio::M4a);
    }

    // MP3 frame sync (11 ones): FF FB / FF FA / FF F3 / FF F2 common
    if bytes.len() >= 2 && bytes[0] == 0xFF && (bytes[1] & 0xE0) == 0xE0 {
        let layer = (bytes[1] >> 1) & 0x03;
        // Layer III = 0b01 in MPEG header bits
        if layer == 0b01 {
            return Ok(DetectedAudio::Mp3);
        }
    }

    // AAC ADTS sync
    if bytes.len() >= 2 && bytes[0] == 0xFF && (bytes[1] == 0xF1 || bytes[1] == 0xF9) {
        return Ok(DetectedAudio::Adts);
    }

    Err(CliError::with_suggestion(
        ErrorKind::Data,
        "unrecognized media magic (not a known audio container)",
        crate::i18n::suggestion_key("audio_magic_invalid", None),
    ))
}

/// Map filesystem path I/O errors to agent-native CliError (suggestion on permission).
pub(crate) fn io_path_err(path: &std::path::Path, op: &str, e: &std::io::Error) -> CliError {
    let msg = format!("audio {op} {}: {e}", path.display());
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

/// Read up to [`AUDIO_MAGIC_PROBE_BYTES`] from a path without loading the whole file.
pub fn probe_path_magic(path: &std::path::Path) -> Result<DetectedAudio, CliError> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).map_err(|e| io_open_err(path, &e))?;
    let mut buf = vec![0u8; AUDIO_MAGIC_PROBE_BYTES];
    let n = f.read(&mut buf).map_err(|e| io_open_err(path, &e))?;
    buf.truncate(n);
    detect_container(&buf)
}
