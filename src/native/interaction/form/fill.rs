// SPDX-License-Identifier: MIT OR Apache-2.0

use super::check::{check, uncheck};
use super::select::select_option;
use crate::native::cdp::client::CdpClient;
use crate::native::cdp::types::*;
use crate::native::element::{resolve_element_object_id, RefMap};
use serde_json::Value;

/// Replace a field's contents in one step.
///
/// # Errors
///
/// Propagates
/// [`resolve_element_object_id`],
/// then the CDP error raised by the `Runtime.callFunctionOn` that focuses the
/// element, by the second one that clears it, and by the `Input.insertText`
/// that types `value`. A failure after the clear leaves the field empty.
pub async fn fill(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    value: &str,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
) -> Result<(), String> {
    let (object_id, effective_session_id) = resolve_element_object_id(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
    )
    .await?;

    // Focus the element
    client
        .send_command_typed::<_, Value>(
            "Runtime.callFunctionOn",
            &CallFunctionOnParams {
                function_declaration: "function() { this.focus(); }".to_string(),
                object_id: Some(object_id.clone()),
                arguments: None,
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(&effective_session_id),
        )
        .await?;

    // Select all + delete to clear
    client
        .send_command_typed::<_, Value>(
            "Runtime.callFunctionOn",
            &CallFunctionOnParams {
                function_declaration: r#"function() {
                    this.select && this.select();
                    this.value = '';
                    this.dispatchEvent(new Event('input', { bubbles: true }));
                }"#
                .to_string(),
                object_id: Some(object_id),
                arguments: None,
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(&effective_session_id),
        )
        .await?;

    // Insert text (keyboard input dispatched at page level, use parent session_id)
    client
        .send_command_typed::<_, Value>(
            "Input.insertText",
            &InsertTextParams {
                text: value.to_string(),
            },
            Some(session_id),
        )
        .await?;

    Ok(())
}

/// Smart fill matching DevTools agent `fill` semantics:
/// - `<select>` → option match by value or label
/// - checkbox/radio → `"true"`/`"false"` (radio true clicks to select)
/// - otherwise → text fill via insertText
///
/// # Errors
///
/// Fails when the element cannot be resolved or the kind probe is refused,
/// then propagates the branch it chose: [`select_option`] for a `<select>`
/// (including "no option matched"), [`check`] / [`uncheck`] for a checkbox,
/// and [`fill`] for anything else.
///
/// A radio with a falsy `value` fails with `"radio cannot be set to false via
/// fill; select another radio option"`: unticking a radio is not a state HTML
/// lets you reach, so the caller must name the option to select instead.
pub async fn fill_smart(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    value: &str,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
) -> Result<(), String> {
    let kind = detect_fill_kind(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
    )
    .await?;
    match kind.as_str() {
        "select" => {
            select_option(
                client,
                session_id,
                ref_map,
                selector_or_ref,
                &[value.to_string()],
                iframe_sessions,
            )
            .await
        }
        "checkbox" => {
            let want = parse_checkbox_intent(value);
            if want {
                check(
                    client,
                    session_id,
                    ref_map,
                    selector_or_ref,
                    iframe_sessions,
                )
                .await
            } else {
                uncheck(
                    client,
                    session_id,
                    ref_map,
                    selector_or_ref,
                    iframe_sessions,
                )
                .await
            }
        }
        "radio" => {
            let want = parse_checkbox_intent(value);
            if want {
                // Radio true: force select via click path used by check()
                check(
                    client,
                    session_id,
                    ref_map,
                    selector_or_ref,
                    iframe_sessions,
                )
                .await
            } else {
                Err("radio cannot be set to false via fill; select another radio option".into())
            }
        }
        _ => {
            fill(
                client,
                session_id,
                ref_map,
                selector_or_ref,
                value,
                iframe_sessions,
            )
            .await
        }
    }
}

/// Read a caller's intent for a checkbox or radio from a form-field value.
///
/// # Why this is not the config boolean parser
///
/// It was called `parse_boolish` and shared that name with
/// `xdg::config_ops::validate::parse_boolish`, while accepting a different
/// vocabulary. Two functions with one name and two grammars in the same crate
/// is a trap for the next reader, so the form one took the specific name.
///
/// The grammars stay apart on purpose. `checked` belongs here, because it is
/// what an HTML attribute says, and belongs nowhere near a config file. And a
/// form value is caller data rather than operator configuration: an
/// unrecognised token here means "do not tick the box", which is a defensible
/// reading, whereas the same token in `config.toml` is an operator mistake that
/// must be reported.
fn parse_checkbox_intent(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "1" | "on" | "yes" | "checked"
    )
}

async fn detect_fill_kind(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
) -> Result<String, String> {
    let (object_id, effective_session_id) = resolve_element_object_id(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
    )
    .await?;
    let result = client
        .send_command_typed::<_, Value>(
            "Runtime.callFunctionOn",
            &CallFunctionOnParams {
                function_declaration: r#"function() {
                    const tag = (this.tagName || '').toUpperCase();
                    if (tag === 'SELECT') return 'select';
                    if (tag === 'INPUT') {
                        const t = (this.type || 'text').toLowerCase();
                        if (t === 'checkbox') return 'checkbox';
                        if (t === 'radio') return 'radio';
                    }
                    if (this.getAttribute && this.getAttribute('role') === 'checkbox') return 'checkbox';
                    if (this.getAttribute && this.getAttribute('role') === 'radio') return 'radio';
                    return 'text';
                }"#
                .to_string(),
                object_id: Some(object_id),
                arguments: None,
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(&effective_session_id),
        )
        .await?;
    Ok(result
        .get("result")
        .and_then(|r| r.get("value"))
        .and_then(|v| v.as_str())
        .unwrap_or("text")
        .to_string())
}
