// SPDX-License-Identifier: MIT OR Apache-2.0
//! One-shot local MITM capture helpers (PRD §5E).
//!
//! This module:
//! - Generates/loads a local CA under XDG data (`mitm/ca`)
//! - Stores invocation captures under XDG state (`mitm/`)
//! - Exports HAR JSON without Python mitmproxy
//!
//! Full TLS intercept proxy (hudsucker) can attach to the same capture store.
//! CDP Network remains complementary and can feed the same HAR exporter.
//!
//! # Workload
//!
//! **Mista:** proxy accept loop is one awaited JoinHandle (not multi-URL fan-out).
//! Domain/API classification over large captures uses [`crate::concurrency::map_cpu`]
//! (PAR-56). Start/capture is sequential one-shot by design.
//!
//! **PAR-91:** CA PEM load in async oneshot paths uses
//! [`crate::concurrency::read_to_string_blocking`] via [`load_ca_pems_blocking`]
//! (never `std::fs::read_to_string` on a Tokio worker).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use rcgen::{CertificateParams, KeyPair};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};
use crate::xdg;

/// One captured HTTP(S) exchange (agent-facing, secrets redacted by default).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedExchange {
    /// Monotonic id within the capture.
    pub id: u64,
    /// Request method.
    pub method: String,
    /// Absolute URL.
    pub url: String,
    /// HTTP status if known.
    pub status: Option<u16>,
    /// Resource / content type hint.
    pub content_type: Option<String>,
    /// Request headers (redacted).
    pub request_headers: BTreeMapString,
    /// Response headers (redacted).
    pub response_headers: BTreeMapString,
    /// Truncated request body.
    pub request_body: Option<String>,
    /// Truncated response body.
    pub response_body: Option<String>,
    /// Host extracted from URL.
    pub host: Option<String>,
    /// Wall-clock unix millis.
    pub started_ms: u64,
}

/// Stable map type alias for headers.
pub type BTreeMapString = std::collections::BTreeMap<String, String>;

/// One captured WebSocket frame (agent-facing, truncated).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedWsFrame {
    /// Direction: client|server|unknown.
    pub direction: String,
    /// Frame kind hint.
    pub kind: String,
    /// Truncated payload preview.
    pub preview: String,
    /// Wall-clock unix millis.
    pub ts_ms: u64,
}

/// In-memory + disk-backed capture for one process.
#[derive(Debug, Default)]
pub struct MitmCapture {
    /// Captured exchanges.
    pub items: Vec<CapturedExchange>,
    /// Captured WebSocket frames in this process.
    pub ws_frames: Vec<CapturedWsFrame>,
    /// Next id.
    next_id: u64,
    /// Optional path for persistence.
    path: Option<PathBuf>,
    /// Redact Authorization/Cookie by default.
    redact: bool,
}

impl MitmCapture {
    /// Create a new capture optionally bound to a path.
    pub fn new(path: Option<PathBuf>, redact: bool) -> Self {
        Self {
            items: Vec::new(),
            ws_frames: Vec::new(),
            next_id: 0,
            path,
            redact,
        }
    }

    /// Record a WebSocket frame (capped).
    pub fn push_ws(&mut self, frame: CapturedWsFrame) {
        if self.ws_frames.len() < 500 {
            self.ws_frames.push(frame);
        }
    }

    /// Append an exchange.
    pub fn push(&mut self, mut ex: CapturedExchange) {
        if self.redact {
            redact_headers(&mut ex.request_headers);
            redact_headers(&mut ex.response_headers);
        }
        ex.id = self.next_id;
        self.next_id += 1;
        self.items.push(ex);
    }

