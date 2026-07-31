// SPDX-License-Identifier: MIT OR Apache-2.0
//! Single-act DOM interaction command rows.

use super::CmdRow;

/// Interaction family rows in table order.
pub(super) const ROWS: &[CmdRow] = &[
    (
        "goto",
        "sequential_justified",
        None,
        Some("single interactive act (N-138)"),
    ),
    (
        "view",
        "mixed",
        Some("join_bounded multi-ref CDP"),
        Some("snapshot internal fan-out"),
    ),
    (
        "press",
        "sequential_justified",
        None,
        Some("single DOM act (N-135)"),
    ),
    (
        "click-at",
        "sequential_justified",
        None,
        Some("single coordinate act"),
    ),
    (
        "write",
        "sequential_justified",
        None,
        Some("single fill act"),
    ),
    (
        "keys",
        "sequential_justified",
        None,
        Some("ordered key events"),
    ),
    (
        "type",
        "sequential_justified",
        None,
        Some("ordered chars (N-141)"),
    ),
    (
        "wait",
        "sequential_justified",
        None,
        Some("poll loop single page"),
    ),
    ("hover", "sequential_justified", None, Some("single act")),
    (
        "drag",
        "sequential_justified",
        None,
        Some("ordered pointer path"),
    ),
    (
        "fill-form",
        "sequential_justified",
        None,
        Some("DOM focus order (N-135)"),
    ),
    (
        "select-option",
        "sequential_justified",
        None,
        Some("single act"),
    ),
    ("pick", "sequential_justified", None, Some("single act")),
    ("upload", "sequential_justified", None, Some("single act")),
];
