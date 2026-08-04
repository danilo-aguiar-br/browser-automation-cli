// SPDX-License-Identifier: MIT OR Apache-2.0
//! Agent-native field projection and text caps for scrape envelopes.
//!
//! Row filtering/sorting/dedup lives in [`super::page_ops`]; failure rows live
//! in [`super::error_page`]. This module owns the shape of what is emitted.

use serde_json::{json, Map, Value};

use super::page_ops::{dedup_pages_envelope, filter_pages_envelope, sort_pages_envelope};

/// Project scrape envelopes with agent-friendly select aliases.
///
/// Multi-format envelopes (`formats` map) promote nested fields to top-level
/// when selected (e.g. `--select source_url,markdown` after multi `--format`).
pub fn project_fields(value: Value, select: Option<&str>) -> Value {
    let Some(sel) = select.map(str::trim).filter(|s| !s.is_empty()) else {
        return value;
    };
    // Flatten multi-format for agent CLEAN select before projection.
    let value = flatten_formats_for_select(value, sel);
    let Some(obj) = value.as_object() else {
        return value;
    };
    let mut expanded = Vec::new();
    for key in sel.split([',', ' ']) {
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        if obj.contains_key(key) {
            expanded.push(key.to_string());
            continue;
        }
        let mapped = match key {
            "url" => first_present(obj, &["source_url", "url"]),
            "md" => first_present(obj, &["markdown"]),
            "body" => first_present(obj, &["text", "markdown"]),
            "status" => first_present(obj, &["status_code", "status"]),
            "links_count" | "link_count" => first_present(obj, &["link_count"]),
            _ => None,
        };
        if let Some(m) = mapped {
            expanded.push(m.to_string());
        } else {
            expanded.push(key.to_string());
        }
    }
    crate::json_util::project_fields_plain(value, Some(&expanded.join(",")))
}

/// Promote selected keys from nested `formats.<name>.*` onto the top-level object.
fn flatten_formats_for_select(mut value: Value, sel: &str) -> Value {
    let Some(obj) = value.as_object_mut() else {
        return value;
    };
    let Some(Value::Object(formats)) = obj.get("formats").cloned() else {
        return value;
    };
    let wanted: Vec<&str> = sel
        .split([',', ' '])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    for key in wanted {
        if obj.contains_key(key) {
            continue;
        }
        // Prefer leaf field inside any format payload (markdown string, not whole object).
        let mut found = false;
        for (_fname, part) in &formats {
            if let Some(v) = part.get(key) {
                obj.insert(key.to_string(), v.clone());
                found = true;
                break;
            }
        }
        if found {
            continue;
        }
        // Else promote whole format object when select names a format key.
        if let Some(part) = formats.get(key) {
            obj.insert(key.to_string(), part.clone());
        }
    }
    value
}

fn first_present<'a>(obj: &Map<String, Value>, keys: &[&'a str]) -> Option<&'a str> {
    keys.iter().copied().find(|k| obj.contains_key(*k))
}

/// Truncate string fields `text` and `markdown` to `max_chars` (Unicode scalars).
///
/// Sets `truncated: true` when any field was cut. `max_chars == 0` means no cap.
pub fn apply_max_text_chars(mut value: Value, max_chars: usize) -> Value {
    if max_chars == 0 {
        return value;
    }
    let Some(obj) = value.as_object_mut() else {
        return value;
    };
    let mut truncated = false;
    for key in ["text", "markdown", "summary", "html"] {
        if let Some(Value::String(s)) = obj.get(key) {
            if s.chars().count() > max_chars {
                let cut: String = s.chars().take(max_chars).collect();
                obj.insert(key.to_string(), json!(format!("{cut}…")));
                truncated = true;
            }
        }
    }
    // Multi-format map under "formats"
    if let Some(Value::Object(formats)) = obj.get_mut("formats") {
        for (_k, v) in formats.iter_mut() {
            if let Some(inner) = v.as_object_mut() {
                for key in ["text", "markdown", "summary", "html"] {
                    if let Some(Value::String(s)) = inner.get(key) {
                        if s.chars().count() > max_chars {
                            let cut: String = s.chars().take(max_chars).collect();
                            inner.insert(key.to_string(), json!(format!("{cut}…")));
                            truncated = true;
                        }
                    }
                }
            }
        }
    }
    // Batch/crawl: truncate each page/result
    for arr_key in ["pages", "results"] {
        if let Some(Value::Array(pages)) = obj.get_mut(arr_key) {
            for page in pages.iter_mut() {
                *page = apply_max_text_chars(page.take(), max_chars);
            }
        }
    }
    if truncated {
        obj.insert("truncated".into(), json!(true));
        obj.insert("max_text_chars".into(), json!(max_chars));
    }
    value
}

