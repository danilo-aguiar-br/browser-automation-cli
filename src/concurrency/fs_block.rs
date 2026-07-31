// SPDX-License-Identifier: MIT OR Apache-2.0
//! Disk I/O via spawn_blocking.

/// Write bytes on the Tokio blocking pool (docsrs: never pin async workers with
/// `std::fs` for non-trivial payloads).
///
/// # Cancel safety
///
/// `spawn_blocking` work is **not** abortable after start (docsrs). Cancellation
/// must cut admission at the async gate; this helper is for short-lived disk I/O.
pub async fn write_bytes_blocking(
    path: std::path::PathBuf,
    bytes: Vec<u8>,
) -> Result<(), std::io::Error> {
    // GAP-026: refuse a binary artifact aimed outside the allowed roots.
    crate::fs_roots::ensure_write_allowed(&path)
        .map_err(|e| std::io::Error::other(e.message().to_string()))?;
    tokio::task::spawn_blocking(move || {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(&path, bytes)
    })
    .await
    .map_err(|e| std::io::Error::other(format!("write_bytes_blocking join: {e}")))?
}

/// Read one line from stdin on the blocking pool (`None` at EOF).
///
/// Streaming `run --script -` blocks between lines waiting for the caller, so
/// the read must never sit on an async worker: `spawn_blocking` keeps the Tokio
/// runtime free to service the live CDP session while stdin is idle.
pub async fn read_stdin_line_blocking() -> Result<Option<String>, std::io::Error> {
    tokio::task::spawn_blocking(|| {
        use std::io::BufRead;
        let mut line = String::new();
        let read = std::io::stdin().lock().read_line(&mut line)?;
        if read == 0 {
            Ok(None)
        } else {
            Ok(Some(line))
        }
    })
    .await
    .map_err(|e| std::io::Error::other(format!("read_stdin_line_blocking join: {e}")))?
}

/// `create_dir_all` on the blocking pool.
pub async fn create_dir_all_blocking(path: std::path::PathBuf) -> Result<(), std::io::Error> {
    tokio::task::spawn_blocking(move || std::fs::create_dir_all(path))
        .await
        .map_err(|e| std::io::Error::other(format!("create_dir_all_blocking join: {e}")))?
}

/// Read a file fully on the blocking pool.
pub async fn read_bytes_blocking(path: std::path::PathBuf) -> Result<Vec<u8>, std::io::Error> {
    tokio::task::spawn_blocking(move || std::fs::read(path))
        .await
        .map_err(|e| std::io::Error::other(format!("read_bytes_blocking join: {e}")))?
}

/// Read a UTF-8 file on the blocking pool (PAR-77: never `fs::read_to_string` on async worker).
pub async fn read_to_string_blocking(path: std::path::PathBuf) -> Result<String, std::io::Error> {
    tokio::task::spawn_blocking(move || std::fs::read_to_string(path))
        .await
        .map_err(|e| std::io::Error::other(format!("read_to_string_blocking join: {e}")))?
}

/// `rename` on the blocking pool (PAR-80: state rotation must not pin async workers).
pub async fn rename_blocking(
    from: std::path::PathBuf,
    to: std::path::PathBuf,
) -> Result<(), std::io::Error> {
    tokio::task::spawn_blocking(move || std::fs::rename(from, to))
        .await
        .map_err(|e| std::io::Error::other(format!("rename_blocking join: {e}")))?
}

/// Sync write helper for **outer CLI dispatch** (no active Tokio worker). Prefer
/// [`write_bytes_blocking`] when inside `async fn` / `block_on_*`.
pub fn write_bytes_sync(path: &std::path::Path, bytes: &[u8]) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, bytes)
}
