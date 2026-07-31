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
    pub async fn from_browser(browser: Browser, mut handler: Handler) -> Result<Self, String> {
        let handler_task = tokio::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });

        let (event_tx, _) = broadcast::channel(crate::xdg::policy::policy_usize(
            crate::xdg::policy::key::CDP_EVENT_BROADCAST_CAPACITY,
        ));

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
}
