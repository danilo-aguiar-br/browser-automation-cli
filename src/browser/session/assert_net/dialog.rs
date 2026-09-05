// SPDX-License-Identifier: MIT OR Apache-2.0
//! OneShotSession methods (componentized; single-responsibility impl blocks).

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};

use super::super::OneShotSession;

impl OneShotSession {
    /// Accept or dismiss a JavaScript dialog.
    ///
    /// With `if_present`, a missing dialog is success rather than an error, so
    /// a script can handle a dialog that only appears sometimes (GAP-006).
    ///
    /// # Errors
    ///
    /// Fails with [`ErrorKind::Browser`] —
    /// `"dialog failed: …"`, carrying the `dialog_open_required` suggestion —
    /// when `Page.handleJavaScriptDialog` is refused, which is what Chrome
    /// answers when no dialog is showing. The `if_present` tolerance lives in
    /// the caller, not here: this method always expects a dialog.
    ///
    /// A settle that times out is **not** an error: it is reported as
    /// `dialog_settled: false`, so the caller can tell "answered and observed
    /// closed" from "answered, close not seen".
    pub async fn dialog(
        &mut self,
        accept: bool,
        prompt_text: Option<&str>,
    ) -> Result<Value, CliError> {
        self.manager
            .handle_dialog(accept, prompt_text)
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Browser,
                    format!("dialog failed: {e}"),
                    crate::i18n::suggestion_key("dialog_open_required", None),
                )
            })?;
        // GAP-054: suppress stale Opening and wait for Closed (or budget) so the
        // next page-observing step is not refused with precondition 75.
        let settled = self.settle_after_dialog_answer().await;
        Ok(json!({
            "dialog": if accept { "accept" } else { "dismiss" },
            "prompt_text": prompt_text,
            "dialog_settled": settled,
        }))
    }
}
