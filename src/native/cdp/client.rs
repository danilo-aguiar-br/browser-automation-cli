//! CDP client over chromiumoxide (single connection — no dual WebSocket).
#![allow(missing_docs)]
//!
//! Chrome one-shot: `Browser::launch` only.
//! Lightpanda / attach path: `Browser::connect` only.
//! PROIBIDO: second `tokio-tungstenite` attach to the same browser.

use std::borrow::Cow;
use std::sync::Arc;

use chromiumoxide::browser::Browser;
use chromiumoxide::cdp::browser_protocol::fetch::EventRequestPaused;
use chromiumoxide::cdp::browser_protocol::network::{
    EventLoadingFailed, EventLoadingFinished, EventRequestWillBeSent,
};
use chromiumoxide::cdp::browser_protocol::page::{
    EventDomContentEventFired, EventJavascriptDialogOpening, EventLoadEventFired,
    EventScreencastFrame,
};
use chromiumoxide::cdp::browser_protocol::tracing::{EventDataCollected, EventTracingComplete};
use chromiumoxide::cdp::js_protocol::heap_profiler::{
    EventAddHeapSnapshotChunk, EventReportHeapSnapshotProgress,
};
use chromiumoxide::cdp::js_protocol::runtime::EventConsoleApiCalled;
use chromiumoxide::error::CdpError;
use chromiumoxide::page::Page;
use chromiumoxide::types::{Command, Method, MethodId};
use chromiumoxide::Handler;
use futures::StreamExt;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::{broadcast, Mutex};
use tokio::task::JoinHandle;

use super::types::CdpEvent;

/// Dynamic CDP command for `Browser::execute` / `Page::execute`.
#[derive(Debug, Clone)]
struct RawCdpCommand {
    method: String,
    params: Value,
}

impl Serialize for RawCdpCommand {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match &self.params {
            Value::Null => {
                use serde::ser::SerializeMap;
                let map = serializer.serialize_map(Some(0))?;
                map.end()
            }
            other => other.serialize(serializer),
        }
    }
}

impl Method for RawCdpCommand {
    fn identifier(&self) -> MethodId {
        Cow::Owned(self.method.clone())
    }
}

impl Command for RawCdpCommand {
    type Response = Value;
}

pub struct CdpClient {
    browser: Arc<Mutex<Browser>>,
    event_tx: broadcast::Sender<CdpEvent>,
    _handler: JoinHandle<()>,
    _event_forwarders: Vec<JoinHandle<()>>,
}

impl CdpClient {
    /// Build client from an already-launched or connected chromiumoxide Browser + handler.
    pub async fn from_browser(browser: Browser, mut handler: Handler) -> Result<Self, String> {
        let handler_task = tokio::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });

        let (event_tx, _) = broadcast::channel(4096);

        let browser = Arc::new(Mutex::new(browser));
        let event_forwarders = spawn_event_forwarders(browser.clone(), event_tx.clone()).await?;

        Ok(Self {
            browser,
            event_tx,
            _handler: handler_task,
            _event_forwarders: event_forwarders,
        })
    }

    /// Attach via chromiumoxide `Browser::connect` (lightpanda only).
    pub async fn connect(url: &str) -> Result<Self, String> {
        Self::connect_with_headers(url, None).await
    }

    /// Headers are ignored on the oxide path (chromiumoxide connect has no custom WS headers API).
    pub async fn connect_with_headers(
        url: &str,
        _headers: Option<Vec<(String, String)>>,
    ) -> Result<Self, String> {
        let (browser, handler) = Browser::connect(url)
            .await
            .map_err(|e| format!("CDP Browser::connect failed: {e}"))?;
        Self::from_browser(browser, handler).await
    }

    /// Shared browser handle (for FINALIZE close/wait/kill).
    pub fn browser(&self) -> Arc<Mutex<Browser>> {
        self.browser.clone()
    }

    pub async fn send_command(
        &self,
        method: &str,
        params: Option<Value>,
        session_id: Option<&str>,
    ) -> Result<Value, String> {
        let cmd = RawCdpCommand {
            method: method.to_string(),
            params: params.unwrap_or(Value::Null),
        };

        let result = if let Some(sid) = session_id.filter(|s| !s.is_empty()) {
            let page = self.page_for_session(sid).await?;
            page.execute(cmd)
                .await
                .map_err(|e| format_cdp_err(method, e))?
        } else {
            let browser = self.browser.lock().await;
            browser
                .execute(cmd)
                .await
                .map_err(|e| format_cdp_err(method, e))?
        };

        Ok(result.result)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<CdpEvent> {
        self.event_tx.subscribe()
    }

    pub async fn send_command_typed<P: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: &P,
        session_id: Option<&str>,
    ) -> Result<R, String> {
        let params_value = serde_json::to_value(params)
            .map_err(|e| format!("Failed to serialize params: {}", e))?;
        let result = self
            .send_command(method, Some(params_value), session_id)
            .await?;
        serde_json::from_value(result)
            .map_err(|e| format!("Failed to deserialize CDP response for {}: {}", method, e))
    }

    pub async fn send_command_no_params(
        &self,
        method: &str,
        session_id: Option<&str>,
    ) -> Result<Value, String> {
        self.send_command(method, None, session_id).await
    }

    /// Best-effort command (still awaits oxide execute).
    pub async fn send_command_no_wait(
        &self,
        method: &str,
        params: Option<Value>,
        session_id: Option<&str>,
    ) -> Result<(), String> {
        let _ = self.send_command(method, params, session_id).await;
        Ok(())
    }

    async fn page_for_session(&self, session_id: &str) -> Result<Page, String> {
        let browser = self.browser.lock().await;
        let pages = browser
            .pages()
            .await
            .map_err(|e| format!("Browser::pages failed: {e}"))?;
        for page in pages {
            if page.session_id().as_ref() == session_id {
                return Ok(page);
            }
        }
        // Fallback: first page if only one exists (session id mismatch after attach).
        let pages = browser
            .pages()
            .await
            .map_err(|e| format!("Browser::pages failed: {e}"))?;
        if pages.len() == 1 {
            return Ok(pages.into_iter().next().unwrap());
        }
        Err(format!(
            "No chromiumoxide Page for session_id={session_id} (pages={})",
            pages.len()
        ))
    }
}

