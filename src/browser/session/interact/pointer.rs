// SPDX-License-Identifier: MIT OR Apache-2.0
//! Pointer interaction: hover and HTML5 drag-and-drop.

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};
use crate::native::interaction;
use crate::native::interaction::DragRequest;

use super::super::OneShotSession;

impl OneShotSession {
    /// Hover the pointer over a target (ref, selector, or role).
    pub async fn hover(&mut self, target: &str, include_snapshot: bool) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        interaction::hover(
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
                format!("hover failed: {e}"),
                crate::i18n::suggestion_key("target_ref_from_view", None),
            )
        })?;
        self.drain_events();
        let data = json!({ "hovered": target });
        self.attach_snapshot_if(include_snapshot, data).await
    }

    /// Drag from one target to another (HTML5 intercept when available).
    pub async fn drag(
        &mut self,
        from: &str,
        to: &str,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        self.drag_ex(
            DragRequest {
                from: from.to_string(),
                to: Some(to.to_string()),
                ..DragRequest::default()
            },
            include_snapshot,
        )
        .await
    }

    /// HTML5 drag-and-drop that exercises the page's own `dragstart` (GAP-030).
    ///
    /// Route selection, in order:
    ///
    /// 1. `synthetic_payload` set — the caller explicitly asked to inject a
    ///    `DataTransfer`, so the page's `dragstart` is bypassed on purpose.
    /// 2. `Input.setInterceptDrags(true)` plus a real mouse gesture. The page
    ///    fires its own `dragstart`; the resulting `Input.dragIntercepted`
    ///    payload is what gets dropped on the destination.
    /// 3. The browser never emitted `dragIntercepted` — degrade to a plain mouse
    ///    gesture and say so in `warning`. That route does **not** prove the
    ///    page's drop handler ran.
    ///
    /// The chosen route is always reported in `route`.
    /// Extended drag with explicit coordinates and synthetic payload options.
    pub async fn drag_ex(
        &mut self,
        req: DragRequest,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        use crate::native::interaction::{self as ix, DragRoute};

        // Pure-argv validation first: a missing destination is a usage error and
        // must not be reported as "source element not found" by the resolver.
        if req.to.is_none() && !(req.to_x.is_some() && req.to_y.is_some()) {
            return Err(CliError::with_suggestion(
                ErrorKind::Usage,
                "drag needs a destination: --to <target> or both --to-x and --to-y",
                crate::i18n::suggestion_key("drag_destination_required", None),
            ));
        }

        self.drain_events();
        self.drag_intercepted = None;

        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        let client = std::sync::Arc::clone(&self.manager.client);

        let map_err = |e: String| {
            CliError::with_suggestion(
                ErrorKind::Browser,
                format!("drag failed: {e}"),
                crate::i18n::suggestion_key("target_ref_from_view", None),
            )
        };

        let (from_point, from_sid) = ix::source_point(
            &client,
            &session_id,
            &self.ref_map,
            &req.from,
            &self.iframe_sessions,
        )
        .await
        .map_err(map_err)?;

        let (to_point, to_sid) = self.resolve_drop_point(&req, &session_id).await?;
        if let Some(sid) = to_sid.as_deref() {
            if sid != from_sid {
                return Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    "drag endpoints must share the same frame/session",
                    crate::i18n::suggestion_key("drag_same_frame", None),
                ));
            }
        }
        let sid = from_sid;

        // Route 1: caller-owned payload, page dragstart deliberately bypassed.
        if let Some(payload) = req.synthetic_payload.as_ref() {
            let data = ix::validate_synthetic_payload(payload).map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Usage,
                    format!("invalid synthetic drag payload: {e}"),
                    r#"Pass CDP DragData: {"items":[{"mimeType":"text/plain","data":"x"}]}"#,
                )
            })?;
            ix::complete_drop(&client, &sid, to_point, &data)
                .await
                .map_err(map_err)?;
            return self
                .finish_drag(
                    &req,
                    from_point,
                    to_point,
                    DragRoute::SyntheticPayload,
                    Some(data),
                    Some(
                        "synthetic payload injected by request; the page's own dragstart handler was not exercised"
                            .to_string(),
                    ),
                    include_snapshot,
                )
                .await;
        }

        // Route 2: intercept the page's real dragstart.
        let intercept_armed = ix::set_intercept_drags(&client, &sid, true).await.is_ok();
        if !intercept_armed {
            let warning = "Input.setInterceptDrags unavailable on this browser; \
                 fell back to a synthetic mouse gesture that does not prove the page's drag handler ran"
                .to_string();
            return self
                .synthetic_mouse_drag(&req, from_point, to_point, &sid, warning, include_snapshot)
                .await;
        }

        let gesture = ix::start_drag_gesture(&client, &sid, from_point, to_point).await;
        let intercepted = match gesture {
            Ok(()) => self.await_drag_intercepted().await,
            Err(e) => {
                let _ = ix::set_intercept_drags(&client, &sid, false).await;
                return Err(map_err(e));
            }
        };

        match intercepted {
            Some(params) => {
                let data = ix::normalize_drag_data(&params).map_err(|e| {
                    CliError::new(
                        ErrorKind::Browser,
                        format!("drag failed: intercepted payload unusable: {e}"),
                    )
                })?;
                let drop_res = ix::complete_drop(&client, &sid, to_point, &data).await;
                let _ = ix::release_drag_gesture(&client, &sid, to_point).await;
                let _ = ix::set_intercept_drags(&client, &sid, false).await;
                drop_res.map_err(map_err)?;
                self.settle_after_interaction().await;
                self.finish_drag(
                    &req,
                    from_point,
                    to_point,
                    DragRoute::Intercepted,
                    Some(data),
                    None,
                    include_snapshot,
                )
                .await
            }
            None => {
                // Undo the half-open gesture before retrying without interception.
                let _ = ix::release_drag_gesture(&client, &sid, from_point).await;
                let _ = ix::set_intercept_drags(&client, &sid, false).await;
                let warning = format!(
                    "no Input.dragIntercepted within {}ms; fell back to a synthetic mouse gesture \
                     that does not prove the page's drag handler ran",
                    crate::constants::DRAG_INTERCEPT_BUDGET_MS
                );
                self.synthetic_mouse_drag(
                    &req,
                    from_point,
                    to_point,
                    &sid,
                    warning,
                    include_snapshot,
                )
                .await
            }
        }
    }
}