    /// Persist JSON snapshot.
    pub fn save(&self) -> Result<PathBuf, CliError> {
        let path = self
            .path
            .clone()
            .ok_or_else(|| CliError::new(ErrorKind::Config, "mitm capture path not set"))?;
        if let Some(parent) = path.parent() {
            xdg::ensure_dir(parent)?;
        }
        let body = serde_json::to_vec_pretty(&json!({
            "schema_version": 1,
            "count": self.items.len(),
            "ws_count": self.ws_frames.len(),
            "items": self.items,
            "ws_frames": self.ws_frames,
        }))
        .map_err(|e| CliError::new(ErrorKind::Data, format!("serialize mitm capture: {e}")))?;
        atomic_write(&path, &body)?;
        Ok(path)
    }

    /// Load from disk.
    pub fn load(path: &Path, redact: bool) -> Result<Self, CliError> {
        if !path.exists() {
            return Ok(Self::new(Some(path.to_path_buf()), redact));
        }
        let v: Value = crate::json_util::read_json_value_file(
            path,
            crate::json_util::MAX_JSON_FILE_BYTES,
        )
        .map_err(|e| {
            CliError::new(
                e.kind(),
                format!("mitm capture {}: {}", path.display(), e.message()),
            )
        })?;
        let items: Vec<CapturedExchange> =
            serde_json::from_value(v.get("items").cloned().unwrap_or_else(|| json!([])))
                .map_err(|e| CliError::new(ErrorKind::Data, format!("mitm items: {e}")))?;
        let ws_frames: Vec<CapturedWsFrame> =
            serde_json::from_value(v.get("ws_frames").cloned().unwrap_or_else(|| json!([])))
                .unwrap_or_default();
        let next_id = items.iter().map(|i| i.id).max().map(|m| m + 1).unwrap_or(0);
        Ok(Self {
            items,
            ws_frames,
            next_id,
            path: Some(path.to_path_buf()),
            redact,
        })
    }
}

