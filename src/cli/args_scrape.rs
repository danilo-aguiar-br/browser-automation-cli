// SPDX-License-Identifier: MIT OR Apache-2.0
//! Argument structs for the scrape family.
//!
//! # Why these left the enum
//!
//! `Commands` declared every field of every variant inline, which put
//! `src/cli/commands.rs` past the 300-code-line gate for three releases behind
//! an exception that claimed the enum was indivisible. It is not: the standard
//! clap shape is one field per variant — `Crawl(CrawlArgs)` — with the
//! arguments in a family module.
//!
//! The enum stays ONE enum, so exhaustiveness is untouched and the dispatcher
//! keeps its single exhaustive `match`. Only the field declarations move, and
//! the dispatcher gets shorter too: an arm that used to name twenty-four
//! bindings now names one.

use clap::{ArgAction, Args, ValueHint};

/// Navigate and return body text / formats (local HTTP or CDP scrape)
#[derive(Debug, Clone, Args)]
pub struct ScrapeArgs {
    /// Absolute URL to fetch
    #[arg(value_hint = ValueHint::Url)]
    pub url: String,
    /// text | markdown | html | rawHtml | links | metadata | screenshot | summary | product | branding | images | jsonld | json | feed (CSV or repeatable)
    #[arg(long = "format", alias = "formats", value_delimiter = ',', num_args = 1.., default_value = "text")]
    pub format: Vec<String>,
    /// http (reqwest+scraper) or browser (CDP); default XDG scrape_default_engine (http)
    #[arg(long, default_value = "http")]
    pub engine: String,
    /// Prefer main/article content heuristics
    #[arg(long, action = ArgAction::SetTrue)]
    pub only_main_content: bool,
    /// Optional one-shot webhook POST of the result envelope data (127.0.0.1/operator URL)
    #[arg(long)]
    pub webhook_url: Option<String>,
    /// Project data fields (CSV); agent CLEAN STDOUT
    #[arg(long)]
    pub select: Option<String>,
    /// Cap text/markdown/html chars (0 = XDG scrape_max_text_chars)
    #[arg(long)]
    pub max_text_chars: Option<usize>,
    /// CSS selectors to include (repeatable)
    #[arg(long = "include-selector")]
    pub include_selector: Vec<String>,
    /// CSS selectors to exclude (repeatable)
    #[arg(long = "exclude-selector")]
    pub exclude_selector: Vec<String>,
    /// Redact email/phone/card-like patterns in text/markdown
    #[arg(long, action = ArgAction::SetTrue)]
    pub redact_pii: bool,
    /// Include content_hash (sha256 of text/markdown)
    #[arg(long, action = ArgAction::SetTrue)]
    pub with_content_hash: bool,
    /// JSON Schema file for format=json LLM extract
    #[arg(long, value_hint = ValueHint::FilePath)]
    pub schema_json: Option<std::path::PathBuf>,
    /// Question for format=json LLM extract
    #[arg(long)]
    pub question: Option<String>,
    /// Extra HTTP header `Name: value` (repeatable; cookies/auth)
    #[arg(long = "header")]
    pub header: Vec<String>,
    /// Browser engine: wait ms after navigation before capture (base waitFor)
    #[arg(long = "wait-ms", default_value_t = 0)]
    pub wait_ms: u64,

    /// CSS selector to read an attribute from, for `--format attributes` (repeatable)
    ///
    /// Pairs positionally with `--attribute-name`: the first selector goes
    /// with the first name, and so on. Counts that do not match are rejected
    /// at argv time rather than silently truncated, because a dropped pair is
    /// a question the caller thinks it asked and never did.
    #[arg(long = "attribute-selector", value_name = "CSS")]
    pub attribute_selector: Vec<String>,

    /// Attribute to read from the matching selector, for `--format attributes` (repeatable)
    #[arg(long = "attribute-name", value_name = "NAME")]
    pub attribute_name: Vec<String>,

    /// Act on the page before scraping it: one `run --script` step as JSON (repeatable)
    ///
    /// Same grammar as a `run --script` line, deliberately: one syntax for
    /// acting on a page, so a `record` capture stays replayable and there is
    /// no second dialect to learn. Example:
    /// `--action '{"cmd":"press","target":"#load-more"}'`
    ///
    /// Runs in order, after navigation and before extraction. Browser engine
    /// only — `--engine http` has no page to act on, and accepting the flag
    /// there would make it parse and do nothing.
    #[arg(long = "action", value_name = "JSON")]
    pub action: Vec<String>,

