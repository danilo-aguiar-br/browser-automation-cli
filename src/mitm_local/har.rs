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
            json!({
                "startedDateTime": chrono_like(e.started_ms),
                "time": 0,
                "request": {
                    "method": e.method,
                    "url": e.url,
                    "httpVersion": "HTTP/1.1",
                    "headers": req_headers,
                    "queryString": [],
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
                "timings": { "send": 0, "wait": 0, "receive": 0 },
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
