// SPDX-License-Identifier: MIT OR Apache-2.0
//! Nested multi-item / disk subcommand rows with dotted keys (PAR-76).

use super::CmdRow;

/// Nested subcommand rows in table order.
pub(super) const ROWS: &[CmdRow] = &[
    (
        "console.list",
        "mixed",
        Some("filter_cpu when large"),
        Some("type/sw filter on capture buffer"),
    ),
    (
        "console.dump",
        "mixed",
        Some("write_bytes_blocking"),
        Some("serialize+disk off async/block_on worker"),
    ),
    (
        "net.list",
        "mixed",
        Some("filter_cpu when large"),
        Some("resource_type filter on capture buffer"),
    ),
    (
        "net.get",
        "mixed",
        Some("write_bytes_blocking on --path"),
        Some("optional request/response path dumps"),
    ),
    (
        "heap.dup-strings",
        "parallel_cpu",
        Some("map_cpu"),
        Some("independent string score after idom"),
    ),
    (
        "heap.summary",
        "mixed",
        Some("map_cpu when large"),
        Some("offline parse; graph passes sequential"),
    ),
    (
        "heap.take",
        "mixed",
        Some("write_bytes_blocking"),
        Some("CDP chunks join then disk off async"),
    ),
    (
        "mitm.domains",
        "parallel_cpu",
        Some("map_cpu"),
        Some("host extract over capture items"),
    ),
    (
        "mitm.apis",
        "parallel_cpu",
        Some("map_cpu"),
        Some("API classify over capture items"),
    ),
    (
        "assert.console",
        "mixed",
        Some("filter_cpu when large"),
        Some("level filter on console buffer"),
    ),
    (
        "assert.console-empty",
        "sequential_justified",
        None,
        Some("count check; cost ≪ overhead"),
    ),
    (
        "assert.console-no-match",
        "mixed",
        Some("filter_cpu when large"),
        Some("pattern filter on console buffer"),
    ),
    (
        "state.save",
        "mixed",
        Some("write_bytes_blocking + create_dir_all_blocking"),
        Some("CDP collect then disk off async"),
    ),
    (
        "state.load",
        "mixed",
        Some("read_bytes_blocking; multi-origin sequential N-143"),
        Some("disk off async; navigates sequential"),
    ),
    (
        "state.list",
        "sequential_justified",
        None,
        Some("few session files; cost ≪ Rayon"),
    ),
    (
        "perf.stop",
        "mixed",
        Some("write_bytes_blocking"),
        Some("trace dump off async worker"),
    ),
    (
        "perf.insight",
        "mixed",
        Some("map_cpu when large"),
        Some("offline event fold with threshold"),
    ),
    (
        "screencast.stop",
        "mixed",
        Some("spawn_blocking+rayon frames"),
        Some("decode+write N frames"),
    ),
];
