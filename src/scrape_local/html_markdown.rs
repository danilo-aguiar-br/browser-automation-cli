// SPDX-License-Identifier: MIT OR Apache-2.0
//! HTML to Markdown conversion (headings, paragraphs, lists, code, links).

use scraper::{Html, Selector};

use super::html_text::{join_text_collapsed, visible_text};

/// Convert HTML to a simple Markdown document, falling back to visible text.
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
    // Unordered lists
    if let Ok(selector) = Selector::parse("li") {
        for el in doc.select(&selector) {
            let t = join_text_collapsed(el.text());
            if !t.is_empty() {
                out.push_str("- ");
                out.push_str(&t);
                out.push('\n');
            }
        }
        if out.ends_with('\n') {
            out.push('\n');
        }
    }
    // Code blocks (simple)
    if let Ok(selector) = Selector::parse("pre") {
        for el in doc.select(&selector) {
            let t = el.text().collect::<String>();
            if !t.trim().is_empty() {
                out.push_str("```\n");
                out.push_str(t.trim());
                out.push_str("\n```\n\n");
            }
        }
    }
    // Anchors as markdown links (capped)
    if let Ok(selector) = Selector::parse("a[href]") {
        let mut n = 0usize;
        for el in doc.select(&selector) {
            if n >= 32 {
                break;
            }
            let href = el.value().attr("href").unwrap_or("").trim();
            if href.is_empty() || href.starts_with('#') {
                continue;
            }
            let t = join_text_collapsed(el.text());
            if t.is_empty() {
                continue;
            }
            out.push_str(&format!("[{t}]({href})\n"));
            n += 1;
        }
    }
    if out.trim().is_empty() {
        out = visible_text(&doc);
    }
    out
}
