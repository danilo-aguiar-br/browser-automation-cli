// SPDX-License-Identifier: MIT OR Apache-2.0
//! Capture store query / policy / import surfaces (sync CLI handlers).

use std::path::PathBuf;

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};
use crate::xdg;

use super::ca::ensure_ca;
use super::types::{BTreeMapString, CapturedExchange, MitmCapture};
use super::util::{atomic_write, now_ms};

/// Default capture file for this user (latest).
pub fn default_capture_path() -> Result<PathBuf, CliError> {
    Ok(xdg::mitm_capture_dir()?.join("capture.json"))
}

/// Resolve the capture file a query should read (GAP-009).
///
/// Returns the path plus whether the operator named it explicitly, which is
/// what allows a cross-invocation read.
pub fn resolve_capture_path(explicit: Option<&str>) -> Result<(PathBuf, bool), CliError> {
    match explicit {
        Some(p) if !p.trim().is_empty() => {
            let path = PathBuf::from(p.trim());
            crate::fs_roots::ensure_read_allowed(&path)?;
            Ok((path, true))
        }
        _ => Ok((default_capture_path()?, false)),
    }
}

/// Status of MITM readiness + capture counts.
pub fn status(capture_path: Option<&str>) -> Result<Value, CliError> {
    let ca = ensure_ca()?;
    let (path, explicit) = resolve_capture_path(capture_path)?;
    let cap = MitmCapture::load_scoped(&path, true, explicit)?;
    Ok(json!({
        "ok": true,
        "ca": ca,
        "capture_path": path.display().to_string(),
        "count": cap.items.len(),
        "ws_count": cap.ws_frames.len(),
        "websocket": true,
        "bind_policy": "127.0.0.1 only",
        "proxy_running": false,
        "note": "one-shot: use `mitm start --seconds N` (hudsucker on 127.0.0.1 only; WS frames recorded)",
    }))
}

/// List captured requests.
pub fn list(
    host_filter: Option<&str>,
    limit: usize,
    capture_path: Option<&str>,
) -> Result<Value, CliError> {
    let (path, explicit) = resolve_capture_path(capture_path)?;
    let cap = MitmCapture::load_scoped(&path, true, explicit)?;
    let limit = limit.clamp(
        1,
        crate::xdg::policy::policy_usize(crate::xdg::policy::key::MITM_LIST_LIMIT_MAX),
    );
    let items: Vec<Value> = cap
        .items
        .iter()
        .filter(|e| {
            host_filter
                .map(|h| e.host.as_deref() == Some(h) || e.url.contains(h))
                .unwrap_or(true)
        })
        .take(limit)
        .map(|e| {
            json!({
                "id": e.id,
                "method": e.method,
                "url": e.url,
                "status": e.status,
                "host": e.host,
                "content_type": e.content_type,
            })
        })
        .collect();
    Ok(json!({
        "count": items.len(),
        "items": items,
        "capture_path": path.display().to_string(),
    }))
}

/// Get one exchange by id.
pub fn get(id: u64, capture_path: Option<&str>) -> Result<Value, CliError> {
    let (path, explicit) = resolve_capture_path(capture_path)?;
    let cap = MitmCapture::load_scoped(&path, true, explicit)?;
    let item = cap
        .items
        .iter()
        .find(|e| e.id == id)
        .ok_or_else(|| CliError::new(ErrorKind::NoInput, format!("mitm id not found: {id}")))?;
    serde_json::to_value(item).map_err(|e| CliError::new(ErrorKind::Data, format!("mitm get: {e}")))
}

