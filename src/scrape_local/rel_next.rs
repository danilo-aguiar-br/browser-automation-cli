// SPDX-License-Identifier: MIT OR Apache-2.0
//! `rel=next` pagination discovery for crawl continuation.
//!
//! A paginated series is often reachable only through `<link rel="next">` in
//! `<head>`, which carries no anchor text and is therefore invisible to
//! ordinary `<a href>` extraction. This module surfaces both spellings —
//! `<link rel="next">` and `<a rel="next">` — as absolute URLs.
//!
//! Discovery is deliberately *only* discovery: the returned URLs are handed to
//! the normal crawl frontier, so `--limit`, `--max-depth`, `--same-host`,
//! path filters, robots and politeness all still apply unchanged.

use std::collections::BTreeSet;

use scraper::{Html, Selector};
use url::Url;

/// Absolute `rel=next` targets found in the document, deduplicated.
///
/// `base` resolves relative hrefs. Fragment-only and `javascript:` targets are
/// dropped, matching anchor extraction. Order is stable (document order for
/// `<link>` then `<a>`), so a crawl is reproducible.
pub(crate) fn extract_rel_next(base: &str, doc: &Html) -> Vec<String> {
    let base_url = Url::parse(base).ok();
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for sel in ["link[rel~=\"next\" i][href]", "a[rel~=\"next\" i][href]"] {
        let Ok(selector) = Selector::parse(sel) else {
            continue;
        };
        for el in doc.select(&selector) {
            let href = el.value().attr("href").unwrap_or("").trim();
            if href.is_empty() || href.starts_with('#') || href.starts_with("javascript:") {
                continue;
            }
            let Some(abs) = absolutize(base_url.as_ref(), href) else {
                continue;
            };
            let abs = super::path_filter::normalize_url_for_dedup(&abs);
            if seen.insert(abs.clone()) {
                out.push(abs);
            }
        }
    }
    out
}

/// Resolve `href` against `base`, keeping only http(s) targets.
///
/// A crawl frontier can only fetch over HTTP, so any other scheme (`mailto:`,
/// `data:`, …) is discarded here rather than failing later in the fetch.
fn absolutize(base: Option<&Url>, href: &str) -> Option<String> {
    let resolved = match Url::parse(href) {
        Ok(u) => u,
        Err(_) => base?.join(href).ok()?,
    };
    match resolved.scheme() {
        "http" | "https" => Some(resolved.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn urls(html: &str) -> Vec<String> {
        extract_rel_next("https://example.com/page/1", &Html::parse_document(html))
    }

    #[test]
    fn head_link_rel_next_is_found() {
        let got = urls(r#"<html><head><link rel="next" href="/page/2"></head></html>"#);
        assert_eq!(got, vec!["https://example.com/page/2".to_string()]);
    }

    #[test]
    fn anchor_rel_next_is_found() {
        let got = urls(r#"<html><body><a rel="next" href="/page/2">Next</a></body></html>"#);
        assert_eq!(got, vec!["https://example.com/page/2".to_string()]);
    }

    #[test]
    fn rel_token_list_and_case_are_handled() {
        let got = urls(r#"<html><body><a rel="noopener NEXT" href="/page/2">n</a></body></html>"#);
        assert_eq!(got, vec!["https://example.com/page/2".to_string()]);
    }

    #[test]
    fn rel_prev_is_not_followed() {
        assert!(urls(r#"<html><head><link rel="prev" href="/page/0"></head></html>"#).is_empty());
    }

    #[test]
    fn duplicate_next_targets_collapse() {
        let got = urls(
            r#"<html><head><link rel="next" href="/page/2"></head>
               <body><a rel="next" href="https://example.com/page/2">n</a></body></html>"#,
        );
        assert_eq!(got.len(), 1);
    }

    #[test]
    fn non_http_scheme_is_dropped() {
        assert!(urls(r#"<html><a rel="next" href="mailto:a@example.com">n</a></html>"#).is_empty());
        assert!(urls(r##"<html><a rel="next" href="#more">n</a></html>"##).is_empty());
    }

    #[test]
    fn absent_rel_next_yields_empty() {
        assert!(urls(r#"<html><body><a href="/page/2">Next</a></body></html>"#).is_empty());
    }
}
