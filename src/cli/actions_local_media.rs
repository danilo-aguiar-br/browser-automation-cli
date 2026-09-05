// SPDX-License-Identifier: MIT OR Apache-2.0
//! Local media pipelines that do not require Chrome (video, audio).

use clap::{Subcommand, ValueHint};

/// One-shot local video ops (no Chrome): info, download, convert, to-mp3, trim,
/// thumbnail, manifest.
#[derive(Debug, Clone, Subcommand)]
pub enum VideoAction {
    /// Probe container magic + streams (ffprobe when available; no media dump)
    Info {
        /// Video file path (omit with `--stdin`)
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
        /// Materialize stdin to a capped temp file
        #[arg(long, action = clap::ArgAction::SetTrue)]
        stdin: bool,
        /// Batch: one video path per line (`#` comments skipped). Mutually exclusive with `--path` and `--stdin`
        ///
        /// Emits one envelope holding a per-item result. A failing item is
        /// reported and the run continues, so `ok: true` means the batch
        /// produced SOMETHING, never that every item passed — read
        /// `error_count`. If NO item succeeds the run fails with exit 65.
        #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
        paths_file: Option<std::path::PathBuf>,
        /// Project envelope fields (CSV): path,container,streams,duration_secs,sha256,…; aliases: format→container|container_out, bytes|size→size_bytes|bytes_out, path→path|path_out, duration→duration_secs
        #[arg(long, value_name = "FIELDS")]
        select: Option<String>,
    },
    /// Download a direct media URL (SSRF + body cap + magic verify)
    Download {
        /// http(s) URL of the media file (not a site player page)
        #[arg(value_hint = ValueHint::Url)]
        url: String,
        /// Output path (default under XDG cache)
        #[arg(long, short = 'o', value_hint = ValueHint::FilePath)]
        out: Option<std::path::PathBuf>,
        /// Max body bytes (default: XDG video_download_max_bytes)
        #[arg(long)]
        max_bytes: Option<usize>,
        /// Fail closed unless body is a known video container magic (default)
        #[arg(long, default_value_t = true, action = clap::ArgAction::SetTrue)]
        require_video: bool,
        /// Allow non-video bodies (disables magic fail-closed)
        #[arg(long, action = clap::ArgAction::SetTrue)]
        allow_non_video: bool,
        /// Project envelope fields (CSV); aliases: format→container|container_out, bytes|size→size_bytes|bytes_out, path→path|path_out, duration→duration_secs
        #[arg(long, value_name = "FIELDS")]
        select: Option<String>,
    },
    /// Convert / remux container (path→path via optional ffmpeg; smart copy/re-encode)
    Convert {
        /// Input video path (omit with `--stdin`)
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
        /// Materialize stdin to a capped temp file
        #[arg(long, action = clap::ArgAction::SetTrue)]
        stdin: bool,
        /// Batch: one video path per line (`#` comments skipped). Mutually exclusive with `--path` and `--stdin`
        ///
        /// Each output is derived beside its input as `input.<target format>`;
        /// `--out` is refused here because one destination cannot serve N
        /// inputs. An item whose derived path already exists, or equals its own
        /// input, fails and the batch continues, so `ok: true` means the batch
        /// produced SOMETHING, never that every item passed — read
        /// `error_count`. If NO item succeeds the run fails with exit 65.
        #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
        paths_file: Option<std::path::PathBuf>,
        /// Output container: mp4 | webm | mkv | mov | avi | m4v
        #[arg(long)]
        format: Option<String>,
        /// Output path (default under XDG cache)
        #[arg(long, short = 'o', value_hint = ValueHint::FilePath)]
        out: Option<std::path::PathBuf>,
        /// Video codec: copy | h264 | hevc | vp9 | av1 | … (default: copy intent; auto re-encode if needed)
        #[arg(long)]
        video_codec: Option<String>,
        /// Audio codec: copy | aac | opus | mp3 | … (default: copy intent)
        #[arg(long)]
        audio_codec: Option<String>,
        /// CRF for re-encode (default: XDG video_default_crf)
        #[arg(long)]
        crf: Option<u8>,
        /// Disable moov-before-mdat for MP4/MOV/M4V (default: faststart applied)
        #[arg(long, action = clap::ArgAction::SetTrue)]
        no_faststart: bool,
        /// Strip container metadata (opt-in; default preserves)
        #[arg(long, action = clap::ArgAction::SetTrue)]
        strip_metadata: bool,
        /// Drop all audio tracks (opt-in; default keeps audio)
        #[arg(long, action = clap::ArgAction::SetTrue)]
        drop_audio: bool,
        /// Project envelope fields (CSV); aliases: format→container|container_out, bytes|size→size_bytes|bytes_out, path→path|path_out, duration→duration_secs
        #[arg(long, value_name = "FIELDS")]
        select: Option<String>,
    },
    /// Extract audio to MP3 (path→path via optional ffmpeg)
    ToMp3 {
        /// Input video path (omit with `--stdin`)
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
        /// Materialize stdin to a capped temp file
        #[arg(long, action = clap::ArgAction::SetTrue)]
        stdin: bool,
        /// Batch: one video path per line (`#` comments skipped). Mutually exclusive with `--path` and `--stdin`
        ///
        /// Each output is derived beside its input as `input.mp3`; `--out` is
        /// refused here because one destination cannot serve N inputs. An item
        /// whose derived path already exists fails and the batch continues, so
        /// `ok: true` means the batch produced SOMETHING, never that every item
        /// passed — read `error_count`. If NO item succeeds, exit 65.
        #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
        paths_file: Option<std::path::PathBuf>,
        /// Output path (default under XDG cache)
        #[arg(long, short = 'o', value_hint = ValueHint::FilePath)]
        out: Option<std::path::PathBuf>,
        /// Audio bitrate (default: XDG video_default_audio_bitrate)
        #[arg(long)]
        bitrate: Option<String>,
        /// Audio stream index (default 0)
        #[arg(long)]
        audio_stream: Option<u32>,
        /// Project envelope fields (CSV); aliases: format→container|container_out, bytes|size→size_bytes|bytes_out, path→path|path_out, duration→duration_secs
        #[arg(long, value_name = "FIELDS")]
        select: Option<String>,
    },
    /// Trim a time range (path→path via optional ffmpeg)
    Trim {
        /// Input video path (omit with `--stdin`)
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
        /// Materialize stdin to a capped temp file
        #[arg(long, action = clap::ArgAction::SetTrue)]
        stdin: bool,
        /// Batch: one video path per line (`#` comments skipped). Mutually exclusive with `--path` and `--stdin`
        ///
        /// Each output is derived beside its input as `input.<target container>`; `--out` is
        /// refused here because one destination cannot serve N inputs. An item
        /// whose derived path already exists, or equals its own input, fails and
        /// the batch continues, so `ok: true` means the batch produced
        /// SOMETHING, never that every item passed — read `error_count`. If NO
        /// item succeeds the run fails with exit 65.
        #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
        paths_file: Option<std::path::PathBuf>,
        /// Start time in seconds
        #[arg(long)]
        start: f64,
        /// Duration in seconds (mutually exclusive with `--to`)
        #[arg(long)]
        duration: Option<f64>,
        /// End time in seconds (exclusive end; use with `--start`)
        #[arg(long = "to")]
        to: Option<f64>,
        /// Output path (default under XDG cache)
        #[arg(long, short = 'o', value_hint = ValueHint::FilePath)]
        out: Option<std::path::PathBuf>,
        /// Output container override (default: inferred / XDG)
        #[arg(long)]
        format: Option<String>,
        /// Video codec intent (default: copy)
        #[arg(long)]
        video_codec: Option<String>,
        /// Audio codec intent (default: copy)
        #[arg(long)]
        audio_codec: Option<String>,
        /// Project envelope fields (CSV); aliases: format→container|container_out, bytes|size→size_bytes|bytes_out, path→path|path_out, duration→duration_secs
        #[arg(long, value_name = "FIELDS")]
        select: Option<String>,
    },
    /// Extract a single frame as an image path (png/jpg via `-o` extension)
    Thumbnail {
        /// Input video path (omit with `--stdin`)
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
        /// Materialize stdin to a capped temp file
        #[arg(long, action = clap::ArgAction::SetTrue)]
        stdin: bool,
        /// Batch: one video path per line (`#` comments skipped). Mutually exclusive with `--path` and `--stdin`
        ///
        /// Each output is derived beside its input as `input.png`; `--out` is
        /// refused here because one destination cannot serve N inputs. An item
        /// whose derived path already exists, or equals its own input, fails and
        /// the batch continues, so `ok: true` means the batch produced
        /// SOMETHING, never that every item passed — read `error_count`. If NO
        /// item succeeds the run fails with exit 65.
        #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
        paths_file: Option<std::path::PathBuf>,
        /// Timestamp in seconds (default 0)
        #[arg(long)]
        at: Option<f64>,
        /// Output image path (default under XDG cache as .png)
        #[arg(long, short = 'o', value_hint = ValueHint::FilePath)]
        out: Option<std::path::PathBuf>,
        /// Project envelope fields (CSV); aliases: format→container|container_out, bytes|size→size_bytes|bytes_out, path→path|path_out, duration→duration_secs
        #[arg(long, value_name = "FIELDS")]
        select: Option<String>,
    },
    /// Summarise an HLS (.m3u8) or DASH (.mpd) manifest without downloading media
    ///
    /// Structural only: it reads the playlist body and reports the variant /
    /// representation ladder. `video info` probes container magic and rejects a
    /// manifest, which is correct for a container probe and useless for a
    /// playlist — this action is the other half.
    Manifest {
        /// Manifest file path (omit with `--stdin`)
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
        /// Read the manifest body from stdin
        #[arg(long, action = clap::ArgAction::SetTrue)]
        stdin: bool,
        /// Batch: one manifest path per line (`#` comments skipped). Mutually exclusive with `--path` and `--stdin`
        ///
        /// Emits one envelope holding a per-item result. A failing item is
        /// reported and the run continues, so `ok: true` means the batch
        /// produced SOMETHING, never that every item passed — read
        /// `error_count`. If NO item succeeds the run fails with exit 65.
        #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
        paths_file: Option<std::path::PathBuf>,
        /// Manifest URL used to absolutise relative segment / representation URIs
        #[arg(long, value_name = "URL", value_hint = ValueHint::Url)]
        base_url: Option<String>,
        /// Project envelope fields (CSV), e.g. kind,variant_count,representations
        #[arg(long, value_name = "FIELDS")]
        select: Option<String>,
    },
}

