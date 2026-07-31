// SPDX-License-Identifier: MIT OR Apache-2.0
//! Text and markup content queries (`textContent`, `innerText`, `innerHTML`).

use crate::native::cdp::client::CdpClient;

use super::super::refs::RefMap;
use super::call::call_on_element;

/// Visible text of the element, with a fallback for hidden subtrees.
///
/// Prefers `innerText`, which reflects what is RENDERED, and falls back to
/// `textContent` so a node that renders nothing still yields its source text.
/// A missing element yields an empty string, not an error.
pub async fn get_element_text(
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
        "function() { return this.innerText || this.textContent || ''; }".to_string(),
    )
    .await?;

    Ok(value
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default())
}

/// Strictly `innerText`: rendered text only, no `textContent` fallback.
pub async fn get_element_inner_text(
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
        "function() { return this.innerText || ''; }".to_string(),
    )
    .await?;

    Ok(value
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default())
}

/// Serialized markup inside the element (`innerHTML`).
pub async fn get_element_inner_html(
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
        "function() { return this.innerHTML || ''; }".to_string(),
    )
    .await?;

    Ok(value
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default())
}
