// SPDX-License-Identifier: MIT OR Apache-2.0
//! Single shared hudsucker CaptureHandler (DRY for start + capture_url).

use hudsucker::{
    hyper::{Request, Response},
    tokio_tungstenite::tungstenite::Message,
    Body, HttpContext, HttpHandler, RequestOrResponse, WebSocketContext, WebSocketHandler,
};

use http_body_util::{BodyExt, Limited};

use super::body::{
    body_is_bufferable, content_type_of, redact_then_clip, retain_budget, BUFFER_CEILING_BYTES,
};
use super::types::{BTreeMapString, BlockRule, CapturedExchange, CapturedWsFrame};
use super::util::{lock_capture, now_ms, SharedCapture};

/// Records HTTP exchanges and WebSocket frames into [`SharedCapture`].
///
/// # Concurrency
///
/// Handlers take the mutex only for short push/update sections and **never**
/// hold the guard across `.await` (rules: no lock across await).
#[derive(Clone)]
pub(super) struct CaptureHandler {
    pub(super) cap: SharedCapture,
    /// Slot opened by `handle_request`, completed by `handle_response`.
    ///
    /// Safe to keep on the handler because hudsucker documents that "each
    /// request/response pair is passed to the same instance of the handler",
    /// and clones it per pair. That guarantee is what makes this exact rather
    /// than a better guess than `items.last_mut()`.
    pub(super) slot: Option<usize>,
    /// Body retention policy, resolved once at construction.
    pub(super) body_policy: BodyPolicy,
    /// Hosts whose TLS may be decrypted. Empty means every host.
    ///
    /// `--hosts` was accepted, documented as an allowlist, and thrown away:
    /// `capture_url_oneshot` bound it to `_hosts` and never read it. Measured
    /// against a single-host target, the capture came back with nine hosts.
    pub(super) intercept_hosts: std::sync::Arc<[String]>,
    /// Hosts whose exchanges are written to the capture. Empty means every host.
    ///
    /// # Why this is not `intercept_hosts`
    ///
    /// `intercept_hosts` decides what gets DECRYPTED, from the SNI, before a
    /// byte is read. It cannot narrow the record, because plaintext HTTP and
    /// the CDP merge never pass through that decision at all. Measured against
    /// a single-host target with `--hosts` set, the capture still came back
    /// with nine hosts, eight of them the browser's own background chatter.
    ///
    /// This one decides what gets WRITTEN, from the request host, after the
    /// exchange is known. An operator who asks for one host and gets nine has
    /// no way to tell which they asked for once the artifact is on disk.
    pub(super) record_hosts: std::sync::Arc<[String]>,
    /// Rules from `mitm block`, resolved once at proxy construction.
    ///
    /// # Why this is not read per request
    ///
    /// The rules live in a JSON file under XDG state. Re-reading it for every
    /// request would put a filesystem round trip on the proxy hot path to
    /// observe a value that only `mitm block` changes, and that command runs in
    /// a different process. Loading once at construction is the same decision
    /// `body_policy` already makes.
    ///
    /// Empty means "refuse nothing", which is the state of every capture that
    /// never called `mitm block`.
    pub(super) block_rules: std::sync::Arc<[BlockRule]>,
}

/// What the operator asked to keep of each body.
///
/// Built from `--mitm-max-body-bytes` and `--mitm-no-media-bodies`, which were
/// declared on the CLI and read by nobody until now: the help text promised a
/// ceiling and a media filter that no code applied.
#[derive(Clone, Copy, Debug)]
pub(super) struct BodyPolicy {
    /// Max bytes retained per body. `0` keeps none.
    pub(super) max_bytes: usize,
    /// Drop image/video/audio payloads.
    pub(super) skip_media: bool,
}

