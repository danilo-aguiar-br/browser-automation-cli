// SPDX-License-Identifier: MIT OR Apache-2.0
//! Link/image/branding extraction plus the stable HTML helper facade.
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `html_jsonld` | JSON-LD blocks and Product typing |
//! | `html_sanitize` | selector-based DOM reduction and PII redaction |
//! | `html_text` | DOM text nodes and `<meta>` values |
//! | `html_markdown` | HTML to Markdown conversion |
//!
//! Consumers keep importing everything from `super::html::*`.

use std::collections::BTreeSet;
use std::sync::LazyLock;

use regex::Regex;
use scraper::{Html, Selector};
use serde_json::{json, Value};
use url::Url;

pub(crate) use super::html_jsonld::{extract_all_json_ld, extract_json_ld_product};
pub(crate) use super::html_markdown::html_to_markdown_simple;
pub(crate) use super::html_sanitize::{filter_html_by_selectors, redact_pii};
pub(crate) use super::html_text::{join_text_collapsed, meta_content, text_of_first, visible_text};

/// SHA-256 hex of UTF-8 text (content_hash for agent compare).
pub(crate) fn content_hash_hex(text: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(text.as_bytes());
    hex::encode(h.finalize())
}

/// Compiled once: hex color samples in branding heuristics.
///
/// `LazyLock` (MSRV ≥ 1.80): fixed closure, no runtime args — preferred over
/// `OnceLock` for static regex catalogs (rules_rust_const_static_inicializacao).
pub(crate) fn re_hex_color() -> &'static Regex {
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"#[0-9A-Fa-f]{3,8}\b").expect("hex color regex"));
    &RE
}

/// Heuristic branding signal: title plus a capped set of hex color samples.
pub(crate) fn extract_branding_hints(html: &str, title: &str) -> Value {
    let mut colors = BTreeSet::new();
    for m in re_hex_color().find_iter(html).take(32) {
        colors.insert(m.as_str().to_string());
    }
    json!({
        "title": title,
        "color_samples": colors.into_iter().collect::<Vec<_>>(),
        "note": "heuristic branding; not a full brand kit",
    })
}

/// First main-content container of the document, when one is present.
pub(crate) fn extract_main_html(doc: &Html) -> Option<String> {
    for sel in [
        "main",
        "article",
        "[role=main]",
        "#content",
        ".content",
        "#main",
    ] {
        if let Ok(selector) = Selector::parse(sel) {
            if let Some(el) = doc.select(&selector).next() {
                return Some(el.html());
            }
        }
    }
    None
}

/// Extract image URLs (absolute) with optional alt text (capped).
pub(crate) fn extract_images(base: &str, doc: &Html) -> Vec<Value> {
    let Ok(selector) = Selector::parse("img[src]") else {
        return Vec::new();
    };
    let base_url = Url::parse(base).ok();
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    let cap = crate::constants::DEFAULT_SCRAPE_LINK_TEXT_CHARS;
    for el in doc.select(&selector) {
        let src = el.value().attr("src").unwrap_or("").trim();
        if src.is_empty() || src.starts_with("data:") {
            continue;
        }
        let abs = match (&base_url, Url::parse(src)) {
            (_, Ok(u)) if u.scheme() == "http" || u.scheme() == "https" => u.to_string(),
            (Some(b), _) => b
                .join(src)
                .map(|u| u.to_string())
                .unwrap_or_else(|_| src.to_string()),
            _ => src.to_string(),
        };
        if !seen.insert(abs.clone()) {
            continue;
        }
        let alt = el.value().attr("alt").unwrap_or("").trim();
        let alt_capped: String = alt.chars().take(cap).collect();
        let mut item = serde_json::Map::new();
        item.insert("url".into(), json!(abs));
        if !alt_capped.is_empty() {
            item.insert("alt".into(), json!(alt_capped));
        }
        out.push(Value::Object(item));
    }
    out
}

/// Extract deduplicated absolute anchor targets with capped link text.
pub(crate) fn extract_links(base: &str, doc: &Html, honor_nofollow: bool) -> Vec<Value> {
    let Ok(selector) = Selector::parse("a[href]") else {
        return Vec::new();
    };
    let base_url = Url::parse(base).ok();
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    let cap = crate::constants::DEFAULT_SCRAPE_LINK_TEXT_CHARS;
    for el in doc.select(&selector) {
        let href = el.value().attr("href").unwrap_or("").trim();
        if href.is_empty() || href.starts_with('#') || href.starts_with("javascript:") {
            continue;
        }
        let nofollow = super::directives::rel_has_nofollow(el.value().attr("rel"));
        if honor_nofollow && nofollow {
            continue;
        }
        let abs = match (&base_url, Url::parse(href)) {
            (_, Ok(u)) if u.scheme() == "http" || u.scheme() == "https" || u.scheme() == "file" => {
                u.to_string()
            }
            (Some(b), _) => b
                .join(href)
                .map(|u| u.to_string())
                .unwrap_or_else(|_| href.to_string()),
            _ => href.to_string(),
        };
        let abs = super::path_filter::normalize_url_for_dedup(&abs);
        if seen.insert(abs.clone()) {
            let text = join_text_collapsed(el.text());
            let text: String = text.chars().take(cap).collect();
            let mut item = serde_json::Map::new();
            item.insert("url".into(), json!(abs));
            if !text.is_empty() {
                item.insert("text".into(), json!(text));
            }
            if nofollow {
                item.insert("nofollow".into(), json!(true));
            }
            out.push(Value::Object(item));
        }
    }
    out
}
