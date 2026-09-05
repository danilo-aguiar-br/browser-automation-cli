// SPDX-License-Identifier: MIT OR Apache-2.0
//! Interaction-state predicates (visible, enabled, checked).

use crate::native::cdp::client::CdpClient;

use super::super::refs::RefMap;
use super::call::call_on_element;

/// Whether the element is actually rendered and visible to a user.
///
/// Checks geometry AND style: a node with zero area, `visibility: hidden`,
/// `display: none` or zero opacity is not visible even though it exists in the
/// DOM. Anything that cannot be resolved counts as NOT visible.
///
/// # Errors
///
/// Propagates
/// [`resolve_element_object_id`](crate::native::element::resolve_element_object_id)
/// and the CDP error raised by `Runtime.callFunctionOn`. A call that returns
/// no boolean is not an error: it answers `false`, which is the safe reading
/// of "we could not prove this is visible".
pub async fn is_element_visible(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
) -> Result<bool, String> {
    let value = call_on_element(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
        r#"function() {
                    const rect = this.getBoundingClientRect();
                    const style = window.getComputedStyle(this);
                    return rect.width > 0 && rect.height > 0 &&
                           style.visibility !== 'hidden' &&
                           style.display !== 'none' &&
                           parseFloat(style.opacity) > 0;
                }"#
        .to_string(),
    )
    .await?;

    Ok(value.and_then(|v| v.as_bool()).unwrap_or(false))
}

/// Whether the element accepts interaction (not `disabled`).
///
/// # Errors
///
/// Propagates
/// [`resolve_element_object_id`](crate::native::element::resolve_element_object_id)
/// and the CDP error raised by `Runtime.callFunctionOn`. A call that returns
/// no boolean answers `true`: an element with no `disabled` property — a
/// `div`, say — is enabled.
pub async fn is_element_enabled(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
) -> Result<bool, String> {
    let value = call_on_element(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
        "function() { return !this.disabled; }".to_string(),
    )
    .await?;

    Ok(value.and_then(|v| v.as_bool()).unwrap_or(true))
}

/// Checked state of a checkbox or radio input.
///
/// # Errors
///
/// Propagates
/// [`resolve_element_object_id`](crate::native::element::resolve_element_object_id)
/// and the CDP error raised by `Runtime.callFunctionOn`. An element that is
/// neither a native checkbox/radio, nor ARIA-checkable, nor a label or
/// wrapper around one is not an error: every branch falls through to `false`.
pub async fn is_element_checked(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
) -> Result<bool, String> {
    // Mirrors Playwright's getChecked() with follow-label retargeting:
    // 1. If element is a native checkbox/radio input, return .checked
    // 2. If element has an ARIA checked role, return aria-checked
    // 3. Follow label → input association (label.control)
    // 4. Check for nested checkbox/radio input as last resort
    let value = call_on_element(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
        r#"function() {
                    var el = this;
                    // Native checkbox/radio input
                    var tag = el.tagName && el.tagName.toUpperCase();
                    if (tag === 'INPUT' && (el.type === 'checkbox' || el.type === 'radio')) {
                        return el.checked;
                    }
                    // ARIA role-based checked state
                    var role = el.getAttribute && el.getAttribute('role');
                    var ariaCheckedRoles = ['checkbox','radio','switch','menuitemcheckbox','menuitemradio','option','treeitem'];
                    if (role && ariaCheckedRoles.indexOf(role) !== -1) {
                        return el.getAttribute('aria-checked') === 'true';
                    }
                    // Follow label association (Playwright follow-label retarget)
                    var label = el;
                    if (tag !== 'LABEL') {
                        label = el.closest && el.closest('label');
                    }
                    if (label && label.tagName && label.tagName.toUpperCase() === 'LABEL' && label.control) {
                        var ctrl = label.control;
                        if (ctrl.type === 'checkbox' || ctrl.type === 'radio') {
                            return ctrl.checked;
                        }
                    }
                    // Check for nested native input
                    var input = el.querySelector && el.querySelector('input[type="checkbox"], input[type="radio"]');
                    if (input) return input.checked;
                    return false;
                }"#
        .to_string(),
    )
    .await?;

    Ok(value.and_then(|v| v.as_bool()).unwrap_or(false))
}