impl HttpHandler for CaptureHandler {
    async fn handle_request(
        &mut self,
        _ctx: &HttpContext,
        req: Request<Body>,
    ) -> RequestOrResponse {
        let method = req.method().to_string();
        let url = req.uri().to_string();
        let host = request_host(&req);
        // REFUSAL COMES FIRST, BEFORE ANY WORK IS PAID FOR
        //
        // This is the short circuit `mitm block` has always advertised and
        // never performed: the command wrote its rule to `block_rules.json`,
        // answered `{"ok": true}`, and no code read the file back — so the
        // traffic the operator asked to refuse went through untouched, with a
        // success envelope saying otherwise.
        //
        // Answering here means the request never reaches the network: no DNS,
        // no connection, no upstream byte. `204 No Content` is the honest
        // answer for "this was refused locally" — the request was understood
        // and deliberately produced no body, which a browser handles cleanly.
        //
        // The refusal is recorded, because a block that leaves no trace turns
        // a missing exchange into a mystery: an operator reading the capture
        // cannot tell a refused request from one that never happened.
        if self
            .block_rules
            .iter()
            .any(|r| r.matches(host.as_deref(), req.uri().path()))
        {
            lock_capture(&self.cap).push_error("blocked", format!("{method} {url}"));
            self.slot = None;
            return RequestOrResponse::Response(
                Response::builder()
                    .status(hudsucker::hyper::StatusCode::NO_CONTENT)
                    .body(Body::empty())
                    .unwrap_or_else(|_| Response::new(Body::empty())),
            );
        }
        if !record_allowed(host.as_deref(), &self.record_hosts) {
            // Forwarded untouched, simply never recorded: `--capture-hosts` is a
            // filter on the artifact, not a block rule. `mitm block` is the
            // command that refuses traffic.
            self.slot = None;
            return req.into();
        }
        let mut headers = BTreeMapString::new();
        for (k, v) in req.headers() {
            if let Ok(val) = v.to_str() {
                headers.insert(k.to_string(), val.to_string());
            }
        }
        let declared = content_type_of(req.headers());
        let (req, request_body) = read_and_restore_request(req, self.body_policy, &declared).await;
        self.slot = {
            let mut g = lock_capture(&self.cap);
            g.push(CapturedExchange {
                id: 0,
                method,
                url,
                status: None,
                content_type: None,
                request_headers: headers,
                response_headers: BTreeMapString::new(),
                request_body,
                response_body: None,
                host,
                started_ms: now_ms(),
                finished_ms: None,
            })
        };
        req.into()
    }

    /// Decrypt only the hosts the operator named.
    ///
    /// Default stays "everything", which is what an exploratory capture wants.
    /// Once `--hosts` is given, refusing the rest is not a nicety: decrypting a
    /// host nobody asked about writes that host's traffic into an artifact on
    /// disk, and the browser's own background chatter is a large share of it.
    /// The decision is made from the SNI, before any byte is decrypted, which is
    /// the earliest point where the host is known and the cheapest place to say
    /// no: a refused connection is forwarded as an opaque tunnel and never
    /// produces a key for a host nobody asked about.
    async fn should_intercept_tls(
        &mut self,
        _ctx: &HttpContext,
        client_hello: hudsucker::rustls::server::ClientHello<'_>,
    ) -> bool {
        if self.intercept_hosts.is_empty() {
            return true;
        }
        let Some(host) = client_hello.server_name().map(|h| h.to_ascii_lowercase()) else {
            // No SNI means no way to honour the allowlist. Tunnelling it is the
            // conservative reading: the operator named the hosts they wanted.
            return false;
        };
        self.intercept_hosts
            .iter()
            .any(|allowed| host_matches(&host, allowed))
    }