/// Import CDP-style network events (array of {method,url,status,...}) into capture.
pub fn import_cdp_network(events: &[Value]) -> Result<Value, CliError> {
    let path = default_capture_path()?;
    let mut cap = MitmCapture::load(&path, true)?;
    let mut n = 0u64;
    for ev in events {
        let method = ev
            .get("method")
            .or_else(|| ev.get("request_method"))
            .and_then(|v| v.as_str())
            .unwrap_or("GET")
            .to_string();
        let url = ev
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if url.is_empty() {
            continue;
        }
        let host = url::Url::parse(&url)
            .ok()
            .and_then(|u| u.host_str().map(|s| s.to_string()));
        let status = ev
            .get("status")
            .or_else(|| ev.get("status_code"))
            .and_then(|v| v.as_u64())
            .map(|n| n as u16);
        let mut req_h = BTreeMapString::new();
        if let Some(obj) = ev.get("request_headers").and_then(|h| h.as_object()) {
            for (k, v) in obj {
                if let Some(s) = v.as_str() {
                    req_h.insert(k.clone(), s.to_string());
                }
            }
        }
        let mut res_h = BTreeMapString::new();
        if let Some(obj) = ev.get("response_headers").and_then(|h| h.as_object()) {
            for (k, v) in obj {
                if let Some(s) = v.as_str() {
                    res_h.insert(k.clone(), s.to_string());
                }
            }
        }
        cap.push(CapturedExchange {
            id: 0,
            method,
            url,
            status,
            content_type: ev
                .get("mimeType")
                .or_else(|| ev.get("content_type"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            request_headers: req_h,
            response_headers: res_h,
            request_body: None,
            response_body: None,
            host,
            started_ms: now_ms(),
            finished_ms: None,
        });
        n += 1;
    }
    let saved = cap.save()?;
    Ok(json!({ "imported": n, "path": saved.display().to_string(), "total": cap.items.len() }))
}

/// List GraphQL-ish exchanges from the current capture (GAP-019).
///
/// Fixes key mismatch: [`super::analyze::apis`] emits `"apis"`, not `"endpoints"`.
pub fn graphql(limit: usize, capture_path: Option<&str>) -> Result<Value, CliError> {
    super::analyze::apis(Some("graphql"), capture_path).map(|mut v| {
        if let Some(arr) = v.get_mut("apis").and_then(|x| x.as_array_mut()) {
            arr.truncate(limit.max(1));
            v["count"] = json!(arr.len());
        }
        v["kind"] = json!("graphql");
        v
    })
}

/// List WebSocket frames from capture (GAP-019).
pub fn ws_list(limit: usize, capture_path: Option<&str>) -> Result<Value, CliError> {
    let (path, explicit) = resolve_capture_path(capture_path)?;
    let cap = MitmCapture::load_scoped(&path, true, explicit)?;
    let items: Vec<_> = cap.ws_frames.iter().take(limit.max(1)).cloned().collect();
    Ok(json!({
        "count": items.len(),
        "total": cap.ws_frames.len(),
        "frames": items,
    }))
}

/// Get one WebSocket frame by index id (GAP-019).
pub fn ws_get(id: u64, capture_path: Option<&str>) -> Result<Value, CliError> {
    let (path, explicit) = resolve_capture_path(capture_path)?;
    let cap = MitmCapture::load_scoped(&path, true, explicit)?;
    let frame = cap
        .ws_frames
        .get(id as usize)
        .ok_or_else(|| CliError::new(ErrorKind::NoInput, format!("ws frame id {id} not found")))?;
    serde_json::to_value(frame)
        .map_err(|e| CliError::new(ErrorKind::Data, format!("ws get serialize: {e}")))
}

/// Persist block rule note under XDG state (applied on next capture when hosts filter used).
pub fn block_rule(host: Option<&str>, path: Option<&str>) -> Result<Value, CliError> {
    if host.is_none() && path.is_none() {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            "mitm block requires --host and/or --path",
            crate::i18n::suggestion_key("mitm_block_target", None),
        ));
    }
    let dir = xdg::mitm_capture_dir()?;
    let rules = dir.join("block_rules.json");
    let mut list: Vec<Value> = if rules.exists() {
        crate::json_util::read_json_value_file(&rules, crate::xdg::resolve_max_json_file_bytes())
            .ok()
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    list.push(json!({ "host": host, "path": path }));
    let bytes = serde_json::to_vec_pretty(&list)
        .map_err(|e| CliError::new(ErrorKind::Data, format!("block rules json: {e}")))?;
    atomic_write(&rules, &bytes)?;
    Ok(json!({ "ok": true, "rules_path": rules.display().to_string(), "count": list.len() }))
}

/// Persist allowlist host under XDG state.
pub fn allow_host(host: &str) -> Result<Value, CliError> {
    let dir = xdg::mitm_capture_dir()?;
    let rules = dir.join("allow_hosts.json");
    let mut list: Vec<String> = if rules.exists() {
        crate::json_util::read_json_file(&rules, crate::xdg::resolve_max_json_file_bytes())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    if !list.iter().any(|h| h == host) {
        list.push(host.to_string());
    }
    let bytes = serde_json::to_vec_pretty(&list)
        .map_err(|e| CliError::new(ErrorKind::Data, format!("allow hosts json: {e}")))?;
    atomic_write(&rules, &bytes)?;
    Ok(json!({ "ok": true, "hosts": list, "path": rules.display().to_string() }))
}

/// Redact policy status (always redacts Authorization/Cookie by default in capture store).
pub fn redact_policy(secrets: bool) -> Result<Value, CliError> {
    Ok(json!({
        "ok": true,
        "redact_secrets": secrets,
        "note": "Capture store redacts Authorization/Cookie when redact=true on load/save",
    }))
}
