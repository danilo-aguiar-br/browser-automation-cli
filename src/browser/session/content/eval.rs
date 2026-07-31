// SPDX-License-Identifier: MIT OR Apache-2.0
//! eval, eval_service_worker, wait_ms

use std::path::Path;

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};

use super::super::OneShotSession;
use crate::browser::helpers::normalize_eval_expression;

impl OneShotSession {
    /// Evaluate JavaScript inside an extension service worker.
    pub async fn eval_service_worker(
        &mut self,
        service_worker_id: &str,
        expression: &str,
    ) -> Result<Value, CliError> {
        self.pump_events().await;
        let listed = self.extension_list().await?;
        let targets = listed
            .get("extensions")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let match_t = targets.iter().find(|t| {
            t.get("id")
                .and_then(|v| v.as_str())
                .map(|s| s == service_worker_id || s.starts_with(service_worker_id))
                .unwrap_or(false)
                && t.get("type").and_then(|v| v.as_str()) == Some("service_worker")
        });
        let Some(t) = match_t else {
            return Err(CliError::with_suggestion(
                ErrorKind::NoInput,
                format!("service_worker not found for id: {service_worker_id}"),
                crate::i18n::suggestion_key("extension_list_first", None),
            ));
        };
        let target_id = t
            .get("targetId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CliError::new(ErrorKind::Browser, "missing targetId"))?
            .to_string();
        let attach = self
            .manager
            .client
            .send_command(
                "Target.attachToTarget",
                Some(json!({ "targetId": target_id, "flatten": true })),
                None,
            )
            .await
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("attach SW: {e}")))?;
        let session_id = attach
            .get("sessionId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let result = self
            .manager
            .client
            .send_command(
                "Runtime.evaluate",
                Some(json!({
                    "expression": expression,
                    "returnByValue": true,
                    "awaitPromise": true,
                })),
                session_id.as_deref(),
            )
            .await
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("SW evaluate: {e}")))?;
        Ok(json!({
            "result": result.get("result").cloned().unwrap_or(result),
            "service_worker_id": service_worker_id,
            "targetId": target_id,
        }))
    }

    /// Evaluate JavaScript in the page and return the value.
    ///
    /// Structured returns come back as structure, not as text to parse twice
    /// (GAP-035); `typed` additionally reports the JS type.
    pub async fn eval(
        &mut self,
        expression: &str,
        args_json: Option<&str>,
        dialog_action: Option<&str>,
        file_path: Option<&Path>,
    ) -> Result<Value, CliError> {
        self.eval_ex(expression, args_json, dialog_action, file_path, false)
            .await
    }

    /// `eval` with the page-reported type of the returned value (GAP-035).
    ///
    /// With `typed = false` the envelope keeps the legacy shape `{ "result": … }`.
    /// With `typed = true` it carries the deserialized structure under `value`
    /// plus `value_type` / `subtype` / `class_name` exactly as the page's
    /// `RemoteObject` reported them, so a caller can tell `null` apart from
    /// `undefined`, and an array apart from a plain object, without re-probing.
    pub async fn eval_ex(
        &mut self,
        expression: &str,
        args_json: Option<&str>,
        dialog_action: Option<&str>,
        file_path: Option<&Path>,
        typed: bool,
    ) -> Result<Value, CliError> {
        use crate::native::cdp::types::{EvaluateParams, EvaluateResult};

        // dialogAction: accept | dismiss | prompt text (default accept)
        let action = dialog_action.unwrap_or("accept");
        let _ = action; // applied via auto-accept path below; dismiss handled when needed
        let session_id = self.session_id()?;

        // Build expression: call bare functions once; never re-invoke IIFEs.
        // Bug: wrapping `(() => 1)()` as `((() => 1)())()` yields "is not a function".
        let expr = normalize_eval_expression(expression, args_json)?;

        let client = std::sync::Arc::clone(&self.manager.client);
        let expr_owned = expr.clone();
        let mut eval_fut = Box::pin(async move {
            let result: EvaluateResult = client
                .send_command_typed(
                    "Runtime.evaluate",
                    &EvaluateParams {
                        expression: expr_owned,
                        return_by_value: Some(true),
                        await_promise: Some(true),
                    },
                    Some(&session_id),
                )
                .await?;
            if let Some(ref details) = result.exception_details {
                let msg = details
                    .exception
                    .as_ref()
                    .and_then(|e| e.description.as_deref())
                    .unwrap_or(&details.text);
                return Err(format!("Evaluation error: {msg}"));
            }
            // Keep the whole RemoteObject: typed mode needs `type` / `subtype`,
            // which is the only way to tell `undefined` from `null` after
            // return-by-value collapses both to JSON null.
            Ok(result.result)
        });
        let remote = loop {
            tokio::select! {
                res = &mut eval_fut => {
                    break res.map_err(|e| {
                        CliError::with_suggestion(
                            ErrorKind::Browser,
                            format!("eval failed: {e}"),
                            "Check the JS expression; use return-by-value expressions",
                        )
                    })?;
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(crate::xdg::resolve_eval_drain_slice_ms())) => {
                    self.drain_events();
                    if self.dialog_open_on_active_page() {
                        let accept = !action.eq_ignore_ascii_case("dismiss");
                        let prompt = if action.eq_ignore_ascii_case("accept")
                            || action.eq_ignore_ascii_case("dismiss")
                        {
                            None
                        } else {
                            Some(action)
                        };
                        let _ = self.manager.handle_dialog(accept, prompt).await;
                        let _ = self.settle_after_dialog_answer().await;
                    }
                }
            }
        };
        // Allow consoleAPICalled events to land after evaluate.
        tokio::time::sleep(std::time::Duration::from_millis(
            crate::xdg::resolve_event_pump_slice_ms(),
        ))
        .await;
        self.drain_events();
        let v = remote.value.clone().unwrap_or(Value::Null);
        let payload = if typed {
            json!({
                "value": v,
                "value_type": remote.object_type,
                "subtype": remote.subtype,
                "class_name": remote.class_name,
                "description": remote.description,
                "typed": true,
            })
        } else {
            json!({ "result": v })
        };
        let data = self.with_capture_fields(payload);
        if let Some(path) = file_path {
            let body = serde_json::to_vec_pretty(&data).map_err(|e| {
                CliError::new(ErrorKind::Io, format!("eval serialize for file: {e}"))
            })?;
            crate::concurrency::write_bytes_blocking(path.to_path_buf(), body)
                .await
                .map_err(|e| {
                    CliError::new(ErrorKind::Io, format!("eval write {}: {e}", path.display()))
                })?;
        }
        Ok(data)
    }

    /// Sleep for `ms`, as an explicit step in a script.
    ///
    /// A fixed sleep is the last resort: prefer a `wait` on a condition, which
    /// returns as soon as it holds instead of always paying the full delay.
    pub async fn wait_ms(&mut self, ms: u64) -> Result<Value, CliError> {
        // Pump in slices so screencast FrameAck keeps frames flowing during waits.
        let mut remaining = ms;
        while remaining > 0 {
            let slice = remaining.min(crate::xdg::resolve_event_pump_slice_ms());
            tokio::time::sleep(std::time::Duration::from_millis(slice)).await;
            remaining = remaining.saturating_sub(slice);
            if self.screencast_active {
                self.pump_events().await;
            } else {
                self.drain_events();
            }
        }
        Ok(json!({ "waited_ms": ms }))
    }
}
