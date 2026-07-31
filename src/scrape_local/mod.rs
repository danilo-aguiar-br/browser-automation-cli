// SPDX-License-Identifier: MIT OR Apache-2.0
//! Local scrape/crawl/map/search/parse (one-shot HTTP and file extract; no SaaS).
//!
//! Engines:
//! - `http` — reqwest + scraper HTML (no Chrome)
//! - `browser` — chromiumoxide via [`crate::browser::OneShotSession`]
//!
//! # Workload
//!
//! - **HTTP engine:** **I/O-bound** network + **CPU** HTML parse off-async.
//!   Fan-out via `tokio::task::JoinSet` + `Arc<Semaphore>::acquire_owned` gated by
//!   [`crate::concurrency::effective_limit`]
//!   (or per-command `--concurrency`, `0` = process budget). Hard cap
//!   [`crate::concurrency::HARD_CAP`]. Parse uses `spawn_blocking` (docsrs:
//!   CPU work must not starve Tokio workers; bound by the same permit).
//! - **Browser engine:** **I/O-bound** CDP (subprocess Chrome). Sequential per
//!   shared session (product law one residual); use `--engine http` for fan-out.
//! - **Crawl:** BFS with **bounded parallel frontier** (JoinSet + budget);
//!   link discovery stays **inside** the permit; `abort_all` at limit; cancel token.
//! - Shared [`crate::robots::shared_http_client`] avoids per-request Client build.
//! - Compiled `Regex` patterns live in `LazyLock` (never recompile per call).
//!
//! ## Module map (componentization)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `types` | formats, options, User-Agent |
//! | `scheme` | HTTP engine URL validation |
//! | `http` | single-URL fetch + payload |
//! | `html` | DOM extractors / PII / markdown |
//! | `batch` | JoinSet batch scrape |
//! | `crawl` | BFS crawl + map |
//! | `search` | public HTML SERP search |
//! | `parse` | local file parse |
//! | `urls` | URL list file helpers |

mod batch;
mod crawl;
mod formats_map;
mod html;
mod http;
mod parse;
mod scheme;
mod search;
mod types;
mod urls;

#[cfg(test)]
mod tests;

pub use batch::batch_scrape_http;
pub use crawl::{crawl_http, map_http};
pub use formats_map::build_formats_map;
pub use html::redact_pii;
pub use http::{build_scrape_payload, scrape_http};
pub use parse::{parse_file, parse_file_opts};
pub use scheme::reject_non_http_scheme_for_http_engine;
pub use search::search_http;
pub use types::{ScrapeFormat, ScrapeOpts, HTTP_USER_AGENT};
pub use urls::{read_urls_file, sorted_keys};
