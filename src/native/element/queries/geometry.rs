// SPDX-License-Identifier: MIT OR Apache-2.0
//! Layout geometry and computed style queries.

use serde_json::Value;

use crate::native::cdp::client::CdpClient;

use super::super::refs::RefMap;
use super::call::call_on_element;

/// Bounding rectangle of the element in CSS pixels, viewport-relative.
pub async fn get_element_bounding_box(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
) -> Result<Value, String> {
    let value = call_on_element(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
        r#"function() {
                    const r = this.getBoundingClientRect();
                    return { x: r.x, y: r.y, width: r.width, height: r.height };
                }"#
        .to_string(),
    )
    .await?;

    value.ok_or_else(|| format!("Could not get bounding box for: {selector_or_ref}"))
}

/// Computed styles of the element.
///
/// `properties` narrows the result to the named ones; `None` returns the whole
/// computed set, which is large enough to matter in an agent's token budget.
pub async fn get_element_styles(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    properties: Option<Vec<String>>,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
) -> Result<Value, String> {
    let js = match properties {
        Some(props) => {
            let props_json = serde_json::to_string(&props).unwrap_or("[]".to_string());
            format!(
                r#"function() {{
                    const s = window.getComputedStyle(this);
                    const props = {props_json};
                    const result = {{}};
                    for (const p of props) result[p] = s.getPropertyValue(p);
                    return result;
                }}"#
            )
        }
        None => r#"function() {
                    const s = window.getComputedStyle(this);
                    const result = {};
                    for (let i = 0; i < s.length; i++) {
                        const p = s[i];
                        result[p] = s.getPropertyValue(p);
                    }
                    return result;
                }"#
        .to_string(),
    };

    let value = call_on_element(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
        js,
    )
    .await?;

    Ok(value.unwrap_or(Value::Null))
}
