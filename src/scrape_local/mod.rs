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
//! | `http` | single-URL fetch (robots, cache, retry, charset) |
//! | `payload` | per-format agent envelope construction |
//! | `html` | link/image/branding extraction + HTML helper facade |
//! | `html_jsonld` | JSON-LD blocks and Product typing |
//! | `html_sanitize` | selector-based DOM reduction and PII redaction |
//! | `html_text` | DOM text nodes and `<meta>` values |
//! | `html_markdown` | HTML to Markdown conversion |
//! | `batch` | JoinSet batch scrape |
//! | `crawl` | BFS crawl engine |
//! | `crawl_frontier` | frontier admission (normalize, dedup, host/path gate) |
//! | `crawl_map` | site map URL discovery |
//! | `project` | field projection and text caps |
//! | `page_ops` | row filter / sort / dedup |
//! | `error_page` | structured failure rows |
//! | `feed` | RSS/Atom/JSON Feed entry extraction |
//! | `rel_next` | `rel=next` pagination discovery |
//! | `dedup_similar` | near-duplicate row collapse by SimHash |
//! | `simhash` | SimHash content fingerprints for near-duplicate collapse |
//! | `search` | public HTML SERP search |
//! | `parse` | local file parse |
//! | `urls` | URL list file helpers |

mod batch;
mod crawl;
mod crawl_frontier;
mod crawl_map;
mod dedup_similar;
mod directives;
mod emit;
mod encoding;
mod error_page;
mod feed;
mod formats_map;
mod html;
mod html_jsonld;
mod html_markdown;
mod html_meta;
mod html_sanitize;
mod html_text;
mod http;
mod llms_txt;
mod page_ops;
mod parse;
mod path_filter;
mod payload;
mod project;
mod rel_next;
mod scheme;
mod search;
mod simhash;
mod sitemap;
mod types;
mod urls;

#[cfg(test)]
mod tests;

pub use batch::batch_scrape_http;
pub use crawl::crawl_http;
pub use crawl_map::map_http;
pub use dedup_similar::dedup_similar_pages_envelope;
pub use emit::{emit_csv_array, emit_ndjson_array, emit_scrape_collection};
pub use error_page::{http_error_page, status_from_error_message};
pub use formats_map::build_formats_map;
pub use html_sanitize::redact_pii;
pub use http::scrape_http;
pub use page_ops::{
    dedup_pages_envelope, filter_pages_envelope, page_matches_filter, sort_pages_envelope,
};
pub use parse::{parse_file, parse_file_opts};
pub use path_filter::{normalize_url_for_dedup, normalize_url_for_dedup_ex, PathFilter};
pub use payload::build_scrape_payload;
pub use project::{
    apply_max_text_chars, finalize_scrape_value, finalize_scrape_value_ex, project_fields,
    project_pages_envelope,
};
pub use scheme::reject_non_http_scheme_for_http_engine;
pub use search::search_http;
pub use simhash::SimHash;
pub use sitemap::{discover_sitemap_urls, parse_sitemap_xml};
pub use types::{ScrapeFormat, ScrapeOpts, HTTP_USER_AGENT};
pub use urls::read_urls_file;
