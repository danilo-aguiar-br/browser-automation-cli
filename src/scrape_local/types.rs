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
    /// Raw HTML body.
    Html,
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
}

impl ScrapeFormat {
    /// Parse from CLI flag (comma-separated first token).
    pub fn parse(s: &str) -> Result<Self, CliError> {
        match s.trim().to_ascii_lowercase().as_str() {
            "text" | "body" => Ok(Self::Text),
            "markdown" | "md" => Ok(Self::Markdown),
            "html" | "raw-html" | "rawhtml" | "raw_html" | "rawHtml" => Ok(Self::Html),
            "links" => Ok(Self::Links),
            "metadata" | "meta" => Ok(Self::Metadata),
            "screenshot" | "shot" | "image" => Ok(Self::Screenshot),
            "summary" => Ok(Self::Summary),
            "product" => Ok(Self::Product),
            "branding" => Ok(Self::Branding),
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
}

impl Default for ScrapeOpts {
    fn default() -> Self {
        Self {
            format: ScrapeFormat::Text,
            only_main_content: false,
            engine: "browser".into(),
            max_body_bytes: crate::xdg::resolve_scrape_max_body_bytes(),
        }
    }
}
