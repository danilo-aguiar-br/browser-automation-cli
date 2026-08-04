// SPDX-License-Identifier: MIT OR Apache-2.0
//! Row operations on scrape envelopes: filter, sort and dedup of pages/results.

use serde_json::{json, Value};

/// Filter a page object by simple `key=value` / `key!=value` expressions (AND).
///
/// Empty filter keeps the page. Used by batch/crawl.
pub fn page_matches_filter(page: &Value, filter: Option<&str>) -> bool {
    let Some(f) = filter.map(str::trim).filter(|s| !s.is_empty()) else {
        return true;
    };
    for part in f.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (key, op_ne, expect) = if let Some((k, v)) = part.split_once("!=") {
            (k.trim(), true, v.trim())
        } else if let Some((k, v)) = part.split_once('=') {
            (k.trim(), false, v.trim())
        } else {
            continue;
        };
        let actual = page
            .get(key)
            .map(|v| match v {
                Value::String(s) => s.clone(),
                Value::Bool(b) => b.to_string(),
                Value::Number(n) => n.to_string(),
                Value::Null => "null".into(),
                _ => v.to_string(),
            })
            // Success pages omit http_error; treat missing as false so
            // agent `--filter http_error=false` keeps OK rows (P0 residual-04).
            .unwrap_or_else(|| {
                if key == "http_error" {
                    "false".into()
                } else {
                    String::new()
                }
            });
        let ok = if op_ne {
            actual != expect
        } else {
            actual == expect
        };
        if !ok {
            return false;
        }
    }
    true
}

/// Apply filter to a pages/results array inside a batch/crawl envelope.
pub fn filter_pages_envelope(mut value: Value, filter: Option<&str>) -> Value {
    if filter.map(str::trim).filter(|s| !s.is_empty()).is_none() {
        return value;
    }
    let Some(obj) = value.as_object_mut() else {
        return value;
    };
    let mut new_count = None;
    for arr_key in ["pages", "results"] {
        if let Some(Value::Array(pages)) = obj.get_mut(arr_key) {
            pages.retain(|p| page_matches_filter(p, filter));
            new_count = Some(pages.len());
        }
    }
    if let Some(n) = new_count {
        obj.insert("count".into(), json!(n));
    }
    value
}

/// Sort `pages` / `results` / `urls` arrays by a string field (asc).
pub fn sort_pages_envelope(mut value: Value, sort_key: Option<&str>) -> Value {
    let Some(key) = sort_key.map(str::trim).filter(|s| !s.is_empty()) else {
        return value;
    };
    let Some(obj) = value.as_object_mut() else {
        return value;
    };
    for arr_key in ["pages", "results", "urls"] {
        if let Some(Value::Array(arr)) = obj.get_mut(arr_key) {
            arr.sort_by(|a, b| {
                let sa = page_sort_str(a, key);
                let sb = page_sort_str(b, key);
                sa.cmp(&sb)
            });
        }
    }
    value
}

fn page_sort_str(v: &Value, key: &str) -> String {
    if let Some(s) = v.get(key).and_then(|x| x.as_str()) {
        return s.to_string();
    }
    if let Some(s) = v.as_str() {
        // map urls array of strings
        if key == "url" || key == "source_url" {
            return s.to_string();
        }
    }
    v.get(key).map(|x| x.to_string()).unwrap_or_default()
}

/// Deduplicate `pages`/`results` by a field (first wins).
///
/// For `source_url` / `url` keys, normalize via path_filter (strip fragment;
/// optional query already handled when crawl used `--ignore-query-params`).
pub fn dedup_pages_envelope(mut value: Value, dedup_key: Option<&str>) -> Value {
    let Some(key) = dedup_key.map(str::trim).filter(|s| !s.is_empty()) else {
        return value;
    };
    let Some(obj) = value.as_object_mut() else {
        return value;
    };
    let normalize = key == "source_url" || key == "url";
    for arr_key in ["pages", "results"] {
        if let Some(Value::Array(arr)) = obj.get_mut(arr_key) {
            let mut seen = std::collections::BTreeSet::new();
            arr.retain(|p| {
                let k = page_dedup_str(p, key, normalize);
                seen.insert(k)
            });
        }
    }
    if let Some(Value::Array(arr)) = obj.get_mut("urls") {
        let mut seen = std::collections::BTreeSet::new();
        arr.retain(|p| {
            let k = page_dedup_str(p, key, normalize);
            seen.insert(k)
        });
    }
    value
}

fn page_dedup_str(v: &Value, key: &str, normalize_url: bool) -> String {
    let raw = page_sort_str(v, key);
    if !normalize_url || raw.is_empty() {
        return raw;
    }
    // ignore_query=true collapses ?a vs bare; trailing slash handled in normalize
    super::path_filter::normalize_url_for_dedup_ex(&raw, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_status() {
        let p = json!({"status_code": 200, "http_error": false});
        assert!(page_matches_filter(&p, Some("status_code=200")));
        assert!(!page_matches_filter(&p, Some("status_code=404")));
        assert!(page_matches_filter(&p, Some("http_error=false")));
        // missing http_error treated as false (P0 residual-04)
        let ok = json!({"source_url": "u", "status_code": 200});
        assert!(page_matches_filter(&ok, Some("http_error=false")));
    }
}
