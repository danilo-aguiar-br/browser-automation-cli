// SPDX-License-Identifier: MIT OR Apache-2.0

//! Connect / from_browser construction of [`CdpClient`].

use std::sync::Arc;

use chromiumoxide::browser::Browser;
use chromiumoxide::Handler;
use futures::StreamExt;
use tokio::sync::{broadcast, Mutex};

use super::forwarders::spawn_event_forwarders;
use super::types::CdpClient;

impl CdpClient {
    /// Wrap an already-launched browser and start pumping its event handler.
    ///
    /// The handler must be driven for the connection to progress, so it is spawned
    /// here rather than left to the caller to remember.
    ///
    /// # Errors
    ///
    /// Propagates `spawn_event_forwarders`: the first page listing or
    /// per-page event subscription refused by the freshly attached browser.
    /// Constructing the broadcast channel and spawning the handler task cannot
    /// fail.
    pub async fn from_browser(browser: Browser, mut handler: Handler) -> Result<Self, String> {
        // The pump used to `break` on any error with no trace at all, and a dead
        // pump makes every later CDP call fail the same anonymous way: the
        // command times out after 30 s while `Browser::pages` waits forever.
        // Ruling that out cost a full instrumented run on 2026-09-04 — the pump
        // was alive and Chrome had simply answered `Input.dispatchKeyEvent`
        // 30_183 ms late — so the line below is what makes the next reader spend
        // one `rg` instead of one browser session.
        //
        // `warn` and not `error`: FINALIZE awaits `Browser.close` BEFORE calling
        // [`stop_event_pump`](Self::stop_event_pump), so the pump can still
        // observe the reset Chrome leaves behind on a run that SUCCEEDED. The
        // default filter is `error`, which keeps that case silent, and `-v`
        // raises it to `info` and shows the line to whoever went looking.
        let handler_task = tokio::spawn(async move {
            while let Some(h) = handler.next().await {
                if let Err(e) = h {
                    tracing::warn!(error = %e, "CDP event pump stopped on handler error");
                    break;
                }
            }
            tracing::debug!("CDP event pump ended");
        });

        let (event_tx, _) = broadcast::channel(crate::xdg::policy::policy_usize(
            crate::xdg::policy::key::CDP_EVENT_BROADCAST_CAPACITY,
        ));

        let browser = Arc::new(Mutex::new(browser));
        let event_forwarders = spawn_event_forwarders(browser.clone(), event_tx.clone()).await?;

        Ok(Self {
            browser,
            event_tx,
            handler: handler_task,
            _event_forwarders: event_forwarders,
        })
    }

    /// Stop pumping CDP events, before the transport is expected to go away.
    ///
    /// # Why FINALIZE calls this
    ///
    /// `Browser.close` makes Chrome drop the WebSocket without a closing
    /// handshake. chromiumoxide logs that as
    /// `ERROR chromiumoxide::handler: WS Connection error:
    /// Ws(Protocol(ResetWithoutClosingHandshake))` from inside `handler.next()`,
    /// so the message escapes to stderr on a perfectly successful one-shot run.
    /// Our own loop already breaks on the error; the log is emitted before we
    /// ever see it.
    ///
    /// An ERROR line on a clean shutdown is worse than noise: an operator
    /// reading `-v` output cannot tell it apart from a real transport failure.
    /// Aborting the pump first means the reset is never observed, so nothing is
    /// suppressed or downgraded — the event simply does not occur.
    ///
    /// Idempotent, and safe on an already-finished task.
    pub fn stop_event_pump(&self) {
        self.handler.abort();
    }

    /// Attach via chromiumoxide `Browser::connect` (lightpanda only).
    ///
    /// # Errors
    ///
    /// Propagates
    /// [`connect_with_headers`](Self::connect_with_headers): the WebSocket at
    /// `url` is unreachable or rejects the upgrade, or the event forwarders
    /// cannot be armed.
    pub async fn connect(url: &str) -> Result<Self, String> {
        Self::connect_with_headers(url, None).await
    }

    /// Headers are ignored on the oxide path (chromiumoxide connect has no custom WS headers API).
    ///
    /// # Errors
    ///
    /// Fails when `Browser::connect` cannot reach `url` — endpoint down, wrong
    /// port, or a WebSocket upgrade refused — and otherwise propagates
    /// [`from_browser`](Self::from_browser).
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
}
