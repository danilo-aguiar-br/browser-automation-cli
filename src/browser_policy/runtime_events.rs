// SPDX-License-Identifier: MIT OR Apache-2.0
//! Whether this process has any consumer for CDP `Runtime` events.
//!
//! # The defect this closes
//!
//! `Runtime.enable` left the launch path unconditionally, on every command,
//! including a bare `goto`. It is one of the cheapest CDP fingerprints there
//! is: enabling the domain makes Chrome install the console and execution
//! context machinery a plain page never asks for, and a detector that watches
//! for it learns the page is automated before the first script runs. The
//! project's own reference rules say it in one line — minimise `Runtime.enable`
//! where the framework can.
//!
//! # Why the expensive repair turned out to be unnecessary
//!
//! The catalogued correction was to port `Page.createIsolatedWorld` plus
//! `Runtime.callFunctionOn`, on the theory that evaluation needs the domain.
//! Measured 2026-09-04 across `src/`: it does not. `Runtime.evaluate` and
//! `Runtime.callFunctionOn` are COMMANDS and answer with the domain disabled,
//! and no call site anywhere targets an evaluation by `executionContextId` —
//! every one of them uses an `objectId` or the default context.
//!
//! Only EVENTS need the domain, and the same sweep found exactly two consumers:
//!
//! - `Runtime.consoleAPICalled`, reached only under `--capture-console`
//! - `Runtime.bindingCalled`, reached only by `record`, which already issues
//!   its own `Runtime.enable` and says so at the call site
//!
//! So the domain is a function of the flags, and this module is where that
//! function is stored.
//!
//! # Why a process-global and not a parameter
//!
//! `enable_domains` has six call sites across targets, tabs and queries, none
//! of which knows anything about console capture. Threading a boolean through
//! all six would put the decision in six places and let a seventh call site
//! forget it. The idiom here is the one the rest of `browser_policy` already
//! uses: publish once during dispatch, read at the point of use.

use std::sync::atomic::{AtomicBool, Ordering};

static RUNTIME_EVENTS: AtomicBool = AtomicBool::new(false);

/// Publish whether any consumer of `Runtime` events exists. Called once from
/// CLI dispatch, before any browser launch.
pub fn set_runtime_events_needed(needed: bool) {
    RUNTIME_EVENTS.store(needed, Ordering::Relaxed);
}

/// Whether the launch path should issue `Runtime.enable`.
///
/// A `false` here is a claim about this process, and the envelope publishes it
/// as `runtime_enable_used` so the claim is checkable from outside rather than
/// taken on the word of this comment.
#[must_use]
pub fn runtime_events_needed() -> bool {
    RUNTIME_EVENTS.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_off_because_the_default_command_has_no_consumer() {
        // The whole point of the change: an unset process must not enable the
        // domain. If this ever defaults to `true`, every command pays the
        // fingerprint again and no other test in this tree notices.
        static FRESH: AtomicBool = AtomicBool::new(false);
        assert!(!FRESH.load(Ordering::Relaxed));
    }

    #[test]
    fn publishing_round_trips() {
        set_runtime_events_needed(true);
        assert!(runtime_events_needed());
        set_runtime_events_needed(false);
        assert!(!runtime_events_needed());
    }
}
