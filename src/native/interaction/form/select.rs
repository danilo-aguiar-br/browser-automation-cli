// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::native::cdp::client::CdpClient;
use crate::native::cdp::types::*;
use crate::native::element::{resolve_element_object_id, RefMap};
use serde_json::Value;

/// Choose an option in a native `<select>` (GAP-055).
///
/// Dispatches BOTH `input` and `change`: a reactive form listens for `input`,
/// a plain handler listens for `change`, and sending only one leaves half the
/// pages unresponsive.
///
/// # Errors
///
/// Propagates
/// [`resolve_element_object_id`]
/// and the CDP error raised by `Runtime.callFunctionOn` — which is what an
/// element with no `options` collection raises.
///
/// Fails with `"No option matched […]. Available options: …"` when no entry in
/// `values` matches an option value or trimmed label. That case is an error on
/// purpose: a silent success would let an agent act on a form it never
/// changed, so the message lists every option the `<select>` offered.
pub async fn select_option(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    values: &[String],
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

    // Matching nothing must be an error, not a silent success: an agent that
    // selects a misspelled option otherwise sees "Done", and only discovers
    // the page state is wrong after more commands. List what was available.
    // Event order is shared with pick (DISPATCH_INPUT_AND_CHANGE / GAP-055).
    let js = format!(
        r#"function(vals) {{
            const options = Array.from(this.options);
            let matched = 0;
            for (const opt of options) {{
                opt.selected = vals.includes(opt.value) || vals.includes(opt.textContent.trim());
                if (opt.selected) matched += 1;
            }}
            if (matched === 0) {{
                const available = options.map(o => o.value + ' ("' + o.textContent.trim() + '")').join(', ');
                return {{ error: 'No option matched ' + JSON.stringify(vals) + '. Available options: ' + available }};
            }}
            {events}
            return {{ matched }};
        }}"#,
        events = super::DISPATCH_INPUT_AND_CHANGE
    );

    let result = client
        .send_command_typed::<_, Value>(
            "Runtime.callFunctionOn",
            &CallFunctionOnParams {
                function_declaration: js,
                object_id: Some(object_id),
                arguments: Some(vec![CallArgument {
                    value: Some(serde_json::json!(values)),
                    object_id: None,
                }]),
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(&effective_session_id),
        )
        .await?;

    if let Some(error) = result
        .get("result")
        .and_then(|r| r.get("value"))
        .and_then(|v| v.get("error"))
        .and_then(|e| e.as_str())
    {
        return Err(error.to_string());
    }

    Ok(())
}
