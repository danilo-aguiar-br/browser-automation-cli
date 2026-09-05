// SPDX-License-Identifier: MIT OR Apache-2.0
//! Disk I/O via spawn_blocking.

/// Write bytes on the Tokio blocking pool (docsrs: never pin async workers with
/// `std::fs` for non-trivial payloads).
///
/// # Cancel safety
///
/// `spawn_blocking` work is **not** abortable after start (docsrs). Cancellation
/// must cut admission at the async gate; this helper is for short-lived disk I/O.
///
/// # Errors
///
/// `std::io::Error` from [`crate::fs_roots::ensure_write_allowed`], restated via
/// `Error::other`, when `path` falls outside the allowed roots (GAP-026); from
/// `create_dir_all` on the parent; from `std::fs::write`; and from
/// `Error::other` wrapping a `JoinError` when the blocking task panics.
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
///
/// # Errors
///
/// `std::io::Error` from `BufRead::read_line` — a closed or non-UTF-8 stdin —
/// and from `Error::other` wrapping a `JoinError` when the blocking task panics.
/// EOF is not an error: it yields `Ok(None)`.
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
///
/// # Errors
///
/// `std::io::Error` from `std::fs::create_dir_all` — permission denied, a
/// non-directory component in the path, or a read-only filesystem — and from
/// `Error::other` wrapping a `JoinError` when the blocking task panics.
pub async fn create_dir_all_blocking(path: std::path::PathBuf) -> Result<(), std::io::Error> {
    tokio::task::spawn_blocking(move || std::fs::create_dir_all(path))
        .await
        .map_err(|e| std::io::Error::other(format!("create_dir_all_blocking join: {e}")))?
}

/// Read a file fully on the blocking pool.
///
/// # Errors
///
/// `std::io::Error` from `std::fs::read` — missing file, permission denied, or a
/// mid-read I/O failure — and from `Error::other` wrapping a `JoinError` when the
/// blocking task panics.
pub async fn read_bytes_blocking(path: std::path::PathBuf) -> Result<Vec<u8>, std::io::Error> {
    tokio::task::spawn_blocking(move || std::fs::read(path))
        .await
        .map_err(|e| std::io::Error::other(format!("read_bytes_blocking join: {e}")))?
}

/// Read a UTF-8 file on the blocking pool (PAR-77: never `fs::read_to_string` on async worker).
///
/// # Errors
///
/// `std::io::Error` from `std::fs::read_to_string` — missing file, permission
/// denied, or bytes that are not valid UTF-8 (`InvalidData`) — and from
/// `Error::other` wrapping a `JoinError` when the blocking task panics.
pub async fn read_to_string_blocking(path: std::path::PathBuf) -> Result<String, std::io::Error> {
    tokio::task::spawn_blocking(move || std::fs::read_to_string(path))
        .await
        .map_err(|e| std::io::Error::other(format!("read_to_string_blocking join: {e}")))?
}

/// `rename` on the blocking pool (PAR-80: state rotation must not pin async workers).
///
/// # Errors
///
/// `std::io::Error` from `std::fs::rename` — a missing source, a cross-device
/// move, or permission denied on either path — and from `Error::other` wrapping
/// a `JoinError` when the blocking task panics.
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
///
/// # Errors
///
/// `std::io::Error` from `crate::fs_roots::ensure_write_allowed` when the target
/// falls outside the allowed roots, or from `create_dir_all` on the parent
/// directory or `std::fs::write` — permission denied, a read-only filesystem, or
/// a full disk.
///
/// # Why the root check is here now
///
/// This helper used to skip it, on the stated ground that "callers reaching it
/// from dispatch have already resolved the target". That was an assumption about
/// callers, not a property of the function. Measured: all four call sites —
/// `mitm export --out`, `monitor --baseline`, the monitor diff sidecar and the
/// capture dump — write a path the OPERATOR named on argv, which is precisely
/// the input `ensure_write_allowed` exists to bound. Its async twin
/// [`write_bytes_blocking`] checked; this one did not, so the jail depended on
/// which of two interchangeable helpers a caller happened to pick.
pub fn write_bytes_sync(path: &std::path::Path, bytes: &[u8]) -> Result<(), std::io::Error> {
    crate::fs_roots::ensure_write_allowed(path)
        .map_err(|e| std::io::Error::other(e.message().to_string()))?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, bytes)
}
