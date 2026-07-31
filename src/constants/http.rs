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
