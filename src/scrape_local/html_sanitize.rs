// SPDX-License-Identifier: MIT OR Apache-2.0
//! DOM reduction by CSS selectors and PII redaction of extracted text.

use std::sync::LazyLock;

use regex::Regex;
use scraper::{Html, Selector};

/// Filter document HTML by include/exclude CSS selectors (agent anti-token).
///
/// Include (when non-empty): concatenate matching outer HTML fragments.
/// Exclude: strip matching nodes from a working copy via re-parse of reduced tree
/// (best-effort: remove by re-selecting on full doc and rebuilding from include
/// or body minus excluded text via attribute marker is complex; we drop nodes
/// by serializing only non-excluded children of body when no include set).
pub(crate) fn filter_html_by_selectors(
    html: &str,
    include: &[String],
    exclude: &[String],
) -> String {
    if include.is_empty() && exclude.is_empty() {
        return html.to_string();
    }
    let doc = Html::parse_document(html);
    if !include.is_empty() {
        let mut parts = Vec::new();
        for sel_s in include {
            let Ok(sel) = Selector::parse(sel_s.trim()) else {
                continue;
            };
            for el in doc.select(&sel) {
                parts.push(el.html());
            }
        }
        if !parts.is_empty() {
            let joined = parts.join("\n");
            return if exclude.is_empty() {
                joined
            } else {
                strip_exclude_from_html(&joined, exclude)
            };
        }
    }
    strip_exclude_from_html(html, exclude)
}

fn strip_exclude_from_html(html: &str, exclude: &[String]) -> String {
    if exclude.is_empty() {
        return html.to_string();
    }
    // Best-effort: remove outer HTML of each excluded match from the string.
    let doc = Html::parse_document(html);
    let mut out = html.to_string();
    for sel_s in exclude {
        let Ok(sel) = Selector::parse(sel_s.trim()) else {
            continue;
        };
        for el in doc.select(&sel) {
            let frag = el.html();
            if !frag.is_empty() {
                out = out.replace(&frag, "");
            }
        }
    }
    out
}

/// Compiled once: email / phone / card-like PII redaction.
pub(crate) struct PiiRegexes {
    email: Regex,
    phone: Regex,
    card: Regex,
}

/// Process-wide PII regex catalog (`LazyLock`: never recompile per call).
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

/// Redact common PII patterns in text (email, phone, card-like digits).
pub fn redact_pii(text: &str) -> String {
    let re = pii_regexes();
    let mut out = re.email.replace_all(text, "[REDACTED_EMAIL]").into_owned();
    out = re.phone.replace_all(&out, "[REDACTED_PHONE]").into_owned();
    out = re.card.replace_all(&out, "[REDACTED_CARD]").into_owned();
    out
}
