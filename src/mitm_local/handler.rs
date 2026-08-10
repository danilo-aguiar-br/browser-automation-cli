// SPDX-License-Identifier: MIT OR Apache-2.0
//! Single shared hudsucker CaptureHandler (DRY for start + capture_url).

use hudsucker::{
    hyper::{Request, Response},
    tokio_tungstenite::tungstenite::Message,
    Body, HttpContext, HttpHandler, RequestOrResponse, WebSocketContext, WebSocketHandler,
};

use http_body_util::BodyExt;

use super::types::{BTreeMapString, CapturedExchange, CapturedWsFrame};
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
    pub(super) intercept_hosts: std::sync::Arc<Vec<String>>,
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
        let host = req.uri().host().map(|s| s.to_string());
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
    let Ok(collected) = body.collect().await else {
        // A body that fails to read cannot be forwarded either; an empty one is
        // the only thing left to send, and the caller sees no captured body.
        return (Request::from_parts(parts, Body::empty()), None);
    };
    let bytes = collected.to_bytes();
    let (kept, truncated) = clip(&bytes, budget);
    let rendered = render_body(kept, content_type, truncated);
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
    let Ok(collected) = body.collect().await else {
        return (Response::from_parts(parts, Body::empty()), None);
    };
    let bytes = collected.to_bytes();
    let (kept, truncated) = clip(&bytes, budget);
    let rendered = render_body(kept, content_type, truncated);
    (Response::from_parts(parts, Body::from(bytes)), rendered)
}

/// Hard ceiling on buffering, independent of what the operator wants retained.
///
/// Buffering means holding the whole body before forwarding a byte, so this is
/// the point past which observing a response would cost more than the response
/// is worth. Retention (`--mitm-max-body-bytes`) decides how much is WRITTEN;
/// this decides how much is ever held.
const BUFFER_CEILING_BYTES: usize = 8 * 1024 * 1024;

/// Whether this body can be buffered without risking the page.
///
/// # Why `Content-Length` alone was not enough
///
/// The first cut refused every body without a declared length. That is safe and
/// nearly useless: measured against a real site, the responses came back
/// `transfer-encoding: chunked`, which is the norm on HTTP/1.1 and HTTP/2, so
/// the capture stayed empty for the ordinary case.
///
/// # What is refused now
///
/// A declared length above the ceiling, and any endless-by-design content type.
/// An event stream has no last byte: buffering one does not finish late, it
/// never finishes, and the page waits on a proxy that waits on the server.
/// Everything else is buffered up to the ceiling.
fn body_is_bufferable(
    headers: &hudsucker::hyper::HeaderMap,
    content_type: &Option<String>,
) -> bool {
    if content_type
        .as_deref()
        .is_some_and(|ct| ct == "text/event-stream")
    {
        return false;
    }
    match headers
        .get(hudsucker::hyper::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<usize>().ok())
    {
        Some(len) => len <= BUFFER_CEILING_BYTES,
        // Undeclared length is the common case; the ceiling still applies while
        // reading, so this is bounded rather than open-ended.
        None => true,
    }
}

/// Cut retained bytes to the budget, reporting whether anything was dropped.
fn clip(bytes: &[u8], budget: usize) -> (&[u8], bool) {
    if bytes.len() <= budget {
        return (bytes, false);
    }
    // Never split a UTF-8 sequence: the retained text is rendered for an agent,
    // and half a codepoint is worse than one fewer character.
    let mut end = budget;
    while end > 0 && (bytes[end] & 0b1100_0000) == 0b1000_0000 {
        end -= 1;
    }
    (&bytes[..end], true)
}

/// Match a host against one allowlist entry, covering its subdomains.
///
/// `example.com` admits `api.example.com`, because a site's own API is what a
/// caller means by "capture this site". It does not admit `notexample.com`:
/// the boundary is a dot, not a substring, or `evil-example.com` would pass.
fn host_matches(host: &str, allowed: &str) -> bool {
    let allowed = allowed.trim().to_ascii_lowercase();
    if allowed.is_empty() {
        return false;
    }
    host == allowed || host.ends_with(&format!(".{allowed}"))
}

/// Split a comma-separated `--hosts` value into allowlist entries.
pub(super) fn parse_hosts(raw: Option<&str>) -> Vec<String> {
    raw.map(|s| {
        s.split(',')
            .map(|h| h.trim().to_ascii_lowercase())
            .filter(|h| !h.is_empty())
            .collect()
    })
    .unwrap_or_default()
}

/// Read `Content-Type`, lowercased, without the parameters after `;`.
fn content_type_of(headers: &hudsucker::hyper::HeaderMap) -> Option<String> {
    headers
        .get(hudsucker::hyper::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(';').next().unwrap_or(s).trim().to_ascii_lowercase())
}

/// Whether this content type is media the operator asked to skip.
fn is_media(content_type: &Option<String>) -> bool {
    content_type.as_deref().is_some_and(|ct| {
        ct.starts_with("image/") || ct.starts_with("video/") || ct.starts_with("audio/")
    })
}

/// Decide how many bytes of this body to keep.
fn retain_budget(policy: BodyPolicy, content_type: &Option<String>) -> usize {
    if policy.skip_media && is_media(content_type) {
        return 0;
    }
    policy.max_bytes
}

/// Render retained bytes for an agent: text as text, anything else as a note.
///
/// Binary is never emitted raw. A capture is read by a model over stdout, and
/// pasting a PNG into that stream costs tokens and tells the reader nothing.
fn render_body(bytes: &[u8], content_type: &Option<String>, truncated: bool) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    match std::str::from_utf8(bytes) {
        Ok(s) if !is_media(content_type) => Some(if truncated {
            format!("{s}…[truncated]")
        } else {
            s.to_string()
        }),
        _ => Some(format!(
            "<{} bytes {}>",
            bytes.len(),
            content_type.as_deref().unwrap_or("binary")
        )),
    }
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
