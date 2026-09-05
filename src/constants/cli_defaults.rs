// SPDX-License-Identifier: MIT OR Apache-2.0
//! Compile-time defaults for numeric CLI flags.
//!
//! These are the values clap prints in `--help` when the operator omits the
//! flag. They lived as bare literals inside `#[arg(default_value_t = …)]`, which
//! made the published default invisible to anyone reading the runtime and
//! impossible to reason about next to the ceiling it interacts with — a
//! `--limit` default of 20 means nothing without the `scrape_crawl_limit_max`
//! clamp beside it.
//!
//! Sentinel zeros are NOT here. `--timeout 0`, `--concurrency 0` and
//! `--wait-ms 0` do not mean "zero"; they mean "fall back to XDG or to auto",
//! and naming them would suggest a quantity where there is a mode.

/// Default `find-paths --limit`.
///
/// Path records are tiny and an operator listing them wants the whole picture;
/// the ceiling exists to bound a pathological tree, not to paginate.
pub const FIND_PATHS_LIMIT: usize = 10_000;

/// Default `--scale` for viewport emulation (1.0 = the host's own ratio).
pub const EMULATE_DEFAULT_SCALE: f64 = 1.0;

/// Default `sg-scan --limit` of structural findings.
///
/// A scan that reports more than this is telling the operator the rule is
/// mismatched rather than the code is broken, so truncating is the honest
/// answer and the envelope says it truncated.
pub const SG_SCAN_FINDINGS_LIMIT: usize = 500;

/// Default `mitm list --limit` of captured exchanges.
///
/// Clamped at runtime by XDG `mitm_list_limit_max`; a capture of a real page
/// holds thousands of exchanges and an unbounded default would put all of them
/// on stdout.
pub const MITM_LIST_LIMIT: usize = 100;

/// Default seconds for a MITM proxy window (`mitm start`, `mitm capture-url`).
///
/// Clamped at runtime by XDG `mitm_proxy_seconds_max`; this is only the value
/// used when the operator names no window at all.
pub const MITM_DEFAULT_SECONDS: u64 = 30;

/// Default `mitm graphql --limit` of discovered operations.
pub const MITM_GRAPHQL_LIMIT: usize = 100;

/// Default `mitm ws list --limit` of WebSocket frames.
pub const MITM_WS_FRAMES_LIMIT: usize = 100;

/// Default `crawl --limit` of pages.
///
/// Deliberately small: a crawl is the one verb that can turn a single command
/// into hundreds of requests against someone else's server, so the default is
/// a sample and the operator has to ask for a sweep.
pub const CRAWL_DEFAULT_LIMIT: usize = 20;

/// Default BFS depth for `crawl` and `map`.
pub const FRONTIER_DEFAULT_MAX_DEPTH: usize = 2;

/// Default `map --limit` of discovered URLs.
///
/// Higher than the crawl default because mapping fetches far less per URL.
pub const MAP_DEFAULT_LIMIT: usize = 50;

/// Default `search --limit` of results.
pub const SEARCH_DEFAULT_LIMIT: usize = 10;

/// Default `heap retainers --max-depth`.
///
/// `u64` to match the flag's own type: a retainer depth is compared against
/// snapshot node counts, which are `u64` throughout the heap graph.
pub const HEAP_RETAINERS_MAX_DEPTH: u64 = 8;
