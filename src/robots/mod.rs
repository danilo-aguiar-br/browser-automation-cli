// SPDX-License-Identifier: MIT OR Apache-2.0
//! robots.txt honor for one-shot invocations (no cross-process cache).
//!
//! Default policy is honor. Override requires BOTH `--ignore-robots` and
//! `--i-accept-robots-risk`. Uses `robotstxt::DefaultMatcher` (Google port).
//!
//! # Workload
//!
//! **I/O-bound** (network GET of `/robots.txt`). Reuses process-wide
//! [`shared_http_client`](crate::robots::shared_http_client) so keep-alive / TLS session cache survive batch scrapes.
//! Not CPU-bound → no rayon. Not a long-lived daemon → no connection pool beyond
//! reqwest's internal pool for this one-shot process.

mod fetch;
mod policy;
mod politeness;

#[cfg(test)]
mod tests;

pub use fetch::enforce_robots;
pub use policy::*;
pub use politeness::{
    effective_delay_secs, origin_key, parse_crawl_delay_secs, remember_crawl_delay,
    remembered_delay_secs, wait_origin,
};

use std::sync::OnceLock;
use std::time::Duration;

use crate::constants::{HTTP_POOL_MAX_IDLE_PER_HOST, HTTP_REDIRECT_MAX, HTTP_USER_AGENT};
use crate::error::{CliError, ErrorKind};

/// Process-wide async HTTP client (rules: create `reqwest::Client` once, reuse).
///
/// Total + connect timeouts from XDG (`http_timeout_secs` /
/// `http_connect_timeout_secs`). **Never** honors system `HTTP_PROXY*` env
/// (`no_proxy` — product law: CLI+XDG only).
///
/// Uses `get_or_init` (stable). `get_or_try_init` is still unstable (`once_cell_try`);
/// on first failure we surface the error without poisoning the lock.
pub fn shared_http_client() -> Result<&'static reqwest::Client, CliError> {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    if let Some(c) = CLIENT.get() {
        return Ok(c);
    }
    let total = crate::xdg::resolve_http_timeout_secs();
    let connect = crate::xdg::resolve_http_connect_timeout_secs();
    let built = reqwest::Client::builder()
        .user_agent(HTTP_USER_AGENT)
        .timeout(Duration::from_secs(total))
        .connect_timeout(Duration::from_secs(connect))
        .redirect(reqwest::redirect::Policy::limited(HTTP_REDIRECT_MAX))
        .pool_max_idle_per_host(HTTP_POOL_MAX_IDLE_PER_HOST)
        // Explicit TCP_NODELAY: reduce small-response latency on robots/scrape
        // (rules_rust_latencia_reduzir — socket options on owned HTTP path).
        // CDP WebSocket is owned by chromiumoxide; not configurable here.
        .tcp_nodelay(true)
        // Product law: do not inherit HTTP_PROXY / HTTPS_PROXY / ALL_PROXY env.
        .no_proxy()
        .build()
        .map_err(|e| CliError::new(ErrorKind::Software, format!("http client: {e}")))?;
    Ok(CLIENT.get_or_init(|| built))
}
