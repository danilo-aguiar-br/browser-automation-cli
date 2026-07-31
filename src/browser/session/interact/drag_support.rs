// SPDX-License-Identifier: MIT OR Apache-2.0
//! Drag support: drop-point resolution, CDP intercept wait, synthetic fallback.
//!
//! Helpers are `pub(super)` so the sibling `pointer` module can drive them; they
//! are not part of the public session surface.

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};
use crate::native::interaction::DragRequest;

use super::super::OneShotSession;

impl OneShotSession {
    /// Drop point: explicit coordinates win, else the destination rect anchored
    /// per `anchor` (edge anchors disambiguate ordered-list insertion).
    pub(super) async fn resolve_drop_point(
        &mut self,
        req: &DragRequest,
        session_id: &str,
    ) -> Result<((f64, f64), Option<String>), CliError> {
        use crate::native::interaction as ix;

        if let (Some(x), Some(y)) = (req.to_x, req.to_y) {
            return Ok(((x, y), None));
        }
        let Some(to) = req.to.as_deref() else {
            return Err(CliError::with_suggestion(
                ErrorKind::Usage,
                "drag needs a destination: --to <target> or both --to-x and --to-y",
                crate::i18n::suggestion_key("drag_destination_required", None),
            ));
        };
        let (rect, sid) = ix::element_rect(
            &self.manager.client,
            session_id,
            &self.ref_map,
            to,
            &self.iframe_sessions,
        )
        .await
        .map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Browser,
                format!("drag failed: {e}"),
                crate::i18n::suggestion_key("target_ref_from_view", None),
            )
        })?;
        // A single explicit axis overrides the anchored one.
        let (ax, ay) = rect.anchor_point(req.anchor);
        Ok(((req.to_x.unwrap_or(ax), req.to_y.unwrap_or(ay)), Some(sid)))
    }

    /// Poll the event bus for `Input.dragIntercepted` within the budget.
    pub(super) async fn await_drag_intercepted(&mut self) -> Option<Value> {
        let deadline = std::time::Instant::now()
            + std::time::Duration::from_millis(crate::constants::DRAG_INTERCEPT_BUDGET_MS);
        loop {
            self.drain_events();
            if let Some(params) = self.drag_intercepted.take() {
                return Some(params);
            }
            if std::time::Instant::now() >= deadline {
                return None;
            }
            tokio::time::sleep(std::time::Duration::from_millis(
                crate::xdg::resolve_event_pump_slice_ms(),
            ))
            .await;
        }
    }

    pub(super) async fn synthetic_mouse_drag(
        &mut self,
        req: &DragRequest,
        from: (f64, f64),
        to: (f64, f64),
        session_id: &str,
        warning: String,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        use crate::native::interaction::{self as ix, DragRoute};

        let client = std::sync::Arc::clone(&self.manager.client);
        ix::start_drag_gesture(&client, session_id, from, to)
            .await
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("drag failed: {e}")))?;
        ix::release_drag_gesture(&client, session_id, to)
            .await
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("drag failed: {e}")))?;
        self.settle_after_interaction().await;
        self.finish_drag(
            req,
            from,
            to,
            DragRoute::SyntheticMouse,
            None,
            Some(warning),
            include_snapshot,
        )
        .await
    }

    pub(super) async fn settle_after_interaction(&mut self) {
        tokio::time::sleep(std::time::Duration::from_millis(
            crate::xdg::resolve_interact_settle_ms(),
        ))
        .await;
        self.drain_events();
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn finish_drag(
        &mut self,
        req: &DragRequest,
        from: (f64, f64),
        to: (f64, f64),
        route: crate::native::interaction::DragRoute,
        data: Option<Value>,
        warning: Option<String>,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        self.drain_events();
        let mut out = json!({
            "dragged_from": req.from,
            "dragged_to": req.to,
            "from_point": { "x": from.0, "y": from.1 },
            "to_point": { "x": to.0, "y": to.1 },
            "anchor": req.anchor.as_str(),
            "route": route.as_str(),
            "exercised_page_dragstart": route
                == crate::native::interaction::DragRoute::Intercepted,
        });
        if let Some(d) = data {
            out["data_transfer"] = d;
        }
        if let Some(w) = warning {
            out["warning"] = json!(w);
        }
        self.attach_snapshot_if(include_snapshot, out).await
    }
}