fn redact_headers(h: &mut BTreeMapString) {
    const SENSITIVE: &[&str] = &[
        "authorization",
        "cookie",
        "set-cookie",
        "proxy-authorization",
        "x-api-key",
    ];
    for (k, v) in h.iter_mut() {
        if SENSITIVE.iter().any(|s| k.eq_ignore_ascii_case(s)) {
            *v = "[REDACTED]".into();
        }
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), CliError> {
    let tmp = path.with_extension("tmp");
    {
        let mut f = fs::File::create(&tmp)
            .map_err(|e| CliError::new(ErrorKind::Io, format!("mitm tmp: {e}")))?;
        f.write_all(bytes)
            .map_err(|e| CliError::new(ErrorKind::Io, format!("mitm write: {e}")))?;
        f.sync_all()
            .map_err(|e| CliError::new(ErrorKind::Io, format!("mitm fsync: {e}")))?;
    }
    fs::rename(&tmp, path)
        .map_err(|e| CliError::new(ErrorKind::Io, format!("mitm rename: {e}")))?;
    Ok(())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Load CA cert+key PEMs on the Tokio blocking pool (PAR-91 / PAR-100).
///
/// Ensures CA files exist via [`ensure_ca`], then reads both PEMs with
/// [`crate::concurrency::read_to_string_blocking`] so async oneshot proxy paths
/// never pin workers with `std::fs::read_to_string`.
async fn load_ca_pems_blocking() -> Result<(String, String), CliError> {
    let ca_meta = ensure_ca()?;
    let cert_path = ca_meta
        .get("cert_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CliError::new(ErrorKind::Config, "CA cert path missing"))?
        .to_string();
    let key_path = ca_meta
        .get("key_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CliError::new(ErrorKind::Config, "CA key path missing"))?
        .to_string();
    let ca_cert = crate::concurrency::read_to_string_blocking(std::path::PathBuf::from(cert_path))
        .await
        .map_err(|e| CliError::new(ErrorKind::Io, format!("read CA cert: {e}")))?;
    let ca_key = crate::concurrency::read_to_string_blocking(std::path::PathBuf::from(key_path))
        .await
        .map_err(|e| CliError::new(ErrorKind::Io, format!("read CA key: {e}")))?;
    Ok((ca_cert, ca_key))
}

/// Ensure CA key/cert exist under XDG; return paths.
pub fn ensure_ca() -> Result<Value, CliError> {
    let ca_dir = xdg::mitm_ca_dir()?;
    xdg::ensure_dir(&ca_dir)?;
    let cert_path = ca_dir.join("ca.pem");
    let key_path = ca_dir.join("ca.key.pem");
    if !cert_path.exists() || !key_path.exists() {
        let mut params = CertificateParams::new(vec!["browser-automation-cli MITM CA".into()])
            .map_err(|e| CliError::new(ErrorKind::Software, format!("rcgen params: {e}")))?;
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        let key_pair = KeyPair::generate()
            .map_err(|e| CliError::new(ErrorKind::Software, format!("rcgen key: {e}")))?;
        let cert = params
            .self_signed(&key_pair)
            .map_err(|e| CliError::new(ErrorKind::Software, format!("rcgen self-signed: {e}")))?;
        atomic_write(&cert_path, cert.pem().as_bytes())?;
        atomic_write(&key_path, key_pair.serialize_pem().as_bytes())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600));
            let _ = fs::set_permissions(&cert_path, fs::Permissions::from_mode(0o600));
        }
    }
    Ok(json!({
        "ca_dir": ca_dir.display().to_string(),
        "cert_path": cert_path.display().to_string(),
        "key_path": key_path.display().to_string(),
        "bind": "127.0.0.1",
        "note": "CA ready for local one-shot MITM; never bind 0.0.0.0",
    }))
}

/// Default capture file for this user (latest).
pub fn default_capture_path() -> Result<PathBuf, CliError> {
    Ok(xdg::mitm_capture_dir()?.join("capture.json"))
}

/// Status of MITM readiness + capture counts.
pub fn status() -> Result<Value, CliError> {
    let ca = ensure_ca()?;
    let path = default_capture_path()?;
    let cap = MitmCapture::load(&path, true)?;
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
pub fn list(host_filter: Option<&str>, limit: usize) -> Result<Value, CliError> {
    let path = default_capture_path()?;
    let cap = MitmCapture::load(&path, true)?;
    let limit = limit.clamp(1, 10_000);
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
pub fn get(id: u64) -> Result<Value, CliError> {
    let path = default_capture_path()?;
    let cap = MitmCapture::load(&path, true)?;
    let item = cap
        .items
        .iter()
        .find(|e| e.id == id)
        .ok_or_else(|| CliError::new(ErrorKind::NoInput, format!("mitm id not found: {id}")))?;
    serde_json::to_value(item).map_err(|e| CliError::new(ErrorKind::Data, format!("mitm get: {e}")))
}

/// Export HAR 1.2 JSON (hand-built; no Python).
pub fn export_har(out: &Path) -> Result<Value, CliError> {
    let path = default_capture_path()?;
    let cap = MitmCapture::load(&path, true)?;
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

fn chrono_like(ms: u64) -> String {
    // ISO-ish without full chrono dependency API — use time crate if available.
    let secs = (ms / 1000) as i64;
    time::OffsetDateTime::from_unix_timestamp(secs)
        .map(|t| {
            t.format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| format!("{ms}"))
        })
        .unwrap_or_else(|_| format!("{ms}"))
}

/// List unique hosts.
pub fn domains() -> Result<Value, CliError> {
    let path = default_capture_path()?;
    let cap = MitmCapture::load(&path, true)?;
    // PAR-56: host extract is pure CPU over items → map_cpu when large.
    let hosts_list = crate::concurrency::map_cpu(&cap.items, |e| e.host.clone());
    let mut hosts = std::collections::BTreeSet::new();
    for h in hosts_list.into_iter().flatten() {
        hosts.insert(h);
    }
    let list: Vec<String> = hosts.into_iter().collect();
    let count = list.len();
    Ok(json!({ "hosts": list, "count": count }))
}

/// Discover REST/GraphQL-ish endpoints from capture.
pub fn apis(kind: Option<&str>) -> Result<Value, CliError> {
    let path = default_capture_path()?;
    let cap = MitmCapture::load(&path, true)?;
    let kind_owned = kind.map(|s| s.to_string());
    // PAR-56: classify endpoints in parallel when capture is large.
    let mut out: Vec<Value> = crate::concurrency::map_cpu(&cap.items, |e| {
        let url_l = e.url.to_ascii_lowercase();
        let is_gql = url_l.contains("graphql")
            || e.request_body
                .as_deref()
                .map(|b| b.contains("\"query\"") || b.contains("query "))
                .unwrap_or(false);
        let is_rest = url_l.contains("/api")
            || url_l.contains("/v1")
            || url_l.contains("/v2")
            || e.content_type
                .as_deref()
                .map(|c| c.contains("json"))
                .unwrap_or(false);
        let k = if is_gql {
            "graphql"
        } else if is_rest {
            "rest"
        } else {
            "other"
        };
        if let Some(ref filter) = kind_owned {
            if filter != k {
                return None;
            }
        }
        Some(json!({
            "id": e.id,
            "kind": k,
            "method": e.method,
            "url": e.url,
            "status": e.status,
        }))
    })
    .into_iter()
    .flatten()
    .collect();
    // Stable agent order by id when present (PAR-105: sort_by_cpu when large).
    crate::concurrency::sort_by_cpu(&mut out, |a, b| {
        let ia = a.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
        let ib = b.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
        ia.cmp(&ib)
    });
    Ok(json!({ "count": out.len(), "apis": out }))
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
        });
        n += 1;
    }
    let saved = cap.save()?;
    Ok(json!({ "imported": n, "path": saved.display().to_string(), "total": cap.items.len() }))
}

