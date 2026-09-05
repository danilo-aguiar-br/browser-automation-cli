// SPDX-License-Identifier: MIT OR Apache-2.0

//! Page-scoped CDP event attach (console / network / session).

use chromiumoxide::cdp::browser_protocol::input::EventDragIntercepted;
use chromiumoxide::cdp::browser_protocol::network::{
    EventRequestWillBeSent, EventResponseReceived,
};
use chromiumoxide::cdp::browser_protocol::page::{
    EventJavascriptDialogClosed, EventJavascriptDialogOpening, EventScreencastFrame,
};
use chromiumoxide::cdp::js_protocol::heap_profiler::{
    EventAddHeapSnapshotChunk, EventReportHeapSnapshotProgress,
};
use chromiumoxide::cdp::js_protocol::runtime::{EventBindingCalled, EventConsoleApiCalled};

use super::forwarders::attach_page_event_forwarder;
use super::types::CdpClient;

impl CdpClient {
    /// Page-level `Runtime.consoleAPICalled` (page-scoped CDP events).
    ///
    /// Console capture is per page, so this has to run for each page the
    /// invocation owns; nothing is buffered before the forwarder is attached.
    ///
    /// # Errors
    ///
    /// Fails when `Browser::pages` cannot enumerate the open targets, or when
    /// any page refuses the `Runtime.consoleAPICalled` listener. The first
    /// failing page wins; listeners already attached to other pages stay
    /// armed.
    pub async fn attach_page_console_forwarders(&self) -> Result<(), String> {
        self.attach_page_event_forwarders_console().await
    }

    /// Page-level `Runtime.bindingCalled` (page-scoped CDP events).
    ///
    /// Attached on demand by `record` rather than at launch: a binding event
    /// only exists once `Runtime.addBinding` published one, so forwarding it
    /// unconditionally would spawn a listener task every invocation pays for
    /// and no invocation reads.
    ///
    /// # Errors
    ///
    /// Fails when `Browser::pages` cannot enumerate the open targets, or when
    /// any page refuses the `Runtime.bindingCalled` listener.
    pub async fn attach_page_binding_forwarders(&self) -> Result<(), String> {
        let pages = {
            let browser = self.browser.lock().await;
            browser
                .pages()
                .await
                .map_err(|e| format!("Browser::pages for binding listeners: {e}"))?
        };
        let event_tx = self.event_tx.clone();
        let limit =
            crate::concurrency::effective_limit_capped(crate::concurrency::CDP_ATTACH_FANOUT_CAP);
        let futs: Vec<_> = pages
            .into_iter()
            .map(|page| {
                let event_tx = event_tx.clone();
                async move {
                    attach_page_event_forwarder::<EventBindingCalled>(
                        &page,
                        "Runtime.bindingCalled",
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

    /// Page-level network events: `requestWillBeSent` and `responseReceived`.
    ///
    /// Page-scoped events reach the broadcast channel ONLY for methods listed
    /// here, so a handler for an unlisted method is dead code no matter how
    /// correct it looks. `responseReceived` was unlisted until 0.1.9, which is
    /// why `status` and `mimeType` had no path into the capture log and the
    /// mitm consumer reading `status` could never be satisfied — the gap ran
    /// three layers deep: consumer, producer, and this forwarder.
    ///
    /// # Errors
    ///
    /// Fails when `Browser::pages` cannot enumerate the open targets, or when
    /// any page refuses either listener.
    pub async fn attach_page_network_forwarders(&self) -> Result<(), String> {
        let pages = {
            let browser = self.browser.lock().await;
            browser
                .pages()
                .await
                .map_err(|e| format!("Browser::pages for network listeners: {e}"))?
        };
        let event_tx = self.event_tx.clone();
        let limit =
            crate::concurrency::effective_limit_capped(crate::concurrency::CDP_ATTACH_FANOUT_CAP);
        let futs: Vec<_> = pages
            .into_iter()
            .map(|page| {
                let event_tx = event_tx.clone();
                async move {
                    attach_page_event_forwarder::<EventRequestWillBeSent>(
                        &page,
                        "Network.requestWillBeSent",
                        event_tx.clone(),
                    )
                    .await?;
                    attach_page_event_forwarder::<EventResponseReceived>(
                        &page,
                        "Network.responseReceived",
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
        let limit =
            crate::concurrency::effective_limit_capped(crate::concurrency::CDP_ATTACH_FANOUT_CAP);
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
    ///
    /// # Errors
    ///
    /// Fails when `Browser::pages` cannot enumerate the open targets, or when
    /// a page refuses any one of the six listeners attached in order —
    /// `HeapProfiler.addHeapSnapshotChunk`,
    /// `HeapProfiler.reportHeapSnapshotProgress`, `Page.screencastFrame`,
    /// `Page.javascriptDialogOpening`, `Page.javascriptDialogClosed`,
    /// `Input.dragIntercepted`. A refusal aborts that page only; the listeners
    /// already attached before it are not rolled back.
    pub async fn attach_page_session_forwarders(&self) -> Result<(), String> {
        let pages = {
            let browser = self.browser.lock().await;
            browser
                .pages()
                .await
                .map_err(|e| format!("Browser::pages for session listeners: {e}"))?
        };
        let event_tx = self.event_tx.clone();
        let limit =
            crate::concurrency::effective_limit_capped(crate::concurrency::CDP_ATTACH_FANOUT_CAP);
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
