// SPDX-License-Identifier: MIT OR Apache-2.0
//! Multi-step script driver command rows.

use super::CmdRow;

/// Scripting family rows in table order.
pub(super) const ROWS: &[CmdRow] = &[
    (
        "run",
        "sequential_justified",
        None,
        Some("ordered script (N-134); internal steps may fan-out"),
    ),
    (
        "exec",
        "sequential_justified",
        None,
        Some("ordered script (N-134)"),
    ),
];
