// SPDX-License-Identifier: MIT OR Apache-2.0
//! CSS/selector JS builders for CDP Runtime.evaluate.

use crate::native::cdp::types::*;

/// Strip Playwright-style `css=` so querySelector receives a valid selector.
pub(super) fn normalize_css_selector(selector: &str) -> &str {
    selector
        .strip_prefix("css=")
        .or_else(|| selector.strip_prefix("Css="))
        .unwrap_or(selector)
}

/// Build a JS expression that finds a DOM element by CSS selector or XPath.
pub(super) fn build_find_element_js(selector: &str) -> String {
    build_find_element_js_in("document", selector)
}

/// Same as build_find_element_js but rooted at an arbitrary Document
/// expression (e.g. an iframe's contentDocument).
pub(super) fn build_find_element_js_in(root: &str, selector: &str) -> String {
    if let Some(xpath) = selector.strip_prefix("xpath=") {
        format!(
            "{root}.evaluate({xpath}, {root}, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null).singleNodeValue",
            xpath = serde_json::to_string(xpath).unwrap_or_default(),
        )
    } else {
        let css = normalize_css_selector(selector);
        format!(
            "{root}.querySelector({selector})",
            selector = serde_json::to_string(css).unwrap_or_default(),
        )
    }
}

/// Build a JS expression that counts matching DOM elements by CSS selector or XPath.
pub(super) fn build_count_elements_js(selector: &str) -> String {
    if let Some(xpath) = selector.strip_prefix("xpath=") {
        format!(
            "document.evaluate({}, document, null, XPathResult.ORDERED_NODE_SNAPSHOT_TYPE, null).snapshotLength",
            serde_json::to_string(xpath).unwrap_or_default()
        )
    } else {
        let css = normalize_css_selector(selector);
        format!(
            "document.querySelectorAll({}).length",
            serde_json::to_string(css).unwrap_or_default()
        )
    }
}

/// Require a DOM objectId from Runtime.evaluate; treat thrown selectors as not found.
pub(super) fn object_id_from_evaluate(
    result: EvaluateResult,
    selector: &str,
) -> Result<String, String> {
    if let Some(exc) = result.exception_details {
        let detail = exc
            .exception
            .as_ref()
            .and_then(|e| e.description.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or(exc.text);
        return Err(format!("Element not found: {selector} ({detail})"));
    }
    result
        .result
        .object_id
        .ok_or_else(|| format!("Element not found: {selector}"))
}

/// JS function source for `blockerAt(doc, el, x, y)`: returns a short
/// description of the element that would actually receive a click at (x, y)
/// when that element is unrelated to `el`, or null when the click would land
/// on `el` (or something that activates it). Relations that count as "lands
/// on el": shadow-including ancestors/descendants in either direction, and
/// label/control association (custom checkboxes hide the input under a styled
/// sibling inside the same label).
pub(super) const BLOCKER_AT_JS: &str = r#"(doc, el, x, y) => {
    // Descend from the given document through same-origin iframes so a point
    // over a frame resolves to the element inside it, in that frame's space.
    let d = doc, lx = x, ly = y;
    let hit = d.elementFromPoint(lx, ly);
    while (hit && (hit.tagName === 'IFRAME' || hit.tagName === 'FRAME') && hit.contentDocument && hit !== el) {
        const r = hit.getBoundingClientRect();
        lx -= r.x + hit.clientLeft;
        ly -= r.y + hit.clientTop;
        d = hit.contentDocument;
        hit = d.elementFromPoint(lx, ly);
    }
    if (!hit || hit === el) return null;
    const up = (n) => n.parentNode || n.host || (n.getRootNode && n.getRootNode().host) || null;
    for (let n = hit; n; n = up(n)) { if (n === el) return null; }
    for (let n = el; n; n = up(n)) { if (n === hit) return null; }
    const hitLabel = hit.closest ? hit.closest('label') : null;
    if (hitLabel && (hitLabel.control === el || hitLabel.contains(el))) return null;
    const elLabel = el.closest ? el.closest('label') : null;
    if (elLabel && elLabel.contains(hit)) return null;
    let desc = hit.tagName.toLowerCase();
    if (hit.id) desc += '#' + hit.id;
    else if (typeof hit.className === 'string' && hit.className.trim())
        desc += '.' + hit.className.trim().split(/\s+/).slice(0, 2).join('.');
    if (!hit.id && hit.closest) {
        const anchored = hit.closest('[id]');
        if (anchored && anchored !== hit)
            desc += ' inside ' + anchored.tagName.toLowerCase() + '#' + anchored.id;
    }
    return desc;
}"#;

pub(super) fn build_selector_js(selector: &str) -> String {
    let find_expr = build_find_element_js(selector);
    // Input events dispatch at viewport coordinates, so an element outside the
    // viewport must be scrolled into view first or the click lands on nothing.
    // The blocker check reports an overlay covering the click point instead of
    // letting the input land on it and silently doing the wrong thing.
    format!(
        r#"(() => {{
            const el = {find_expr};
            if (!el) return null;
            const inView = (r) => r.width > 0 && r.height > 0 &&
                r.bottom > 0 && r.right > 0 &&
                r.top < (window.innerHeight || document.documentElement.clientHeight) &&
                r.left < (window.innerWidth || document.documentElement.clientWidth);
            let rect = el.getBoundingClientRect();
            if (!inView(rect)) {{
                el.scrollIntoView({{ block: 'center', inline: 'center', behavior: 'instant' }});
                rect = el.getBoundingClientRect();
            }}
            const x = rect.x + rect.width / 2;
            const y = rect.y + rect.height / 2;
            const blockerAt = {BLOCKER_AT_JS};
            return {{ x: x, y: y, blocker: blockerAt(document, el, x, y) }};
        }})()"#,
    )
}
