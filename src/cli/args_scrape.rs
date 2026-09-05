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
    /// text | markdown | html | rawHtml | links | metadata | screenshot | summary | product | branding | images | jsonld | json | feed | attributes (CSV or repeatable)
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
    #[arg(long = "include-selector", value_parser = crate::scrape_local::validate_css_selector_arg)]
    pub include_selector: Vec<String>,
    /// CSS selectors to exclude (repeatable)
    #[arg(long = "exclude-selector", value_parser = crate::scrape_local::validate_css_selector_arg)]
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
    /// Prefer main/article content heuristics
    ///
    /// Present here because `scrape` has had it since it existed, and the
    /// natural way to build a batch is to validate one `scrape` and then swap
    /// the URL for `--urls-file`. Until 0.1.9 that swap exited 2 on this flag
    /// alone, while `--format`, `--include-selector`, `--exclude-selector`,
    /// `--max-text-chars`, `--select`, `--redact-pii` and `--with-content-hash`
    /// all crossed over — a single unguessable hole in an otherwise shared set,
    /// and one that only shows up after the URL list is already built.
    #[arg(long, action = ArgAction::SetTrue)]
    pub only_main_content: bool,
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
    #[arg(long = "include-selector", value_parser = crate::scrape_local::validate_css_selector_arg)]
    pub include_selector: Vec<String>,
    /// CSS selectors to exclude (repeatable)
    #[arg(long = "exclude-selector", value_parser = crate::scrape_local::validate_css_selector_arg)]
    pub exclude_selector: Vec<String>,
    /// Redact PII in text/markdown
    #[arg(long, action = ArgAction::SetTrue)]
    pub redact_pii: bool,
    /// Include content_hash
    #[arg(long, action = ArgAction::SetTrue)]
    pub with_content_hash: bool,
    /// Optional one-shot webhook POST of the collection envelope (operator URL)
    #[arg(long)]
    pub webhook_url: Option<String>,
}

/// Crawl from a seed URL (HTTP BFS or browser, one-shot)
#[derive(Debug, Clone, Args)]
pub struct CrawlArgs {
    /// Seed URL the breadth-first crawl starts from
    #[arg(value_hint = ValueHint::Url)]
    pub url: String,
    /// Maximum number of pages to fetch
    #[arg(long, alias = "max-pages", default_value_t = crate::constants::CRAWL_DEFAULT_LIMIT)]
    pub limit: usize,
    /// Maximum link depth from the seed
    #[arg(long, default_value_t = crate::constants::FRONTIER_DEFAULT_MAX_DEPTH)]
    pub max_depth: usize,
    /// Output format(s) applied to every page (CSV or repeatable)
    #[arg(long = "format", alias = "formats", value_delimiter = ',', num_args = 1.., default_value = "text")]
    pub format: Vec<String>,
    /// Stay on seed host (default)
    #[arg(long, default_value_t = true)]
    pub same_host: bool,
    /// Follow links off the seed host, disabling `--same-host`
    ///
    /// The sibling exists because `--same-host` is a bare `bool` with
    /// `default_value_t = true`, which clap derives as `SetTrue`. Measured
    /// 2026-09-01: `--same-host false` exits 2 with `unexpected argument
    /// 'false'`, so the ON value was the only one an operator could express and
    /// a cross-host crawl could not be requested at all. Same shape as
    /// `--require-image` / `--allow-non-image` elsewhere in this file, and
    /// additive on purpose: turning `same_host` into `Option<bool>` would make
    /// today's `--same-host` exit 2.
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub no_same_host: bool,
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
    /// Include paths/URLs matching this regex (repeatable)
    #[arg(long = "include-regex")]
    pub include_regex: Vec<String>,
    /// Exclude paths/URLs matching this regex (repeatable)
    #[arg(long = "exclude-regex")]
    pub exclude_regex: Vec<String>,
    /// Optional one-shot webhook POST of the collection envelope (operator URL)
    #[arg(long)]
    pub webhook_url: Option<String>,
    /// Seed frontier from sitemap.xml (default: XDG scrape_use_sitemap)
    #[arg(long, num_args = 0..=1, default_missing_value = "true", require_equals = false)]
    pub use_sitemap: Option<bool>,