    async fn handle_response(&mut self, _ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
        // Decoded before reading, or the captured body is compressed bytes and
        // renders as `<318 bytes text/html>` — technically true and useless to
        // the agent that asked what the page said. `decode_response` strips the
        // `content-encoding` header along with the encoding, so what is
        // forwarded stays self-consistent.
        //
        // Only attempted when there is an encoding to remove. `decode_response`
        // consumes the response, so a failure leaves nothing to forward; not
        // calling it on the plain case keeps that risk off the common path.
        let res = if res
            .headers()
            .contains_key(hudsucker::hyper::header::CONTENT_ENCODING)
        {
            match hudsucker::decode_response(res) {
                Ok(decoded) => decoded,
                Err(e) => {
                    // The original is gone with the failed call. An empty body
                    // is all that remains, and the capture will show no body
                    // for this exchange rather than a wrong one.
                    tracing::debug!(error = %e, "mitm: response decode failed");
                    lock_capture(&self.cap).push_error("decode", e.to_string());
                    Response::new(Body::empty())
                }
            }
        } else {
            res
        };
        let status = res.status().as_u16();
        let mut headers = BTreeMapString::new();
        for (k, v) in res.headers() {
            if let Ok(val) = v.to_str() {
                headers.insert(k.to_string(), val.to_string());
            }
        }
        let content_type = content_type_of(res.headers());
        let (res, response_body) =
            read_and_restore_response(res, self.body_policy, &content_type).await;
        if let Some(slot) = self.slot.take() {
            let mut g = lock_capture(&self.cap);
            g.complete(slot, status, headers, content_type, response_body);
        }
        res
    }

    /// Record a forwarding failure instead of letting it vanish from the capture.
    ///
    /// The trait default logs through `tracing` and answers 502, which is the
    /// right response but leaves no trace in the artifact the operator actually
    /// reads. Without this, a request that `handle_request` already pushed stays
    /// in the capture with `status: None` forever — indistinguishable from one
    /// still in flight — and the reason it never completed is only in a log the
    /// agent parsing stdout never sees. A capture that omits the failures is the
    /// worst kind of evidence: it looks complete.
    ///
    /// The 502 is preserved byte for byte; what is added is the record.
    async fn handle_error(
        &mut self,
        _ctx: &HttpContext,
        err: hudsucker::hyper_util::client::legacy::Error,
    ) -> Response<Body> {
        tracing::debug!(error = %err, "mitm: forward failed");
        {
            let mut g = lock_capture(&self.cap);
            g.push_error("forward", err.to_string());
            // Close the open exchange so the artifact shows a finished failure
            // rather than an eternally pending request.
            if let Some(slot) = self.slot.take() {
                g.complete(
                    slot,
                    hudsucker::hyper::StatusCode::BAD_GATEWAY.as_u16(),
                    BTreeMapString::new(),
                    None,
                    None,
                );
            }
        }
        // Built by hand rather than with `.expect`, which this crate forbids in
        // production: a failed builder here would panic inside a proxy loop.
        Response::builder()
            .status(hudsucker::hyper::StatusCode::BAD_GATEWAY)
            .body(Body::empty())
            .unwrap_or_else(|_| Response::new(Body::empty()))
    }
}

/// Buffer a request body under the policy and hand back an equivalent request.
///
/// The body is REBUILT, never consumed. A handler that reads the body and
/// forwards the drained request sends an empty payload upstream, which breaks
/// the page it was supposed to observe.
async fn read_and_restore_request(
    req: Request<Body>,
    policy: BodyPolicy,
    content_type: &Option<String>,
) -> (Request<Body>, Option<String>) {
    let budget = retain_budget(policy, content_type);
    if budget == 0 || !body_is_bufferable(req.headers(), content_type) {
        return (req, None);
    }
    let (parts, body) = req.into_parts();
    // The ceiling is enforced HERE, on the read itself. Checking only a declared
    // `Content-Length` leaves the chunked case — which `body_is_bufferable` calls
    // the norm — reading without any bound, so the peer decided how much memory
    // this process allocated. `clip` does not help: it runs after the whole body
    // is already resident and trims what is RETAINED, not what is HELD.
    let Ok(collected) = Limited::new(body, BUFFER_CEILING_BYTES).collect().await else {
        // Either the read failed or the body ran past the ceiling. Both leave a
        // partly consumed stream that cannot be forwarded intact, so an empty
        // body is the only honest thing left to send.
        return (Request::from_parts(parts, Body::empty()), None);
    };
    let bytes = collected.to_bytes();
    // Redact BEFORE clipping: a JSON payload past the retain budget used to
    // reach the redactor as a fragment `serde_json` refused, so a secret inside
    // it survived into the capture. See `body::redact_then_clip`.
    let rendered = redact_then_clip(&bytes, content_type, budget);
    (Request::from_parts(parts, Body::from(bytes)), rendered)
}