    /// Ignore the response cache and fetch from origin (default: XDG scrape_no_cache = false)
    ///
    /// A READ bypass only. The fresh response is still stored, so this
    /// refreshes the entry for later callers rather than leaving a stale one.
    ///
    /// There is no equivalent way to say this with `scrape_http_cache_ttl_secs`:
    /// a TTL of zero already means "never expires", which is the opposite.
    #[arg(long = "no-cache", num_args = 0..=1, default_missing_value = "true", require_equals = false)]
    pub no_cache: Option<bool>,
}

/// Scrape many URLs from a file (HTTP or browser engine, one-shot)
#[derive(Debug, Clone, Args)]
pub struct BatchScrapeArgs {
    /// File with one absolute URL per line
    #[arg(long, value_hint = ValueHint::FilePath)]
    pub urls_file: std::path::PathBuf,
    /// Output format(s) applied to every URL (CSV / repeatable)
    #[arg(long = "format", alias = "formats", value_delimiter = ',', num_args = 1.., default_value = "text")]
    pub format: Vec<String>,
    /// Concurrent HTTP fetches (`0` = use global `--max-concurrency` / auto)
    #[arg(long, default_value_t = 0)]
    pub concurrency: usize,
    /// http (default) or browser (CDP per URL; GAP-010)
    #[arg(long, default_value = "http")]
    pub engine: String,
    /// Project data fields (CSV)
    #[arg(long)]
    pub select: Option<String>,
    /// Cap text/markdown/html chars (0 = XDG default)
    #[arg(long)]
    pub max_text_chars: Option<usize>,
    /// Filter pages: key=value AND expressions (e.g. http_error=false)
    #[arg(long)]
    pub filter: Option<String>,
    /// json (default), ndjson, or csv (header row)
    #[arg(long, default_value = "json")]
    pub output_mode: String,
    /// Sort pages by field (asc)
    #[arg(long)]
    pub sort: Option<String>,
    /// Deduplicate pages by field (first wins)
    #[arg(long = "dedup-key")]
    pub dedup_key: Option<String>,
    /// Collapse near-duplicate results by content similarity; threshold is XDG
    /// scrape_dedup_similar_distance (default: XDG scrape_dedup_similar = false)
    #[arg(long, num_args = 0..=1, default_missing_value = "true", require_equals = false)]
    pub dedup_similar: Option<bool>,
    /// CSS selectors to include (repeatable)
    #[arg(long = "include-selector")]
    pub include_selector: Vec<String>,
    /// CSS selectors to exclude (repeatable)
    #[arg(long = "exclude-selector")]
    pub exclude_selector: Vec<String>,
    /// Redact PII in text/markdown
    #[arg(long, action = ArgAction::SetTrue)]
    pub redact_pii: bool,
    /// Include content_hash
    #[arg(long, action = ArgAction::SetTrue)]
    pub with_content_hash: bool,
}

