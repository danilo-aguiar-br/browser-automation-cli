// SPDX-License-Identifier: MIT OR Apache-2.0

//! Browser-level CDP event forwarders onto the shared broadcast channel.

use std::sync::Arc;

use chromiumoxide::browser::Browser;
use chromiumoxide::cdp::browser_protocol::fetch::EventRequestPaused;
use chromiumoxide::cdp::browser_protocol::input::EventDragIntercepted;
use chromiumoxide::cdp::browser_protocol::network::{
    EventLoadingFailed, EventLoadingFinished, EventRequestWillBeSent,
};
use chromiumoxide::cdp::browser_protocol::page::{
    EventDomContentEventFired, EventJavascriptDialogClosed, EventJavascriptDialogOpening,
    EventLoadEventFired, EventScreencastFrame,
};
use chromiumoxide::cdp::browser_protocol::tracing::{EventDataCollected, EventTracingComplete};
use chromiumoxide::cdp::js_protocol::heap_profiler::{
    EventAddHeapSnapshotChunk, EventReportHeapSnapshotProgress,
};
use chromiumoxide::cdp::js_protocol::runtime::EventConsoleApiCalled;
use chromiumoxide::page::Page;
use futures::StreamExt;
use serde_json::Value;
use tokio::sync::{broadcast, Mutex};
use tokio::task::JoinHandle;

use super::super::types::CdpEvent;

/// Forward a CDP event stream onto the shared broadcast channel.
///
/// `session_id` is set for **page-scoped** listeners (`Page::session_id`) so
/// multi-tab dialogs, console, and network frames attribute to the correct
/// target. Browser-scoped listeners pass `None`; consumers fall back to the
/// active session (GAP-041 / GAP-054). Event bodies such as
/// `Page.javascriptDialogClosed` do not carry `sessionId` (docs.rs
/// chromiumoxide_cdp), so the forwarder is the only source of truth.
pub(crate) fn spawn_cdp_event_forwarder<T, St>(
    mut stream: St,
    method: &'static str,
    event_tx: broadcast::Sender<CdpEvent>,
    session_id: Option<String>,
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
                session_id: session_id.clone(),
            });
        }
    })
}

/// Subscribe to one browser-level CDP event type and spawn a forwarder task.
pub(crate) async fn attach_browser_event_forwarder<T>(
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
    // Browser-scope has no page session; dialog_key_for falls back to active.
    Ok(spawn_cdp_event_forwarder(stream, method, event_tx, None))
}

/// Subscribe to one page-level CDP event type and spawn a forwarder task.
pub(crate) async fn attach_page_event_forwarder<T>(
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
    // Page-scoped: stamp the target session so multi-tab attribution works.
    let session_id = Some(page.session_id().as_ref().to_string());
    // Page-scoped tasks are fire-and-forget for the session lifetime (same as
    // pre-refactor); browser-level handles are retained on `CdpClient`.
    let _handle = spawn_cdp_event_forwarder(stream, method, event_tx, session_id);
    Ok(())
}

pub(crate) async fn spawn_event_forwarders(
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
    // GAP-054: Closed must be forwarded or settle_after_dialog_answer always
    // times out with dialog_settled:false (docs.rs EventJavascriptDialogClosed).
    handles.push(
        attach_browser_event_forwarder::<EventJavascriptDialogClosed>(
            &b,
            "Page.javascriptDialogClosed",
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
    // GAP-030: carries the DataTransfer the page itself built in `dragstart`,
    // which is the only honest payload for a subsequent drop.
    handles.push(
        attach_browser_event_forwarder::<EventDragIntercepted>(
            &b,
            "Input.dragIntercepted",
            event_tx.clone(),
        )
        .await?,
    );

    drop(b);
    Ok(handles)
}