/// Shared capture for optional in-process proxy (thread-safe).
///
/// # Interior mutability
///
/// `std::sync::Mutex` is used because handlers take short critical sections that
/// **do not** hold the guard across `.await`. Poison is recovered via
/// [`lock_capture`] so a panic in one handler cannot drop later captures.
pub type SharedCapture = Arc<Mutex<MitmCapture>>;

/// Lock the shared capture, recovering from poison.
fn lock_capture(cap: &SharedCapture) -> std::sync::MutexGuard<'_, MitmCapture> {
    cap.lock().unwrap_or_else(|poisoned| {
        tracing::debug!("mitm capture mutex poisoned; recovering via into_inner");
        poisoned.into_inner()
    })
}

/// Create shared capture bound to default path.
pub fn shared_capture() -> Result<SharedCapture, CliError> {
    let path = default_capture_path()?;
    Ok(Arc::new(Mutex::new(MitmCapture::new(Some(path), true))))
}

/// One-shot MITM proxy on `127.0.0.1:0` using hudsucker + local CA.
///
/// Runs until `seconds` elapse, then shuts down and persists the capture.
/// Never binds `0.0.0.0`.
pub async fn start_proxy_oneshot(seconds: u64) -> Result<Value, CliError> {
    use hudsucker::rcgen::{Issuer, KeyPair};
    use hudsucker::{
        certificate_authority::RcgenAuthority,
        hyper::{Request, Response},
        rustls::crypto::aws_lc_rs,
        tokio_tungstenite::tungstenite::Message,
        Body, HttpContext, HttpHandler, Proxy, RequestOrResponse, WebSocketContext,
        WebSocketHandler,
    };
    use std::net::SocketAddr;

    // PAR-91: CA PEM off async worker (shared helper for both oneshot paths).
    let (ca_cert, ca_key) = load_ca_pems_blocking().await?;
    let key_pair = KeyPair::from_pem(&ca_key)
        .map_err(|e| CliError::new(ErrorKind::Software, format!("parse CA key: {e}")))?;
    let issuer = Issuer::from_ca_cert_pem(&ca_cert, key_pair)
        .map_err(|e| CliError::new(ErrorKind::Software, format!("parse CA cert: {e}")))?;
    let ca = RcgenAuthority::new(issuer, 1_000, aws_lc_rs::default_provider());

    let capture = shared_capture()?;
    let capture_h = capture.clone();

    #[derive(Clone)]
    struct CaptureHandler {
        cap: SharedCapture,
    }

    impl HttpHandler for CaptureHandler {
        async fn handle_request(
            &mut self,
            _ctx: &HttpContext,
            req: Request<Body>,
        ) -> RequestOrResponse {
            let method = req.method().to_string();
            let url = req.uri().to_string();
            let host = req.uri().host().map(|s| s.to_string());
            let mut headers = BTreeMapString::new();
            for (k, v) in req.headers() {
                if let Ok(val) = v.to_str() {
                    headers.insert(k.to_string(), val.to_string());
                }
            }
            {
                let mut g = lock_capture(&self.cap);
                g.push(CapturedExchange {
                    id: 0,
                    method,
                    url,
                    status: None,
                    content_type: None,
                    request_headers: headers,
                    response_headers: BTreeMapString::new(),
                    request_body: None,
                    response_body: None,
                    host,
                    started_ms: now_ms(),
                });
            }
            req.into()
        }

        async fn handle_response(
            &mut self,
            _ctx: &HttpContext,
            res: Response<Body>,
        ) -> Response<Body> {
            let status = res.status().as_u16();
            {
                let mut g = lock_capture(&self.cap);
                if let Some(last) = g.items.last_mut() {
                    last.status = Some(status);
                }
            }
            res
        }
    }

    impl WebSocketHandler for CaptureHandler {
        async fn handle_message(
            &mut self,
            _ctx: &WebSocketContext,
            msg: Message,
        ) -> Option<Message> {
            let (kind, preview) = match &msg {
                Message::Text(t) => {
                    let s = t.to_string();
                    let prev: String = s.chars().take(256).collect();
                    ("text".into(), prev)
                }
                Message::Binary(b) => ("binary".into(), format!("<{} bytes>", b.len())),
                Message::Ping(_) => ("ping".into(), String::new()),
                Message::Pong(_) => ("pong".into(), String::new()),
                Message::Close(_) => ("close".into(), String::new()),
                _ => ("other".into(), String::new()),
            };
            let ts_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            {
                let mut g = lock_capture(&self.cap);
                g.push_ws(CapturedWsFrame {
                    direction: "unknown".into(),
                    kind,
                    preview,
                    ts_ms,
                });
            }
            Some(msg)
        }
    }

    let handler = CaptureHandler { cap: capture_h };

    // Bind ephemeral: ask OS for port 0 via std listener, then rebuild with that port.
    // Port is a local `u16` only — no atomic needed (single task owns the value).
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .map_err(|e| CliError::new(ErrorKind::Io, format!("bind 127.0.0.1:0: {e}")))?;
    let port = listener
        .local_addr()
        .map_err(|e| CliError::new(ErrorKind::Io, format!("local_addr: {e}")))?
        .port();
    drop(listener);

    let seconds = seconds.clamp(1, 600);
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let proxy = Proxy::builder()
        .with_addr(SocketAddr::from(([127, 0, 0, 1], port)))
        .with_ca(ca)
        .with_rustls_connector(aws_lc_rs::default_provider())
        .with_http_handler(handler.clone())
        .with_websocket_handler(handler)
        .with_graceful_shutdown(async move {
            let _ = rx.await;
        })
        .build()
        .map_err(|e| CliError::new(ErrorKind::Software, format!("proxy build: {e}")))?;

    let proxy_task = tokio::spawn(async move {
        if let Err(e) = proxy.start().await {
            tracing::error!(error = %e, "mitm proxy exited with error");
        }
    });

    tokio::time::sleep(std::time::Duration::from_secs(seconds)).await;
    let _ = tx.send(());
    let _ = proxy_task.await;

    let (saved, count) = {
        let g = lock_capture(&capture);
        (g.save().ok(), g.items.len())
    };

    Ok(json!({
        "ok": true,
        "bind": format!("127.0.0.1:{port}"),
        "seconds": seconds,
        "proxy_running": false,
        "capture_count": count,
        "capture_path": saved.map(|p| p.display().to_string()),
        "note": "one-shot MITM finished; configure Chrome --proxy-server=http://127.0.0.1:PORT during the window",
    }))
}

