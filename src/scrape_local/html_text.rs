// SPDX-License-Identifier: MIT OR Apache-2.0
//! DOM text and `<meta>` extraction (whitespace-collapsed, allocation-lean).

use scraper::{Html, Selector};

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

/// Collapsed text of the document `<body>` (empty when absent).
pub(crate) fn visible_text(doc: &Html) -> String {
    let Ok(selector) = Selector::parse("body") else {
        return String::new();
    };
    doc.select(&selector)
        .next()
        .map(|e| join_text_collapsed(e.text()))
        .unwrap_or_default()
}

/// Trimmed text of the first node matching `sel` (empty when absent/invalid).
pub(crate) fn text_of_first(doc: &Html, sel: &str) -> String {
    let Ok(selector) = Selector::parse(sel) else {
        return String::new();
    };
    doc.select(&selector)
        .next()
        .map(|e| e.text().collect::<String>().trim().to_string())
        .unwrap_or_default()
}

/// First non-empty `content` of `meta[name]`, `meta[property]` or `og:` variant.
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