/// Crawl from a seed URL (HTTP BFS or browser, one-shot)
#[derive(Debug, Clone, Args)]
pub struct CrawlArgs {
    /// Seed URL the breadth-first crawl starts from
    #[arg(value_hint = ValueHint::Url)]
    pub url: String,
    /// Maximum number of pages to fetch
    #[arg(long, alias = "max-pages", default_value_t = 20)]
    pub limit: usize,
    /// Maximum link depth from the seed
    #[arg(long, default_value_t = 2)]
    pub max_depth: usize,
    /// Output format(s) applied to every page (CSV or repeatable)
    #[arg(long = "format", alias = "formats", value_delimiter = ',', num_args = 1.., default_value = "text")]
    pub format: Vec<String>,
    /// Stay on seed host
    #[arg(long, default_value_t = true)]
    pub same_host: bool,
    /// http (default) or browser (GAP-010)
    #[arg(long, default_value = "http")]
    pub engine: String,
    /// Project data fields (CSV)
    #[arg(long)]
    pub select: Option<String>,
    /// Cap text/markdown/html chars (0 = XDG default)
    #[arg(long)]
    pub max_text_chars: Option<usize>,
    /// Filter pages: key=value AND expressions
    #[arg(long)]
    pub filter: Option<String>,
    /// json (default), ndjson, csv, or llms-txt (site summary for models)
    #[arg(long, default_value = "json")]
    pub output_mode: String,
    /// Resolve and print the effective plan without fetching anything
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,
    /// Include only path prefixes (repeatable)
    #[arg(long = "include-path")]
    pub include_path: Vec<String>,
    /// Exclude path prefixes (repeatable)
    #[arg(long = "exclude-path")]
    pub exclude_path: Vec<String>,
    /// Seed frontier from sitemap.xml (default: XDG scrape_use_sitemap)
    #[arg(long, num_args = 0..=1, default_missing_value = "true", require_equals = false)]
    pub use_sitemap: Option<bool>,
    /// Collapse query params when deduping URLs
    #[arg(long, action = ArgAction::SetTrue)]
    pub ignore_query_params: bool,
    /// Follow `rel=next` pagination links (default: XDG scrape_follow_rel_next = false)
    #[arg(long, num_args = 0..=1, default_missing_value = "true", require_equals = false)]
    pub follow_rel_next: Option<bool>,
    /// Collapse near-duplicate pages by content similarity; threshold is XDG
    /// scrape_dedup_similar_distance (default: XDG scrape_dedup_similar = false)
    #[arg(long, num_args = 0..=1, default_missing_value = "true", require_equals = false)]
    pub dedup_similar: Option<bool>,
    /// Sort pages by field
    #[arg(long)]
    pub sort: Option<String>,
    /// Deduplicate pages by field
    #[arg(long = "dedup-key")]
    pub dedup_key: Option<String>,
    /// CSS include selectors
    #[arg(long = "include-selector")]
    pub include_selector: Vec<String>,
    /// CSS exclude selectors
    #[arg(long = "exclude-selector")]
    pub exclude_selector: Vec<String>,
    /// Redact PII
    #[arg(long, action = ArgAction::SetTrue)]
    pub redact_pii: bool,
    /// Include content_hash
    #[arg(long, action = ArgAction::SetTrue)]
    pub with_content_hash: bool,
}

/// Map site URLs from a seed (HTTP)
#[derive(Debug, Clone, Args)]
pub struct MapArgs {
    /// Seed URL to expand into a URL list
    #[arg(value_hint = ValueHint::Url)]
    pub url: String,
    /// Maximum number of URLs to return
    #[arg(long, default_value_t = 50)]
    pub limit: usize,
    /// Maximum link depth from the seed
    #[arg(long, default_value_t = 2)]
    pub max_depth: usize,
    /// Project data fields (CSV)
    #[arg(long)]
    pub select: Option<String>,
    /// Include only path prefixes (repeatable)
    #[arg(long = "include-path")]
    pub include_path: Vec<String>,
    /// Exclude path prefixes (repeatable)
    #[arg(long = "exclude-path")]
    pub exclude_path: Vec<String>,
    /// Enrich with sitemap.xml (default: XDG scrape_use_sitemap)
    #[arg(long, num_args = 0..=1, default_missing_value = "true", require_equals = false)]
    pub use_sitemap: Option<bool>,
    /// Filter URLs by substring (case-insensitive)
    #[arg(long)]
    pub search: Option<String>,
    /// Sort urls
    #[arg(long)]
    pub sort: Option<String>,
    /// Deduplicate urls key
    #[arg(long = "dedup-key")]
    pub dedup_key: Option<String>,
    /// Only return sitemap URLs (no HTML link BFS)
    #[arg(long, action = ArgAction::SetTrue)]
    pub sitemap_only: bool,
}

/// Local search (HTTP SERP links or URL map)
#[derive(Debug, Clone, Args)]
pub struct SearchArgs {
    /// Search terms sent to the configured HTML endpoint
    pub query: String,
    /// Maximum number of results to return
    #[arg(long, default_value_t = 10)]
    pub limit: usize,
    /// Project data fields (CSV)
    #[arg(long)]
    pub select: Option<String>,
    /// Sort results by field
    #[arg(long)]
    pub sort: Option<String>,
    /// Deduplicate results by field
    #[arg(long = "dedup-key")]
    pub dedup_key: Option<String>,
}
