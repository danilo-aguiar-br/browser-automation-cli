// SPDX-License-Identifier: MIT OR Apache-2.0
//! Lifecycle unit tests.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::*;

#[test]
fn finalize_is_idempotent() {
    let lc = Lifecycle::new();
    lc.finalize();
    lc.finalize();
    assert!(lc.finalize_done.load(Ordering::SeqCst));
}

#[test]
fn with_ledger_mut_recovers_from_poison() {
    let lc = Lifecycle::new();
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = lc.ledger.lock().unwrap();
        panic!("poison ledger for test");
    }));
    assert!(lc.ledger.is_poisoned());
    lc.record_chrome(Some(42));
    lc.with_ledger_mut(|ledger| {
        assert!(ledger.chrome_launched);
        assert_eq!(ledger.chrome_pid, Some(42));
    });
    lc.clear_chrome();
    lc.with_ledger_mut(|ledger| {
        assert!(!ledger.chrome_launched);
        assert!(ledger.chrome_pid.is_none());
    });
    lc.finalize();
}

#[test]
fn current_cancel_tracks_active_lifecycle() {
    let lc = Lifecycle::new();
    assert!(!current_cancel().is_cancelled());
    lc.cancel.cancel();
    assert!(current_cancel().is_cancelled());
    lc.finalize();
}

#[test]
fn current_lifecycle_is_registered() {
    let lc = Lifecycle::new();
    let cur = current_lifecycle().expect("registered");
    assert!(Arc::ptr_eq(&cur.finalize_done, &lc.finalize_done));
    lc.finalize();
}

#[test]
fn finalize_child_grace_is_bounded() {
    assert!(FINALIZE_CHILD_GRACE <= Duration::from_secs(5));
    assert!(FINALIZE_CHILD_GRACE >= Duration::from_millis(100));
}

#[test]
fn finalize_clears_tls_registration() {
    let lc = Lifecycle::new();
    assert!(current_lifecycle().is_some());
    lc.finalize();
    assert!(current_lifecycle().is_none());
}

#[test]
fn finalize_clears_chrome_ownership() {
    let lc = Lifecycle::new();
    lc.record_chrome(Some(42));
    lc.with_ledger_mut(|l| assert!(l.chrome_launched));
    lc.finalize();
    lc.with_ledger_mut(|l| {
        assert!(!l.chrome_launched, "finalize must clear chrome_launched");
        assert!(l.chrome_pid.is_none(), "finalize must take chrome_pid");
    });
}

#[cfg(unix)]
#[test]
fn kill_unix_graceful_on_missing_pid_returns_quickly() {
    let start = Instant::now();
    kill_unix_graceful(4_294_967_294, Duration::from_millis(0));
    assert!(start.elapsed() < Duration::from_secs(1));
}
