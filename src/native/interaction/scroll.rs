// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::native::cdp::client::CdpClient;
use crate::native::cdp::types::*;
use crate::native::element::{resolve_element_object_id, RefMap};
use serde_json::Value;

/// One scroll request against the viewport or a scrollable container (GAP-031).
///
/// `to_x` / `to_y` request an absolute scroll offset. When either is set the
/// absolute path wins for that axis and the matching delta is ignored; an unset
/// axis keeps its current offset.
#[derive(Debug, Clone, Copy, Default)]
pub struct ScrollRequest<'a> {
    /// CSS selector or `@eN` ref of a scrollable container; `None` = viewport.
    pub target: Option<&'a str>,
    /// Horizontal scroll amount in CSS pixels. Ignored when `to_x` is set.
    pub delta_x: f64,
    /// Vertical scroll amount in CSS pixels. Ignored when `to_y` is set.
    pub delta_y: f64,
    /// Absolute horizontal offset to scroll to.
    pub to_x: Option<f64>,
    /// Absolute vertical offset to scroll to.
    pub to_y: Option<f64>,
}

/// JS body shared by the container and viewport paths.
///
/// `this` is the scrolling element in both cases, so the same source reports
/// `scrollHeight` / `clientHeight` for an overflow container and for the
/// document scrolling element. No stylesheet or attribute is injected.
const SCROLL_FN: &str = r#"function(dx, dy, tx, ty) {
    const before = { x: this.scrollLeft, y: this.scrollTop };
    if (tx !== null || ty !== null) {
        this.scrollTo({
            left: tx !== null ? tx : this.scrollLeft,
            top: ty !== null ? ty : this.scrollTop,
            behavior: 'instant',
        });
    } else {
        this.scrollBy({ left: dx, top: dy, behavior: 'instant' });
    }
    return {
        before,
        after: { x: this.scrollLeft, y: this.scrollTop },
        scrollHeight: this.scrollHeight,
        clientHeight: this.clientHeight,
        scrollWidth: this.scrollWidth,
        clientWidth: this.clientWidth,
        scrollable: this.scrollHeight > this.clientHeight
            || this.scrollWidth > this.clientWidth,
    };
}"#;

pub(super) fn scroll_arguments(req: &ScrollRequest<'_>) -> Vec<CallArgument> {
    [
        serde_json::json!(req.delta_x),
        serde_json::json!(req.delta_y),
        req.to_x.map_or(Value::Null, |v| serde_json::json!(v)),
        req.to_y.map_or(Value::Null, |v| serde_json::json!(v)),
    ]
    .into_iter()
    .map(|value| CallArgument {
        value: Some(value),
        object_id: None,
    })
    .collect()
}

/// Scroll the viewport or a container that owns its own scrollbar (GAP-031).
///
/// Returns the position before and after plus the container metrics, so a caller
/// can tell "scrolled to the end" apart from "container is not scrollable".
pub async fn scroll(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    req: ScrollRequest<'_>,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
) -> Result<Value, String> {
    let (object_id, effective_session_id) = match req.target {
        Some(sel) => {
            let (object_id, sid) =
                resolve_element_object_id(client, session_id, ref_map, sel, iframe_sessions)
                    .await?;
            (object_id, sid)
        }
        None => {
            // Resolve the document scrolling element as a RemoteObject so the
            // viewport path runs the exact same function body as the container path.
            let resolved: Value = client
                .send_command_typed(
                    "Runtime.evaluate",
                    &EvaluateParams {
                        expression: "document.scrollingElement || document.documentElement"
                            .to_string(),
                        return_by_value: Some(false),
                        await_promise: Some(false),
                    },
                    Some(session_id),
                )
                .await?;
            let object_id = resolved
                .pointer("/result/objectId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "scroll: no document scrolling element".to_string())?
                .to_string();
            (object_id, session_id.to_string())
        }
    };

    let result: Value = client
        .send_command_typed(
            "Runtime.callFunctionOn",
            &CallFunctionOnParams {
                function_declaration: SCROLL_FN.to_string(),
                object_id: Some(object_id),
                arguments: Some(scroll_arguments(&req)),
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(&effective_session_id),
        )
        .await?;

    if let Some(text) = result
        .pointer("/exceptionDetails/text")
        .and_then(|v| v.as_str())
    {
        return Err(format!("scroll failed in page: {text}"));
    }
    result
        .pointer("/result/value")
        .cloned()
        .ok_or_else(|| "scroll returned no metrics".to_string())
}

/// Select every element matching a selector, for a bulk read.
pub async fn select_all(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
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

    client
        .send_command_typed::<_, Value>(
            "Runtime.callFunctionOn",
            &CallFunctionOnParams {
                function_declaration: r#"function() {
                    this.focus();
                    if (typeof this.select === 'function') {
                        this.select();
                    } else {
                        const range = document.createRange();
                        range.selectNodeContents(this);
                        const sel = window.getSelection();
                        sel.removeAllRanges();
                        sel.addRange(range);
                    }
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

    Ok(())
}

/// Scroll an element into the viewport before interacting with it.
pub async fn scroll_into_view(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
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

    client
        .send_command_typed::<_, Value>(
            "Runtime.callFunctionOn",
            &CallFunctionOnParams {
                function_declaration:
                    "function() { this.scrollIntoView({ block: 'center', inline: 'center' }); }"
                        .to_string(),
                object_id: Some(object_id),
                arguments: None,
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(&effective_session_id),
        )
        .await?;

    Ok(())
}

/// Draw a temporary overlay on an element, for headed debugging.
pub async fn highlight(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
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

    client
        .send_command_typed::<_, Value>(
            "Runtime.callFunctionOn",
            &CallFunctionOnParams {
                function_declaration: r#"function() {
                    this.style.outline = '2px solid red';
                    this.style.outlineOffset = '2px';
                    const el = this;
                    setTimeout(() => {
                        el.style.outline = '';
                        el.style.outlineOffset = '';
                    }, 3000);
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

    Ok(())
}
