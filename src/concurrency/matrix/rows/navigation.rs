// SPDX-License-Identifier: MIT OR Apache-2.0
//! History navigation command rows.

use super::CmdRow;

/// Navigation family rows in table order.
pub(super) const ROWS: &[CmdRow] = &[
    (
        "back",
        "sequential_justified",
        None,
        Some("single navigation"),
    ),
    (
        "forward",
        "sequential_justified",
        None,
        Some("single navigation"),
    ),
    (
        "reload",
        "sequential_justified",
        None,
        Some("single navigation"),
    ),
];
