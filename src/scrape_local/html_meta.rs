// SPDX-License-Identifier: MIT OR Apache-2.0
//! Document metadata harvesting for the `metadata` scrape format.
//!
//! # Why this exists as its own module
//!
//! The `metadata` format used to emit five fields — title, description,
//! status_code, source_url, link_count — while the tags carrying Open Graph,
//! Dublin Core, article timestamps, canonical URL and favicon were already
//! parsed into the same `Html` document and thrown away. An agent asking for
//! "metadata" and receiving a link count is being told the page has no author,
//! no publish date and no canonical URL, none of which the CLI actually checked.
//!
//! Everything here reads from the document already parsed by
//! `build_scrape_payload`, so the extra coverage costs one selector pass per
//! field and no new dependency.

use scraper::{Html, Selector};
use serde_json::{Map, Value};

use super::html_text::meta_content;

/// `<meta>` names harvested verbatim when present.
///
/// Ordered by how often agents ask for them, not alphabetically: the first
/// entries are the ones that answer "what is this page and when was it written".
const SIMPLE_META: &[&str] = &[
    "keywords",
    "author",
    "language",
    "robots",
    "viewport",
    "generator",
    "theme-color",
];

/// Open Graph properties, harvested without the `og:` prefix in the output key.
const OG_META: &[&str] = &[
    "title",
    "description",
    "image",
    "url",
    "site_name",
    "type",
    "locale",
    "audio",
    "video",
];

/// Dublin Core terms, commonly present on publisher and academic pages.
const DC_META: &[&str] = &["title", "creator", "date", "subject", "publisher", "rights"];

/// `article:` properties that carry authorship and timestamps.
const ARTICLE_META: &[&str] = &[
    "published_time",
    "modified_time",
    "expiration_time",
    "author",
    "section",
    "tag",
];

/// Twitter card properties.
const TWITTER_META: &[&str] = &["card", "title", "description", "image", "site", "creator"];

/// Collect every metadata field the document actually carries.
///
/// Absent fields are omitted rather than emitted as null: the envelope contract
/// is CLEAN stdout, and a null tells an agent nothing that an absent key does
/// not already tell it, while costing bytes on every response.
pub(crate) fn collect_metadata(doc: &Html) -> Map<String, Value> {
    let mut out = Map::new();

    for name in SIMPLE_META {
        insert_if_present(&mut out, name, meta_content(doc, name));
    }
    collect_prefixed(&mut out, doc, "og", OG_META);
    collect_prefixed(&mut out, doc, "dc", DC_META);
    collect_prefixed(&mut out, doc, "article", ARTICLE_META);
    collect_prefixed(&mut out, doc, "twitter", TWITTER_META);

    insert_if_present(&mut out, "canonical", link_href(doc, "canonical"));
    insert_if_present(&mut out, "favicon", favicon(doc));
    insert_if_present(&mut out, "charset", charset(doc));
    insert_if_present(&mut out, "html_lang", html_lang(doc));

    out
}

/// Harvest `prefix:name` properties, keyed as `prefix_name` in the output.
fn collect_prefixed(out: &mut Map<String, Value>, doc: &Html, prefix: &str, names: &[&str]) {
    for name in names {
        let qualified = format!("{prefix}:{name}");
        insert_if_present(
            out,
            &format!("{prefix}_{name}"),
            meta_property(doc, &qualified),
        );
    }
}

/// Exact `meta[property]` or `meta[name]` lookup (no `og:` fallback).
///
/// [`meta_content`] adds an implicit `og:` variant, which would make
/// `dc:title` silently answer with `og:title`. Qualified prefixes need the
/// literal match or the harvest reports fields the page never declared.
fn meta_property(doc: &Html, qualified: &str) -> Option<String> {
    let sel = format!("meta[property=\"{qualified}\"], meta[name=\"{qualified}\"]");
    let selector = Selector::parse(&sel).ok()?;
    doc.select(&selector)
        .find_map(|e| e.value().attr("content").map(|s| s.trim().to_string()))
        .filter(|s| !s.is_empty())
}

