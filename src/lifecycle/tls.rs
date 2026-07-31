// SPDX-License-Identifier: MIT OR Apache-2.0
//! Thread-local cancel + lifecycle registration for the active one-shot invocation.

use std::cell::RefCell;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use super::ledger::Lifecycle;

// Thread-local cancel + lifecycle for the active invocation (main-thread one-shot).
// Used by block_on_browser_timeout so SIGINT/SIGTERM abort work without
// threading Lifecycle through every call site.
//
// `const { RefCell::new(None) }` avoids runtime init on first access (MSRV ≥ 1.59).
// `RefCell` is correct here: TLS is not `Sync` across threads; each thread owns its cell.
thread_local! {
    static CURRENT_CANCEL: RefCell<Option<CancellationToken>> = const { RefCell::new(None) };
    static CURRENT_LIFE: RefCell<Option<Lifecycle>> = const { RefCell::new(None) };
}

/// Return the cancel token for the active invocation, or a fresh inert token.
///
/// Uses `try_borrow` so a re-entrant path that already holds the TLS `RefCell`
/// cannot panic (returns a fresh inert token instead).
pub fn current_cancel() -> CancellationToken {
    CURRENT_CANCEL.with(|c| {
        c.try_borrow()
            .ok()
            .and_then(|slot| slot.clone())
            .unwrap_or_default()
    })
}

/// Return a clone of the active [`Lifecycle`], if one was registered.
///
/// Used by the second-signal force path to run residual finalize without
/// threading `Lifecycle` through every browser call site.
///
/// Uses `try_borrow` so re-entrancy cannot panic on the TLS `RefCell`.
pub fn current_lifecycle() -> Option<Lifecycle> {
    CURRENT_LIFE.with(|c| c.try_borrow().ok().and_then(|slot| slot.clone()))
}

// The ledger of resources owned by an invocation (browser, profile, temp dirs)
// is defined with the `Lifecycle` type; this module only clears the TLS slots.

/// Clear TLS cancel/lifecycle slots when they still point at `lc`.
///
/// Uses `Arc::ptr_eq` on `finalize_done` (stable identity across clones) and
/// `try_borrow_mut` so re-entrant signal paths cannot panic the TLS `RefCell`.
/// Register cancel token for the active invocation (Lifecycle::new).
pub(crate) fn register_cancel(cancel: CancellationToken) {
    CURRENT_CANCEL.with(|c| {
        if let Ok(mut slot) = c.try_borrow_mut() {
            *slot = Some(cancel);
        }
    });
}

/// Register lifecycle clone for the active invocation (Lifecycle::new).
pub(crate) fn register_lifecycle(lc: Lifecycle) {
    CURRENT_LIFE.with(|c| {
        if let Ok(mut slot) = c.try_borrow_mut() {
            *slot = Some(lc);
        }
    });
}

pub(crate) fn clear_tls_registration(lc: &Lifecycle) {
    CURRENT_LIFE.with(|c| {
        let Ok(mut slot) = c.try_borrow_mut() else {
            return;
        };
        let is_self = slot
            .as_ref()
            .is_some_and(|reg| Arc::ptr_eq(&reg.finalize_done, &lc.finalize_done));
        if !is_self {
            return;
        }
        *slot = None;
        // Nested TLS key: distinct RefCell from CURRENT_LIFE (safe re-entry).
        CURRENT_CANCEL.with(|cc| {
            if let Ok(mut cs) = cc.try_borrow_mut() {
                *cs = None;
            }
        });
    });
}
