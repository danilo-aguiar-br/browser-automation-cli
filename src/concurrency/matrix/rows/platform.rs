// SPDX-License-Identifier: MIT OR Apache-2.0
//! Extension, third-party bridge, shell integration, and host-state command rows.

use super::CmdRow;

/// Platform family rows in table order.
pub(super) const ROWS: &[CmdRow] = &[
    (
        "extension",
        "mixed",
        Some("join_bounded multi-closeTarget"),
        Some("single load; multi-target unload fan-out"),
    ),
    (
        "devtools3p",
        "sequential_justified",
        None,
        Some("single bridge"),
    ),
    (
        "webmcp",
        "sequential_justified",
        None,
        Some("single tool call"),
    ),
    (
        "completions",
        "sequential_justified",
        None,
        Some("meta emit"),
    ),
    ("man", "sequential_justified", None, Some("meta emit")),
    (
        "install",
        "sequential_justified",
        None,
        Some("few version dirs"),
    ),
    (
        "state",
        "mixed",
        Some("write/read_bytes_blocking; multi-origin sequential N-143"),
        None,
    ),
    (
        "cache",
        "sequential_justified",
        None,
        Some("single key ops"),
    ),
    (
        "residual",
        "mixed",
        Some("index_proc_cmdlines once + map_cpu check/wipe"),
        Some("PAR-89/90: never N×/proc under Rayon"),
    ),
];
