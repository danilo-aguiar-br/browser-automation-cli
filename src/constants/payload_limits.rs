// SPDX-License-Identifier: MIT OR Apache-2.0
//! Input-size ceilings and anti-DoS budget clamps for files and crawls.

/// Upper bound for a single source file read by `sg` scan/rewrite (16 MiB).
///
/// Prevents hostile or accidental giant files from forcing an unbounded
/// `read_to_string` allocation (rules_rust_gerenciamento_memoria — fallible
/// allocation budget on external input). Named constant; not a product env.
pub const MAX_SG_FILE_BYTES: u64 = 16 * 1024 * 1024;

/// Default ceiling for one JSON/NDJSON **script or manifest** file (bytes).
///
/// Operator override: XDG `config set max_json_file_bytes <n>` (`> 0`).
pub const DEFAULT_MAX_JSON_FILE_BYTES: u64 = 32 * 1024 * 1024;

/// Default ceiling for one NDJSON line (bytes).
///
/// Operator override: XDG `config set max_ndjson_line_bytes <n>` (`> 0`).
pub const DEFAULT_MAX_NDJSON_LINE_BYTES: usize = 1024 * 1024;

/// Default ceiling for CLI flag JSON payloads (`--fields-json`, cookies, …).
///
/// Operator override: XDG `config set max_cli_json_payload_bytes <n>` (`> 0`).
pub const DEFAULT_MAX_CLI_JSON_PAYLOAD_BYTES: usize = 4 * 1024 * 1024;

/// Max crawl page budget (anti-DoS clamp for `--limit`).
pub const SCRAPE_CRAWL_LIMIT_MAX: usize = 500;
/// Max BFS depth for crawl/map.
pub const SCRAPE_CRAWL_MAX_DEPTH: usize = 10;
/// Max search result budget (anti-DoS clamp).
pub const SCRAPE_SEARCH_LIMIT_MAX: usize = 50;
/// Max local file parse size (bytes) before reject.
pub const SCRAPE_MAX_PARSE_BYTES: usize = 50_000_000;
