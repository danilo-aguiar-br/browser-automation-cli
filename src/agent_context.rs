// SPDX-License-Identifier: MIT OR Apache-2.0
//! Process-local agent correlation context (one-shot; no product env vars).
//!
//! Agents may pass `--correlation-id` once per process. Envelopes and NDJSON
//! steps echo it when set so multi-tool workflows can join stdout records.

use std::sync::Mutex;

use crate::sync_util::lock_recover;

/// Process-wide correlation id slot (optional; empty = unset).
///
/// # Interior mutability choice
///
/// - Needs a process-wide `Sync` static with a non-`Copy` payload (`String`) →
///   `std::sync::Mutex` (not `Cell`/`RefCell`/`Atomic*`).
/// - Direct `Mutex::new(None)` (MSRV ≥ 1.63) — **no** `OnceLock`/`LazyLock`
///   wrapper (rules: prefer const constructor over redundant lazy init).
/// - Poison is recovered via `lock_recover` so a prior panic cannot sticky-fail
///   envelope emission.
static CORRELATION_ID: Mutex<Option<String>> = Mutex::new(None);

/// Install the correlation id for this process (call once after argv parse).
///
/// Empty strings are treated as unset. Poisoned mutex recovers by replacing.
pub fn set_correlation_id(id: Option<String>) {
    let cleaned = id.and_then(|s| {
        let t = s.trim();
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    });
    *lock_recover(&CORRELATION_ID) = cleaned;
}

/// Current process correlation id, if any.
pub fn correlation_id() -> Option<String> {
    lock_recover(&CORRELATION_ID).clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get_roundtrip() {
        set_correlation_id(Some("  agent-42  ".into()));
        assert_eq!(correlation_id().as_deref(), Some("agent-42"));
        set_correlation_id(Some(String::new()));
        assert_eq!(correlation_id(), None);
        set_correlation_id(None);
        assert_eq!(correlation_id(), None);
    }
}
