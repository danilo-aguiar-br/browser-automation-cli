// SPDX-License-Identifier: MIT OR Apache-2.0
//! Shared helpers: redaction, atomic I/O, clocks, capture lock.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::constants::MITM_REDACTED_PLACEHOLDER;
use crate::error::{CliError, ErrorKind};

use super::types::{BTreeMapString, MitmCapture};

/// Shared capture for optional in-process proxy (thread-safe).
///
/// # Interior mutability
///
/// `std::sync::Mutex` is used because handlers take short critical sections that
/// **do not** hold the guard across `.await`. Poison is recovered via
/// `lock_capture` so a panic in one handler cannot drop later captures.
pub type SharedCapture = Arc<Mutex<MitmCapture>>;

/// Redact sensitive header values in place.
pub(super) fn redact_headers(h: &mut BTreeMapString) {
    const SENSITIVE: &[&str] = &[
        "authorization",
        "cookie",
        "set-cookie",
        "proxy-authorization",
        "x-api-key",
    ];
    for (k, v) in h.iter_mut() {
        if SENSITIVE.iter().any(|s| k.eq_ignore_ascii_case(s)) {
            *v = MITM_REDACTED_PLACEHOLDER.into();
        }
    }
}

/// Atomic write via tmp + rename (sync; callers are sync CLI or off-async).
pub(super) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), CliError> {
    let tmp = path.with_extension("tmp");
    {
        let mut f = fs::File::create(&tmp)
            .map_err(|e| CliError::new(ErrorKind::Io, format!("mitm tmp: {e}")))?;
        f.write_all(bytes)
            .map_err(|e| CliError::new(ErrorKind::Io, format!("mitm write: {e}")))?;
        f.sync_all()
            .map_err(|e| CliError::new(ErrorKind::Io, format!("mitm fsync: {e}")))?;
    }
    fs::rename(&tmp, path)
        .map_err(|e| CliError::new(ErrorKind::Io, format!("mitm rename: {e}")))?;
    // GAP-009: captured bodies may carry secrets; never world-readable.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
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
    Ok(Arc::new(Mutex::new(MitmCapture::new(Some(path), true))))
}

#[cfg(test)]
pub(super) use redact_headers as redact_headers_for_test;
