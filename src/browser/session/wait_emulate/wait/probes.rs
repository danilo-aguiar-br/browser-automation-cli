// SPDX-License-Identifier: MIT OR Apache-2.0
//! Self-contained condition probes used by the `wait` polling loop.
//!
//! Each probe answers ONE question and holds no loop state, so the polling
//! loop in `conditions` stays readable as the OR algebra it implements.

use super::super::super::OneShotSession;

/// Cheap DOM fingerprint: element count plus serialized document length.
///
/// Reads only; nothing is injected into the page and no observer is left behind.
/// It detects any mutation that changes the serialized document — which includes
/// attribute edits, since `outerHTML` serializes attributes. A mutation that
/// rewrites the document to a byte-identical form is not detectable this way.
const DOM_SIGNATURE_JS: &str = "(function(){\
const d=document.documentElement;\
return d? d.getElementsByTagName('*').length + ':' + d.outerHTML.length : '0:0';\
})()";

impl OneShotSession {
    /// Serialized-document fingerprint, or `None` when the page cannot answer.
    ///
    /// Extracted from the polling loop so the JS payload and its caveats live
    /// next to each other; the semantics are unchanged.
    pub(super) async fn dom_signature(&mut self) -> Option<String> {
        self.manager
            .evaluate(DOM_SIGNATURE_JS, None)
            .await
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
    }
}

impl OneShotSession {
    /// True when no request has been in flight for the whole `window_ms`.
    ///
    /// `started` is the wait's own start instant: with no network event at all,
    /// the connection counts as quiet since the wait began rather than since
    /// process start.
    pub(super) fn network_is_quiet(&self, window_ms: u64, started: std::time::Instant) -> bool {
        let window = std::time::Duration::from_millis(window_ms);
        let last = self.net_last_activity.unwrap_or(started);
        self.net_inflight == 0 && last.elapsed() >= window
    }
}
