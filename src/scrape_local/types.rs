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
    /// Named attributes pulled from caller-chosen CSS selectors.
    ///
    /// The other formats answer "what is on this page?" with a fixed shape.
    /// This one answers "what is at these exact places?", which is the
    /// question a caller already knows the answer's shape to. Without it, the
    /// only way to read one attribute off a list of elements through the HTTP
    /// engine was to pull `rawHtml` and parse it outside the binary — which is
    /// exactly the work this product exists to keep out of the model.
    Attributes,
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
            "attributes" | "attribute" | "attr" => Ok(Self::Attributes),
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
    ///
    /// Read through [`ScrapeOpts::engine_kind`] rather than compared as a
    /// string: the transport differs between the two, and code that gets the
    /// comparison wrong reports the wrong transport in the envelope.
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
    /// Ignore the response cache on READ and always fetch from origin.
    ///
    /// The write still happens, so a bypassing caller refreshes the entry for
    /// everyone else instead of leaving a stale one behind. A command whose
    /// question is "did this page change" must set this: a cache hit makes it
    /// compare a stored page against itself and answer "no" with `ok: true`.
    pub no_cache: bool,
    /// Selector/attribute pairs for [`ScrapeFormat::Attributes`].
    ///
    /// Paired at the CLI layer, so a caller cannot end up with three selectors
    /// and two attribute names and no way to tell which pair the binary
    /// dropped.
    pub attribute_targets: Vec<(String, String)>,
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
            no_cache: crate::xdg::resolve_scrape_no_cache(),
            attribute_targets: Vec::new(),
        }
    }
}

/// Which transport actually fetched the page.
///
/// # Why this is a type and not a string comparison
///
/// The two engines are different clients with different fingerprints. `Http`
/// is this crate's `reqwest` build: no TLS impersonation, no control over
/// header order. `Browser` is a real Chrome, which owns its own JA3 and emits
/// headers in Chrome's own order because it *is* Chrome.
///
/// The scrape envelope reports those properties. While the engine was a bare
/// `String`, the report was written once for `reqwest` and emitted for both,
/// so a browser scrape claimed `tls_impersonation: false` about a transport
/// that does impersonate — the product understated exactly the engine it calls
/// its most valuable asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrapeEngine {
    /// This crate's `reqwest` client.
    Http,
    /// A real Chrome driven over CDP.
    Browser,
}

impl ScrapeEngine {
    /// Parse an engine name, defaulting to [`ScrapeEngine::Http`].
    ///
    /// Unknown input resolves to `Http` rather than failing, because the CLI
    /// layer already rejects unknown `--engine` values; reaching here with one
    /// means an internal caller, and the cheap transport is the safe guess.
    #[must_use]
    pub fn parse(name: &str) -> Self {
        if name.eq_ignore_ascii_case("browser") {
            Self::Browser
        } else {
            Self::Http
        }
    }

    /// The wire spelling used in the envelope.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Browser => "browser",
        }
    }
}

impl ScrapeOpts {
    /// The engine as a type, for code that must branch on the transport.
    #[must_use]
    pub fn engine_kind(&self) -> ScrapeEngine {
        ScrapeEngine::parse(&self.engine)
    }
}