fn format_cdp_err(method: &str, e: CdpError) -> String {
    format!("CDP error ({method}): {e}")
}

async fn spawn_event_forwarders(
    browser: Arc<Mutex<Browser>>,
    event_tx: broadcast::Sender<CdpEvent>,
) -> Result<Vec<JoinHandle<()>>, String> {
    let mut handles = Vec::new();
    let b = browser.lock().await;

    macro_rules! fwd {
        ($ty:ty, $method:expr) => {{
            let mut stream = b
                .event_listener::<$ty>()
                .await
                .map_err(|e| format!("event_listener {}: {e}", $method))?;
            let tx = event_tx.clone();
            handles.push(tokio::spawn(async move {
                while let Some(ev) = stream.next().await {
                    let params = serde_json::to_value(ev.as_ref()).unwrap_or(Value::Null);
                    let _ = tx.send(CdpEvent {
                        method: $method.to_string(),
                        params,
                        // Browser-level listeners do not expose session; lifecycle accepts None.
                        session_id: None,
                    });
                }
            }));
        }};
    }

    fwd!(EventLoadEventFired, "Page.loadEventFired");
    fwd!(EventDomContentEventFired, "Page.domContentEventFired");
    fwd!(EventRequestWillBeSent, "Network.requestWillBeSent");
    fwd!(EventLoadingFinished, "Network.loadingFinished");
    fwd!(EventLoadingFailed, "Network.loadingFailed");
    fwd!(EventRequestPaused, "Fetch.requestPaused");
    fwd!(EventJavascriptDialogOpening, "Page.javascriptDialogOpening");
    // Console API (context7/docs-rs): required for --capture-console.
    fwd!(EventConsoleApiCalled, "Runtime.consoleAPICalled");
    // Heap / tracing / screencast: required for heap take, perf stop, screencast frames.
    fwd!(
        EventAddHeapSnapshotChunk,
        "HeapProfiler.addHeapSnapshotChunk"
    );
    fwd!(
        EventReportHeapSnapshotProgress,
        "HeapProfiler.reportHeapSnapshotProgress"
    );
    fwd!(EventDataCollected, "Tracing.dataCollected");
    fwd!(EventTracingComplete, "Tracing.tracingComplete");
    fwd!(EventScreencastFrame, "Page.screencastFrame");

    drop(b);
    Ok(handles)
}

impl CdpClient {
    /// Attach page-level console listeners (context7 pattern: page.event_listener).
    /// Complements browser-level forwarders when Runtime events are page-scoped.
    pub async fn attach_page_console_forwarders(&self) -> Result<(), String> {
        self.attach_page_event_forwarders_console().await
    }

