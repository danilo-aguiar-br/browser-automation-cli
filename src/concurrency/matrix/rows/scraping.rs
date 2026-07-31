// SPDX-License-Identifier: MIT OR Apache-2.0
//! Scrape, crawl, map, search, and local parse command rows.

use super::CmdRow;

/// Scraping family rows in table order.
pub(super) const ROWS: &[CmdRow] = &[
    (
        "scrape",
        "mixed",
        Some("spawn_blocking parse (http)"),
        Some("single URL; batch for multi"),
    ),
    (
        "batch-scrape",
        "parallel_io",
        Some("JoinSet+Semaphore http; browser sequential N-129"),
        None,
    ),
    (
        "crawl",
        "parallel_io",
        Some("JoinSet+Semaphore http; browser sequential N-129"),
        None,
    ),
    ("map", "parallel_io", Some("crawl_http under budget"), None),
    (
        "search",
        "parallel_io",
        Some("scrape/map under budget"),
        None,
    ),
    (
        "parse",
        "mixed",
        Some("CPU parse sync path"),
        Some("single file"),
    ),
    ("qr", "sequential_justified", None, Some("single payload")),
];