/// Project nested page/result objects; project top-level only for single-page or map envelopes.
pub fn project_pages_envelope(mut value: Value, select: Option<&str>) -> Value {
    if select.map(str::trim).filter(|s| !s.is_empty()).is_none() {
        return value;
    }
    let has_pages = value.get("pages").is_some() || value.get("results").is_some();
    if has_pages {
        if let Some(obj) = value.as_object_mut() {
            for arr_key in ["pages", "results"] {
                if let Some(Value::Array(pages)) = obj.get_mut(arr_key) {
                    for page in pages.iter_mut() {
                        *page = project_fields(std::mem::take(page), select);
                    }
                }
            }
        }
        // Keep envelope keys (count, seed, errors, …); only pages/results are projected.
        return value;
    }
    // map (urls) or single scrape page object
    project_fields(value, select)
}

/// Post-process scrape/batch/crawl/map envelopes for agent CLEAN STDOUT.
///
/// Order: filter → sort → dedup → max_text → project (including nested pages/results).
pub fn finalize_scrape_value(
    value: Value,
    select: Option<&str>,
    filter: Option<&str>,
    max_text: Option<usize>,
) -> Value {
    finalize_scrape_value_ex(value, select, filter, max_text, None, None)
}

/// Extended finalize with sort/dedup keys (agent CLEAN ops).
pub fn finalize_scrape_value_ex(
    value: Value,
    select: Option<&str>,
    filter: Option<&str>,
    max_text: Option<usize>,
    sort_key: Option<&str>,
    dedup_key: Option<&str>,
) -> Value {
    let mut v = filter_pages_envelope(value, filter);
    v = sort_pages_envelope(v, sort_key);
    v = dedup_pages_envelope(v, dedup_key);
    if let Some(max) = max_text {
        if max > 0 {
            v = apply_max_text_chars(v, max);
        }
    }
    project_pages_envelope(v, select)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_projects() {
        let v = json!({"source_url":"u","title":"t","html":"<x>","text":"hi"});
        let p = project_fields(v, Some("source_url,title,text"));
        assert!(p.get("html").is_none());
        assert_eq!(p["text"], "hi");
    }

    #[test]
    fn max_text_truncates() {
        let v = json!({"text": "abcdefghij"});
        let p = apply_max_text_chars(v, 4);
        assert_eq!(p["truncated"], true);
        assert!(p["text"].as_str().unwrap().starts_with("abcd"));
    }

    #[test]
    fn finalize_select_filter() {
        let v = json!({
            "count": 2,
            "pages": [
                {"source_url":"a","status_code":200,"http_error":false,"html":"<x>","text":"ok"},
                {"source_url":"b","status_code":404,"http_error":true,"text":"no"}
            ]
        });
        let p = finalize_scrape_value(
            v,
            Some("source_url,status_code"),
            Some("http_error=false"),
            None,
        );
        let pages = p["pages"].as_array().unwrap();
        assert_eq!(pages.len(), 1);
        assert!(pages[0].get("html").is_none());
        assert_eq!(pages[0]["source_url"], "a");
    }
}