    /// Page-level Network.requestWillBeSent (page-scoped CDP events).
    pub async fn attach_page_network_forwarders(&self) -> Result<(), String> {
        let browser = self.browser.lock().await;
        let pages = browser
            .pages()
            .await
            .map_err(|e| format!("Browser::pages for network listeners: {e}"))?;
        for page in pages {
            let mut stream = page
                .event_listener::<EventRequestWillBeSent>()
                .await
                .map_err(|e| format!("page EventRequestWillBeSent listener: {e}"))?;
            let tx = self.event_tx.clone();
            tokio::spawn(async move {
                while let Some(ev) = stream.next().await {
                    let params = serde_json::to_value(ev.as_ref()).unwrap_or(Value::Null);
                    let _ = tx.send(CdpEvent {
                        method: "Network.requestWillBeSent".to_string(),
                        params,
                        session_id: None,
                    });
                }
            });
        }
        Ok(())
    }

    async fn attach_page_event_forwarders_console(&self) -> Result<(), String> {
        let browser = self.browser.lock().await;
        let pages = browser
            .pages()
            .await
            .map_err(|e| format!("Browser::pages for console listeners: {e}"))?;
        for page in pages {
            let mut stream = page
                .event_listener::<EventConsoleApiCalled>()
                .await
                .map_err(|e| format!("page EventConsoleApiCalled listener: {e}"))?;
            let tx = self.event_tx.clone();
            tokio::spawn(async move {
                while let Some(ev) = stream.next().await {
                    let params = serde_json::to_value(ev.as_ref()).unwrap_or(Value::Null);
                    let _ = tx.send(CdpEvent {
                        method: "Runtime.consoleAPICalled".to_string(),
                        params,
                        session_id: None,
                    });
                }
            });
        }
        Ok(())
    }

    /// Page-scoped CDP events (heap chunks, screencast frames, JS dialogs).
    /// Browser-level listeners miss target-session events; attach after pages exist.
    pub async fn attach_page_session_forwarders(&self) -> Result<(), String> {
        let browser = self.browser.lock().await;
        let pages = browser
            .pages()
            .await
            .map_err(|e| format!("Browser::pages for session listeners: {e}"))?;
        for page in pages {
            // HeapProfiler.addHeapSnapshotChunk
            {
                let mut stream = page
                    .event_listener::<EventAddHeapSnapshotChunk>()
                    .await
                    .map_err(|e| format!("page EventAddHeapSnapshotChunk: {e}"))?;
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    while let Some(ev) = stream.next().await {
                        let params = serde_json::to_value(ev.as_ref()).unwrap_or(Value::Null);
                        let _ = tx.send(CdpEvent {
                            method: "HeapProfiler.addHeapSnapshotChunk".to_string(),
                            params,
                            session_id: None,
                        });
                    }
                });
            }
            // HeapProfiler.reportHeapSnapshotProgress
            {
                let mut stream = page
                    .event_listener::<EventReportHeapSnapshotProgress>()
                    .await
                    .map_err(|e| format!("page EventReportHeapSnapshotProgress: {e}"))?;
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    while let Some(ev) = stream.next().await {
                        let params = serde_json::to_value(ev.as_ref()).unwrap_or(Value::Null);
                        let _ = tx.send(CdpEvent {
                            method: "HeapProfiler.reportHeapSnapshotProgress".to_string(),
                            params,
                            session_id: None,
                        });
                    }
                });
            }
            // Page.screencastFrame
            {
                let mut stream = page
                    .event_listener::<EventScreencastFrame>()
                    .await
                    .map_err(|e| format!("page EventScreencastFrame: {e}"))?;
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    while let Some(ev) = stream.next().await {
                        let params = serde_json::to_value(ev.as_ref()).unwrap_or(Value::Null);
                        let _ = tx.send(CdpEvent {
                            method: "Page.screencastFrame".to_string(),
                            params,
                            session_id: None,
                        });
                    }
                });
            }
            // Page.javascriptDialogOpening (page-scoped; required for eval auto-accept)
            {
                let mut stream = page
                    .event_listener::<EventJavascriptDialogOpening>()
                    .await
                    .map_err(|e| format!("page EventJavascriptDialogOpening: {e}"))?;
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    while let Some(ev) = stream.next().await {
                        let params = serde_json::to_value(ev.as_ref()).unwrap_or(Value::Null);
                        let _ = tx.send(CdpEvent {
                            method: "Page.javascriptDialogOpening".to_string(),
                            params,
                            session_id: None,
                        });
                    }
                });
            }
        }
        Ok(())
    }
}
