// SPDX-License-Identifier: MIT OR Apache-2.0
//! Custom option picking (badge / popover / role=option).

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};
use crate::native::cdp::types::{CallArgument, CallFunctionOnParams};
use crate::native::element::resolve_element_object_id;

use super::super::super::OneShotSession;

/// Select an option on a native `<select>` and announce it (GAP-055).
///
/// Clicking an `<option>` is NOT how a browser changes a `<select>`: the click
/// lands, the value does not move, and the command reported success while the
/// page kept the old value. That false positive is worse than a failure,
/// because the script carries the stale value forward and fails somewhere else.
///
/// Event order is shared with `form::select` via `DISPATCH_INPUT_AND_CHANGE`.
///
/// Returns `{native: false}` for anything that is not a `<select>`, so the
/// caller falls through to the popover path.
fn native_select_fn() -> String {
    format!(
        r#"function(want) {{
    if (this.tagName !== 'SELECT') {{
        return {{ native: false }};
    }}
    const options = Array.from(this.options || []);
    const match =
        options.find((o) => o.value === want) ||
        options.find((o) => (o.textContent || '').trim() === want) ||
        options.find((o) => (o.textContent || '').trim().includes(want));
    if (!match) {{
        return {{
            native: true,
            ok: false,
            available: options.map((o) => (o.textContent || '').trim()),
        }};
    }}
    this.value = match.value;
    this.selectedIndex = match.index;
    {events}
    return {{
        native: true,
        ok: true,
        value: this.value,
        text: (match.textContent || '').trim(),
        index: this.selectedIndex,
    }};
}}"#,
        events = crate::native::interaction::DISPATCH_INPUT_AND_CHANGE
    )
}

impl OneShotSession {
    /// Pick a custom option (HIG badge/popover / role=option). GAP-023.
    ///
    /// A native `<select>` takes a different route entirely (GAP-055); see
    /// `native_select_fn` in this module (shared `input`+`change` events).
    ///
    /// # Errors
    ///
    /// On the native `<select>` route, fails with
    /// [`ErrorKind::Browser`] —
    /// `"select-option failed: …"` — when the target cannot be resolved or the
    /// page call is refused, and with
    /// [`ErrorKind::Data`] when `option`
    /// matches no option value or label, listing the ones that exist.
    ///
    /// On the custom-widget route, fails with
    /// [`ErrorKind::Data`] —
    /// `"pick option not found: target=… option=…"` — when no popover entry,
    /// `role=option` label or CSS match is found, after the click fallback has
    /// also been tried.
    ///
    /// Both misses are errors rather than silent successes: an agent that
    /// picked nothing would otherwise read "done" and act on a form it never
    /// changed.
    pub async fn pick_option(
        &mut self,
        target: &str,
        option: &str,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        if let Some(data) = self.pick_native_select(target, option).await? {
            return self.attach_snapshot_if(include_snapshot, data).await;
        }
        // 1) open trigger
        let _ = self.press(target, false, false).await?;
        // brief settle for popover
        let _ = self.wait_ms(150).await?;
        // 2) try role=option by accessible name, then CSS, then text match click
        let option_escaped = option.replace('\\', "\\\\").replace('\'', "\\'");
        let js = format!(
            r#"(function(){{
  const want = '{option_escaped}';
  const byRole = Array.from(document.querySelectorAll('[role="option"], [role="menuitem"], [role="listbox"] [role="option"]'));
  for (const el of byRole) {{
    const t = (el.textContent || '').trim();
    if (t === want || t.includes(want)) {{ el.click(); return {{ok:true, via:'role', text:t}}; }}
  }}
  try {{
    const css = document.querySelector(want);
    if (css) {{ css.click(); return {{ok:true, via:'css'}}; }}
  }} catch (_) {{}}
  const all = Array.from(document.querySelectorAll('button, a, li, span, div, label'));
  for (const el of all) {{
    const t = (el.textContent || '').trim();
    if (t === want || t.includes(want)) {{
      el.click();
      return {{ok:true, via:'text', text:t}};
    }}
  }}
  return {{ok:false, error:'option not found: ' + want}};
}})()"#
        );
        let result = self.eval(&js, None, None, None).await?;
        let ok = result
            .pointer("/result/ok")
            .or_else(|| result.get("ok"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        // Also inspect raw evaluate value shapes
        let ok = ok
            || result
                .get("value")
                .and_then(|v| v.get("ok"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            || result
                .pointer("/result/value/ok")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
        if !ok {
            // Try direct click on option as selector fallback
            if let Ok(data) = self.press(option, false, false).await {
                let out = json!({
                    "pick": true,
                    "target": target,
                    "option": option,
                    "via": "click_fallback",
                    "data": data,
                });
                return self.attach_snapshot_if(include_snapshot, out).await;
            }
            return Err(CliError::with_suggestion(
                ErrorKind::Data,
                format!("pick option not found: target={target} option={option}"),
                crate::i18n::suggestion_key("pick_option_target", None),
            ));
        }
        let out = json!({
            "pick": true,
            "target": target,
            "option": option,
            "result": result,
        });
        self.attach_snapshot_if(include_snapshot, out).await
    }

    /// Native `<select>` route (GAP-055), or `None` when the target is not one.
    ///
    /// `None` means "not my case, carry on"; it is not a failure. An error is
    /// returned only when the element IS a `<select>` and the option is absent,
    /// because succeeding there would be the false positive this route removes.
    async fn pick_native_select(
        &mut self,
        target: &str,
        option: &str,
    ) -> Result<Option<Value>, CliError> {
        let session_id = self.session_id()?;
        let Ok((object_id, effective_session_id)) = resolve_element_object_id(
            &self.manager.client,
            &session_id,
            &self.ref_map,
            target,
            &self.iframe_sessions,
        )
        .await
        else {
            // Unresolvable target is not this route's problem to report: the
            // popover path raises it with its own message and suggestion.
            return Ok(None);
        };

        let raw: Value = self
            .manager
            .client
            .send_command_typed(
                "Runtime.callFunctionOn",
                &CallFunctionOnParams {
                    function_declaration: native_select_fn(),
                    object_id: Some(object_id),
                    arguments: Some(vec![CallArgument {
                        value: Some(json!(option)),
                        object_id: None,
                    }]),
                    return_by_value: Some(true),
                    await_promise: Some(false),
                },
                Some(&effective_session_id),
            )
            .await
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("select-option failed: {e}")))?;

        let report = raw.pointer("/result/value").cloned().unwrap_or(Value::Null);
        if report.get("native").and_then(Value::as_bool) != Some(true) {
            return Ok(None);
        }
        if report.get("ok").and_then(Value::as_bool) != Some(true) {
            let available = report
                .get("available")
                .map(|v| v.to_string())
                .unwrap_or_default();
            return Err(CliError::with_suggestion(
                ErrorKind::Data,
                format!("select-option failed: `{option}` is not an option of `{target}`"),
                format!("Available options: {available}"),
            ));
        }
        Ok(Some(json!({
            "pick": true,
            "target": target,
            "option": option,
            "via": "native_select",
            "value": report.get("value").cloned().unwrap_or(Value::Null),
            "text": report.get("text").cloned().unwrap_or(Value::Null),
            "selected_index": report.get("index").cloned().unwrap_or(Value::Null),
            "events_dispatched": ["input", "change"],
        })))
    }
}
