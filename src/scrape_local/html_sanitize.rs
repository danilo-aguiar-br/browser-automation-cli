// SPDX-License-Identifier: MIT OR Apache-2.0
//! DOM reduction by CSS selectors and PII redaction of extracted text.

use std::sync::LazyLock;

use regex::Regex;
use scraper::{Html, Selector};

/// Outcome of [`filter_html_by_selectors`]: the reduced HTML plus the witness
/// the envelope needs.
///
/// Without the witness, "your selector matched the whole page" and "your
/// selector matched nothing" reach the caller as the same bytes under the same
/// `ok: true`, and no amount of reading the payload separates them.
pub(crate) struct SelectorFilter {
    /// HTML after include/exclude reduction. EMPTY when include selectors were
    /// asked for and matched nothing.
    pub html: String,
    /// Whether the include request was satisfied. Vacuously true when no include
    /// selector was asked for, because then there was no subset to frustrate.
    pub matched: bool,
    /// How many elements the include selectors matched, summed over all of them.
    pub match_count: usize,
}

/// clap `value_parser` for a CSS selector passed on the argv.
///
/// `Selector::parse` returns a `Result`, so a malformed selector is knowable
/// from the argv alone, before a socket is opened. Until 0.1.9 the filter below
/// swallowed that error with `continue` and fell through to the whole document,
/// so `--include-selector 'h1['` answered with the entire page under `ok: true`
/// and exit 0. Measured on `example.com` in `--format text`: 127 characters
/// returned, against the 14 that a valid `h1` selector yields. An operator typo
/// is a usage error, and a usage error belongs on the argv.
///
/// Mirrors [`crate::net::resource_type::validate_resource_types_arg`], which is
/// how this crate already refuses a bad `--resource-types` value.
pub fn validate_css_selector_arg(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("empty CSS selector".to_string());
    }
    match Selector::parse(trimmed) {
        Ok(_) => Ok(trimmed.to_string()),
        Err(e) => Err(format!("invalid CSS selector `{trimmed}`: {e}")),
    }
}

/// Filter document HTML by include/exclude CSS selectors (agent anti-token).
///
/// Include (when non-empty): concatenate matching outer HTML fragments.
/// Exclude: strip matching nodes from a working copy via re-parse of reduced
/// tree.
///
/// FAIL-CLOSED on include. When include selectors are present and match
/// nothing, this returns EMPTY html with `matched: false`, never the untouched
/// document. Returning the document was the defect: the caller asked for a
/// subset, received the full set, and the envelope marked no difference. Note
/// that exclude has no such failure mode, because excluding a selector that
/// matches nothing correctly leaves the document alone.
pub(crate) fn filter_html_by_selectors(
    html: &str,
    include: &[String],
    exclude: &[String],
) -> SelectorFilter {
    if include.is_empty() && exclude.is_empty() {
        return SelectorFilter {
            html: html.to_string(),
            matched: true,
            match_count: 0,
        };
    }
    let doc = Html::parse_document(html);
    if !include.is_empty() {
        let mut parts = Vec::new();
        for sel_s in include {
            // Unreachable from the argv: `validate_css_selector_arg` refuses a
            // malformed selector with exit 2 before any fetch. Kept so that a
            // future non-argv caller degrades to "matched nothing", which is now
            // an observable outcome, instead of silently widening the result.
            let Ok(sel) = Selector::parse(sel_s.trim()) else {
                continue;
            };
            for el in doc.select(&sel) {
                parts.push(el.html());
            }
        }
        let match_count = parts.len();
        let joined = parts.join("\n");
        let reduced = if exclude.is_empty() {
            joined
        } else {
            strip_exclude_from_html(&joined, exclude)
        };
        return SelectorFilter {
            html: reduced,
            matched: match_count > 0,
            match_count,
        };
    }
    SelectorFilter {
        html: strip_exclude_from_html(html, exclude),
        matched: true,
        match_count: 0,
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = r#"<html><body><h1>Title</h1><article><p>Body</p></article><footer>Foot</footer></body></html>"#;

    /// The whole defect, in one assertion.
    ///
    /// Before this, the fall-through handed back `html` untouched, so "your
    /// selector matched nothing" and "your selector matched everything"
    /// reached the caller as identical bytes under an identical `ok: true`.
    #[test]
    fn an_include_selector_that_matches_nothing_returns_empty_not_the_document() {
        let out = filter_html_by_selectors(DOC, &["nav.absent".to_string()], &[]);
        assert_eq!(
            out.html, "",
            "no match must never widen to the whole document"
        );
        assert!(!out.matched);
        assert_eq!(out.match_count, 0);
    }

    #[test]
    fn an_include_selector_that_matches_reduces_and_counts() {
        let out = filter_html_by_selectors(DOC, &["h1".to_string()], &[]);
        assert!(out.html.contains("Title"));
        assert!(
            !out.html.contains("Foot"),
            "reduction must drop what was not selected"
        );
        assert!(out.matched);
        assert_eq!(out.match_count, 1);
    }

    #[test]
    fn no_selector_at_all_is_vacuously_matched_and_passes_through() {
        let out = filter_html_by_selectors(DOC, &[], &[]);
        assert_eq!(out.html, DOC);
        assert!(
            out.matched,
            "no subset was requested, so no subset was frustrated"
        );
    }

    /// Exclude has no frustrated-request failure mode, and must not inherit
    /// include's fail-closed rule.
    #[test]
    fn exclude_that_matches_nothing_leaves_the_document_alone() {
        let out = filter_html_by_selectors(DOC, &[], &["nav.absent".to_string()]);
        assert_eq!(out.html, DOC);
        assert!(out.matched);
    }

    #[test]
    fn exclude_strips_the_named_node() {
        let out = filter_html_by_selectors(DOC, &[], &["footer".to_string()]);
        assert!(!out.html.contains("Foot"));
        assert!(out.matched);
    }

    #[test]
    fn include_counts_every_match_across_every_selector() {
        let out = filter_html_by_selectors(DOC, &["h1".to_string(), "p".to_string()], &[]);
        assert_eq!(out.match_count, 2);
        assert!(out.matched);
    }

    /// A typo is knowable from the argv alone, so it must never reach the
    /// filter and never cost the caller a fetch.
    #[test]
    fn a_malformed_selector_is_refused_rather_than_swallowed() {
        assert!(validate_css_selector_arg("h1[").is_err());
        assert!(validate_css_selector_arg(">>>bad<<<").is_err());
        assert!(validate_css_selector_arg("   ").is_err());
    }

    #[test]
    fn a_valid_selector_is_accepted_and_trimmed() {
        assert_eq!(
            validate_css_selector_arg("  article.main  ").expect("valid selector"),
            "article.main"
        );
    }
}
