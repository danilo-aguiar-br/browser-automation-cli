// SPDX-License-Identifier: MIT OR Apache-2.0
//! Audio input source (path or capped stdin materialization).

use std::io::{Read, Write};
use std::path::PathBuf;

use super::super::limits::AudioLimits;
use crate::error::{CliError, ErrorKind};
use crate::xdg;

/// Where audio bytes / paths come from for one-shot ops.
#[derive(Debug, Clone)]
pub enum AudioSource {
    /// Filesystem path (preferred — path→path).
    Path(PathBuf),
    /// Materialize stdin to a capped temp file under XDG cache.
    Stdin,
}

impl AudioSource {
    /// Resolve to an on-disk path (stdin writes a temp file under the input cap).
    pub fn resolve_path(&self, limits: AudioLimits) -> Result<(PathBuf, bool), CliError> {
        match self {
            Self::Path(p) => {
                // GAP-026: bound the operator-supplied source path.
                crate::fs_roots::ensure_read_allowed(p)?;
                let meta = std::fs::metadata(p)
                    .map_err(|e| crate::audio_local::magic::io_open_err(p, &e))?;
                limits.check_input_len(meta.len())?;
                Ok((p.clone(), false))
            }
            Self::Stdin => {
                let dir = xdg::cache_dir().unwrap_or_else(|_| PathBuf::from("."));
                let stamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0);
                let path = dir.join(format!("audio-stdin-{stamp}.bin"));
                let mut out = std::fs::File::create(&path)
                    .map_err(|e| crate::audio_local::magic::io_path_err(&path, "create", &e))?;
                let stdin = std::io::stdin();
                let mut handle = stdin.lock();
                let mut chunk = vec![0u8; crate::constants::MEDIA_STREAM_CHUNK_BYTES];
                let mut total = 0usize;
                loop {
                    let n = handle.read(&mut chunk).map_err(|e| {
                        crate::audio_local::magic::io_path_err(&path, "stdin-read", &e)
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
                                "stdin audio exceeds audio_max_input_bytes {}",
                                limits.max_input_bytes
                            ),
                            crate::i18n::suggestion_key("audio_too_large", None),
                        ));
                    }
                    out.write_all(&chunk[..n]).map_err(|e| {
                        crate::audio_local::magic::io_path_err(&path, "stdin-write", &e)
                    })?;
                }
                if total == 0 {
                    let _ = std::fs::remove_file(&path);
                    return Err(CliError::new(ErrorKind::NoInput, "empty stdin for audio"));
                }
                out.sync_all().ok();
                Ok((path, true))
            }
        }
    }
}
