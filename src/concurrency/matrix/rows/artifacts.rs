// SPDX-License-Identifier: MIT OR Apache-2.0
//! Artifact-producing command rows (eval result, screenshot, PDF, baseline).

use super::CmdRow;

/// Artifact family rows in table order.
pub(super) const ROWS: &[CmdRow] = &[
    (
        "eval",
        "mixed",
        Some("write_bytes_blocking on --file"),
        Some("single JS; disk off async"),
    ),
    (
        "grab",
        "mixed",
        Some("join_bounded multi-rect + save_screenshot_async"),
        Some("multi-target CDP"),
    ),
    (
        "print-pdf",
        "mixed",
        Some("write_bytes_blocking"),
        Some("single PDF off async worker"),
    ),
    (
        "monitor",
        "sequential_justified",
        None,
        Some("single baseline hash path"),
    ),
];
