//! Offline analysis of Chrome Tracing event dumps produced by `perf stop`.
//!
//! Input formats supported:
//! - NDJSON lines, each an array of trace events
//! - A single JSON array of events
//! - Nested arrays from `Tracing.dataCollected` value payloads

use std::collections::HashMap;
use std::path::Path;

use serde_json::{json, Value};

#[derive(Default)]
struct Acc {
    event_count: u64,
    by_name: HashMap<String, u64>,
    by_cat: HashMap<String, u64>,
    durations_ms: HashMap<String, f64>,
    navigation_start: Option<f64>,
    fcp_ms: Option<f64>,
    lcp_ms: Option<f64>,
    dcl_ms: Option<f64>,
    load_ms: Option<f64>,
    ttfb_ms: Option<f64>,
    cls_score: Option<f64>,
    long_tasks: u64,
    layout_shifts: u64,
}

/// Analyze a trace file written by `perf stop` (NDJSON of event arrays).
pub fn analyze_file(path: &Path, name_filter: Option<&str>) -> Result<Value, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| format!("trace read: {e}"))?;
    analyze_text(&raw, name_filter, Some(path.to_string_lossy().as_ref()))
}

/// Analyze in-memory NDJSON / JSON chunks joined with newlines.
pub fn analyze_text(
    raw: &str,
    name_filter: Option<&str>,
    path: Option<&str>,
) -> Result<Value, String> {
    let mut acc = Acc::default();
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(json!({
            "perf": "insight",
            "path": path,
            "name": name_filter,
            "event_count": 0,
            "note": "empty trace; run perf start then stop with --path in the same run",
            "offline": true,
        }));
    }

    // Prefer whole-file JSON array first.
    if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
        walk_value(&v, &mut acc);
    } else {
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<Value>(line) {
                walk_value(&v, &mut acc);
            }
        }
    }

    if let Some(filter) = name_filter {
        acc.by_name.retain(|k, _| k.contains(filter));
    }

    let mut top_events: Vec<(String, u64)> = acc.by_name.into_iter().collect();
    top_events.sort_by_key(|b| std::cmp::Reverse(b.1));
    top_events.truncate(25);

    let mut top_cats: Vec<(String, u64)> = acc.by_cat.into_iter().collect();
    top_cats.sort_by_key(|b| std::cmp::Reverse(b.1));
    top_cats.truncate(15);

    let mut slowest: Vec<(String, f64)> = acc.durations_ms.into_iter().collect();
    slowest.sort_by(|a, b| match b.1.partial_cmp(&a.1) {
        Some(o) => o,
        None => std::cmp::Ordering::Equal,
    });
    slowest.truncate(15);

    Ok(json!({
        "perf": "insight",
        "path": path,
        "name": name_filter,
        "event_count": acc.event_count,
        "top_events": top_events.into_iter().map(|(n, c)| json!({"name": n, "count": c})).collect::<Vec<_>>(),
        "top_categories": top_cats.into_iter().map(|(n, c)| json!({"category": n, "count": c})).collect::<Vec<_>>(),
        "slowest_ms": slowest.into_iter().map(|(n, d)| json!({"name": n, "duration_ms": d})).collect::<Vec<_>>(),
        "insights": {
            "long_tasks": acc.long_tasks,
            "layout_shifts": acc.layout_shifts,
            "has_lcp": acc.lcp_ms.is_some(),
            "has_fcp": acc.fcp_ms.is_some(),
            "has_cls": acc.cls_score.is_some(),
        },
        "web_vitals": {
            "fcp_ms": acc.fcp_ms,
            "lcp_ms": acc.lcp_ms,
            "cls": acc.cls_score,
            "ttfb_ms": acc.ttfb_ms,
            "dom_content_loaded_ms": acc.dcl_ms,
            "load_ms": acc.load_ms,
            "navigation_start": acc.navigation_start,
        },
        "offline": true,
    }))
}

fn walk_value(v: &Value, acc: &mut Acc) {
    match v {
        Value::Array(items) => {
            for item in items {
                if item.is_array() {
                    walk_value(item, acc);
                } else if item.is_object() {
                    ingest_event(item, acc);
                }
            }
        }
        Value::Object(_) => ingest_event(v, acc),
        _ => {}
    }
}