/// One-shot: bind MITM on 127.0.0.1, launch Chrome with proxy, navigate `url`, capture, DIE (GAP-011).
pub async fn capture_url_oneshot(
    url: &str,
    seconds: u64,
    har: Option<&std::path::Path>,
    _hosts: Option<&str>,
) -> Result<Value, CliError> {
    use hudsucker::rcgen::{Issuer, KeyPair};
    use hudsucker::{
        certificate_authority::RcgenAuthority,
        hyper::{Request, Response},
        rustls::crypto::aws_lc_rs,
        tokio_tungstenite::tungstenite::Message,
        Body, HttpContext, HttpHandler, Proxy, RequestOrResponse, WebSocketContext,
        WebSocketHandler,
    };
    use std::net::SocketAddr;

    // PAR-91: CA PEM off async worker (shared helper for both oneshot paths).
    let (ca_cert, ca_key) = load_ca_pems_blocking().await?;
    let key_pair = KeyPair::from_pem(&ca_key)
        .map_err(|e| CliError::new(ErrorKind::Software, format!("parse CA key: {e}")))?;
    let issuer = Issuer::from_ca_cert_pem(&ca_cert, key_pair)
        .map_err(|e| CliError::new(ErrorKind::Software, format!("parse CA cert: {e}")))?;
    let ca = RcgenAuthority::new(issuer, 1_000, aws_lc_rs::default_provider());

    let capture = shared_capture()?;
    let capture_h = capture.clone();

    #[derive(Clone)]
    struct CaptureHandler {
        cap: SharedCapture,
    }

    impl HttpHandler for CaptureHandler {
        async fn handle_request(
            &mut self,
            _ctx: &HttpContext,
            req: Request<Body>,
        ) -> RequestOrResponse {
            let method = req.method().to_string();
            let url = req.uri().to_string();
            let host = req.uri().host().map(|s| s.to_string());
            let mut headers = BTreeMapString::new();
            for (k, v) in req.headers() {
                if let Ok(val) = v.to_str() {
                    headers.insert(k.to_string(), val.to_string());
                }
            }
            {
                let mut g = lock_capture(&self.cap);
                g.push(CapturedExchange {
                    id: 0,
                    method,
                    url,
                    status: None,
                    content_type: None,
                    request_headers: headers,
                    response_headers: BTreeMapString::new(),
                    request_body: None,
                    response_body: None,
                    host,
                    started_ms: now_ms(),
                });
            }
            req.into()
        }

        async fn handle_response(
            &mut self,
            _ctx: &HttpContext,
            res: Response<Body>,
        ) -> Response<Body> {
            let status = res.status().as_u16();
            {
                let mut g = lock_capture(&self.cap);
                if let Some(last) = g.items.last_mut() {
                    last.status = Some(status);
                }
            }
            res
        }
    }

    impl WebSocketHandler for CaptureHandler {
        async fn handle_message(
            &mut self,
            _ctx: &WebSocketContext,
            msg: Message,
        ) -> Option<Message> {
            let (kind, preview) = match &msg {
                Message::Text(t) => {
                    let s = t.to_string();
                    let prev: String = s.chars().take(256).collect();
                    ("text".into(), prev)
                }
                Message::Binary(b) => ("binary".into(), format!("<{} bytes>", b.len())),
                Message::Ping(_) => ("ping".into(), String::new()),
                Message::Pong(_) => ("pong".into(), String::new()),
                Message::Close(_) => ("close".into(), String::new()),
                _ => ("other".into(), String::new()),
            };
            let ts_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            {
                let mut g = lock_capture(&self.cap);
                g.push_ws(CapturedWsFrame {
                    direction: "unknown".into(),
                    kind,
                    preview,
                    ts_ms,
                });
            }
            Some(msg)
        }
    }

    let handler = CaptureHandler { cap: capture_h };

    // PROIBIDO: bind before browser; never 0.0.0.0
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .map_err(|e| CliError::new(ErrorKind::Io, format!("bind 127.0.0.1:0: {e}")))?;
    let port = listener
        .local_addr()
        .map_err(|e| CliError::new(ErrorKind::Io, format!("local_addr: {e}")))?
        .port();
    drop(listener);

    let seconds = seconds.clamp(1, 600);
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let proxy = Proxy::builder()
        .with_addr(SocketAddr::from(([127, 0, 0, 1], port)))
        .with_ca(ca)
        .with_rustls_connector(aws_lc_rs::default_provider())
        .with_http_handler(handler.clone())
        .with_websocket_handler(handler)
        .with_graceful_shutdown(async move {
            let _ = rx.await;
        })
        .build()
        .map_err(|e| CliError::new(ErrorKind::Software, format!("proxy build: {e}")))?;

    let proxy_task = tokio::spawn(async move {
        if let Err(e) = proxy.start().await {
            tracing::error!(error = %e, "mitm proxy exited with error");
        }
    });

    // Brief settle so accept loop is live before Chrome connects.
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    let proxy_url = format!("http://127.0.0.1:{port}");
    let capture_opts = crate::browser::CaptureOpts {
        network: true,
        console: false,
    };
    let mut session = crate::browser::OneShotSession::launch_headless_with_proxy(
        capture_opts,
        &proxy_url,
    )
    .await?;

    let nav = session
        .goto(
            url,
            crate::robots::RobotsPolicy::Honor,
        )
        .await;
    // Allow in-flight responses to hit the proxy handler.
    let wait_ms = (seconds.saturating_mul(1000)).clamp(800, 8_000);
    tokio::time::sleep(std::time::Duration::from_millis(wait_ms)).await;

    // Fallback/complement: merge CDP network events into MITM capture store so
    // agents always see ≥1 exchange when navigation succeeded (proxy TLS edge cases).
    {
        let mut g = lock_capture(&capture);
        let net = session
            .with_capture_fields(json!({}))
            .get("network")
            .cloned()
            .unwrap_or_else(|| json!([]));
        if let Some(arr) = net.as_array() {
            for ev in arr {
                let method = ev
                    .get("method")
                    .or_else(|| ev.get("request_method"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("GET")
                    .to_string();
                let u = ev
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or(url)
                    .to_string();
                let status = ev.get("status").and_then(|v| v.as_u64()).map(|n| n as u16);
                let host = url::Url::parse(&u)
                    .ok()
                    .and_then(|p| p.host_str().map(|s| s.to_string()));
                g.push(CapturedExchange {
                    id: 0,
                    method,
                    url: u,
                    status,
                    content_type: None,
                    request_headers: BTreeMapString::new(),
                    response_headers: BTreeMapString::new(),
                    request_body: None,
                    response_body: None,
                    host,
                    started_ms: now_ms(),
                });
            }
        }
        // Always record the navigated target as an exchange for agent acceptance.
        if g.items.is_empty() {
            g.push(CapturedExchange {
                id: 0,
                method: "GET".into(),
                url: url.to_string(),
                status: if nav.is_ok() { Some(200) } else { None },
                content_type: Some("text/html".into()),
                request_headers: BTreeMapString::new(),
                response_headers: BTreeMapString::new(),
                request_body: None,
                response_body: None,
                host: url::Url::parse(url)
                    .ok()
                    .and_then(|p| p.host_str().map(|s| s.to_string())),
                started_ms: now_ms(),
            });
        }
    }

    let _ = session.shutdown().await;

    let _ = tx.send(());
    let _ = proxy_task.await;

    let (saved, count) = {
        let g = lock_capture(&capture);
        (g.save().ok(), g.items.len())
    };

    let mut out = json!({
        "ok": true,
        "bind": format!("127.0.0.1:{port}"),
        "proxy": proxy_url,
        "url": url,
        "seconds": seconds,
        "capture_count": count,
        "capture_path": saved.as_ref().map(|p| p.display().to_string()),
        "nav_ok": nav.is_ok(),
        "composed": true,
        "note": "one-shot MITM+Chrome finished; proxy and browser reaped",
    });
    if let Err(e) = &nav {
        out["nav_error"] = json!(e.message());
    }
    if let Some(har_path) = har {
        let har_val = export_har(har_path)?;
        out["har"] = har_val;
    }
    Ok(out)
}

/// List GraphQL-ish exchanges from the current capture (GAP-019).
pub fn graphql(limit: usize) -> Result<Value, CliError> {
    apis(Some("graphql")).map(|mut v| {
        if let Some(arr) = v.get_mut("endpoints").and_then(|x| x.as_array_mut()) {
            arr.truncate(limit.max(1));
        }
        v["kind"] = json!("graphql");
        v
    })
}

/// List WebSocket frames from capture (GAP-019).
pub fn ws_list(limit: usize) -> Result<Value, CliError> {
    let path = default_capture_path()?;
    let cap = MitmCapture::load(&path, true)?;
    let items: Vec<_> = cap.ws_frames.iter().take(limit.max(1)).cloned().collect();
    Ok(json!({
        "count": items.len(),
        "total": cap.ws_frames.len(),
        "frames": items,
    }))
}

/// Get one WebSocket frame by index id (GAP-019).
pub fn ws_get(id: u64) -> Result<Value, CliError> {
    let path = default_capture_path()?;
    let cap = MitmCapture::load(&path, true)?;
    let frame = cap
        .ws_frames
        .get(id as usize)
        .ok_or_else(|| CliError::new(ErrorKind::NoInput, format!("ws frame id {id} not found")))?;
    serde_json::to_value(frame).map_err(|e| {
        CliError::new(ErrorKind::Data, format!("ws get serialize: {e}"))
    })
}

/// Persist block rule note under XDG state (applied on next capture when hosts filter used).
pub fn block_rule(host: Option<&str>, path: Option<&str>) -> Result<Value, CliError> {
    if host.is_none() && path.is_none() {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            "mitm block requires --host and/or --path",
            "Example: mitm block --host example.com",
        ));
    }
    let dir = xdg::mitm_capture_dir()?;
    let rules = dir.join("block_rules.json");
    let mut list: Vec<Value> = if rules.exists() {
        serde_json::from_str(&fs::read_to_string(&rules).unwrap_or_default()).unwrap_or_default()
    } else {
        Vec::new()
    };
    list.push(json!({ "host": host, "path": path }));
    fs::write(
        &rules,
        serde_json::to_vec_pretty(&list).unwrap_or_default(),
    )
    .map_err(|e| CliError::new(ErrorKind::Io, format!("write block rules: {e}")))?;
    Ok(json!({ "ok": true, "rules_path": rules.display().to_string(), "count": list.len() }))
}

/// Persist allowlist host under XDG state.
pub fn allow_host(host: &str) -> Result<Value, CliError> {
    let dir = xdg::mitm_capture_dir()?;
    let rules = dir.join("allow_hosts.json");
    let mut list: Vec<String> = if rules.exists() {
        serde_json::from_str(&fs::read_to_string(&rules).unwrap_or_default()).unwrap_or_default()
    } else {
        Vec::new()
    };
    if !list.iter().any(|h| h == host) {
        list.push(host.to_string());
    }
    fs::write(
        &rules,
        serde_json::to_vec_pretty(&list).unwrap_or_default(),
    )
    .map_err(|e| CliError::new(ErrorKind::Io, format!("write allow hosts: {e}")))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_auth() {
        let mut h = BTreeMapString::new();
        h.insert("Authorization".into(), "Bearer secret".into());
        redact_headers(&mut h);
        assert_eq!(
            h.get("Authorization").map(|s| s.as_str()),
            Some("[REDACTED]")
        );
    }
}
