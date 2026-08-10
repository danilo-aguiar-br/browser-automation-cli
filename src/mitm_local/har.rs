// SPDX-License-Identifier: MIT OR Apache-2.0
//! HAR 1.2 export (hand-built; no Python mitmproxy).

use std::path::Path;

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};
use crate::xdg;

use super::store::resolve_capture_path;
use super::types::MitmCapture;
use super::util::{atomic_write, chrono_like};

/// Export HAR 1.2 JSON.
pub fn export_har(out: &Path, capture_path: Option<&str>) -> Result<Value, CliError> {
    let (path, explicit) = resolve_capture_path(capture_path)?;
    let cap = MitmCapture::load_scoped(&path, true, explicit)?;
    let entries: Vec<Value> = cap
        .items
        .iter()
        .map(|e| {
            let req_headers: Vec<Value> = e
                .request_headers
                .iter()
                .map(|(n, v)| json!({"name": n, "value": v}))
                .collect();
            let res_headers: Vec<Value> = e
                .response_headers
                .iter()
                .map(|(n, v)| json!({"name": n, "value": v}))
                .collect();
            // Elapsed is measured, not stamped as zero. A HAR whose every entry
            // reads `time: 0` is rejected by the analysers the format exists to
            // feed, so the export was present and useless.
            //
            // `wait` carries the whole interval because the proxy observes one
            // request in and one response out; it has no visibility into the DNS,
            // connect and TLS phases of the upstream socket. Those stay `-1`,
            // which HAR 1.2 defines as "not applicable", rather than `0`, which
            // would claim they took no time.
            let elapsed = e
                .finished_ms
                .and_then(|f| f.checked_sub(e.started_ms))
                .unwrap_or(0);
            json!({
                "startedDateTime": chrono_like(e.started_ms),
                "time": elapsed,
                "request": {
                    "method": e.method,
                    "url": e.url,
                    "httpVersion": "HTTP/1.1",
                    "headers": req_headers,
                    "queryString": query_pairs(&e.url),
                    "cookies": [],
                    "headersSize": -1,
                    "bodySize": e.request_body.as_ref().map(|b| b.len() as i64).unwrap_or(0),
                    "postData": e.request_body.as_ref().map(|b| json!({
                        "mimeType": "application/octet-stream",
                        "text": b,
                    })),
                },
                "response": {
                    "status": e.status.unwrap_or(0),
                    "statusText": "",
                    "httpVersion": "HTTP/1.1",
                    "headers": res_headers,
                    "cookies": [],
                    "content": {
                        "size": e.response_body.as_ref().map(|b| b.len()).unwrap_or(0),
                        "mimeType": e.content_type.clone().unwrap_or_else(|| "application/octet-stream".into()),
                        "text": e.response_body.clone().unwrap_or_default(),
                    },
                    "redirectURL": "",
                    "headersSize": -1,
                    "bodySize": e.response_body.as_ref().map(|b| b.len() as i64).unwrap_or(-1),
                },
                "cache": {},
                "timings": {
                    "blocked": -1,
                    "dns": -1,
                    "connect": -1,
                    "ssl": -1,
                    "send": 0,
                    "wait": elapsed,
                    "receive": 0
                },
            })
        })
        .collect();

    let har = json!({
        "log": {
            "version": "1.2",
            "creator": {
                "name": "browser-automation-cli",
                "version": env!("CARGO_PKG_VERSION"),
            },
            "entries": entries,
        }
    });
    let bytes = serde_json::to_vec_pretty(&har)
        .map_err(|e| CliError::new(ErrorKind::Data, format!("har json: {e}")))?;
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            xdg::ensure_dir(parent)?;
        }
    }
    atomic_write(out, &bytes)?;
    Ok(json!({
        "path": out.display().to_string(),
        "entries": entries.len(),
        "format": "HAR 1.2",
    }))
}

/// Split a URL's query string into HAR `queryString` pairs.
///
/// Emitted because the field is required by HAR 1.2 and an empty array claims
/// the URL had no query, which is a different statement from "not parsed".
fn query_pairs(url: &str) -> Vec<Value> {
    let Some(q) = url.split_once('?').map(|(_, q)| q) else {
        return Vec::new();
    };
    q.split('&')
        .filter(|p| !p.is_empty())
        .map(|pair| match pair.split_once('=') {
            Some((name, value)) => json!({"name": name, "value": value}),
            None => json!({"name": pair, "value": ""}),
        })
        .collect()
}
