// SPDX-License-Identifier: MIT OR Apache-2.0
//! press, write, click_at, keys, type_text

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};
use crate::native::interaction;

use super::super::OneShotSession;

impl OneShotSession {
    /// Click an element.
    pub async fn press(
        &mut self,
        target: &str,
        dblclick: bool,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self.session_id()?;

        let result = if dblclick {
            interaction::dblclick(
                &self.manager.client,
                &session_id,
                &self.ref_map,
                target,
                &self.iframe_sessions,
            )
            .await
        } else {
            interaction::click(
                &self.manager.client,
                &session_id,
                &self.ref_map,
                target,
                "left",
                1,
                &self.iframe_sessions,
            )
            .await
        }
        .map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Browser,
                format!("press failed: {e}"),
                crate::i18n::suggestion_key("target_ref_from_view", None),
            )
        })?;

        self.drain_events();
        // GAP-041: the interaction itself reports the dialog synchronously; record
        // it now so the next step's guard sees it without racing the CDP event.
        if result.dialog_opened {
            self.mark_active_page_dialog_open();
        }
        let data = json!({
            "pressed": target,
            "dblclick": dblclick,
            "dialog_opened": result.dialog_opened,
        });
        self.attach_snapshot_if(include_snapshot, data).await
    }

    /// Set a field's value directly, without synthesising key events.
    pub async fn write(
        &mut self,
        target: &str,
        value: &str,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self.session_id()?;

        interaction::fill_smart(
            &self.manager.client,
            &session_id,
            &self.ref_map,
            target,
            value,
            &self.iframe_sessions,
        )
        .await
        .map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Browser,
                format!("write failed: {e}"),
                crate::i18n::suggestion_key("target_ref_from_view", None),
            )
        })?;

        self.drain_events();
        let data = json!({
            "written": target,
            "value_len": value.len(),
            "fill_mode": "smart",
        });
        self.attach_snapshot_if(include_snapshot, data).await
    }

    /// Click at absolute page coordinates (requires experimental vision flag at CLI).
    pub async fn click_at(
        &mut self,
        x: f64,
        y: f64,
        dblclick: bool,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self.session_id()?;
        let result = interaction::click_at(&self.manager.client, &session_id, x, y, dblclick)
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Browser,
                    format!("click-at failed: {e}"),
                    crate::i18n::suggestion_key("vision_coordinates", None),
                )
            })?;
        self.drain_events();
        if result.dialog_opened {
            self.mark_active_page_dialog_open();
        }
        let data = json!({
            "clicked_at": { "x": x, "y": y },
            "dblclick": dblclick,
            "dialog_opened": result.dialog_opened,
        });
        self.attach_snapshot_if(include_snapshot, data).await
    }

    /// Press one named key, such as `Enter` or `Tab`.
    pub async fn keys(&mut self, key: &str, include_snapshot: bool) -> Result<Value, CliError> {
        let session_id = self.session_id()?;

        interaction::press_key(&self.manager.client, &session_id, key)
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Browser,
                    format!("keys failed: {e}"),
                    crate::i18n::suggestion_key("cdp_key_name", None),
                )
            })?;

        self.drain_events();
        let data = json!({ "key": key });
        self.attach_snapshot_if(include_snapshot, data).await
    }

    /// Type text character by character, firing real key events.
    ///
    /// Slower than [`write`](Self::write) and that is the point: a page with
    /// autocomplete, masking or per-keystroke validation only sees this route.
    pub async fn type_text(
        &mut self,
        target: Option<&str>,
        text: &str,
        clear: bool,
        submit: Option<&str>,
        focus_only: bool,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        let session_id = self.session_id()?;

        let typed_target = if focus_only || target.is_none() {
            // tool-ref type_text: type into currently focused element
            if clear {
                // Select-all then type (best-effort clear of focused field)
                let _ =
                    interaction::press_key(&self.manager.client, &session_id, "Control+a").await;
                let _ =
                    interaction::press_key(&self.manager.client, &session_id, "Backspace").await;
            }
            interaction::type_text_into_active_context(
                &self.manager.client,
                &session_id,
                text,
                None,
            )
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Browser,
                    format!("type (focus-only) failed: {e}"),
                    crate::i18n::suggestion_key("target_ref_from_view", None),
                )
            })?;
            target.unwrap_or("(focused)").to_string()
        } else {
            let t = match target {
                Some(s) => s,
                None => {
                    return Err(CliError::new(
                        ErrorKind::Usage,
                        "type requires --target or --focus-only",
                    ));
                }
            };
            interaction::type_text(
                &self.manager.client,
                &session_id,
                &self.ref_map,
                t,
                text,
                clear,
                None,
                &self.iframe_sessions,
            )
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Browser,
                    format!("type failed: {e}"),
                    crate::i18n::suggestion_key("target_ref_from_view", None),
                )
            })?;
            t.to_string()
        };

        if let Some(key) = submit {
            interaction::press_key(&self.manager.client, &session_id, key)
                .await
                .map_err(|e| {
                    CliError::with_suggestion(
                        ErrorKind::Browser,
                        format!("type --submit key failed: {e}"),
                        crate::i18n::suggestion_key("cdp_key_name", None),
                    )
                })?;
        }

        self.drain_events();
        let data = json!({
            "typed": typed_target,
            "text_len": text.len(),
            "cleared": clear,
            "submit": submit,
            "focus_only": focus_only || target.is_none(),
        });
        self.attach_snapshot_if(include_snapshot, data).await
    }
}
