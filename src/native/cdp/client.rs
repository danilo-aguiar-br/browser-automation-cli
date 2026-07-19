// SPDX-License-Identifier: MIT OR Apache-2.0
//! CDP client over chromiumoxide (single connection — no dual WebSocket).
#![allow(missing_docs)]
//!
//! Chrome one-shot: `Browser::launch` only.
//! Lightpanda / attach path: `Browser::connect` only.
//! FORBIDDEN: second `tokio-tungstenite` attach to the same browser.
//!
//! # Workload
//!
//! **I/O-bound** CDP WebSocket. Multi-page listener attach fans out with
//! [`crate::concurrency::join_bounded`] after releasing `browser.lock`
//! (rules: never hold a lock across unbounded sequential awaits when pages
//! are independent).

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

/// CDP client wrapping a shared chromiumoxide [`Browser`].
///
/// # Interior mutability
///
/// `browser` uses **`tokio::sync::Mutex`** because guards are held across
/// `.await` points (`Browser::execute`, `pages()`, `event_listener`). A
/// `std::sync::Mutex` here would block the async runtime (rules: never hold
/// std mutex across `.await`). The mutex is not exposed in the public agent
/// JSON API — only as an internal handle for FINALIZE.
///
/// # Ownership
///
/// Holds the event-handler task and shared browser mutex — do not discard
/// without FINALIZE (`#[must_use]`).
#[must_use = "CdpClient owns the CDP connection and handler tasks"]
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
                .map_err(|e| format_cdp_err(method, &e))?
        } else {
            let browser = self.browser.lock().await;
            browser
                .execute(cmd)
                .await
                .map_err(|e| format_cdp_err(method, &e))?
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
        let n = pages.len();
        if n == 1 {
            if let Some(page) = pages.into_iter().next() {
                return Ok(page);
            }
        }
        Err(format!(
            "No chromiumoxide Page for session_id={session_id} (pages={n})"
        ))
    }
}

/// Format a chromiumoxide error for agent-facing messages (Display only → borrow).
fn format_cdp_err(method: &str, e: &CdpError) -> String {
    format!("CDP error ({method}): {e}")
}

/// Forward a typed CDP event stream onto the shared broadcast channel.
///
/// # Macro policy (`rules_rust_macros`)
///
/// Prefer **generics + monomorphization** over `macro_rules!` when the only
/// variation is a type parameter and a method string. A previous local
/// `macro_rules! fwd` expanded identical bodies for each CDP event type; that
/// is exactly what a generic function does without hygiene / follow-set /
/// double-evaluation concerns.
///
/// Browser-level listeners do not expose a session id; lifecycle accepts `None`.
fn spawn_cdp_event_forwarder<T, St>(
    mut stream: St,
    method: &'static str,
    event_tx: broadcast::Sender<CdpEvent>,
) -> JoinHandle<()>
where
    T: serde::Serialize + Send + Sync + 'static,
    St: futures::Stream<Item = Arc<T>> + Send + Unpin + 'static,
{
    tokio::spawn(async move {
        while let Some(ev) = stream.next().await {
            let params = serde_json::to_value(ev.as_ref()).unwrap_or(Value::Null);
            let _ = event_tx.send(CdpEvent {
                method: method.to_string(),
                params,
                session_id: None,
            });
        }
    })
}

/// Subscribe to one browser-level CDP event type and spawn a forwarder task.
async fn attach_browser_event_forwarder<T>(
    browser: &Browser,
    method: &'static str,
    event_tx: broadcast::Sender<CdpEvent>,
) -> Result<JoinHandle<()>, String>
where
    T: chromiumoxide::cdp::IntoEventKind + serde::Serialize + Unpin + 'static,
{
    let stream = browser
        .event_listener::<T>()
        .await
        .map_err(|e| format!("event_listener {method}: {e}"))?;
    Ok(spawn_cdp_event_forwarder(stream, method, event_tx))
}

