// SPDX-License-Identifier: MIT OR Apache-2.0
//! Atomic partial output helpers and ffmpeg error mapping.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::{CliError, ErrorKind};
use crate::platform::ProcessCaptureError;

pub(super) fn partial_path(output: &Path) -> PathBuf {
    // Keep a real media extension so ffmpeg can probe the muxer from the name.
    let parent = output.parent().filter(|p| !p.as_os_str().is_empty());
    let stem = output.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
    let name = match output.extension().and_then(|e| e.to_str()) {
        Some(ext) => format!("{stem}.ba-partial.{ext}"),
        None => format!("{stem}.ba-partial"),
    };
    match parent {
        Some(p) => p.join(name),
        None => PathBuf::from(name),
    }
}

pub(super) fn ensure_parent(output: &Path) -> Result<(), CliError> {
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| crate::video_local::magic::io_path_err(parent, "mkdir", &e))?;
        }
    }
    Ok(())
}

pub(super) fn cleanup_partials(output: &Path, partial: &Path) {
    let _ = std::fs::remove_file(partial);
    if output.exists() {
        let _ = std::fs::remove_file(output);
    }
}

pub(super) fn finalize_partial(partial: &Path, output: &Path) -> Result<(), CliError> {
    if !partial.is_file() {
        return Err(CliError::with_suggestion(
            ErrorKind::Data,
            "ffmpeg reported success but partial output is missing",
            crate::i18n::suggestion_key("ffmpeg_failed", None),
        ));
    }
    if output.exists() {
        let _ = std::fs::remove_file(output);
    }
    std::fs::rename(partial, output).map_err(|e| {
        let _ = std::fs::remove_file(partial);
        crate::video_local::magic::io_path_err(output, "rename", &e)
    })
}

pub(super) fn map_spawn_err(e: ProcessCaptureError, op: &str) -> CliError {
    match e {
        ProcessCaptureError::Timeout => CliError::with_suggestion(
            ErrorKind::Timeout,
            format!(
                "ffmpeg {op} timed out after {}s",
                crate::xdg::resolve_ffmpeg_timeout_secs()
            ),
            crate::i18n::suggestion_key("ffmpeg_timeout", None),
        ),
        other => CliError::with_suggestion(
            ErrorKind::Unavailable,
            format!("ffmpeg spawn ({op}): {other}"),
            crate::i18n::suggestion_key("ffmpeg_missing", None),
        ),
    }
}

pub(crate) fn compact_ffmpeg_stderr(stderr: &[u8]) -> String {
    const MAX: usize = 280;
    let err = String::from_utf8_lossy(stderr);
    let lines: Vec<&str> = err
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let interesting: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|l| {
            let lo = l.to_ascii_lowercase();
            lo.contains("error")
                || lo.contains("permission")
                || lo.contains("invalid")
                || lo.contains("does not")
                || lo.contains("unknown")
                || lo.contains("failed")
                || lo.contains("not found")
                || lo.contains("read-only")
        })
        .collect();
    let pick = if !interesting.is_empty() {
        interesting.join("; ")
    } else if lines.len() > 3 {
        lines[lines.len().saturating_sub(3)..].join("; ")
    } else {
        lines.join("; ")
    };
    let collapsed: String = pick.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() > MAX {
        let truncated: String = collapsed.chars().take(MAX.saturating_sub(1)).collect();
        format!("{truncated}…")
    } else if collapsed.is_empty() {
        "see ffmpeg logs".into()
    } else {
        collapsed
    }
}

pub(super) fn ffmpeg_fail(op: &str, stderr: &[u8]) -> CliError {
    let err = compact_ffmpeg_stderr(stderr);
    let lower = err.to_ascii_lowercase();
    let key = if lower.contains("permission denied")
        || lower.contains("read-only file system")
        || lower.contains("operation not permitted")
    {
        "ffmpeg_io_failed"
    } else {
        "ffmpeg_failed"
    };
    CliError::with_suggestion(
        ErrorKind::Data,
        format!("ffmpeg {op} failed: {err}"),
        crate::i18n::suggestion_key(key, None),
    )
}

pub(super) fn sha256_file(path: &Path) -> Result<String, CliError> {
    use std::io::Read;
    let mut f =
        std::fs::File::open(path).map_err(|e| crate::video_local::magic::io_open_err(path, &e))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; crate::constants::MEDIA_STREAM_CHUNK_BYTES];
    loop {
        let n = f
            .read(&mut buf)
            .map_err(|e| crate::video_local::magic::io_open_err(path, &e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::compact_ffmpeg_stderr;

    #[test]
    fn compact_ffmpeg_stderr_keeps_error_line_and_caps() {
        let raw = b"frame=1\nfps=25\n[out#0/webm] Error opening output /tmp/x.ba-partial.webm: Permission denied\nError opening output file\nError opening output files: Permission denied\n";
        let c = compact_ffmpeg_stderr(raw);
        assert!(c.to_ascii_lowercase().contains("permission denied"), "{c}");
        assert!(!c.contains('\n'), "{c}");
        assert!(c.chars().count() <= 280, "len={}", c.chars().count());
    }
}
