// SPDX-License-Identifier: MIT OR Apache-2.0
//! Local filesystem, lint, sheet, MITM, workflow, and config command rows.

use super::CmdRow;

/// Local tooling family rows in table order.
pub(super) const ROWS: &[CmdRow] = &[
    (
        "find-paths",
        "parallel_cpu",
        Some("WalkBuilder.threads + multi-root flat_map (no Mutex)"),
        None,
    ),
    (
        "sg-scan",
        "parallel_cpu",
        Some("multi-root par + par_iter files + sort_cpu"),
        None,
    ),
    (
        "sg-rewrite",
        "mixed",
        Some("dry-run par+sort_cpu; --apply sequential N-136"),
        None,
    ),
    (
        "sheet-write",
        "sequential_justified",
        None,
        Some("single writer N-137"),
    ),
    (
        "mitm",
        "mixed",
        Some("CA read_to_string_blocking; map_cpu+sort_cpu list filters"),
        None,
    ),
    (
        "workflow",
        "sequential_justified",
        None,
        Some("SQLite single-writer N-130"),
    ),
    (
        "config",
        "sequential_justified",
        None,
        Some("single config file"),
    ),
];
