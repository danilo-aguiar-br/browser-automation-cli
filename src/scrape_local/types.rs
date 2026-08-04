// SPDX-License-Identifier: MIT OR Apache-2.0
//! Scrape format types and shared options.

use crate::error::{CliError, ErrorKind};

/// Identifiable product User-Agent for HTTP scrapes (PRD politeness).
///
/// Re-export of the crate-wide compile-time identity
/// ([`crate::constants::HTTP_USER_AGENT`]) so scrape call sites keep a stable path.
pub use crate::constants::HTTP_USER_AGENT;

/// Output formats for local scrape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrapeFormat {
    /// Visible/plain text.
    Text,
    /// Simplified markdown.
    Markdown,
    /// Processed HTML body: main-content extraction and selector filters applied.
    Html,
    /// The response body exactly as received, before any processing.
    ///
    /// Distinct from [`Self::Html`] on purpose. Accepting `rawHtml` as an alias
    /// of `html` made the CLI answer a request for unprocessed bytes with the
    /// processed ones, under the `html` key — a contract the caller had no way
    /// to check.
    RawHtml,
    /// Extracted anchor links.
    Links,
    /// Title / description / status metadata.
    Metadata,
    /// Screenshot path placeholder (browser engine fills via CDP grab).
    Screenshot,
    /// LLM-oriented short summary (requires --llm path or offline stub from title/text).
    Summary,
    /// Product fields from JSON-LD Product schema when present.
    Product,
    /// Branding colors/fonts heuristics from HTML.
    Branding,
    /// Image `src` list (lightweight).
    Images,
    /// All JSON-LD blocks (try-parse each).
    JsonLd,
    /// Structured JSON via optional LLM schema (handler may fill async).
    Json,
    /// RSS / Atom / JSON Feed entries parsed from the response body.
    Feed,
}

impl ScrapeFormat {
    /// Parse from CLI flag (comma-separated first token).
    pub fn parse(s: &str) -> Result<Self, CliError> {
        match s.trim().to_ascii_lowercase().as_str() {
            "text" | "body" => Ok(Self::Text),
            "markdown" | "md" => Ok(Self::Markdown),
            "html" => Ok(Self::Html),
            // The input is already lowercased above, so a `"rawHtml"` arm here
            // would be unreachable — the camelCase spelling arrives as `rawhtml`.
            "raw-html" | "rawhtml" | "raw_html" => Ok(Self::RawHtml),
            "links" => Ok(Self::Links),
            "metadata" | "meta" => Ok(Self::Metadata),
            "screenshot" | "shot" => Ok(Self::Screenshot),
            "summary" => Ok(Self::Summary),
            "product" => Ok(Self::Product),
            "branding" => Ok(Self::Branding),
            "images" | "image" => Ok(Self::Images),
            "jsonld" | "json-ld" | "json_ld" => Ok(Self::JsonLd),
            "json" => Ok(Self::Json),
            "feed" | "rss" | "atom" => Ok(Self::Feed),
            other => Err(CliError::with_suggestion(
                ErrorKind::Usage,
                format!("unknown scrape format: {other}"),
                crate::i18n::suggestion_key("use_listed_value", None),
            )),
        }
    }
}

/// Shared scrape options.
#[derive(Debug, Clone)]
pub struct ScrapeOpts {
    /// Output format.
    pub format: ScrapeFormat,
    /// Prefer only main content heuristics.
    pub only_main_content: bool,
    /// Engine: "http" or "browser".
    pub engine: String,
    /// Max body bytes for HTTP.
    pub max_body_bytes: usize,
    /// Max chars for text/markdown fields (0 = no cap).
    pub max_text_chars: usize,
    /// Honor meta robots / X-Robots-Tag noindex.
    pub honor_meta_robots: bool,
    /// Skip nofollow links when extracting for crawl.
    pub honor_nofollow: bool,
    /// Surface `rel=next` pagination targets in the payload for crawl continuation.
    pub follow_rel_next: bool,
    /// CSS selectors whose subtrees are kept (empty = full body after only_main).
    pub include_selectors: Vec<String>,
    /// CSS selectors whose subtrees are stripped before extract.
    pub exclude_selectors: Vec<String>,
    /// Redact email/phone/card-like patterns in text/markdown.
    pub redact_pii: bool,
    /// Include sha256 content_hash of text/markdown in payload.
    pub with_content_hash: bool,
    /// Extra HTTP headers (`Name: value`) for legitimate auth/cookies (operator-supplied).
    pub extra_headers: Vec<(String, String)>,
}

impl Default for ScrapeOpts {
    fn default() -> Self {
        Self {
            format: ScrapeFormat::Text,
            only_main_content: false,
            engine: crate::xdg::resolve_scrape_default_engine(),
            max_body_bytes: crate::xdg::resolve_scrape_max_body_bytes(),
            max_text_chars: crate::xdg::resolve_scrape_max_text_chars(),
            honor_meta_robots: crate::xdg::resolve_scrape_honor_meta_robots(),
            honor_nofollow: crate::xdg::resolve_scrape_honor_nofollow(),
            follow_rel_next: crate::xdg::resolve_scrape_follow_rel_next(),
            include_selectors: Vec::new(),
            exclude_selectors: Vec::new(),
            redact_pii: false,
            with_content_hash: false,
            extra_headers: Vec::new(),
        }
    }
}