    /// Fetch only the sitemap.xml frontier (no HTML link BFS)
    ///
    /// `map` already spells this, and a crawl that wants the same guarantee had
    /// to say `--use-sitemap --max-depth 0` and hope the two knobs meant what it
    /// assumed. On a site whose sitemap is authoritative, HTML link discovery
    /// only adds pages the operator did not ask for, on a budget it already
    /// spent. Forces `--use-sitemap` on and `--max-depth` to zero.
    #[arg(long, action = ArgAction::SetTrue)]
    pub sitemap_only: bool,

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
    /// Prefer main/article content heuristics
    ///
    /// Reached `scrape` since it existed and `batch-scrape` in 0.1.9, and
    /// stopped there. `crawl` carries the same selector and redaction family
    /// and was left without this one member. Found 2026-09-01 by
    /// `tests/selector_scope_gate.rs` on its first run — by the tree, this
    /// time, rather than by a user hitting exit 2 mid-crawl.
    #[arg(long, action = ArgAction::SetTrue)]
    pub only_main_content: bool,
    /// CSS include selectors
    #[arg(long = "include-selector", value_parser = crate::scrape_local::validate_css_selector_arg)]
    pub include_selector: Vec<String>,
    /// CSS exclude selectors
    #[arg(long = "exclude-selector", value_parser = crate::scrape_local::validate_css_selector_arg)]
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
    #[arg(long, default_value_t = crate::constants::MAP_DEFAULT_LIMIT)]
    pub limit: usize,
    /// Maximum link depth from the seed
    #[arg(long, default_value_t = crate::constants::FRONTIER_DEFAULT_MAX_DEPTH)]
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

    /// Keep URLs on subdomains of the seed host, not just the seed host itself
    ///
    /// A map is a site inventory, so a row on a host the caller never named is
    /// noise it has to filter downstream. The default is therefore the seed host
    /// exactly; this flag widens it to subdomains of that host, for sites that
    /// split docs, blog and app across them. Foreign hosts are dropped in both
    /// modes.
    ///
    /// BEHAVIOUR CHANGE: before this flag existed, `map` applied no host filter
    /// at all and returned external links harvested from crawled pages. Those
    /// rows are now dropped by default. Widening back to them is not what this
    /// flag does — it was never a deliberate feature, and a flag that only
    /// described the old behaviour would change nothing.
    ///
    /// Subdomain matching is textual against the seed host: `docs.example.com`
    /// matches under `example.com`. There is no public-suffix table, so a seed
    /// on `example.co.uk` matches its own subdomains but nothing is inferred
    /// about `co.uk` being a suffix rather than a domain.
    #[arg(long, action = ArgAction::SetTrue)]
    pub include_subdomains: bool,

    /// Collapse query params when deduping URLs
    ///
    /// `crawl` already spells this. Without it a paginated or tracked site
    /// returns the same page many times under `?page=`, `?utm_source=` and
    /// friends, and the `--limit` budget is spent on duplicates.
    #[arg(long, action = ArgAction::SetTrue)]
    pub ignore_query_params: bool,
}

/// Read an RSS, Atom or JSON Feed document and return its entries
///
/// # Why the flag list is short
///
/// A feed is parsed from the RAW body, because selector and main-content
/// reduction are HTML notions that would destroy an XML document. Every scrape
/// flag that shapes HTML is therefore not just unused here — offering it would
/// be offering a way to break the parse — and the engine is fixed to `http` for
/// the same reason: rendering a feed in Chrome yields the browser's XML viewer.
///
/// The number of entries kept comes from the XDG key `scrape_feed_max_entries`,
/// which is where it already lived; adding a flag that shadows a config key
/// would create two answers to one question.
#[derive(Debug, Clone, Args)]
pub struct FeedArgs {
    /// Feed URL (RSS, Atom or JSON Feed)
    #[arg(value_hint = ValueHint::Url)]
    pub url: String,
    /// Project data fields (CSV)
    #[arg(long)]
    pub select: Option<String>,
    /// Extra request header (repeatable, `Name: value`)
    #[arg(long)]
    pub header: Vec<String>,
    /// Bypass the local response cache
    #[arg(long, num_args = 0..=1, default_missing_value = "true", require_equals = false)]
    pub no_cache: Option<bool>,
    /// The format this verb requests, fixed. Not a user-facing flag.
    ///
    /// Carried as a field so the dispatcher can hand `scrape` a `&[String]`
    /// without allocating one per call, and so the value has exactly one
    /// definition instead of a string literal at the call site.
    #[arg(skip = String::from("feed"))]
    pub format_feed: String,
}

