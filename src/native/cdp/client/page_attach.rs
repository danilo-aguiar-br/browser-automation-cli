// SPDX-License-Identifier: MIT OR Apache-2.0

//! Page-scoped CDP event attach (console / network / session).

use chromiumoxide::cdp::browser_protocol::input::EventDragIntercepted;
use chromiumoxide::cdp::browser_protocol::network::EventRequestWillBeSent;
use chromiumoxide::cdp::browser_protocol::page::{
    EventJavascriptDialogClosed, EventJavascriptDialogOpening, EventScreencastFrame,
};
use chromiumoxide::cdp::js_protocol::heap_profiler::{
    EventAddHeapSnapshotChunk, EventReportHeapSnapshotProgress,
};
use chromiumoxide::cdp::js_protocol::runtime::EventConsoleApiCalled;

use super::forwarders::attach_page_event_forwarder;
use super::types::CdpClient;

impl CdpClient {
    /// Page-level `Runtime.consoleAPICalled` (page-scoped CDP events).
    ///
    /// Console capture is per page, so this has to run for each page the
    /// invocation owns; nothing is buffered before the forwarder is attached.
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
    /// Multi-page attach is I/O-bound → [`join_bounded`](crate::concurrency::join_bounded) after releasing the
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
                        event_tx.clone(),
                    )
                    .await?;
                    // GAP-054: Closed settles dialog_settled for the agent envelope.
                    attach_page_event_forwarder::<EventJavascriptDialogClosed>(
                        &page,
                        "Page.javascriptDialogClosed",
                        event_tx.clone(),
                    )
                    .await?;
                    // GAP-030: Input.dragIntercepted is a target-session event.
                    // A browser-level listener never sees it, which silently
                    // downgraded every drag to the synthetic mouse fallback.
                    attach_page_event_forwarder::<EventDragIntercepted>(
                        &page,
                        "Input.dragIntercepted",
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
