// SPDX-License-Identifier: MIT OR Apache-2.0
//! Shared helpers: redaction, atomic I/O, clocks, capture lock.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{CliError, ErrorKind};

use super::types::MitmCapture;

/// Shared capture for optional in-process proxy (thread-safe).
///
/// # Interior mutability
///
/// `std::sync::Mutex` is used because handlers take short critical sections that
/// **do not** hold the guard across `.await`. Poison is recovered via
/// `lock_capture` so a panic in one handler cannot drop later captures.
pub type SharedCapture = Arc<Mutex<MitmCapture>>;

/// Atomic write via tmp + rename (sync; callers are sync CLI or off-async).
pub(super) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), CliError> {
    // The parent is created HERE, not at each call site, because "an atomic
    // write needs somewhere to write" is a property of this function and not of
    // whoever calls it. Measured 2026-09-04: `mitm redact` created the state
    // directory before calling in (store.rs), `mitm allow` and `mitm block` did
    // not, and on a host whose state directory did not exist yet the second
    // pair failed with `mitm tmp: No such file or directory (os error 2)` and
    // exit 74 — an IO error that names the temp file and not the missing
    // directory, which is the least useful place to read the cause.
    //
    // This is deliberately NOT where the allowed-roots check lives; `har.rs`
    // records why that one belongs at its own call site. Creating a directory
    // and deciding whether a path may be written are different questions.
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| CliError::new(ErrorKind::Io, format!("mitm state dir: {e}")))?;
    }
    let tmp = path.with_extension("tmp");
    {
        // Born `0600`. `File::create` honours the umask, so the tmp file — which
        // may hold the CA private key or a captured body — existed world-readable
        // from creation until the chmod after the rename. Creating it private
        // removes that window rather than shortening it.
        let mut f = crate::platform::create_private_file(&tmp)
            .map_err(|e| CliError::new(ErrorKind::Io, format!("mitm tmp: {e}")))?;
        f.write_all(bytes)
            .map_err(|e| CliError::new(ErrorKind::Io, format!("mitm write: {e}")))?;
        f.sync_all()
            .map_err(|e| CliError::new(ErrorKind::Io, format!("mitm fsync: {e}")))?;
    }
    fs::rename(&tmp, path)
        .map_err(|e| CliError::new(ErrorKind::Io, format!("mitm rename: {e}")))?;
    // GAP-009: captured bodies may carry secrets; never world-readable.
    //
    // The mode is already `0600` from creation and survives the rename; this
    // call covers a path that existed with looser permissions. It PROPAGATES
    // failure: this same function writes the MITM root CA private key, and a
    // chmod that failed silently left that key readable by every local user,
    // which is interception of the whole machine's TLS rather than a local bug.
    crate::platform::restrict_to_owner(path, 0o600)
        .map_err(|e| CliError::new(ErrorKind::Io, format!("mitm restrict perms: {e}")))?;
    Ok(())
}

/// Wall-clock unix millis (non-monotonic; agent timestamps only).
pub(super) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// RFC3339-ish timestamp from unix millis for HAR entries.
pub(super) fn chrono_like(ms: u64) -> String {
    let secs = (ms / 1000) as i64;
    time::OffsetDateTime::from_unix_timestamp(secs)
        .map(|t| {
            t.format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| format!("{ms}"))
        })
        .unwrap_or_else(|_| format!("{ms}"))
}

/// Lock the shared capture, recovering from poison (DRY: [`crate::sync_util::lock_recover`]).
pub(super) fn lock_capture(cap: &SharedCapture) -> std::sync::MutexGuard<'_, MitmCapture> {
    crate::sync_util::lock_recover(cap)
}

/// Create shared capture bound to default path.
pub fn shared_capture() -> Result<SharedCapture, CliError> {
    let path = super::store::default_capture_path()?;
    Ok(Arc::new(Mutex::new(MitmCapture::new(
        Some(path),
        super::policy::redact_secrets(),
    ))))
}
