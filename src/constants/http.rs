// SPDX-License-Identifier: MIT OR Apache-2.0
//! HTTP client, robots, and webhook infra constants.

/// Default total HTTP request timeout for the process-wide async client (seconds).
///
/// Also the default robots fetch timeout. Override: XDG `http_timeout_secs`.
pub const ROBOTS_FETCH_TIMEOUT_SECS: u64 = 30;

/// Alias for the shared HTTP client total timeout default.
pub const DEFAULT_HTTP_TIMEOUT_SECS: u64 = ROBOTS_FETCH_TIMEOUT_SECS;

/// HTTP connect-phase timeout for the process-wide clients (seconds).
///
/// Override: XDG `http_connect_timeout_secs`.
pub const DEFAULT_HTTP_CONNECT_TIMEOUT_SECS: u64 = 10;

/// Robots.txt HEAD/probe timeout (seconds).
pub const ROBOTS_PROBE_TIMEOUT_SECS: u64 = 5;

/// Max robots.txt body bytes (anti-OOM).
pub const ROBOTS_MAX_BODY_BYTES: usize = 512 * 1024;

/// Default max HTTP scrape body bytes. Override: XDG `scrape_max_body_bytes`.
pub const DEFAULT_SCRAPE_MAX_BODY_BYTES: usize = 5_000_000;

/// Default max body for browser-engine scrape helpers.
pub const DEFAULT_BROWSER_SCRAPE_MAX_BODY_BYTES: usize = 2_000_000;

/// Max HTTP redirects followed by product clients.
pub const HTTP_REDIRECT_MAX: usize = 10;

/// reqwest pool max idle connections per host (one-shot process).
pub const HTTP_POOL_MAX_IDLE_PER_HOST: usize = 4;

/// Default LLM/webhook blocking HTTP client timeout (seconds).
///
/// Override: XDG `llm_http_timeout_secs`.
pub const DEFAULT_LLM_HTTP_TIMEOUT_SECS: u64 = 60;

/// Operator webhook POST timeout (seconds).
pub const WEBHOOK_POST_TIMEOUT_SECS: u64 = 15;

/// Webhook retry base delay (milliseconds); doubles each attempt.
pub const WEBHOOK_RETRY_BASE_DELAY_MS: u64 = 50;

/// Webhook max attempts (inclusive of first try).
pub const WEBHOOK_MAX_ATTEMPTS: u32 = 3;

/// Default max text/markdown chars in scrape envelopes (agent anti-token).
///
/// Override: XDG `scrape_max_text_chars`. `0` means no cap.
pub const DEFAULT_SCRAPE_MAX_TEXT_CHARS: usize = 32_768;

/// Floor delay between GETs to the same origin (milliseconds).
///
/// Override: XDG `scrape_min_delay_ms`. Robots Crawl-delay wins when larger.
pub const DEFAULT_SCRAPE_MIN_DELAY_MS: u64 = 0;

/// Max characters kept for each link `text` field (anti-token).
pub const DEFAULT_SCRAPE_LINK_TEXT_CHARS: usize = 200;

/// Default max chars for `summary` format. Override: XDG `scrape_summary_chars`.
pub const DEFAULT_SCRAPE_SUMMARY_CHARS: usize = 400;

/// HTML5 charset sniffing window (bytes). Override: XDG `scrape_charset_peek_bytes`.
pub const DEFAULT_SCRAPE_CHARSET_PEEK_BYTES: usize = 4096;

/// Max sitemap body bytes. Override: XDG `scrape_sitemap_max_bytes`.
pub const DEFAULT_SCRAPE_SITEMAP_MAX_BYTES: usize = 512 * 1024;

/// Politeness delay jitter ratio (0.0 = none, 0.2 = ±20%). Override: XDG `scrape_delay_jitter_ratio`.
pub const DEFAULT_SCRAPE_DELAY_JITTER_RATIO: f64 = 0.2;

/// Default scrape engine when CLI omits `--engine`. Override: XDG `scrape_default_engine`.
pub const DEFAULT_SCRAPE_ENGINE: &str = "http";

/// Max feed entries kept by scrape `--format feed`.
///
/// Override: XDG `scrape_feed_max_entries`. Caps the agent token budget for
/// high-volume RSS/Atom endpoints; `feed_truncated` reports when it bites.
pub const DEFAULT_SCRAPE_FEED_MAX_ENTRIES: usize = 50;

/// Follow `rel=next` pagination links during crawl by default.
///
/// Override: XDG `scrape_follow_rel_next`. `false` keeps historical behaviour
/// where paginated series are only reached through ordinary anchors.
pub const DEFAULT_SCRAPE_FOLLOW_REL_NEXT: bool = false;

/// Collapse near-duplicate pages by content similarity during crawl/batch.
///
/// Override: XDG `scrape_dedup_similar`. Off by default because collapsing
/// changes how many pages the envelope emits, which an agent must opt into.
pub const DEFAULT_SCRAPE_DEDUP_SIMILAR: bool = false;

/// SimHash Hamming distance under which two pages count as near-duplicates.
///
/// Override: XDG `scrape_dedup_similar_distance`. `0` demands identical
/// fingerprints; the practical near-duplicate band is 3..=8 over 64 bits.
pub const DEFAULT_SCRAPE_DEDUP_SIMILAR_DISTANCE: u32 = 3;

/// Shingle width (in words) used to build SimHash content fingerprints.
///
/// Not operator-tunable: changing it changes fingerprint meaning, so it is a
/// compile-time property of the algorithm rather than a config knob.
pub const SCRAPE_SIMHASH_SHINGLE_WORDS: usize = 3;
