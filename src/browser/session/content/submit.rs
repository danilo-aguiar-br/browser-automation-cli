// SPDX-License-Identifier: MIT OR Apache-2.0
//! Form submission that waits for the real outcome, never a fixed sleep (GAP-036).

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};
use crate::native::cdp::types::CallFunctionOnParams;
use crate::native::element::resolve_element_object_id;

use super::super::OneShotSession;

/// Submit the form owning `this` and report what the page did about it.
///
/// `requestSubmit()` is preferred over `submit()` because it fires the `submit`
/// event and runs constraint validation — `submit()` bypasses both, so a page
/// that validates or intercepts would never see the submission.
///
/// `defaultPrevented` is read **after** the synchronous dispatch returns, once
/// every handler has run; reading it inside a listener would race handler order.
const SUBMIT_FN: &str = r#"function() {
    const form = this.tagName === 'FORM' ? this : this.closest('form');
    if (!form) {
        return { ok: false, reason: 'no_form' };
    }
    let evt = null;
    const capture = (e) => { evt = e; };
    form.addEventListener('submit', capture);
    let via = 'submit';
    try {
        if (typeof form.requestSubmit === 'function') {
            via = 'requestSubmit';
            form.requestSubmit();
        } else {
            form.submit();
        }
    } finally {
        form.removeEventListener('submit', capture);
    }
    return {
        ok: true,
        via,
        fired: evt !== null,
        prevented: evt !== null ? evt.defaultPrevented : false,
        valid: typeof form.checkValidity === 'function' ? form.checkValidity() : true,
        action: form.action || '',
        form_method: (form.method || 'get').toLowerCase(),
        href_before: location.href,
    };
}"#;

impl OneShotSession {
    /// Submit a form (or the form owning a field) and wait for its outcome.
    ///
    /// The wait ends on the first of: a URL change (navigation), or a request
    /// started by the submission that then completes (XHR / `fetch` handler).
    /// There is no fixed sleep. A submission the page cancelled with
    /// `preventDefault()` and no network call returns immediately with
    /// `outcome: "prevented"`.
    pub async fn submit(
        &mut self,
        target: &str,
        timeout_ms: Option<u64>,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self.session_id()?;
        let (object_id, effective_session_id) = resolve_element_object_id(
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
                format!("submit failed: {e}"),
                crate::i18n::suggestion_key("target_ref_from_view", None),
            )
        })?;

        let requests_before = self.net_started;
        let url_before = self
            .manager
            .evaluate("location.href", None)
            .await
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default();

        let raw: Value = self
            .manager
            .client
            .send_command_typed(
                "Runtime.callFunctionOn",
                &CallFunctionOnParams {
                    function_declaration: SUBMIT_FN.to_string(),
                    object_id: Some(object_id),
                    arguments: None,
                    return_by_value: Some(true),
                    await_promise: Some(false),
                },
                Some(&effective_session_id),
            )
            .await
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("submit failed: {e}")))?;

        if let Some(text) = raw
            .pointer("/exceptionDetails/text")
            .and_then(Value::as_str)
        {
            return Err(CliError::new(
                ErrorKind::Browser,
                format!("submit failed in page: {text}"),
            ));
        }
        let report = raw.pointer("/result/value").cloned().unwrap_or(Value::Null);

        if report.get("ok").and_then(Value::as_bool) != Some(true) {
            return Err(CliError::with_suggestion(
                ErrorKind::Data,
                format!("submit failed: `{target}` is not a form and has no ancestor form"),
                crate::i18n::suggestion_key("submit_needs_form", None),
            ));
        }
        let fired = report
            .get("fired")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let valid = report.get("valid").and_then(Value::as_bool).unwrap_or(true);
        if !fired && !valid {
            // requestSubmit() runs constraint validation and refuses invalid forms.
            return Err(CliError::with_suggestion(
                ErrorKind::Data,
                format!("submit rejected: form containing `{target}` failed constraint validation"),
                crate::i18n::suggestion_key("submit_validation_failed", None),
            ));
        }
        let prevented = report
            .get("prevented")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let outcome = self
            .await_submit_outcome(&url_before, requests_before, timeout_ms, prevented)
            .await?;

        let url_after = self
            .manager
            .evaluate("location.href", None)
            .await
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default();

        let mut data = json!({
            "submitted": true,
            "target": target,
            "via": report.get("via").cloned().unwrap_or(Value::Null),
            "action": report.get("action").cloned().unwrap_or(Value::Null),
            "form_method": report.get("form_method").cloned().unwrap_or(Value::Null),
            "submit_event_fired": fired,
            "default_prevented": prevented,
            "outcome": outcome,
            "url_before": url_before,
            "url_after": url_after,
            "requests_started": self.net_started.saturating_sub(requests_before),
        });
        if outcome == "prevented" {
            data["warning"] = json!(
                "the page cancelled the default submission and issued no request; \
                 nothing was sent to the server"
            );
        }
        self.attach_snapshot_if(include_snapshot, data).await
    }

    /// Poll for navigation or for a submission-triggered request to finish.
    ///
    /// Returns the outcome tag; errors with `Timeout` only when a real
    /// submission was expected (not prevented) and neither happened.
    async fn await_submit_outcome(
        &mut self,
        url_before: &str,
        requests_before: u64,
        timeout_ms: Option<u64>,
        prevented: bool,
    ) -> Result<&'static str, CliError> {
        let budget = timeout_ms.unwrap_or(10_000).max(1);
        let started = std::time::Instant::now();
        let deadline = started + std::time::Duration::from_millis(budget);
        let quiet =
            std::time::Duration::from_millis(crate::constants::DEFAULT_NETWORK_IDLE_WINDOW_MS);
        let mut saw_request = false;

        loop {
            self.drain_events();

            let href = self
                .manager
                .evaluate("location.href", None)
                .await
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default();
            if !href.is_empty() && href != url_before {
                return Ok("navigation");
            }

            if self.net_started > requests_before {
                saw_request = true;
                let last = self.net_last_activity.unwrap_or(started);
                if self.net_inflight == 0 && last.elapsed() >= quiet {
                    return Ok("network");
                }
            }

            if std::time::Instant::now() >= deadline {
                // A cancelled submit that never touched the network is a real,
                // reportable outcome — not a timeout.
                if prevented && !saw_request {
                    return Ok("prevented");
                }
                return Err(CliError::with_suggestion(
                    ErrorKind::Timeout,
                    format!(
                        "submit produced neither navigation nor a completed request within {budget}ms \
                         (requests started: {})",
                        self.net_started.saturating_sub(requests_before)
                    ),
                    crate::i18n::suggestion_key("raise_timeout", None),
                ));
            }

            // A cancelled submit with no request cannot improve by waiting.
            if prevented && !saw_request && started.elapsed() >= quiet {
                return Ok("prevented");
            }

            tokio::time::sleep(std::time::Duration::from_millis(
                crate::xdg::policy::policy_u64(
                    crate::xdg::policy::key::DEFAULT_NAV_MICRO_SETTLE_MS,
                ),
            ))
            .await;
        }
    }
}
