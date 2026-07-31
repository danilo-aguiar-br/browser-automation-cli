// SPDX-License-Identifier: MIT OR Apache-2.0
//! OneShotSession methods (componentized; single-responsibility impl blocks).

use crate::error::{CliError, ErrorKind};

use super::super::OneShotSession;

impl OneShotSession {
    /// Gracefully close the Chrome session and drop process-local state.
    pub async fn shutdown(mut self) -> Result<(), CliError> {
        self.console_log.clear();
        self.network_log.clear();
        self.heap_chunks.clear();
        self.trace_chunks.clear();
        self.screencast_frames.clear();
        self.screencast_ack_ids.clear();
        self.screencast_dir = None;
        self.last_trace_body = None;
        self.ref_map.clear();
        self.manager.close().await.map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Browser,
                format!("Browser close failed: {e}"),
                "Process reaped by chromiumoxide finalize or Lightpanda process Drop",
            )
        })
    }
}
