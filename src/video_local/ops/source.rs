// SPDX-License-Identifier: MIT OR Apache-2.0
//! Video input source (path or capped stdin materialization).

use std::io::{Read, Write};
use std::path::PathBuf;

use super::super::limits::VideoLimits;
use crate::error::{CliError, ErrorKind};
use crate::xdg;

/// Refusal shared by both [`VideoSource::load_bytes`] arms.
fn oversized(max_bytes: usize) -> CliError {
    CliError::with_suggestion(
        ErrorKind::Data,
        format!("input exceeds manifest_max_bytes {max_bytes}"),
        crate::i18n::suggestion_key("video_too_large", None),
    )
}

/// Where video bytes / paths come from for one-shot ops.
#[derive(Debug, Clone)]
pub enum VideoSource {
    /// Filesystem path (preferred — path→path, no full load for convert).
    Path(PathBuf),
    /// Materialize stdin to a capped temp file under XDG cache.
    Stdin,
}

impl VideoSource {
    /// Read the source into memory under `max_bytes`, never touching disk.
    ///
    /// # Why this is separate from [`Self::resolve_path`]
    ///
    /// `resolve_path` exists for ffmpeg, which wants a filename, so a stdin
    /// video is materialised to a temp file. A manifest is a small text
    /// document parsed in-process: writing it out would create a residual file
    /// for no reason and bill it against the multi-gigabyte video cap instead
    /// of the manifest one. Callers pass `xdg::resolve_manifest_max_bytes()`.
    ///
    /// The path arm checks metadata before reading, so an oversized file is
    /// refused without allocating for it.
    pub fn load_bytes(&self, max_bytes: usize) -> Result<Vec<u8>, CliError> {
        match self {
            Self::Path(p) => {
                let meta = std::fs::metadata(p)
                    .map_err(|e| crate::video_local::magic::io_open_err(p, &e))?;
                if usize::try_from(meta.len()).unwrap_or(usize::MAX) > max_bytes {
                    return Err(oversized(max_bytes));
                }
                std::fs::read(p).map_err(|e| crate::video_local::magic::io_path_err(p, "read", &e))
            }
            Self::Stdin => {
                let mut buf = Vec::new();
                let stdin = std::io::stdin();
                let mut handle = stdin.lock();
                let mut chunk = [0u8; 64 * 1024];
                loop {
                    let n = handle
                        .read(&mut chunk)
                        .map_err(|e| CliError::new(ErrorKind::Io, format!("stdin read: {e}")))?;
                    if n == 0 {
                        break;
                    }
                    if buf.len().saturating_add(n) > max_bytes {
                        return Err(oversized(max_bytes));
                    }
                    buf.extend_from_slice(&chunk[..n]);
                }
                if buf.is_empty() {
                    return Err(CliError::new(ErrorKind::NoInput, "empty stdin"));
                }
                Ok(buf)
            }
        }
    }

    /// Resolve to an on-disk path (stdin writes a temp file under the input cap).
    pub fn resolve_path(&self, limits: VideoLimits) -> Result<(PathBuf, bool), CliError> {
        match self {
            Self::Path(p) => {
                let meta = std::fs::metadata(p)
                    .map_err(|e| crate::video_local::magic::io_open_err(p, &e))?;
                limits.check_input_len(meta.len())?;
                Ok((p.clone(), false))
            }
            Self::Stdin => {
                let dir = xdg::cache_dir().unwrap_or_else(|_| PathBuf::from("."));
                let stamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0);
                let path = dir.join(format!("video-stdin-{stamp}.bin"));
                let mut out = std::fs::File::create(&path)
                    .map_err(|e| crate::video_local::magic::io_path_err(&path, "create", &e))?;
                let stdin = std::io::stdin();
                let mut handle = stdin.lock();
                let mut chunk = [0u8; 64 * 1024];
                let mut total = 0usize;
                loop {
                    let n = handle.read(&mut chunk).map_err(|e| {
                        crate::video_local::magic::io_path_err(&path, "stdin-read", &e)
                    })?;
                    if n == 0 {
                        break;
                    }
                    total = total.saturating_add(n);
                    if total > limits.max_input_bytes {
                        let _ = std::fs::remove_file(&path);
                        return Err(CliError::with_suggestion(
                            ErrorKind::Data,
                            format!(
                                "stdin video exceeds video_max_input_bytes {}",
                                limits.max_input_bytes
                            ),
                            crate::i18n::suggestion_key("video_too_large", None),
                        ));
                    }
                    out.write_all(&chunk[..n]).map_err(|e| {
                        crate::video_local::magic::io_path_err(&path, "stdin-write", &e)
                    })?;
                }
                if total == 0 {
                    let _ = std::fs::remove_file(&path);
                    return Err(CliError::new(ErrorKind::NoInput, "empty stdin for video"));
                }
                out.sync_all().ok();
                Ok((path, true))
            }
        }
    }
}
