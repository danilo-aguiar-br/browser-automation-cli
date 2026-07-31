// SPDX-License-Identifier: MIT OR Apache-2.0
//! extract, attr, text, scroll

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};
use crate::native::element::{self};
use crate::native::interaction;

use super::super::OneShotSession;

impl OneShotSession {
    /// Read an element's text, or one attribute when `attr` is given.
    pub async fn extract(&mut self, target: &str, attr: Option<&str>) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self.session_id()?;

        if let Some(name) = attr {
            let v = element::get_element_attribute(
                &self.manager.client,
                &session_id,
                &self.ref_map,
                target,
                name,
                &self.iframe_sessions,
            )
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Browser,
                    format!("extract attr failed: {e}"),
                    crate::i18n::suggestion_key("target_ref_from_view", None),
                )
            })?;
            Ok(json!({ "target": target, "attr": name, "value": v }))
        } else {
            let text = element::get_element_text(
                &self.manager.client,
                &session_id,
                &self.ref_map,
                target,
                &self.iframe_sessions,
            )
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Browser,
                    format!("extract text failed: {e}"),
                    crate::i18n::suggestion_key("target_ref_from_view", None),
                )
            })?;
            Ok(json!({ "target": target, "text": text }))
        }
    }

    /// Read one attribute of an element.
    pub async fn attr(&mut self, target: &str, name: &str) -> Result<Value, CliError> {
        self.extract(target, Some(name)).await
    }

    /// PRD §7 `text`: extract visible text from a target.
    pub async fn text(&mut self, target: &str) -> Result<Value, CliError> {
        self.extract(target, None).await
    }

    /// PRD §7 `scroll`: scroll window or element by delta pixels.
    pub async fn scroll(
        &mut self,
        target: Option<&str>,
        delta_x: f64,
        delta_y: f64,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        self.scroll_ex(
            interaction::ScrollRequest {
                target,
                delta_x,
                delta_y,
                to_x: None,
                to_y: None,
            },
            include_snapshot,
        )
        .await
    }

    /// Full scroll surface: viewport or overflow container, delta or absolute (GAP-031).
    ///
    /// The envelope carries `before` / `after` offsets and the container metrics so an
    /// agent can distinguish "already at the end" from "target is not scrollable".
    pub async fn scroll_ex(
        &mut self,
        req: interaction::ScrollRequest<'_>,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self.session_id()?;
        let metrics = interaction::scroll(
            &self.manager.client,
            &session_id,
            &self.ref_map,
            req,
            &self.iframe_sessions,
        )
        .await
        .map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Browser,
                format!("scroll failed: {e}"),
                crate::i18n::suggestion_key("target_ref_from_view", None),
            )
        })?;
        let before = metrics.get("before").cloned().unwrap_or(Value::Null);
        let after = metrics.get("after").cloned().unwrap_or(Value::Null);
        let moved = before != after;
        let data = json!({
            "ok": true,
            "target": req.target,
            "delta_x": req.delta_x,
            "delta_y": req.delta_y,
            "to_x": req.to_x,
            "to_y": req.to_y,
            "before": before,
            "after": after,
            "moved": moved,
            "scroll_height": metrics.get("scrollHeight").cloned().unwrap_or(Value::Null),
            "client_height": metrics.get("clientHeight").cloned().unwrap_or(Value::Null),
            "scroll_width": metrics.get("scrollWidth").cloned().unwrap_or(Value::Null),
            "client_width": metrics.get("clientWidth").cloned().unwrap_or(Value::Null),
            "scrollable": metrics.get("scrollable").cloned().unwrap_or(Value::Null),
        });
        self.attach_snapshot_if(include_snapshot, data).await
    }
}