/// Same contract as [`read_and_restore_request`], for the response direction.
async fn read_and_restore_response(
    res: Response<Body>,
    policy: BodyPolicy,
    content_type: &Option<String>,
) -> (Response<Body>, Option<String>) {
    let budget = retain_budget(policy, content_type);
    if budget == 0 || !body_is_bufferable(res.headers(), content_type) {
        return (res, None);
    }
    let (parts, body) = res.into_parts();
    // Same ceiling, same reason as the request direction; see that function.
    let Ok(collected) = Limited::new(body, BUFFER_CEILING_BYTES).collect().await else {
        return (Response::from_parts(parts, Body::empty()), None);
    };
    let bytes = collected.to_bytes();
    // Same ordering fix as the request direction; see that function.
    let rendered = redact_then_clip(&bytes, content_type, budget);
    (Response::from_parts(parts, Body::from(bytes)), rendered)
}

/// Match a host against one allowlist entry, covering its subdomains.
///
/// `example.com` admits `api.example.com`, because a site's own API is what a
/// caller means by "capture this site". It does not admit `notexample.com`:
/// the boundary is a dot, not a substring, or `evil-example.com` would pass.
pub(super) fn host_matches(host: &str, allowed: &str) -> bool {
    let allowed = allowed.trim().to_ascii_lowercase();
    if allowed.is_empty() {
        return false;
    }
    host == allowed || host.ends_with(&format!(".{allowed}"))
}

/// Whether this exchange belongs in the capture under a record allowlist.
///
/// An empty allowlist admits everything, which keeps the exploratory capture
/// unchanged. A named allowlist with an unknown host is refused: the host is
/// what the operator selected on, so a host that cannot be determined is not
/// evidence that it was wanted.
pub(super) fn record_allowed(host: Option<&str>, allow: &[String]) -> bool {
    if allow.is_empty() {
        return true;
    }
    let Some(host) = host else {
        return false;
    };
    let host = host.to_ascii_lowercase();
    allow.iter().any(|a| host_matches(&host, a))
}

/// Host of a proxied request, falling back to the `Host` header.
///
/// A TLS-intercepted request arrives in origin form (`/path`), where
/// `Uri::host` is `None` and the authority lives in the header. Reading only
/// the URI would make every intercepted exchange look host-less, and a
/// host-less exchange is refused by [`record_allowed`].
fn request_host(req: &Request<Body>) -> Option<String> {
    if let Some(h) = req.uri().host() {
        return Some(h.to_ascii_lowercase());
    }
    req.headers()
        .get(hudsucker::hyper::header::HOST)
        .and_then(|v| v.to_str().ok())
        // Strip the port: the allowlist names hosts, not endpoints.
        .map(|s| s.split(':').next().unwrap_or(s).trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
}

/// Split a comma-separated `--hosts` value into allowlist entries.
pub(super) fn parse_hosts(raw: Option<&str>) -> Vec<String> {
    raw.map(crate::agent_ops::path::split_csv_lower)
        .unwrap_or_default()
}

impl WebSocketHandler for CaptureHandler {
    async fn handle_message(&mut self, _ctx: &WebSocketContext, msg: Message) -> Option<Message> {
        let (kind, preview) = match &msg {
            Message::Text(t) => {
                let s = t.to_string();
                let prev: String = s
                    .chars()
                    .take(crate::xdg::policy::policy_usize(
                        crate::xdg::policy::key::MITM_WS_PREVIEW_CHARS,
                    ))
                    .collect();
                ("text".into(), prev)
            }
            Message::Binary(b) => ("binary".into(), format!("<{} bytes>", b.len())),
            Message::Ping(_) => ("ping".into(), String::new()),
            Message::Pong(_) => ("pong".into(), String::new()),
            Message::Close(_) => ("close".into(), String::new()),
            _ => ("other".into(), String::new()),
        };
        let ts_ms = now_ms();
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

/// Split a comma-separated host list, for callers outside this module.
pub(super) fn parse_hosts_public(raw: &str) -> Vec<String> {
    parse_hosts(Some(raw))
}
