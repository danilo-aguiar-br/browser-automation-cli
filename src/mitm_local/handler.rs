// SPDX-License-Identifier: MIT OR Apache-2.0
//! Single shared hudsucker CaptureHandler (DRY for start + capture_url).

use hudsucker::{
    hyper::{Request, Response},
    tokio_tungstenite::tungstenite::Message,
    Body, HttpContext, HttpHandler, RequestOrResponse, WebSocketContext, WebSocketHandler,
};

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

    async fn handle_response(&mut self, _ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
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
