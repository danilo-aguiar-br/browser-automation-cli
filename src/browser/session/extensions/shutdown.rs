// SPDX-License-Identifier: MIT OR Apache-2.0
//! OneShotSession methods (componentized; single-responsibility impl blocks).

use crate::error::{CliError, ErrorKind};

use super::super::OneShotSession;

impl OneShotSession {
    /// Gracefully close the Chrome session and drop process-local state.
    ///
    /// # Errors
    ///
    /// Fails with [`ErrorKind::Browser`] —
    /// `"Browser close failed: …"` — when the graceful `Browser.close` or the
    /// wait that follows is refused.
    ///
    /// An `Err` here means teardown was not graceful, not that the browser
    /// survived: the process is still reaped, by the chromiumoxide finalize
    /// path or by the engine process `Drop`. Every process-local buffer is
    /// cleared before the close is attempted, so a failure leaks no captured
    /// console or network evidence.
    pub async fn shutdown(mut self) -> Result<(), CliError> {
        self.console_log.clear();
        self.network_log.clear();
        self.heap_chunks.clear();
        self.trace_chunks.clear();
        self.heap_bytes = 0;
        self.heap_overflow = false;
        self.trace_dropped = 0;
        self.screencast_frames.clear();
        self.screencast_ack_ids.clear();
        self.screencast_dropped = 0;
        self.screencast_dir = None;
        self.last_trace_body = None;
        self.ref_map.clear();
        self.manager.close().await.map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Browser,
                format!("Browser close failed: {e}"),
                crate::i18n::suggestion_key("browser_close_reaped", None),
            )
        })
    }
}
