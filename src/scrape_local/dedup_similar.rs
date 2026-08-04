// SPDX-License-Identifier: MIT OR Apache-2.0
//! Near-duplicate page collapsing by SimHash content similarity.
//!
//! The ordinary `--dedup-key` path collapses rows whose *key* is equal. This
//! module collapses rows whose *content* is nearly equal — the boilerplate
//! mirror, the `?utm_source=` twin, the printer-friendly variant — which an
//! exact key can never catch.
//!
//! # Emission contract
//!
//! Collapsing silently would let an agent believe pages were lost. The envelope
//! therefore always reports `similar_collapsed` and `similar_distance` when the
//! pass runs, and each surviving row that absorbed duplicates carries
//! `similar_duplicates` (count) plus `similar_duplicate_urls`.

use serde_json::{json, Value};

use super::simhash::SimHash;

/// Collapse near-duplicate rows in a batch/crawl envelope.
///
/// A no-op when `enabled` is false, so the default path costs nothing. First
/// row wins; later rows within `max_distance` bits are absorbed into it.
/// Rows whose extracted content is blank are never collapsed — an empty
/// fingerprint carries no evidence of duplication.
pub fn dedup_similar_pages_envelope(mut value: Value, enabled: bool, max_distance: u32) -> Value {
    if !enabled {
        return value;
    }
    let Some(obj) = value.as_object_mut() else {
        return value;
    };
    // `pages` and `results` alias the same rows in the batch envelope, so the
    // collapse runs **once** and the outcome is mirrored. Collapsing each key
    // independently would double-count `similar_collapsed`.
    let source_key = ["pages", "results"]
        .into_iter()
        .find(|k| matches!(obj.get(*k), Some(Value::Array(_))));
    let Some(source_key) = source_key else {
        obj.insert("similar_collapsed".into(), json!(0));
        obj.insert("similar_distance".into(), json!(max_distance));
        return value;
    };
    let Some(Value::Array(arr)) = obj.get_mut(source_key) else {
        return value;
    };
    let collapsed = collapse_array(arr, max_distance);
    let kept = std::mem::take(arr);
    let count = kept.len();
    for mirror in ["pages", "results"] {
        if obj.contains_key(mirror) {
            obj.insert(mirror.into(), Value::Array(kept.clone()));
        }
    }
    obj.insert("count".into(), json!(count));
    obj.insert("similar_collapsed".into(), json!(collapsed));
    obj.insert("similar_distance".into(), json!(max_distance));
    value
}

/// Collapse one array in place; returns how many rows were absorbed.
fn collapse_array(arr: &mut Vec<Value>, max_distance: u32) -> usize {
    // (fingerprint, index of the surviving row it belongs to)
    let mut kept: Vec<(SimHash, usize)> = Vec::with_capacity(arr.len());
    let mut absorbed_by: Vec<Option<usize>> = Vec::with_capacity(arr.len());
    for row in arr.iter() {
        let hash = SimHash::of(&row_content(row));
        if hash.is_empty() {
            absorbed_by.push(None);
            continue;
        }
        match kept
            .iter()
            .find(|(h, _)| h.is_near_duplicate(hash, max_distance))
        {
            Some((_, survivor)) => absorbed_by.push(Some(*survivor)),
            None => {
                kept.push((hash, absorbed_by.len()));
                absorbed_by.push(None);
            }
        }
    }

    // Record what each survivor absorbed *before* any row is dropped, so the
    // annotation is written while the original indices still line up.
    let mut duplicates: Vec<Vec<String>> = vec![Vec::new(); arr.len()];
    for (idx, survivor) in absorbed_by.iter().enumerate() {
        if let Some(s) = survivor {
            duplicates[*s].push(row_url(&arr[idx]));
        }
    }
    for (idx, dups) in duplicates.into_iter().enumerate() {
        if dups.is_empty() {
            continue;
        }
        if let Some(obj) = arr[idx].as_object_mut() {
            obj.insert("similar_duplicates".into(), json!(dups.len()));
            obj.insert("similar_duplicate_urls".into(), json!(dups));
        }
    }

    let collapsed = absorbed_by.iter().filter(|a| a.is_some()).count();
    let survivors: Vec<Value> = std::mem::take(arr)
        .into_iter()
        .zip(absorbed_by)
        .filter(|(_, absorbed)| absorbed.is_none())
        .map(|(row, _)| row)
        .collect();
    *arr = survivors;
    collapsed
}

/// Text used as the similarity basis, richest field first.
///
/// `text` and `markdown` reflect what the operator asked for; `html` is the
/// last resort for `--format html`. Missing everywhere yields an empty string,
/// which [`SimHash::is_empty`] then protects from collapsing.
fn row_content(row: &Value) -> String {
    for key in ["text", "markdown", "html"] {
        if let Some(s) = row.get(key).and_then(Value::as_str) {
            if !s.trim().is_empty() {
                return s.to_string();
            }
        }
    }
    String::new()
}

