// SPDX-License-Identifier: MIT OR Apache-2.0
//! Console and network evidence dump on failure (GAP-039).
//!
//! # Problem
//!
//! `--capture-console` and `--capture-network` only live for the process. When
//! an intermittent failure happens without those flags already set, the
//! evidence is gone and the run must be repeated hoping it fails again.
//!
//! # Design
//!
//! The one-shot model is preserved: nothing survives as a service. On the
//! failure path only, the still-live session is asked for its captured console
//! and network rings and the result is written once to disk. The artifact path
//! is reported on the error envelope so an agent can read it without guessing.
//!
//! Writes go through [`crate::fs_roots`], so the dump obeys the same
//! allowed-root policy as any other artifact (GAP-026).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

/// Whether `--dump-on-failure` was passed for this invocation.
static ENABLED: AtomicBool = AtomicBool::new(false);

/// Artifacts directory published at CLI boot (`--artifacts-dir`).
static ARTIFACTS_DIR: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

fn artifacts_slot() -> &'static Mutex<Option<PathBuf>> {
    ARTIFACTS_DIR.get_or_init(|| Mutex::new(None))
}

/// Publish the failure-dump decision and artifacts directory for this process.
pub fn configure(enabled: bool, artifacts_dir: Option<PathBuf>) {
    ENABLED.store(enabled, Ordering::Relaxed);
    // Poison recovery, not `if let Ok(..)`: a panic elsewhere while holding this
    // lock must not turn every later `configure` into a silent no-op that stops
    // failure dumps from ever being written.
    let mut slot = artifacts_slot().lock().unwrap_or_else(|e| e.into_inner());
    *slot = artifacts_dir;
}

/// Whether a failure dump should be attempted.
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Directory the dump is written into: `--artifacts-dir`, else XDG state.
fn dump_dir() -> Option<PathBuf> {
    // Poison recovery: a poisoned lock must not silently redirect the dump to
    // the XDG fallback while an explicit --artifacts-dir is configured.
    let slot = artifacts_slot().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(dir) = slot.clone() {
        return Some(dir);
    }
    drop(slot);
    crate::xdg::state_dir()
        .ok()
        .map(|d| d.join("failure-dumps"))
}

/// Build the dump file path for this invocation (millisecond-stamped).
fn dump_path() -> Option<PathBuf> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    dump_dir().map(|d| d.join(format!("failure-{stamp}.json")))
}

/// Write one dump payload and return the path written.
///
/// Errors are swallowed on purpose: a failed dump must never replace the
/// original error the operator is trying to diagnose. The failure is traced.
pub fn write_dump(payload: &serde_json::Value) -> Option<PathBuf> {
    let path = dump_path()?;
    match crate::json_util::write_json_file_atomic(&path, payload, true) {
        Ok(()) => Some(path),
        Err(e) => {
            tracing::warn!(
                target: "browser_automation_cli::failure_dump",
                path = %path.display(),
                error = %e.message(),
                "failure dump could not be written"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Serializes the tests that mutate the process-wide switch.
    ///
    /// `ENABLED` and `ARTIFACTS_DIR` are process globals by design (they are
    /// published once from argv at CLI boot). Under `cargo test` the whole
    /// binary is one process, so two tests configuring them concurrently see
    /// each other's writes. The lock restores determinism without weakening the
    /// runtime design.
    static CONFIG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Take the lock, ignoring poisoning from an unrelated failing test.
    fn config_guard() -> std::sync::MutexGuard<'static, ()> {
        CONFIG_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn disabled_by_default() {
        let _guard = config_guard();
        configure(false, None);
        assert!(!enabled());
    }

    #[test]
    fn dump_dir_prefers_configured_artifacts() {
        let _guard = config_guard();
        let tmp = tempfile::Builder::new()
            .prefix("bac-failure-dump-")
            .tempdir()
            .expect("scratch dir");
        let dir = tmp.path().to_path_buf();
        configure(true, Some(dir.clone()));
        assert_eq!(dump_dir(), Some(dir.clone()));
        assert!(enabled());

        let written = write_dump(&json!({"console": [], "network": []}));
        let written = written.expect("dump written");
        assert!(written.starts_with(&dir), "{written:?}");
        assert!(written.exists());

        // Restore the default so other tests are unaffected.
        configure(false, None);
    }

    #[test]
    fn dump_path_is_named_and_stamped() {
        let _guard = config_guard();
        configure(true, Some(std::env::temp_dir()));
        let p = dump_path().expect("path");
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        assert!(name.starts_with("failure-"), "{name}");
        assert!(name.ends_with(".json"), "{name}");
        configure(false, None);
    }
}