/// `href` of the first `<link rel="...">` matching `rel`.
fn link_href(doc: &Html, rel: &str) -> Option<String> {
    let sel = format!("link[rel=\"{rel}\"]");
    let selector = Selector::parse(&sel).ok()?;
    doc.select(&selector)
        .find_map(|e| e.value().attr("href").map(|s| s.trim().to_string()))
        .filter(|s| !s.is_empty())
}

/// Favicon href, trying the three spellings a page may use.
fn favicon(doc: &Html) -> Option<String> {
    link_href(doc, "icon")
        .or_else(|| link_href(doc, "shortcut icon"))
        .or_else(|| link_href(doc, "apple-touch-icon"))
}

/// Declared charset from `<meta charset>`.
fn charset(doc: &Html) -> Option<String> {
    let selector = Selector::parse("meta[charset]").ok()?;
    doc.select(&selector)
        .find_map(|e| e.value().attr("charset").map(|s| s.trim().to_string()))
        .filter(|s| !s.is_empty())
}

/// `lang` attribute of `<html>`, distinct from the `language` meta tag.
fn html_lang(doc: &Html) -> Option<String> {
    let selector = Selector::parse("html[lang]").ok()?;
    doc.select(&selector)
        .find_map(|e| e.value().attr("lang").map(|s| s.trim().to_string()))
        .filter(|s| !s.is_empty())
}

fn insert_if_present(out: &mut Map<String, Value>, key: &str, value: Option<String>) {
    if let Some(v) = value {
        out.insert(key.to_string(), Value::String(v));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(html: &str) -> Html {
        Html::parse_document(html)
    }

    #[test]
    fn harvests_open_graph_without_the_prefix_in_the_key() {
        let d = doc(r#"<html><head>
            <meta property="og:title" content="Hello">
            <meta property="og:site_name" content="Example">
            </head></html>"#);
        let m = collect_metadata(&d);
        assert_eq!(m.get("og_title").and_then(Value::as_str), Some("Hello"));
        assert_eq!(
            m.get("og_site_name").and_then(Value::as_str),
            Some("Example")
        );
    }

    #[test]
    fn omits_fields_the_page_does_not_declare() {
        let m = collect_metadata(&doc("<html><head></head><body></body></html>"));
        assert!(
            !m.contains_key("og_title"),
            "absent keys must not be emitted"
        );
        assert!(!m.contains_key("favicon"));
    }

    #[test]
    fn dublin_core_does_not_fall_back_to_open_graph() {
        // `meta_content` would answer `dc:title` with `og:title`; a qualified
        // prefix must report only what the page actually declared.
        let d = doc(r#"<html><head><meta property="og:title" content="OG"></head></html>"#);
        let m = collect_metadata(&d);
        assert_eq!(m.get("og_title").and_then(Value::as_str), Some("OG"));
        assert!(!m.contains_key("dc_title"), "dc must not borrow from og");
    }

    #[test]
    fn reads_canonical_favicon_charset_and_lang() {
        let d = doc(r#"<html lang="pt-BR"><head>
            <meta charset="utf-8">
            <link rel="canonical" href="https://example.com/x">
            <link rel="icon" href="/favicon.ico">
            </head></html>"#);
        let m = collect_metadata(&d);
        assert_eq!(
            m.get("canonical").and_then(Value::as_str),
            Some("https://example.com/x")
        );
        assert_eq!(
            m.get("favicon").and_then(Value::as_str),
            Some("/favicon.ico")
        );
        assert_eq!(m.get("charset").and_then(Value::as_str), Some("utf-8"));
        assert_eq!(m.get("html_lang").and_then(Value::as_str), Some("pt-BR"));
    }

    #[test]
    fn harvests_article_timestamps() {
        let d = doc(r#"<html><head>
            <meta property="article:published_time" content="2026-01-02T03:04:05Z">
            <meta property="article:author" content="Ana">
            </head></html>"#);
        let m = collect_metadata(&d);
        assert_eq!(
            m.get("article_published_time").and_then(Value::as_str),
            Some("2026-01-02T03:04:05Z")
        );
        assert_eq!(m.get("article_author").and_then(Value::as_str), Some("Ana"));
    }
}
