// SPDX-License-Identifier: MIT OR Apache-2.0
//! Capture store query / policy / import surfaces (sync CLI handlers).

use std::path::PathBuf;

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};
use crate::xdg;

use super::ca::ensure_ca;
use super::types::{BTreeMapString, BlockRule, CapturedExchange, MitmCapture};
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
    let cap = MitmCapture::load_scoped(&path, super::policy::redact_secrets(), explicit)?;
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
    let cap = MitmCapture::load_scoped(&path, super::policy::redact_secrets(), explicit)?;
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
    let cap = MitmCapture::load_scoped(&path, super::policy::redact_secrets(), explicit)?;
    let item = cap
        .items
        .iter()
        .find(|e| e.id == id)
        .ok_or_else(|| CliError::new(ErrorKind::NoInput, format!("mitm id not found: {id}")))?;
    serde_json::to_value(item).map_err(|e| CliError::new(ErrorKind::Data, format!("mitm get: {e}")))
}

/// Map one CDP-style network event onto a [`CapturedExchange`].
///
/// Returns `None` for an event with no `url`: every other field has a defensible
/// default, but an exchange addressed by nothing is not an exchange.
///
/// # Why this is split out of [`import_cdp_network`]
///
/// The public importer resolves the operator's XDG capture file and writes to
/// it, so exercising it end to end would mutate real user state. That left the
/// key mapping — sixty of its sixty-six lines, and the only part that can be
/// wrong — with no test at all. This module already paid for that once, when
/// `store::graphql` read `endpoints` while `analyze::apis` wrote `apis`.
///
/// The alias reads (`request_method`, `status_code`, `content_type`) are kept
/// because the events arrive from a LIBRARY caller, not from a producer in this
/// repository — unlike the `request_method` fallback removed from `proxy.rs`,
/// which read a key no site here ever wrote. `cdp_event_alias_keys_are_read`
/// pins them, so they are now a tested contract instead of speculation.
pub(super) fn cdp_event_to_exchange(ev: &Value) -> Option<CapturedExchange> {
    let url = ev.get("url").and_then(|v| v.as_str()).unwrap_or("");
    if url.is_empty() {
        return None;
    }
    let url = url.to_string();
    let method = ev
        .get("method")
        .or_else(|| ev.get("request_method"))
        .and_then(|v| v.as_str())
        .unwrap_or("GET")
        .to_string();
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
    Some(CapturedExchange {
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
    })
}