/// List URLs declared by a site's `sitemap.xml` / `sitemapindex`
///
/// # Why a verb for something `map` can already do
///
/// The capability is not new: `map --sitemap-only` has performed exactly this
/// walk, and this command does NOT reimplement it — it delegates to the same
/// handler with that flag fixed. What is new is that the capability is findable.
/// An agent looking for sitemap discovery searches `commands` for `sitemap`; it
/// does not guess that a flag on a differently-named verb is where the feature
/// lives, and the PRD itself recorded the wrong belief that `map` covered link
/// discovery only.
///
/// Every option here is a field of `MapArgs` with the same meaning. The ones
/// that would contradict the verb — `--sitemap-only`, `--use-sitemap`,
/// `--max-depth` — are deliberately absent: this verb IS sitemap-only, so a
/// flag to turn that off would only create a way to ask `sitemap` for something
/// that is not a sitemap.
#[derive(Debug, Clone, Args)]
pub struct SitemapArgs {
    /// Site URL whose sitemap should be read
    #[arg(value_hint = ValueHint::Url)]
    pub url: String,
    /// Maximum number of URLs to return
    #[arg(long, default_value_t = crate::constants::MAP_DEFAULT_LIMIT)]
    pub limit: usize,
    /// Project data fields (CSV)
    #[arg(long)]
    pub select: Option<String>,
    /// Include only path prefixes (repeatable)
    #[arg(long = "include-path")]
    pub include_path: Vec<String>,
    /// Exclude path prefixes (repeatable)
    #[arg(long = "exclude-path")]
    pub exclude_path: Vec<String>,
    /// Filter URLs by substring (case-insensitive)
    #[arg(long)]
    pub search: Option<String>,
    /// Sort urls
    #[arg(long)]
    pub sort: Option<String>,
    /// Deduplicate urls key
    #[arg(long = "dedup-key")]
    pub dedup_key: Option<String>,
    /// Keep URLs on subdomains of the seed host, not just the seed host itself
    #[arg(long, action = ArgAction::SetTrue)]
    pub include_subdomains: bool,
    /// Collapse query params when deduping URLs
    #[arg(long, action = ArgAction::SetTrue)]
    pub ignore_query_params: bool,
}

/// Local search (HTTP SERP links or URL map)
#[derive(Debug, Clone, Args)]
pub struct SearchArgs {
    /// Search terms sent to the configured HTML endpoint
    pub query: String,
    /// Maximum number of results to return
    #[arg(long, default_value_t = crate::constants::SEARCH_DEFAULT_LIMIT)]
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

    /// Keep only results whose host matches one of these domains (CSV)
    ///
    /// A search that has to be re-filtered downstream costs the caller the whole
    /// SERP payload to throw most of it away. Matching is on the result host: an
    /// entry matches when the host equals the domain or is a subdomain of it, so
    /// `--include-domains rust-lang.org` also keeps `doc.rust-lang.org`.
    ///
    /// Applied locally, after the fetch: the public HTML endpoint has no
    /// site-restriction parameter this product can rely on.
    #[arg(long = "include-domains", value_name = "CSV")]
    pub include_domains: Option<String>,

    /// Country code for the search region, e.g. `br` (endpoint-dependent)
    ///
    /// Sent to the search endpoint, not applied locally: a SERP result carries
    /// no country field, so there is nothing to filter after the fetch.
    ///
    /// REQUIRES `--search-lang`. The endpoint expresses region as ONE key
    /// combining country and language, so half of it is a value we cannot
    /// confirm it honours — and a flag that might quietly do nothing reads, from
    /// the caller's side, exactly like one that worked. Passing only one half is
    /// refused with exit 2.
    ///
    /// Also refused with exit 2 when `search_base_url` points elsewhere, because
    /// the parameter names of an arbitrary SERP are unknown.
    #[arg(long = "country", value_name = "CODE")]
    pub country: Option<String>,

    /// Language code for the search region, e.g. `pt` (endpoint-dependent)
    ///
    /// Named `--search-lang`, not `--lang`, because the global `--lang` already
    /// means the language of the CLI's own messages. Two different questions
    /// deserve two different flags.
    ///
    /// REQUIRES `--country`, for the reason given there: the two are halves of
    /// one composed endpoint key. Same endpoint dependency as `--country`.
    #[arg(long = "search-lang", value_name = "CODE")]
    pub search_lang: Option<String>,

    /// Limit results to a recency window: d, w, m or y (endpoint-dependent)
    ///
    /// Sent to the endpoint for the same reason as `--country`: results carry no
    /// publication date, so a local filter would have nothing to read. The value
    /// is validated here, at argv time, so a typo costs an argv error instead of
    /// a fetch that silently ignores it.
    #[arg(long = "time-filter", value_name = "WINDOW", value_parser = ["d", "w", "m", "y"])]
    pub time_filter: Option<String>,

    /// Drop results whose host matches one of these domains (CSV)
    ///
    /// Same host matching as `--include-domains`, and applied after it, so a
    /// domain named in both is dropped. Exists for the common case of muting a
    /// content farm without having to enumerate every domain you do want.
    #[arg(long = "exclude-domains", value_name = "CSV")]
    pub exclude_domains: Option<String>,
}
