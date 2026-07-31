// SPDX-License-Identifier: MIT OR Apache-2.0

use super::js::*;
use super::refs::*;
use super::resolve::{box_model_center, resolve_frame_session};
use crate::native::cdp::types::*;
use rustc_hash::FxHashMap;

#[test]
fn test_parse_ref_at_prefix() {
    assert_eq!(parse_ref("@e1"), Some("e1".to_string()));
    assert_eq!(parse_ref("@e123"), Some("e123".to_string()));
}

#[test]
fn test_parse_ref_equals_prefix() {
    assert_eq!(parse_ref("ref=e1"), Some("e1".to_string()));
}

#[test]
fn test_parse_ref_bare() {
    assert_eq!(parse_ref("e1"), Some("e1".to_string()));
    assert_eq!(parse_ref("e42"), Some("e42".to_string()));
}

#[test]
fn test_parse_ref_invalid() {
    assert_eq!(parse_ref("button"), None);
    assert_eq!(parse_ref("e"), None);
    assert_eq!(parse_ref("1"), None);
    assert_eq!(parse_ref(""), None);
}

#[test]
fn test_ref_map_basic() {
    let mut map = RefMap::new();
    map.add("e1".to_string(), Some(42), "button", "Submit", None);
    assert!(map.get("e1").is_some());
    assert_eq!(map.get("e1").unwrap().role, "button");
    assert!(map.get("e2").is_none());
}

#[test]
fn test_normalize_css_prefix_strips_playwright_style() {
    assert_eq!(normalize_css_selector("css=h1"), "h1");
    assert_eq!(normalize_css_selector("h1"), "h1");
    assert_eq!(normalize_css_selector("css=.foo > bar"), ".foo > bar");
}

#[test]
fn test_build_find_element_js_strips_css_prefix() {
    let js = build_find_element_js("css=h1");
    assert!(js.contains("document.querySelector(\"h1\")"), "got {js}");
    assert!(!js.contains("css=h1"), "got {js}");
}

#[test]
fn test_object_id_from_evaluate_rejects_exception() {
    let result = EvaluateResult {
        result: RemoteObject {
            object_type: "object".into(),
            subtype: Some("error".into()),
            value: None,
            description: Some("SyntaxError".into()),
            object_id: Some("err.1".into()),
            class_name: Some("SyntaxError".into()),
            unserializable_value: None,
            preview: None,
        },
        exception_details: Some(ExceptionDetails {
            text: "Uncaught".into(),
            exception: Some(RemoteObject {
                object_type: "object".into(),
                subtype: Some("error".into()),
                value: None,
                description: Some("not a valid selector".into()),
                object_id: Some("err.1".into()),
                class_name: Some("SyntaxError".into()),
                unserializable_value: None,
                preview: None,
            }),
            line_number: None,
            column_number: None,
        }),
    };
    let err = object_id_from_evaluate(result, "css=h1").unwrap_err();
    assert!(err.contains("Element not found"), "{err}");
    assert!(err.contains("not a valid selector"), "{err}");
}

#[test]
fn test_build_selector_js_css() {
    let js = build_selector_js("#submit-btn");
    assert!(js.contains("document.querySelector(\"#submit-btn\")"));
    assert!(!js.contains("document.evaluate"));
}

#[test]
fn test_build_selector_js_xpath() {
    let js = build_selector_js("xpath=//button[@id='ok']");
    assert!(js.contains("document.evaluate(\"//button[@id='ok']\", document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null)"));
    assert!(!js.contains("document.querySelector"));
}

#[test]
fn test_build_selector_js_xpath_empty() {
    let js = build_selector_js("xpath=");
    assert!(js.contains("document.evaluate"));
}

#[test]
fn test_build_selector_js_not_xpath_prefix() {
    // "xpath" without "=" should be treated as CSS selector
    let js = build_selector_js("xpath//div");
    assert!(js.contains("document.querySelector"));
}

#[test]
fn test_build_count_elements_js_css() {
    let js = build_count_elements_js(".item");
    assert!(js.contains("document.querySelectorAll(\".item\").length"));
    assert!(!js.contains("document.evaluate"));
}

#[test]
fn test_build_count_elements_js_xpath() {
    let js = build_count_elements_js("xpath=//li");
    assert!(js.contains("document.evaluate(\"//li\", document, null, XPathResult.ORDERED_NODE_SNAPSHOT_TYPE, null).snapshotLength"));
    assert!(!js.contains("querySelectorAll"));
}

#[test]
fn test_box_model_center() {
    let model = BoxModel {
        content: vec![10.0, 20.0, 110.0, 20.0, 110.0, 60.0, 10.0, 60.0],
        padding: vec![],
        border: vec![],
        margin: vec![],
        width: 100,
        height: 40,
    };
    let (x, y) = box_model_center(&model);
    assert!((x - 60.0).abs() < 0.01);
    assert!((y - 40.0).abs() < 0.01);
}

// -----------------------------------------------------------------------
// resolve_frame_session tests (Issue #925)
// Cross-origin iframe elements must resolve to the dedicated session.
// -----------------------------------------------------------------------

#[test]
fn test_cross_origin_element_uses_dedicated_session() {
    let mut iframe_sessions = FxHashMap::default();
    iframe_sessions.insert(
        "cross-origin-frame".to_string(),
        "iframe-session".to_string(),
    );

    let session = resolve_frame_session(
        Some("cross-origin-frame"),
        "parent-session",
        &iframe_sessions,
    );

    assert_eq!(session, "iframe-session");
}

#[test]
fn test_same_origin_element_uses_parent_session() {
    let iframe_sessions = FxHashMap::default();

    let session = resolve_frame_session(
        Some("same-origin-frame"),
        "parent-session",
        &iframe_sessions,
    );

    assert_eq!(session, "parent-session");
}

#[test]
fn test_main_frame_element_uses_parent_session() {
    let iframe_sessions = FxHashMap::default();

    let session = resolve_frame_session(None, "parent-session", &iframe_sessions);

    assert_eq!(session, "parent-session");
}
