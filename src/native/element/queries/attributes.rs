// SPDX-License-Identifier: MIT OR Apache-2.0
//! Attribute / property reads and form value writes.

use serde_json::Value;

use crate::native::cdp::client::CdpClient;

use super::super::refs::RefMap;
use super::call::call_on_element;

/// HTML attribute of the element, falling back to the DOM property.
///
/// The two are not the same: `value` on an input is a property that changes as
/// the user types, while the `value` ATTRIBUTE keeps the initial markup. Asking
/// for the attribute alone would report stale data for exactly the fields an
/// agent most often reads, so the property is consulted when the attribute is
/// absent. Returns JSON `null` when neither exists.
///
/// # Errors
///
/// Propagates
/// [`resolve_element_object_id`](crate::native::element::resolve_element_object_id)
/// — unknown `@eN` ref, unresolvable durable locator, or a selector matching
/// nothing — and the CDP error raised by `Runtime.callFunctionOn`. Neither a
/// missing attribute nor a missing property is an error: both yield JSON
/// `null`.
pub async fn get_element_attribute(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    attribute: &str,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
) -> Result<Value, String> {
    let attr_name = serde_json::to_string(attribute).unwrap_or_else(|_| "\"\"".into());
    // Prefer HTML attribute; fall back to DOM property (innerText, value, checked, …).
    let value = call_on_element(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
        format!(
            "function() {{ \
               var n = {attr_name}; \
               var a = this.getAttribute ? this.getAttribute(n) : null; \
               if (a !== null && a !== undefined) return a; \
               try {{ var p = this[n]; if (p !== undefined && p !== null) return p; }} catch (e) {{}} \
               return null; \
             }}"
        ),
    )
    .await?;

    Ok(value.unwrap_or(Value::Null))
}

/// Current `value` property of a form control, as a string.
///
/// Non-string values (a `number` input's `valueAsNumber`, for instance) yield
/// an empty string rather than a coerced one.
///
/// # Errors
///
/// Propagates
/// [`resolve_element_object_id`](crate::native::element::resolve_element_object_id)
/// and the CDP error raised by `Runtime.callFunctionOn`. An element with no
/// `value` property — anything that is not a form control — is not an error;
/// it yields the empty string.
pub async fn get_element_input_value(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
) -> Result<String, String> {
    let value = call_on_element(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
        "function() { return typeof this.value === 'string' ? this.value : ''; }".to_string(),
    )
    .await?;

    Ok(value
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default())
}

/// Assign `value` and fire `input` + `change` so the page reacts.
///
/// Setting the property alone is invisible to a framework listening for events,
/// which is why both are dispatched with `bubbles: true`. This is the
/// programmatic path; `type` synthesises real key events instead.
///
/// # Errors
///
/// Propagates
/// [`resolve_element_object_id`](crate::native::element::resolve_element_object_id)
/// and the CDP error raised by `Runtime.callFunctionOn` — which is also where
/// a throwing setter or a read-only `value` surfaces. Assigning to an element
/// that has no `value` property succeeds silently: the JS creates the property
/// and the page ignores it.
pub async fn set_element_value(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    value: &str,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
) -> Result<(), String> {
    let js = format!(
        "function() {{ this.value = {}; this.dispatchEvent(new Event('input', {{bubbles: true}})); this.dispatchEvent(new Event('change', {{bubbles: true}})); }}",
        serde_json::to_string(value).unwrap_or_default()
    );

    call_on_element(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
        js,
    )
    .await?;

    Ok(())
}
