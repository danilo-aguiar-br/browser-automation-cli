// SPDX-License-Identifier: MIT OR Apache-2.0
//! Page inspection and capture-buffer command rows.

use super::CmdRow;

/// Inspection family rows in table order.
pub(super) const ROWS: &[CmdRow] = &[
    (
        "extract",
        "sequential_justified",
        None,
        Some("single target or single LLM call"),
    ),
    ("text", "sequential_justified", None, Some("single target")),
    ("scroll", "sequential_justified", None, Some("single act")),
    (
        "cookie",
        "sequential_justified",
        None,
        Some("CDP cookie ops single session"),
    ),
    ("attr", "sequential_justified", None, Some("single target")),
    (
        "assert",
        "sequential_justified",
        None,
        Some("single check; console filters use filter_cpu"),
    ),
    (
        "console",
        "mixed",
        Some("filter_cpu when large"),
        Some("buffer filter threshold; dump write_bytes_blocking"),
    ),
    (
        "net",
        "mixed",
        Some("filter_cpu when large"),
        Some("buffer filter threshold; get path write_bytes_blocking"),
    ),
    (
        "page",
        "sequential_justified",
        None,
        Some("tab ops on single browser; multi-attach at launch"),
    ),
    (
        "dialog",
        "sequential_justified",
        None,
        Some("single dialog handle"),
    ),
];
