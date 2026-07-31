// SPDX-License-Identifier: MIT OR Apache-2.0
//! Meta/discovery command rows (no Chrome, no disk fan-out).

use super::CmdRow;

/// Meta family rows in table order.
pub(super) const ROWS: &[CmdRow] = &[
    (
        "doctor",
        "sequential_justified",
        None,
        Some("cheap path/which probes; cost ≪ Rayon (N-144/PAR-57)"),
    ),
    (
        "commands",
        "sequential_justified",
        None,
        Some("meta inventory"),
    ),
    (
        "schema",
        "sequential_justified",
        None,
        Some("meta schema emit"),
    ),
    ("version", "sequential_justified", None, Some("meta")),
    ("locale", "sequential_justified", None, Some("meta")),
];