fn ingest_event(ev: &Value, acc: &mut Acc) {
    acc.event_count += 1;
    let name = ev
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    *acc.by_name.entry(name.clone()).or_insert(0) += 1;

    if let Some(cat) = ev.get("cat").and_then(|v| v.as_str()) {
        for part in cat.split(',') {
            let p = part.trim();
            if !p.is_empty() {
                *acc.by_cat.entry(p.to_string()).or_insert(0) += 1;
            }
        }
    }

    let ph = ev.get("ph").and_then(|v| v.as_str()).unwrap_or("");
    let ts = ev.get("ts").and_then(|v| v.as_f64());
    let dur = ev.get("dur").and_then(|v| v.as_f64()).map(|d| d / 1000.0); // µs → ms in Chrome traces

    if let Some(d) = dur {
        let entry = acc.durations_ms.entry(name.clone()).or_insert(0.0);
        if d > *entry {
            *entry = d;
        }
    }

    // Heuristics for common timeline marks.
    match name.as_str() {
        "navigationStart" | "NavigationStart" => {
            if acc.navigation_start.is_none() {
                acc.navigation_start = ts.map(|t| t / 1000.0);
            }
        }
        "firstContentfulPaint" | "firstPaint" => {
            if let (Some(nav), Some(t)) = (acc.navigation_start, ts) {
                acc.fcp_ms = Some(t / 1000.0 - nav);
            } else if let Some(d) = dur {
                acc.fcp_ms = Some(d);
            }
        }
        "largestContentfulPaint::Candidate" | "largestContentfulPaint" => {
            if let (Some(nav), Some(t)) = (acc.navigation_start, ts) {
                let v = t / 1000.0 - nav;
                acc.lcp_ms = Some(acc.lcp_ms.map(|old| old.max(v)).unwrap_or(v));
            }
        }
        "domContentLoadedEventEnd" => {
            if let (Some(nav), Some(t)) = (acc.navigation_start, ts) {
                acc.dcl_ms = Some(t / 1000.0 - nav);
            }
        }
        "loadEventEnd" => {
            if let (Some(nav), Some(t)) = (acc.navigation_start, ts) {
                acc.load_ms = Some(t / 1000.0 - nav);
            }
        }
        "responseStart" | "TimeToFirstByte" => {
            if let (Some(nav), Some(t)) = (acc.navigation_start, ts) {
                acc.ttfb_ms = Some(t / 1000.0 - nav);
            } else if let Some(d) = dur {
                acc.ttfb_ms = Some(d);
            }
        }
        "LayoutShift" => {
            acc.layout_shifts += 1;
            if let Some(score) = ev
                .get("args")
                .and_then(|a| a.get("data"))
                .and_then(|d| d.get("score").or_else(|| d.get("weighted_score_delta")))
                .and_then(|v| v.as_f64())
            {
                acc.cls_score = Some(acc.cls_score.unwrap_or(0.0) + score);
            }
        }
        "RunTask" | "RunMicrotasks" => {
            if let Some(d) = dur {
                if d >= 50.0 {
                    acc.long_tasks += 1;
                }
            }
        }
        _ => {
            if name.contains("LayoutShift") {
                acc.layout_shifts += 1;
            }
            if ph == "R" || ph == "I" {
                // keep counts only
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn analyzes_ndjson_events() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.ndjson");
        let body = r#"[{"name":"navigationStart","cat":"blink.user_timing","ph":"R","ts":1000000},{"name":"firstContentfulPaint","cat":"loading","ph":"R","ts":1250000},{"name":"EvaluateScript","cat":"devtools.timeline","ph":"X","ts":1100000,"dur":50000}]
[{"name":"EvaluateScript","cat":"devtools.timeline","ph":"X","ts":1200000,"dur":10000},{"name":"LayoutShift","cat":"loading","ph":"I","ts":1300000,"args":{"data":{"score":0.05}}},{"name":"RunTask","cat":"devtools.timeline","ph":"X","ts":1400000,"dur":60000}]"#;
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(body.as_bytes()).unwrap();
        let v = analyze_file(&path, None).unwrap();
        assert!(v["event_count"].as_u64().unwrap() >= 3);
        assert_eq!(v["offline"], true);
        assert!(!v["top_events"].as_array().unwrap().is_empty());
        assert!(v["web_vitals"]["cls"].as_f64().unwrap() >= 0.05);
        assert!(v["insights"]["long_tasks"].as_u64().unwrap() >= 1);
        assert!(v["insights"]["layout_shifts"].as_u64().unwrap() >= 1);
    }
}
