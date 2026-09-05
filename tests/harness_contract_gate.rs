// SPDX-License-Identifier: MIT OR Apache-2.0
//! The suite's own skip policy, pinned so it cannot loosen in silence.
//!
//! # Why the harness needs a gate of its own
//!
//! Every other gate in this directory measures the PRODUCT. None measures the
//! thing that decides whether a gate runs at all, and that decision is the one
//! with the worst failure mode: when it is wrong, every gate downstream reports
//! a green it did not earn.
//!
//! Measured 2026-09-04, by an agent chasing an unrelated defect: a gate whose
//! binary was older than `src/` returned early, `libtest` printed `ok`, and the
//! "This is NOT a pass" line went to stderr where nothing looks without
//! `--nocapture`. A green e2e suite immediately after editing `src/` was not
//! evidence about the edit, and nothing on screen said so.
//!
//! `common::enforce_strict` already turns every announced skip into a failure
//! under `--features strict-gates`, which `scripts/ci-check.sh` enables. CI was
//! therefore covered and the daily `cargo test` was not. The repair narrows to
//! the one state where a skip actively misleads, and this file is what stops
//! that narrowing from being widened back by a future edit.

mod common;

use common::{must_refuse, BinaryState};
use std::path::PathBuf;

/// A binary older than `src/` must REFUSE, never skip.
///
/// This is the case the whole repair exists for. If it ever returns `false`
/// again, the suite goes back to printing `ok` for gates that ran against code
/// the binary does not contain.
#[test]
fn a_stale_binary_refuses_instead_of_skipping() {
    assert!(
        must_refuse(&BinaryState::Stale),
        "a gate whose binary predates src/ must fail loudly; skipping prints `ok` and the \
         operator reads it as evidence about the edit they just made"
    );
}

/// A missing binary may still skip, and that asymmetry is the point.
///
/// Refusing here would fail the suite on any environment that has not built the
/// product — a gate that fails for a condition the operator cannot act on from
/// inside the test run is a gate someone switches off, and switched off it stops
/// catching the stale case too.
#[test]
fn an_absent_binary_still_skips_because_the_two_states_mean_opposite_things() {
    assert!(
        !must_refuse(&BinaryState::Absent),
        "an unbuilt tree has nothing to exercise and a remedy the operator can act on; \
         refusing here would push someone to disable the check entirely"
    );
}

/// The ready path must not refuse, or every gate in the suite dies at once.
///
/// Trivial to satisfy and worth pinning anyway: `must_refuse` is a match over
/// three arms, and an edit that inverts the default takes the whole suite with
/// it. This case is the one that would fail first and name the reason.
#[test]
fn a_ready_binary_never_refuses() {
    assert!(
        !must_refuse(&BinaryState::Ready(PathBuf::from(
            "target/debug/browser-automation-cli"
        ))),
        "a usable binary must let its gate run"
    );
}
