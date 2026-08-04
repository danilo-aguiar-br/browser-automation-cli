// SPDX-License-Identifier: MIT OR Apache-2.0
//! JSON-LD block extraction and Product typing.

use scraper::{Html, Selector};
use serde_json::{json, Value};

/// Find the first `schema.org/Product` JSON-LD node (direct, array or `@graph`).
pub(crate) fn extract_json_ld_product(html: &str) -> Value {
    let blocks = extract_all_json_ld(html);
    for v in blocks {
        if is_product_ld(&v) {
            return json!({ "found": true, "json_ld": v });
        }
        if let Some(arr) = v.as_array() {
            for item in arr {
                if is_product_ld(item) {
                    return json!({ "found": true, "json_ld": item.clone() });
                }
            }
        }
        if let Some(graph) = v.get("@graph").and_then(|g| g.as_array()) {
            for item in graph {
                if is_product_ld(item) {
                    return json!({ "found": true, "json_ld": item.clone() });
                }
            }
        }
    }
    json!({ "found": false })
}

/// Collect every `application/ld+json` block (try-parse; skip invalid).
pub(crate) fn extract_all_json_ld(html: &str) -> Vec<Value> {
    let doc = Html::parse_document(html);
    let Ok(sel) = Selector::parse("script[type=\"application/ld+json\"]") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for el in doc.select(&sel) {
        let raw = el.text().collect::<String>();
        if let Ok(v) = crate::json_util::value_from_str(&raw) {
            out.push(v);
        }
    }
    out
}

/// True when `@type` is `Product` (string or array member, case-insensitive).
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