/// Subscribe to one page-level CDP event type and spawn a forwarder task.
async fn attach_page_event_forwarder<T>(
    page: &Page,
    method: &'static str,
    event_tx: broadcast::Sender<CdpEvent>,
) -> Result<(), String>
where
    T: chromiumoxide::cdp::IntoEventKind + serde::Serialize + Unpin + 'static,
{
    let stream = page
        .event_listener::<T>()
        .await
        .map_err(|e| format!("page {method} listener: {e}"))?;
    // Page-scoped tasks are fire-and-forget for the session lifetime (same as
    // pre-refactor); browser-level handles are retained on `CdpClient`.
    let _handle = spawn_cdp_event_forwarder(stream, method, event_tx);
    Ok(())
}

async fn spawn_event_forwarders(
    browser: Arc<Mutex<Browser>>,
    event_tx: broadcast::Sender<CdpEvent>,
) -> Result<Vec<JoinHandle<()>>, String> {
    let mut handles = Vec::with_capacity(13);
    let b = browser.lock().await;

    handles.push(
        attach_browser_event_forwarder::<EventLoadEventFired>(
            &b,
            "Page.loadEventFired",
            event_tx.clone(),
        )
        .await?,
    );
    handles.push(
        attach_browser_event_forwarder::<EventDomContentEventFired>(
            &b,
            "Page.domContentEventFired",
            event_tx.clone(),
        )
        .await?,
    );
    handles.push(
        attach_browser_event_forwarder::<EventRequestWillBeSent>(
            &b,
            "Network.requestWillBeSent",
            event_tx.clone(),
        )
        .await?,
    );
    handles.push(
        attach_browser_event_forwarder::<EventLoadingFinished>(
            &b,
            "Network.loadingFinished",
            event_tx.clone(),
        )
        .await?,
    );
    handles.push(
        attach_browser_event_forwarder::<EventLoadingFailed>(
            &b,
            "Network.loadingFailed",
            event_tx.clone(),
        )
        .await?,
    );
    handles.push(
        attach_browser_event_forwarder::<EventRequestPaused>(
            &b,
            "Fetch.requestPaused",
            event_tx.clone(),
        )
        .await?,
    );
    handles.push(
        attach_browser_event_forwarder::<EventJavascriptDialogOpening>(
            &b,
            "Page.javascriptDialogOpening",
            event_tx.clone(),
        )
        .await?,
    );
    // Console API (context7/docs-rs): required for --capture-console.
    handles.push(
        attach_browser_event_forwarder::<EventConsoleApiCalled>(
            &b,
            "Runtime.consoleAPICalled",
            event_tx.clone(),
        )
        .await?,
    );
    // Heap / tracing / screencast: required for heap take, perf stop, screencast frames.
    handles.push(
        attach_browser_event_forwarder::<EventAddHeapSnapshotChunk>(
            &b,
            "HeapProfiler.addHeapSnapshotChunk",
            event_tx.clone(),
        )
        .await?,
    );
    handles.push(
        attach_browser_event_forwarder::<EventReportHeapSnapshotProgress>(
            &b,
            "HeapProfiler.reportHeapSnapshotProgress",
            event_tx.clone(),
        )
        .await?,
    );
    handles.push(
        attach_browser_event_forwarder::<EventDataCollected>(
            &b,
            "Tracing.dataCollected",
            event_tx.clone(),
        )
        .await?,
    );
    handles.push(
        attach_browser_event_forwarder::<EventTracingComplete>(
            &b,
            "Tracing.tracingComplete",
            event_tx.clone(),
        )
        .await?,
    );
    handles.push(
        attach_browser_event_forwarder::<EventScreencastFrame>(
            &b,
            "Page.screencastFrame",
            event_tx.clone(),
        )
        .await?,
    );

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
        let pages = {
            let browser = self.browser.lock().await;
            browser
                .pages()
                .await
                .map_err(|e| format!("Browser::pages for network listeners: {e}"))?
        };
        let event_tx = self.event_tx.clone();
        let limit = crate::concurrency::effective_limit_capped(8);
        let futs: Vec<_> = pages
            .into_iter()
            .map(|page| {
                let event_tx = event_tx.clone();
                async move {
                    attach_page_event_forwarder::<EventRequestWillBeSent>(
                        &page,
                        "Network.requestWillBeSent",
                        event_tx,
                    )
                    .await
                }
            })
            .collect();
        let results = crate::concurrency::join_bounded(futs, limit).await;
        for r in results {
            r?;
        }
        Ok(())
    }

    async fn attach_page_event_forwarders_console(&self) -> Result<(), String> {
        let pages = {
            let browser = self.browser.lock().await;
            browser
                .pages()
                .await
                .map_err(|e| format!("Browser::pages for console listeners: {e}"))?
        };
        let event_tx = self.event_tx.clone();
        let limit = crate::concurrency::effective_limit_capped(8);
        let futs: Vec<_> = pages
            .into_iter()
            .map(|page| {
                let event_tx = event_tx.clone();
                async move {
                    attach_page_event_forwarder::<EventConsoleApiCalled>(
                        &page,
                        "Runtime.consoleAPICalled",
                        event_tx,
                    )
                    .await
                }
            })
            .collect();
        let results = crate::concurrency::join_bounded(futs, limit).await;
        for r in results {
            r?;
        }
        Ok(())
    }

    /// Page-scoped CDP events (heap chunks, screencast frames, JS dialogs).
    /// Browser-level listeners miss target-session events; attach after pages exist.
    ///
    /// Multi-page attach is I/O-bound → [`join_bounded`] after releasing the
    /// browser lock (PAR-53).
    pub async fn attach_page_session_forwarders(&self) -> Result<(), String> {
        let pages = {
            let browser = self.browser.lock().await;
            browser
                .pages()
                .await
                .map_err(|e| format!("Browser::pages for session listeners: {e}"))?
        };
        let event_tx = self.event_tx.clone();
        let limit = crate::concurrency::effective_limit_capped(8);
        let futs: Vec<_> = pages
            .into_iter()
            .map(|page| {
                let event_tx = event_tx.clone();
                async move {
                    attach_page_event_forwarder::<EventAddHeapSnapshotChunk>(
                        &page,
                        "HeapProfiler.addHeapSnapshotChunk",
                        event_tx.clone(),
                    )
                    .await?;
                    attach_page_event_forwarder::<EventReportHeapSnapshotProgress>(
                        &page,
                        "HeapProfiler.reportHeapSnapshotProgress",
                        event_tx.clone(),
                    )
                    .await?;
                    attach_page_event_forwarder::<EventScreencastFrame>(
                        &page,
                        "Page.screencastFrame",
                        event_tx.clone(),
                    )
                    .await?;
                    // Page-scoped dialog open (required for eval auto-accept).
                    attach_page_event_forwarder::<EventJavascriptDialogOpening>(
                        &page,
                        "Page.javascriptDialogOpening",
                        event_tx,
                    )
                    .await?;
                    Ok::<(), String>(())
                }
            })
            .collect();
        let results = crate::concurrency::join_bounded(futs, limit).await;
        for r in results {
            r?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;
    use serde::Serialize;

    #[derive(Debug, Serialize)]
    struct DummyEvent {
        n: u32,
    }

    #[tokio::test]
    async fn cdp_event_forwarder_serializes_and_publishes() {
        let (tx, mut rx) = broadcast::channel(4);
        let stream = stream::iter(vec![Arc::new(DummyEvent { n: 7 })]);
        let handle = spawn_cdp_event_forwarder(stream, "Test.event", tx);
        let ev = rx.recv().await.expect("event delivered");
        assert_eq!(ev.method, "Test.event");
        assert_eq!(ev.params["n"], 7);
        assert!(ev.session_id.is_none());
        handle.await.expect("forwarder task");
    }
}
