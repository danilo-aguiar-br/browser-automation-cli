// SPDX-License-Identifier: MIT OR Apache-2.0
//! Emulation, performance, and memory profiling command rows.

use super::CmdRow;

/// Profiling family rows in table order.
pub(super) const ROWS: &[CmdRow] = &[
    (
        "emulate",
        "sequential_justified",
        None,
        Some("single CDP device"),
    ),
    (
        "resize",
        "sequential_justified",
        None,
        Some("single viewport"),
    ),
    (
        "perf",
        "mixed",
        Some("write_bytes_blocking stop; map_cpu insight when large"),
        None,
    ),
    (
        "lighthouse",
        "sequential_justified",
        None,
        Some("single subprocess N-140"),
    ),
    (
        "screencast",
        "mixed",
        Some("spawn_blocking+rayon frames on stop"),
        None,
    ),
    (
        "heap",
        "mixed",
        Some("node parse par; idom seq N-142; map_cpu/sort_cpu; write_bytes_blocking"),
        None,
    ),
];
