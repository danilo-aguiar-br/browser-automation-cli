// SPDX-License-Identifier: MIT OR Apache-2.0
//! HTML extractors, PII redaction, markdown (CPU-bound helpers).

use std::collections::BTreeSet;
use std::sync::LazyLock;

use regex::Regex;
use scraper::{Html, Selector};
use serde_json::{json, Value};
use url::Url;

pub(crate) fn extract_json_ld_product(html: &str) -> Value {
    let doc = Html::parse_document(html);
    let Ok(sel) = Selector::parse("script[type=\"application/ld+json\"]") else {
        return json!({ "found": false });
    };
    for el in doc.select(&sel) {
        let raw = el.text().collect::<String>();
        if let Ok(v) = crate::json_util::value_from_str(&raw) {
            if is_product_ld(&v) {
                return json!({ "found": true, "json_ld": v });
            }
            if let Some(arr) = v.as_array() {
                for item in arr {
                    if is_product_ld(item) {
                        return json!({ "found": true, "json_ld": item });
                    }
                }
            }
            if let Some(graph) = v.get("@graph").and_then(|g| g.as_array()) {
                for item in graph {
                    if is_product_ld(item) {
                        return json!({ "found": true, "json_ld": item });
                    }
                }
            }
        }
    }
    json!({ "found": false, "json_ld": null })
}

pub(crate) fn is_product_ld(v: &Value) -> bool {
    match v.get("@type") {
        Some(Value::String(s)) => s.eq_ignore_ascii_case("Product"),
        Some(Value::Array(a)) => a.iter().any(|x| {
            x.as_str()
                .map(|s| s.eq_ignore_ascii_case("Product"))
                .unwrap_or(false)
        }),
        _ => false,
    }
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

/// Compiled once: email / phone / card-like PII redaction.
pub(crate) struct PiiRegexes {
    email: Regex,
    phone: Regex,
    card: Regex,
}

pub(crate) fn pii_regexes() -> &'static PiiRegexes {
    static RE: LazyLock<PiiRegexes> = LazyLock::new(|| PiiRegexes {
        email: Regex::new(r"[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}")
            .expect("email regex"),
        phone: Regex::new(
            r"\b(?:\+?\d{1,3}[-.\s]?)?(?:\(?\d{2,4}\)?[-.\s]?)?\d{3,4}[-.\s]?\d{4}\b",
        )
        .expect("phone regex"),
        card: Regex::new(r"\b(?:\d[ -]*?){13,19}\b").expect("card regex"),
    });
    &RE
}

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

/// Redact common PII patterns in text (email, phone, card-like digits).
pub fn redact_pii(text: &str) -> String {
    let re = pii_regexes();
    let mut out = re.email.replace_all(text, "[REDACTED_EMAIL]").into_owned();
    out = re.phone.replace_all(&out, "[REDACTED_PHONE]").into_owned();
    out = re.card.replace_all(&out, "[REDACTED_CARD]").into_owned();
    out
}

pub(crate) fn text_of_first(doc: &Html, sel: &str) -> String {
    let Ok(selector) = Selector::parse(sel) else {
        return String::new();
    };
    doc.select(&selector)
        .next()
        .map(|e| e.text().collect::<String>().trim().to_string())
        .unwrap_or_default()
}

pub(crate) fn meta_content(doc: &Html, name: &str) -> Option<String> {
    let sel =
        format!("meta[name=\"{name}\"], meta[property=\"{name}\"], meta[property=\"og:{name}\"]");
    let Ok(selector) = Selector::parse(&sel) else {
        return None;
    };
    doc.select(&selector)
        .find_map(|e| e.value().attr("content").map(|s| s.trim().to_string()))
        .filter(|s| !s.is_empty())
}

pub(crate) fn extract_main_html(doc: &Html) -> Option<String> {
    for sel in ["main", "article", "[role=main]", "#content", ".content"] {
        if let Ok(selector) = Selector::parse(sel) {
            if let Some(el) = doc.select(&selector).next() {
                return Some(el.html());
            }
        }
    }
    None
}

/// Join DOM text nodes and collapse internal whitespace without an intermediate
/// `Vec` of tokens.
///
/// Cause: `split_whitespace().collect::<Vec<_>>().join(" ")` double-allocates
/// (token vec + joined string) on every scrape text/link path.
/// Effect: single `String`; hot scrape path pays one heap instead of two.
pub(crate) fn join_text_collapsed<'a, I>(iter: I) -> String
where
    I: Iterator<Item = &'a str>,
{
    let mut out = String::new();
    let mut need_space = false;
    for chunk in iter {
        for word in chunk.split_whitespace() {
            if need_space {
                out.push(' ');
            }
            out.push_str(word);
            need_space = true;
        }
    }
    out
}

pub(crate) fn visible_text(doc: &Html) -> String {
    let Ok(selector) = Selector::parse("body") else {
        return String::new();
    };
    doc.select(&selector)
        .next()
        .map(|e| join_text_collapsed(e.text()))
        .unwrap_or_default()
}

pub(crate) fn html_to_markdown_simple(html: &str, title: &str) -> String {
    let doc = Html::parse_document(html);
    let mut out = String::new();
    if !title.is_empty() {
        out.push_str("# ");
        out.push_str(title);
        out.push_str("\n\n");
    }
    // Headings (static selectors avoid SelectorErrorKind lifetime on dynamic strings).
    const HEADINGS: &[&str] = &["h1", "h2", "h3", "h4", "h5", "h6"];
    for (idx, sel) in HEADINGS.iter().enumerate() {
        let level = idx + 1;
        let Ok(selector) = Selector::parse(sel) else {
            continue;
        };
        for el in doc.select(&selector) {
            let t = el.text().collect::<String>().trim().to_string();
            if !t.is_empty() {
                out.push_str(&"#".repeat(level));
                out.push(' ');
                out.push_str(&t);
                out.push_str("\n\n");
            }
        }
    }
    // Paragraphs
    if let Ok(selector) = Selector::parse("p") {
        for el in doc.select(&selector) {
            let t = join_text_collapsed(el.text());
            if !t.is_empty() {
                out.push_str(&t);
                out.push_str("\n\n");
            }
        }
    }
    if out.trim().is_empty() {
        out = visible_text(&doc);
    }
    out
}

pub(crate) fn extract_links(base: &str, doc: &Html) -> Vec<Value> {
    let Ok(selector) = Selector::parse("a[href]") else {
        return Vec::new();
    };
    let base_url = Url::parse(base).ok();
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for el in doc.select(&selector) {
        let href = el.value().attr("href").unwrap_or("").trim();
        if href.is_empty() || href.starts_with('#') || href.starts_with("javascript:") {
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
        if seen.insert(abs.clone()) {
            let text = join_text_collapsed(el.text());
            out.push(json!({ "url": abs, "text": text }));
        }
    }
    out
}