/// Best available identifier for a collapsed row.
fn row_url(row: &Value) -> String {
    row.get("source_url")
        .or_else(|| row.get("url"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(url: &str, text: &str) -> Value {
        json!({ "source_url": url, "text": text, "status_code": 200 })
    }

    const BODY: &str = "Rust is a systems programming language focused on safety \
                        speed and concurrency without a garbage collector at all";

    #[test]
    fn disabled_is_a_no_op() {
        let env = json!({ "count": 2, "pages": [page("a", BODY), page("b", BODY)] });
        let out = dedup_similar_pages_envelope(env.clone(), false, 3);
        assert_eq!(out, env);
        assert!(out.get("similar_collapsed").is_none());
    }

    #[test]
    fn identical_content_collapses_and_is_reported() {
        let env = json!({ "count": 2, "pages": [page("a", BODY), page("b", BODY)] });
        let out = dedup_similar_pages_envelope(env, true, 3);
        assert_eq!(out["count"], json!(1));
        assert_eq!(out["similar_collapsed"], json!(1));
        assert_eq!(out["similar_distance"], json!(3));
        assert_eq!(out["pages"].as_array().unwrap().len(), 1);
        assert_eq!(out["pages"][0]["source_url"], json!("a"));
        assert_eq!(out["pages"][0]["similar_duplicates"], json!(1));
        assert_eq!(out["pages"][0]["similar_duplicate_urls"], json!(["b"]));
    }

    #[test]
    fn distinct_content_is_preserved() {
        let env = json!({
            "count": 2,
            "pages": [
                page("a", "quarterly financial results revenue growth margins guidance"),
                page("b", "baking sourdough bread patience starter hydration flour oven"),
            ],
        });
        let out = dedup_similar_pages_envelope(env, true, 3);
        assert_eq!(out["count"], json!(2));
        assert_eq!(out["similar_collapsed"], json!(0));
        assert!(out["pages"][0].get("similar_duplicates").is_none());
    }

    #[test]
    fn distance_zero_demands_identical_fingerprints() {
        let env = json!({
            "count": 2,
            "pages": [page("a", BODY), page("b", &format!("{BODY} plus an extra tail clause"))],
        });
        let out = dedup_similar_pages_envelope(env, true, 0);
        assert_eq!(out["count"], json!(2));
        assert_eq!(out["similar_collapsed"], json!(0));
    }

    #[test]
    fn blank_rows_are_never_collapsed() {
        let env = json!({
            "count": 3,
            "pages": [page("a", ""), page("b", ""), page("c", BODY)],
        });
        let out = dedup_similar_pages_envelope(env, true, 8);
        assert_eq!(out["count"], json!(3));
        assert_eq!(out["similar_collapsed"], json!(0));
    }

    #[test]
    fn batch_results_array_is_also_collapsed() {
        let env = json!({ "count": 2, "results": [page("a", BODY), page("b", BODY)] });
        let out = dedup_similar_pages_envelope(env, true, 3);
        assert_eq!(out["results"].as_array().unwrap().len(), 1);
        assert_eq!(out["similar_collapsed"], json!(1));
    }

    #[test]
    fn aliased_pages_and_results_collapse_once() {
        // The batch envelope emits the same rows under both keys; collapsing
        // each independently would report double the real number.
        let rows = json!([
            page("a", BODY),
            page("b", BODY),
            page("c", "unrelated words here")
        ]);
        let env = json!({ "count": 3, "pages": rows, "results": rows });
        let out = dedup_similar_pages_envelope(env, true, 8);
        assert_eq!(out["similar_collapsed"], json!(1));
        assert_eq!(out["count"], json!(2));
        assert_eq!(out["pages"], out["results"]);
    }

    #[test]
    fn envelope_without_rows_still_reports_zero() {
        let out = dedup_similar_pages_envelope(json!({ "count": 0 }), true, 3);
        assert_eq!(out["similar_collapsed"], json!(0));
        assert_eq!(out["similar_distance"], json!(3));
    }

    #[test]
    fn three_way_duplicate_reports_both_absorbed_urls() {
        let env = json!({
            "count": 3,
            "pages": [page("a", BODY), page("b", BODY), page("c", BODY)],
        });
        let out = dedup_similar_pages_envelope(env, true, 3);
        assert_eq!(out["count"], json!(1));
        assert_eq!(out["similar_collapsed"], json!(2));
        assert_eq!(out["pages"][0]["similar_duplicate_urls"], json!(["b", "c"]));
    }
}
