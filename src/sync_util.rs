// SPDX-License-Identifier: MIT OR Apache-2.0
//! Shared `std::sync` helpers for short critical sections (one-shot CLI).
//!
//! # Interior mutability policy
//!
//! - Prefer this module's `lock_recover` when a poisoned mutex must **not**
//!   sticky-fail best-effort paths (ledger, capture, correlation, frame slot).
//! - Prefer `lock().map_err(...)` when poison is a software fault that should
//!   surface as [`crate::error::CliError`] (e.g. L1 [`crate::cache::MemoryCache`]).
//! - Never hold the returned guard across `.await` (use `tokio::sync::Mutex`
//!   when the critical section is async).
//! - Never use `RefCell` / `Cell` behind `Arc` for multi-thread state.

use std::sync::{Mutex, MutexGuard};

/// Lock a `std::sync::Mutex`, recovering from poison via [`PoisonError::into_inner`](std::sync::PoisonError::into_inner).
///
/// # When to use
///
/// Residual / agent-facing best-effort surfaces where a prior panic must not
/// prevent later reads or FINALIZE-adjacent bookkeeping.
///
/// # When **not** to use
///
/// Caches and pure software invariants that should fail closed as `CliError`.
#[inline]
pub fn lock_recover<T: ?Sized>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|poisoned| {
        tracing::debug!("mutex poisoned; recovering via into_inner");
        poisoned.into_inner()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn lock_recover_after_poison() {
        let m = Arc::new(Mutex::new(7u32));
        let m2 = Arc::clone(&m);
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _g = m2.lock().unwrap();
            panic!("poison for test");
        }));
        assert!(m.is_poisoned());
        let g = lock_recover(&m);
        assert_eq!(*g, 7);
    }
}
