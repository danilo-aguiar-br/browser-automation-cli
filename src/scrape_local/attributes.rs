// SPDX-License-Identifier: MIT OR Apache-2.0
//! Pulling named attributes out of caller-chosen CSS selectors.
//!
//! # The question the other formats cannot answer
//!
//! Every other scrape format has a fixed shape: `links` gives you links,
//! `metadata` gives you the head. They answer "what is on this page?".
//!
//! A caller who already knows the page and wants one attribute off a known set
//! of elements — every `data-sku` under `.product`, every `href` in a nav —
//! had exactly one route through the HTTP engine: ask for `rawHtml` and parse
//! it outside the binary. That is the work this product exists to keep out of
//! the model, and it is the most expensive possible way to get a short list of
//! strings.
//!
//! # Why an invalid selector is a row and not a failure
//!
//! A caller passing five selectors gets five rows. If one is malformed, the
//! other four are still the answer to four of the five questions, and failing
//! the whole scrape would throw them away. The bad row carries `error`, so the
//! caller can tell "no matches" from "I typed this wrong" — a distinction that
//! an empty list alone destroys.

use scraper::{Html, Selector};
use serde_json::{json, Value};

/// One row per requested `(selector, attribute)` pair, in the order asked.
///
/// Order is preserved so the caller can zip the result against its own input
/// without matching on strings.
pub(super) fn extract_attributes(html: &str, targets: &[(String, String)]) -> Vec<Value> {
    if targets.is_empty() {
        return Vec::new();
    }
    let document = Html::parse_document(html);
    targets
        .iter()
        .map(|(selector, attribute)| match Selector::parse(selector) {
            Ok(parsed) => {
                let values: Vec<&str> = document
                    .select(&parsed)
                    .filter_map(|el| el.value().attr(attribute))
                    .collect();
                json!({
                    "selector": selector,
                    "attribute": attribute,
                    "values": values,
                    "count": values.len(),
                })
            }
            Err(e) => json!({
                "selector": selector,
                "attribute": attribute,
                "values": Vec::<&str>::new(),
                "count": 0,
                // Named, not swallowed: an empty list from a typo and an empty
                // list from a page that has no such element are different
                // problems with different fixes.
                "error": format!("invalid CSS selector: {e}"),
            }),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = r#"<html><body>
        <a class="nav" href="/one" data-id="1">One</a>
        <a class="nav" href="/two" data-id="2">Two</a>
        <a class="other" href="/three">Three</a>
        <img src="/pic.png">
    </body></html>"#;

    fn target(sel: &str, attr: &str) -> Vec<(String, String)> {
        vec![(sel.to_string(), attr.to_string())]
    }

    #[test]
    fn every_matching_element_contributes_its_attribute() {
        let rows = extract_attributes(PAGE, &target("a.nav", "href"));
        assert_eq!(rows[0]["values"], json!(["/one", "/two"]));
        assert_eq!(rows[0]["count"], json!(2));
    }

    #[test]
    fn a_data_attribute_reads_like_any_other() {
        let rows = extract_attributes(PAGE, &target("a.nav", "data-id"));
        assert_eq!(rows[0]["values"], json!(["1", "2"]));
    }

    #[test]
    fn an_element_without_the_attribute_is_skipped_not_nulled() {
        // A null in the list would force every consumer to filter before use,
        // and `values.len()` would stop meaning "how many I found".
        let rows = extract_attributes(PAGE, &target("a", "data-id"));
        assert_eq!(rows[0]["values"], json!(["1", "2"]));
        assert_eq!(rows[0]["count"], json!(2));
    }

    #[test]
    fn a_selector_matching_nothing_is_an_empty_list_not_an_error() {
        let rows = extract_attributes(PAGE, &target(".absent", "href"));
        assert_eq!(rows[0]["count"], json!(0));
        assert!(rows[0].get("error").is_none());
    }

    #[test]
    fn a_malformed_selector_is_reported_without_losing_the_others() {
        // The regression this guards: failing the whole scrape over one typo
        // throws away the answers to every other question the caller asked.
        let targets = vec![
            (">>>bad".to_string(), "href".to_string()),
            ("a.nav".to_string(), "href".to_string()),
        ];
        let rows = extract_attributes(PAGE, &targets);
        assert!(rows[0].get("error").is_some(), "{:?}", rows[0]);
        assert_eq!(rows[1]["count"], json!(2), "good row must survive");
    }

    #[test]
    fn rows_come_back_in_the_order_they_were_asked() {
        let targets = vec![
            ("img".to_string(), "src".to_string()),
            ("a.nav".to_string(), "href".to_string()),
        ];
        let rows = extract_attributes(PAGE, &targets);
        assert_eq!(rows[0]["selector"], json!("img"));
        assert_eq!(rows[1]["selector"], json!("a.nav"));
    }

    #[test]
    fn no_targets_means_no_rows_rather_than_a_parse() {
        assert!(extract_attributes(PAGE, &[]).is_empty());
    }
}
