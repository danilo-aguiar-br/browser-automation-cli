// SPDX-License-Identifier: MIT OR Apache-2.0
//! Capability table unit tests.

use super::*;

/// GAP-010: only snapshot capture escapes `--category-memory`.
#[test]
fn only_heap_take_is_free_of_category_memory() {
    let gated = [
        "close",
        "compare",
        "summary",
        "details",
        "class-nodes",
        "dominators",
        "dup-strings",
        "edges",
        "retainers",
        "paths",
        "object-details",
    ];
    assert_eq!(
        gated.len(),
        11,
        "reference surface gates eleven heap actions"
    );
    for action in gated {
        assert_eq!(
            required_capabilities("heap", Some(action)),
            &[Capability::Memory],
            "heap {action} must require --category-memory"
        );
    }
    assert!(
        required_capabilities("heap", Some("take")).is_empty(),
        "heap take is the one free action"
    );
}

/// The two actions GAP-010 reported as escaping the gate.
#[test]
fn heap_summary_and_close_are_gated() {
    for action in ["summary", "close"] {
        assert_eq!(
            required_capabilities("heap", Some(action)),
            &[Capability::Memory]
        );
    }
}

/// GAP-011: a disabled capability is never `usage`.
#[test]
fn disabled_capability_maps_to_exit_64() {
    let err = Capability::Memory.disabled_error("heap summary");
    assert_eq!(err.kind(), crate::error::ErrorKind::CapabilityDisabled);
    assert_eq!(err.exit_code(), 64);
    assert_eq!(err.kind().as_str(), "capability-disabled");
    assert!(
        err.message().contains("--category-memory"),
        "message must name the remediation flag: {}",
        err.message()
    );
}

/// GAP-020/041: an unmet precondition is never `usage`.
#[test]
fn unmet_precondition_maps_to_exit_75() {
    let err = Precondition::NoDialogOpen.unmet_error("press");
    assert_eq!(err.kind(), crate::error::ErrorKind::Precondition);
    assert_eq!(err.exit_code(), 75);
    assert_eq!(err.kind().as_str(), "precondition");
}

/// Lookup is specific-first, then falls back to the bare command row.
#[test]
fn lookup_prefers_action_row_then_falls_back() {
    assert!(required_capabilities("heap", Some("take")).is_empty());
    // Unlisted action inherits the command row.
    assert_eq!(
        required_capabilities("heap", Some("not-a-real-action")),
        &[Capability::Memory]
    );
    // No action at all also inherits it.
    assert_eq!(required_capabilities("heap", None), &[Capability::Memory]);
}

/// Underscore and case variants resolve to the same row.
#[test]
fn keys_normalize_underscore_and_case() {
    assert_eq!(
        required_capabilities("HEAP", Some("CLASS_NODES")),
        &[Capability::Memory]
    );
    assert_eq!(
        required_capabilities("click_at", None),
        &[Capability::Vision]
    );
    assert_eq!(
        required_capabilities("assert", Some("console_no_match")),
        &[Capability::CaptureConsole]
    );
}

/// GAP-029: console/net steps declare their capture buffer.
#[test]
fn capture_steps_declare_their_buffer() {
    assert_eq!(
        required_capabilities("console", None),
        &[Capability::CaptureConsole]
    );
    assert_eq!(
        required_capabilities("net", None),
        &[Capability::CaptureNetwork]
    );
    assert!(Capability::CaptureConsole.is_capture());
    assert!(Capability::CaptureNetwork.is_capture());
    assert!(!Capability::Memory.is_capture());
}

/// GAP-041: the dialog remedy and navigation must never be self-blocking.
#[test]
fn dialog_and_navigation_are_not_blocked() {
    for cmd in [
        "dialog", "goto", "back", "forward", "reload", "page", "wait",
    ] {
        assert!(
            required_preconditions(cmd, None).is_empty(),
            "{cmd} must stay reachable while a dialog is open"
        );
    }
}

/// GAP-041: the mutating and reading surfaces are guarded.
#[test]
fn interaction_and_capture_surfaces_are_dialog_blocked() {
    for cmd in [
        "view",
        "eval",
        "grab",
        "press",
        "click",
        "write",
        "type",
        "keys",
        "hover",
        "drag",
        "upload",
        "scroll",
        "emulate",
        "resize",
        "perf",
        "lighthouse",
        "heap",
        "net",
    ] {
        assert_eq!(
            required_preconditions(cmd, None),
            &[Precondition::NoDialogOpen],
            "{cmd} must be blocked while a dialog is open"
        );
    }
}

/// Every row must be reachable: no row shadowed by an earlier duplicate key.
#[test]
fn table_has_no_shadowed_rows() {
    let rows = capability_rows();
    for (i, row) in rows.iter().enumerate() {
        let first = rows
            .iter()
            .position(|r| r.cmd == row.cmd && r.action == row.action)
            .expect("row present");
        assert_eq!(
            first, i,
            "duplicate key {}.{:?} shadows row {i}",
            row.cmd, row.action
        );
    }
}

/// Action rows must precede their command fallback, or lookup never sees them.
#[test]
fn action_rows_precede_command_fallback() {
    let rows = capability_rows();
    for (i, row) in rows.iter().enumerate() {
        let Some(_) = row.action else { continue };
        if let Some(fallback) = rows
            .iter()
            .position(|r| r.cmd == row.cmd && r.action.is_none())
        {
            assert!(
                i < fallback,
                "{}.{:?} must be listed before the bare {} row",
                row.cmd,
                row.action,
                row.cmd
            );
        }
    }
}
