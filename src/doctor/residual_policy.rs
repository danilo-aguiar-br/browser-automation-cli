// SPDX-License-Identifier: MIT OR Apache-2.0
//! Residual-hygiene policy: which counters are a defect, and which are noise.
//!
//! # Why this is a module and not a block inside `run`
//!
//! [`super::run::run_doctor`] assembles roughly a dozen independent probes. Every
//! other probe answers a yes/no question about the host — is Chrome there, is
//! ffmpeg there — and the answer needs no interpretation. Residual is the one
//! that does: [`crate::residual::residual_disk_report`] returns nine counters and
//! NONE of them is a verdict on its own. `cli_marker_dirs = 1` is a healthy
//! concurrent invocation or abandoned residue depending on `orphan_marker_dirs`,
//! and reading those counters wrongly is exactly the false positive GAP-002 and
//! GAP-006 were opened for.
//!
//! Interpretation is therefore a separate responsibility from assembly. Keeping
//! it here means the rule can be read, and changed, without scrolling past the
//! Chrome probe — and `run` goes back to doing one thing: collecting checks.

use serde_json::{json, Value};

use crate::residual::ResidualDiskReport;

/// The verdict this policy reaches, plus the checks that carry it.
pub(super) struct ResidualVerdict {
    /// Checks to append to the doctor checklist, in report order.
    pub checks: Vec<Value>,
    /// True when the verdict is a proven defect, so `doctor` must exit non-zero.
    pub failed: bool,
}

/// Decide the residual status and render its checks.
///
/// # The rule, stated once
///
/// - **fail** — proven defects only: orphan marker dirs (an owner that is dead
///   past the age floor) or ghost holders (a live CLI Chrome whose marker
///   profile dir is gone).
/// - **warn** — residue that exists but is not collectable: live siblings and
///   Chromium singleton leftovers. Nothing to do, so nothing to fail.
/// - **pass** — clean.
///
/// A live sibling invocation is healthy and never fails the check. The earlier
/// raw-marker rule made `doctor` report a false positive whenever two
/// invocations overlapped, which is the failure mode this split protects.
pub(super) fn residual_verdict(residual: &ResidualDiskReport) -> ResidualVerdict {
    let status = classify(residual);
    let mut checks = vec![residual_check(residual, status)];
    if let Some(extra) = process_table_check(residual) {
        checks.push(extra);
    }
    ResidualVerdict {
        checks,
        failed: status == "fail",
    }
}

/// Map the counters onto `pass` / `warn` / `fail`.
fn classify(residual: &ResidualDiskReport) -> &'static str {
    if residual.orphan_marker_dirs > 0 || residual.ghost_marker_processes > 0 {
        "fail"
    } else if residual.cli_marker_dirs > 0 || residual.chromium_tmp_singleton_orphans > 0 {
        "warn"
    } else {
        "pass"
    }
}

/// One-line prose for the verdict.
///
/// The message names the VERDICT, not just the counters. A reader who sees
/// `cli_markers=1` alone cannot tell a healthy concurrent invocation from
/// abandoned residue; only `orphan_markers` separates the two, and that is
/// exactly the distinction `status` encodes.
fn verdict_prose(residual: &ResidualDiskReport, status: &str) -> &'static str {
    match status {
        "fail" if residual.ghost_marker_processes > 0 => {
            "ghost holders: live CLI Chrome with missing marker profile dir"
        }
        "fail" => "abandoned residue: a marker dir past the age floor has a dead owner",
        "warn" => "live siblings only: nothing is collectable, no action needed",
        _ => "clean",
    }
}

/// The `residual_disk` check.
fn residual_check(residual: &ResidualDiskReport, status: &'static str) -> Value {
    json!({
        "id": "residual_disk",
        "status": status,
        "message": format!(
            "{} — cli_markers={} orphan_markers={} ghost_marker_procs={} chromium_singleton_orphans={} scavenge_safe={} sibling_live_procs={} foreign_root_orphans={} proc_table_unavailable={} scanned_roots={}",
            verdict_prose(residual, status),
            residual.cli_marker_dirs,
            residual.orphan_marker_dirs,
            residual.ghost_marker_processes,
            residual.chromium_tmp_singleton_orphans,
            residual.scavenge_safe_candidates,
            residual.sibling_live_processes,
            residual.foreign_root_orphans,
            residual.process_table_unavailable,
            residual.scanned_roots.len()
        ),
        "cli_marker_dirs": residual.cli_marker_dirs,
        "chromium_tmp_singleton_orphans": residual.chromium_tmp_singleton_orphans,
        "scavenge_safe_candidates": residual.scavenge_safe_candidates,
        "live_cli_marker_processes": residual.live_cli_marker_processes,
        "sibling_live_processes": residual.sibling_live_processes,
        "orphan_marker_dirs": residual.orphan_marker_dirs,
        // Present in `data.residual` since 0.1.7 but missing here, so the check
        // and the report disagreed about what residual even consists of. Two
        // views of one fact must carry the same fields or a consumer picks the
        // wrong one and is right about nothing.
        "foreign_root_orphans": residual.foreign_root_orphans,
        "ghost_marker_processes": residual.ghost_marker_processes,
        "scanned_roots": residual.scanned_roots,
        "process_table_unavailable": residual.process_table_unavailable,
    })
}

/// GAP-045: a host without a readable process table cannot prove liveness, so
/// the collector refuses every wipe. Surface that as its own check instead of
/// letting residue counts drift upward with no explanation.
fn process_table_check(residual: &ResidualDiskReport) -> Option<Value> {
    if !residual.process_table_unavailable {
        return None;
    }
    Some(json!({
        "id": "residual_process_table",
        "status": "warn",
        "message": "live process table unavailable: residual GC refuses wipes (fail-closed)",
        "available": false,
    }))
}
