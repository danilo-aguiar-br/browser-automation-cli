// SPDX-License-Identifier: MIT OR Apache-2.0
//! Redis/RESP transport budgets and L2 cache TTLs.
//!
//! Infra timeout / poll constants (HARD-01 anti-hardcode).
//!
//! Operator-facing wall timeouts remain XDG `timeout` / CLI `--timeout`.
//! These values are process-infra only (probes, grace, I/O helpers).

/// Redis port assumed when `cache_redis_url` names a host without one.
///
/// Used ONLY for the absent-port case. A port that is present but unparseable is
/// an error, never this value: silently connecting to 6379 while the operator
/// wrote something else is how a cache "works" against the wrong server.
pub const REDIS_DEFAULT_PORT: u16 = 6379;

/// Redis/RESP connect and stream I/O timeout (seconds).
pub const REDIS_IO_TIMEOUT_SECS: u64 = 3;

/// Redis TCP connect deadline (seconds). Override: XDG `redis_connect_timeout_secs`.
pub const REDIS_CONNECT_TIMEOUT_SECS: u64 = 2;

/// Short Redis mock/test helper I/O timeout (seconds).
pub const REDIS_SHORT_IO_TIMEOUT_SECS: u64 = 2;

/// Redis RESP bulk string size ceiling (bytes).
pub const CACHE_MAX_RESP_BULK_BYTES: usize = 16 * 1024 * 1024;

/// Redis RESP line size ceiling (bytes).
pub const CACHE_MAX_RESP_LINE_BYTES: usize = 16 * 1024 * 1024;

/// HTTP scrape response L2 cache TTL (seconds).
pub const SCRAPE_HTTP_CACHE_TTL_SECS: u64 = 3600;

/// Local file-parse L2 cache TTL (seconds; 24h).
pub const FILE_PARSE_CACHE_TTL_SECS: u64 = 86_400;