/// Import CDP-style network events (array of `{method,url,status,...}`) into capture.
///
/// The mapping itself lives in `cdp_event_to_exchange`, which is where the
/// tests reach it; this function is the I/O shell around it. That name is
/// `pub(super)`, so it is written as plain code and NOT as an intra-doc link:
/// a link from public documentation to a private item is an error under
/// `-D rustdoc::private-intra-doc-links`, which is what `cargo doc` and
/// `scripts/docs-check.sh` both enforce.
///
/// # Errors
///
/// Fails when the XDG capture path cannot be resolved, loaded, or written.
pub fn import_cdp_network(events: &[Value]) -> Result<Value, CliError> {
    let path = default_capture_path()?;
    let mut cap = MitmCapture::load(&path, super::policy::redact_secrets())?;
    let mut n = 0u64;
    for ev in events {
        if let Some(exchange) = cdp_event_to_exchange(ev) {
            cap.push(exchange);
            n += 1;
        }
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
    let cap = MitmCapture::load_scoped(&path, super::policy::redact_secrets(), explicit)?;
    let items: Vec<_> = cap.ws_frames.iter().take(limit.max(1)).cloned().collect();
    Ok(json!({
        "count": items.len(),
        "total": cap.ws_frames.len(),
        "frames": items,
    }))
}

/// Get one WebSocket frame by index id (GAP-019).
///
/// # Why this index agrees with [`ws_list`], and what would break it
///
/// The 0.1.9 audit found `net get 0` and `console get 0` addressing DIFFERENT
/// records than the `list` beside them (findings H5 and H6), and left the mitm
/// pair unmeasured. Measured 2026-08-30: this pair does NOT have that defect,
/// and the reason is worth writing down because it is a property of the code
/// rather than a coincidence.
///
/// [`ws_list`] applies no filter and no offset — it is `take(limit)` from the
/// front — so the Nth frame it returns is the Nth frame of the buffer, which
/// is what this index addresses. [`get`] is safe for a different reason: it
/// matches on the STORED `e.id`, which `list` emits on every row, so the
/// caller never has to infer a position at all.
///
/// The fragile one is this function. Adding a host filter, a `skip`, or any
/// ordering to [`ws_list`] silently breaks the agreement, because nothing
/// here would change and nothing would report it. A filter added there must
/// come with an explicit `id` on each row, the way [`list`] already does.
pub fn ws_get(id: u64, capture_path: Option<&str>) -> Result<Value, CliError> {
    let (path, explicit) = resolve_capture_path(capture_path)?;
    let cap = MitmCapture::load_scoped(&path, super::policy::redact_secrets(), explicit)?;
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

/// Read the persisted `mitm block` rules; empty when there are none.
///
/// # Why this never returns an error
///
/// A missing, unreadable or malformed rules file means "no rules". Refusing to
/// start the proxy over a corrupt cache would trade a degraded feature for an
/// unusable one, and the rules file is a convenience cache, not an input the
/// operator hand-writes.
///
/// # Why it exists
///
/// [`block_rule`] has been writing this file and answering `{"ok": true}` since
/// `mitm block` shipped, and nothing read it back — so the command persisted a
/// rule and refused nothing. This is the read half that makes the write mean
/// something.
#[must_use]
pub fn load_block_rules() -> Vec<BlockRule> {
    let Ok(dir) = xdg::mitm_capture_dir() else {
        return Vec::new();
    };
    let rules = dir.join("block_rules.json");
    if !rules.exists() {
        return Vec::new();
    }
    crate::json_util::read_json_value_file(&rules, crate::xdg::resolve_max_json_file_bytes())
        .ok()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
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

/// Where the operator's persisted redaction preference lives.
fn redact_pref_path() -> Result<std::path::PathBuf, CliError> {
    Ok(xdg::mitm_capture_dir()?.join("redact_policy.json"))
}

/// The persisted redaction preference, or `None` when none was ever set.
///
/// `None` is deliberately not `Some(true)`: "never chose" and "chose to mask"
/// look identical in the capture, but only the first may be overridden by a
/// future default without contradicting the operator.
pub(super) fn persisted_redact_secrets() -> Option<bool> {
    let path = redact_pref_path().ok()?;
    let v: Value =
        crate::json_util::read_json_file(&path, crate::xdg::resolve_max_json_file_bytes()).ok()?;
    v.get("redact_secrets")?.as_bool()
}

/// Persist the redaction policy applied to captures started without a flag.
///
/// # Why this writes instead of echoing
///
/// It used to answer `{"ok": true, "redact_secrets": <argv>}` and do nothing
/// else, while `--help` and `docs/schemas/mitm.schema.json` both promised "Show
/// or set". The value echoed back was the argument the caller had just typed, so
/// the command confirmed a setting that was never stored anywhere — the same
/// shape as [`allow_host`] above, minus the write that makes it true. An
/// operator who ran it and then captured had no way to tell that nothing had
/// changed, because the confirmation was indistinguishable from a real one.
///
/// argv still wins: `--mitm-no-redact-secrets` on the capturing command beats
/// whatever is on disk, so this sets a default and never a lock.
pub fn redact_policy(secrets: Option<bool>) -> Result<Value, CliError> {
    let path = redact_pref_path()?;
    // No argument means SHOW. Reporting a policy must never change it, and
    // making the write unconditional would have turned the read half of "Show
    // or set" into a silent `true` on every invocation — replacing a command
    // that lied about writing with one that writes when asked to read.
    let Some(secrets) = secrets else {
        let stored = persisted_redact_secrets();
        return Ok(json!({
            "ok": true,
            "redact_secrets": super::policy::redact_secrets(),
            "persisted": stored.is_some(),
            "persisted_value": stored,
            "source": if super::policy::redact_from_argv() {
                "argv"
            } else if stored.is_some() {
                "persisted"
            } else {
                "default"
            },
            "path": path.display().to_string(),
        }));
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| CliError::new(ErrorKind::Io, format!("mitm state dir: {e}")))?;
    }
    let bytes = serde_json::to_vec_pretty(&json!({ "redact_secrets": secrets }))
        .map_err(|e| CliError::new(ErrorKind::Data, format!("redact policy json: {e}")))?;
    atomic_write(&path, &bytes)?;
    Ok(json!({
        "ok": true,
        "redact_secrets": secrets,
        "persisted": true,
        "path": path.display().to_string(),
        "note": "default for captures started without --mitm-redact-secrets or --mitm-no-redact-secrets",
    }))
}