/// One-shot local audio ops (no Chrome): info, download, convert, trim.
#[derive(Debug, Clone, Subcommand)]
pub enum AudioAction {
    /// Probe container magic + audio streams (ffprobe when available; no media dump)
    Info {
        /// Audio file path (omit with `--stdin`)
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
        /// Materialize stdin to a capped temp file
        #[arg(long, action = clap::ArgAction::SetTrue)]
        stdin: bool,
        /// Batch: one audio path per line (`#` comments skipped). Mutually exclusive with `--path` and `--stdin`
        ///
        /// Emits one envelope holding a per-item result. A failing item is
        /// reported and the run continues, so `ok: true` means the batch
        /// produced SOMETHING, never that every item passed — read
        /// `error_count`. If NO item succeeds the run fails with exit 65.
        #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
        paths_file: Option<std::path::PathBuf>,
        /// Project envelope fields (CSV): path,container,audio_codec,duration_secs,sha256,…; aliases: format→container|container_out, bytes|size→size_bytes|bytes_out, path→path|path_out, duration→duration_secs, codec→audio_codec
        #[arg(long, value_name = "FIELDS")]
        select: Option<String>,
    },
    /// Download a direct media URL (SSRF + body cap + magic verify)
    Download {
        /// http(s) URL of the media file (not a site player page)
        #[arg(value_hint = ValueHint::Url)]
        url: String,
        /// Output path (default under XDG cache)
        #[arg(long, short = 'o', value_hint = ValueHint::FilePath)]
        out: Option<std::path::PathBuf>,
        /// Max body bytes (default: XDG audio_download_max_bytes)
        #[arg(long)]
        max_bytes: Option<usize>,
        /// Fail closed unless body is a known audio container magic (default)
        #[arg(long, default_value_t = true, action = clap::ArgAction::SetTrue)]
        require_audio: bool,
        /// Allow non-audio bodies (disables magic fail-closed)
        #[arg(long, action = clap::ArgAction::SetTrue)]
        allow_non_audio: bool,
        /// Project envelope fields (CSV); aliases: format→container, bytes→bytes, path→path
        #[arg(long, value_name = "FIELDS")]
        select: Option<String>,
    },
    /// Convert / remux audio (path→path via optional ffmpeg; smart copy/re-encode; `-vn`)
    Convert {
        /// Input audio path (omit with `--stdin`)
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
        /// Materialize stdin to a capped temp file
        #[arg(long, action = clap::ArgAction::SetTrue)]
        stdin: bool,
        /// Batch: one audio path per line (`#` comments skipped). Mutually exclusive with `--path` and `--stdin`
        ///
        /// Each output is derived beside its input as `input.<target format>`; `--out` is
        /// refused here because one destination cannot serve N inputs. An item
        /// whose derived path already exists, or equals its own input, fails and
        /// the batch continues, so `ok: true` means the batch produced
        /// SOMETHING, never that every item passed — read `error_count`. If NO
        /// item succeeds the run fails with exit 65.
        #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
        paths_file: Option<std::path::PathBuf>,
        /// Output format: mp3 | m4a | aac | ogg | opus | flac | wav
        #[arg(long)]
        format: Option<String>,
        /// Output path (default under XDG cache)
        #[arg(long, short = 'o', value_hint = ValueHint::FilePath)]
        out: Option<std::path::PathBuf>,
        /// Audio codec: copy | mp3 | aac | opus | vorbis | flac | pcm | … (default: smart copy)
        #[arg(long)]
        codec: Option<String>,
        /// Audio bitrate for lossy encode (default: XDG audio_default_bitrate)
        #[arg(long)]
        bitrate: Option<String>,
        /// Sample rate Hz (optional re-encode)
        #[arg(long)]
        sample_rate: Option<u32>,
        /// Channel count (optional re-encode)
        #[arg(long)]
        channels: Option<u32>,
        /// Audio stream index (default 0)
        #[arg(long)]
        audio_stream: Option<u32>,
        /// Strip container metadata (opt-in; default preserves)
        #[arg(long, action = clap::ArgAction::SetTrue)]
        strip_metadata: bool,
        /// Project envelope fields (CSV); aliases: format→container|container_out, bytes|size→bytes_out, path→path|path_out, duration→duration_secs, codec→audio_codec
        #[arg(long, value_name = "FIELDS")]
        select: Option<String>,
    },
    /// Trim a time range (path→path via optional ffmpeg)
    Trim {
        /// Input audio path (omit with `--stdin`)
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
        /// Materialize stdin to a capped temp file
        #[arg(long, action = clap::ArgAction::SetTrue)]
        stdin: bool,
        /// Batch: one audio path per line (`#` comments skipped). Mutually exclusive with `--path` and `--stdin`
        ///
        /// Each output is derived beside its input as `input.<target format>`; `--out` is
        /// refused here because one destination cannot serve N inputs. An item
        /// whose derived path already exists, or equals its own input, fails and
        /// the batch continues, so `ok: true` means the batch produced
        /// SOMETHING, never that every item passed — read `error_count`. If NO
        /// item succeeds the run fails with exit 65.
        #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
        paths_file: Option<std::path::PathBuf>,
        /// Start time in seconds
        #[arg(long)]
        start: f64,
        /// Duration in seconds (mutually exclusive with `--to`)
        #[arg(long)]
        duration: Option<f64>,
        /// End time in seconds (exclusive end; use with `--start`)
        #[arg(long = "to")]
        to: Option<f64>,
        /// Output path (default under XDG cache)
        #[arg(long, short = 'o', value_hint = ValueHint::FilePath)]
        out: Option<std::path::PathBuf>,
        /// Output format override (default: inferred / XDG)
        #[arg(long)]
        format: Option<String>,
        /// Audio codec intent (default: copy when muxable)
        #[arg(long)]
        codec: Option<String>,
        /// Bitrate when re-encoding (default: XDG audio_default_bitrate)
        #[arg(long)]
        bitrate: Option<String>,
        /// Project envelope fields (CSV); aliases: format→container|container_out, bytes|size→bytes_out, path→path|path_out, duration→duration_secs, codec→audio_codec
        #[arg(long, value_name = "FIELDS")]
        select: Option<String>,
    },
}
