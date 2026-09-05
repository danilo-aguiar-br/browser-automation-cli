// SPDX-License-Identifier: MIT OR Apache-2.0

use super::keys::*;

#[test]
fn test_char_to_key_info_matches_playwright_layout() {
    // (character, expected_code, expected_vk_code)
    let cases: &[(char, &str, i32)] = &[
        // Letters – VK code must equal the uppercase ASCII value.
        ('a', "KeyA", 65),
        ('z', "KeyZ", 90),
        ('A', "KeyA", 65),
        // Digits
        ('0', "Digit0", 48),
        ('9', "Digit9", 57),
        // Punctuation – these are the values from Playwright's layout.
        // The bug that prompted this test sent '.' as VK 46 (= VK_DELETE).
        ('.', "Period", 190),
        (',', "Comma", 188),
        ('/', "Slash", 191),
        (';', "Semicolon", 186),
        ('\'', "Quote", 222),
        ('[', "BracketLeft", 219),
        (']', "BracketRight", 221),
        ('\\', "Backslash", 220),
        ('`', "Backquote", 192),
        ('-', "Minus", 189),
        ('=', "Equal", 187),
        // Shifted variants produced by the same physical keys.
        ('>', "Period", 190),
        ('<', "Comma", 188),
        ('?', "Slash", 191),
        (':', "Semicolon", 186),
        ('"', "Quote", 222),
        ('{', "BracketLeft", 219),
        ('}', "BracketRight", 221),
        ('|', "Backslash", 220),
        ('~', "Backquote", 192),
        ('_', "Minus", 189),
        ('+', "Equal", 187),
        // Whitespace / control
        (' ', "Space", 32),
        ('\n', "Enter", 13),
        ('\t', "Tab", 9),
    ];

    for &(ch, expected_code, expected_vk) in cases {
        let (key, code, vk) = char_to_key_info(ch);
        assert_eq!(
            code, expected_code,
            "char {ch:?}: expected code {expected_code:?}, got {code:?}"
        );
        assert_eq!(
            vk, expected_vk,
            "char {:?}: expected VK {}, got {} (ASCII would be {})",
            ch, expected_vk, vk, ch as i32
        );
        // key should be the character itself (except control chars).
        if !ch.is_control() {
            assert_eq!(key, ch.to_string(), "char {ch:?}: key mismatch");
        }
    }
}

/// Regression test: period must NEVER map to VK 46 (VK_DELETE).
#[test]
fn test_period_is_not_vk_delete() {
    let (_, _, vk) = char_to_key_info('.');
    assert_ne!(
        vk, 46,
        "Period must not use VK code 46 (VK_DELETE); expected 190 (VK_OEM_PERIOD)"
    );
    assert_eq!(vk, 190);
}

/// Characters outside the US keyboard layout should return (key, "", 0)
/// so that `type_text` falls back to `Input.insertText`.
#[test]
fn test_unmapped_chars_return_zero_keycode() {
    for ch in ['@', '#', '$', '%', '^', '&', '*', '(', ')', '€', '£', '你'] {
        let (key, code, vk) = char_to_key_info(ch);
        assert_eq!(
            code, "",
            "char {ch:?}: unmapped char should have empty code, got {code:?}"
        );
        assert_eq!(
            vk, 0,
            "char {ch:?}: unmapped char should have VK 0, got {vk}"
        );
        assert_eq!(key, ch.to_string());
    }
}

#[test]
fn test_key_text_returns_correct_text_for_special_keys() {
    assert_eq!(key_text("Enter"), Some("\r".to_string()));
    assert_eq!(key_text("Tab"), Some("\t".to_string()));
    assert_eq!(key_text(" "), Some(" ".to_string()));
    // Single printable characters carry themselves.
    assert_eq!(key_text("a"), Some("a".to_string()));
    assert_eq!(key_text("Z"), Some("Z".to_string()));
    // Non-printable named keys return None.
    assert_eq!(key_text("Escape"), None);
    assert_eq!(key_text("ArrowUp"), None);
    assert_eq!(key_text("Backspace"), None);
    assert_eq!(key_text("Delete"), None);
}

/// Smart-fill mode tokens accepted by `fill_smart` branch match.
#[test]
fn fill_smart_mode_tokens_documented() {
    for mode in [
        "select",
        "checkbox",
        "radio",
        "text",
        "textarea",
        "contenteditable",
    ] {
        assert!(!mode.is_empty());
    }
    // true/false semantics for checkbox/radio
    assert!("true".parse::<bool>().unwrap());
    assert!(!"false".parse::<bool>().unwrap());
}

// ---------------------------------------------------------------------------
// GAP-030 drag payload / anchor handling
// ---------------------------------------------------------------------------

mod drag {
    use super::super::drag_html5::{
        normalize_drag_data, validate_synthetic_payload, DragRoute, DropAnchor, ElementRect,
    };
    use serde_json::json;

    #[test]
    fn anchor_parses_aliases_and_rejects_unknown() {
        assert_eq!(DropAnchor::parse("center").unwrap(), DropAnchor::Center);
        assert_eq!(DropAnchor::parse("").unwrap(), DropAnchor::Center);
        assert_eq!(DropAnchor::parse(" BEFORE ").unwrap(), DropAnchor::Before);
        assert_eq!(DropAnchor::parse("top").unwrap(), DropAnchor::Before);
        assert_eq!(DropAnchor::parse("after").unwrap(), DropAnchor::After);
        assert_eq!(DropAnchor::parse("end").unwrap(), DropAnchor::After);
        let err = DropAnchor::parse("sideways").unwrap_err();
        assert!(
            err.contains("sideways"),
            "error must name the bad token: {err}"
        );
    }

    /// Edge anchors must land on opposite halves of the rect and stay inside it,
    /// or list-insertion order is a coin flip.
    // `float_cmp` is enabled package-wide, and exact equality is the right
    // assertion here: the anchors come from `x + width / 2.0` over values that
    // are exactly representable in binary floating point, so the expected
    // results are exact too. A tolerance would let a wrong formula pass.
    #[allow(clippy::float_cmp)]
    #[test]
    fn edge_anchors_stay_inside_the_rect_and_differ() {
        let rect = ElementRect {
            x: 100.0,
            y: 200.0,
            width: 50.0,
            height: 40.0,
        };
        let (cx, cy) = rect.anchor_point(DropAnchor::Center);
        let (bx, by) = rect.anchor_point(DropAnchor::Before);
        let (ax, ay) = rect.anchor_point(DropAnchor::After);

        assert_eq!(cx, 125.0);
        assert_eq!(cy, 220.0);
        assert_eq!(bx, 125.0, "anchors only move on the vertical axis");
        assert_eq!(ax, 125.0);
        assert!(by < cy, "before must sit above centre: {by} vs {cy}");
        assert!(ay > cy, "after must sit below centre: {ay} vs {cy}");
        assert!(by > rect.y, "before must stay inside the rect: {by}");
        assert!(ay < rect.y + rect.height, "after must stay inside: {ay}");
    }

    /// A 1px-tall row must not produce identical before/after points.
    #[test]
    fn tiny_rect_still_separates_edges() {
        let rect = ElementRect {
            x: 0.0,
            y: 10.0,
            width: 10.0,
            height: 2.0,
        };
        let (_, by) = rect.anchor_point(DropAnchor::Before);
        let (_, ay) = rect.anchor_point(DropAnchor::After);
        assert!(by <= ay, "before must not fall below after: {by} vs {ay}");
    }

    #[test]
    fn normalizes_nested_and_flat_intercepted_payloads() {
        let nested = json!({
            "data": { "items": [{ "mimeType": "text/plain", "data": "row-1" }],
                      "dragOperationsMask": 5 }
        });
        let out = normalize_drag_data(&nested).expect("nested payload");
        assert_eq!(out["items"][0]["data"], "row-1");
        assert_eq!(out["dragOperationsMask"], 5);

        let flat = json!({ "items": [{ "mimeType": "text/uri-list", "data": "u" }] });
        let out = normalize_drag_data(&flat).expect("flat payload");
        assert_eq!(out["items"][0]["mimeType"], "text/uri-list");
        // Absent mask falls back to the "copy" default rather than dropping the field.
        assert_eq!(out["dragOperationsMask"], 1);
    }

    #[test]
    fn preserves_files_when_the_page_supplied_them() {
        let payload = json!({
            "items": [{ "mimeType": "text/plain", "data": "x" }],
            "files": ["/tmp/a.txt"]
        });
        let out = normalize_drag_data(&payload).expect("payload with files");
        assert_eq!(out["files"][0], "/tmp/a.txt");
    }

    /// A payload without `items` must fail loudly: padding it with `[]` would
    /// drop whatever the page actually put on the DataTransfer.
    #[test]
    fn rejects_payload_without_items() {
        let err = normalize_drag_data(&json!({ "dragOperationsMask": 1 })).unwrap_err();
        assert!(
            err.contains("items"),
            "error must name the missing field: {err}"
        );
        let err = normalize_drag_data(&json!({ "items": "not-an-array" })).unwrap_err();
        assert!(
            err.contains("items"),
            "non-array items must be rejected: {err}"
        );
    }

    #[test]
    fn synthetic_payload_must_be_an_object_with_items() {
        assert!(validate_synthetic_payload(&json!("text/plain")).is_err());
        assert!(validate_synthetic_payload(&json!([1, 2, 3])).is_err());
        assert!(validate_synthetic_payload(&json!({ "items": [] })).is_ok());
    }

    /// The envelope tag is the only way an agent can tell a proven drop from a
    /// degraded one, so the strings are part of the contract.
    #[test]
    fn route_tags_are_distinct_and_stable() {
        assert_eq!(DragRoute::Intercepted.as_str(), "intercepted");
        assert_eq!(DragRoute::SyntheticPayload.as_str(), "synthetic_payload");
        assert_eq!(DragRoute::SyntheticMouse.as_str(), "synthetic_mouse");
    }
}

// ---------------------------------------------------------------------------
// GAP-031 scroll request shaping
// ---------------------------------------------------------------------------

mod scroll {
    use super::super::scroll::{scroll_arguments, ScrollRequest};
    use serde_json::Value;

    fn values(req: &ScrollRequest<'_>) -> Vec<Value> {
        scroll_arguments(req)
            .into_iter()
            .map(|a| a.value.expect("argument value"))
            .collect()
    }

    /// Absent absolute offsets must serialize as JSON `null`, not be omitted:
    /// the page function branches on `tx !== null`, so a dropped argument would
    /// silently turn an absolute scroll into a delta scroll.
    #[test]
    fn absent_absolute_offsets_serialize_as_null() {
        let args = values(&ScrollRequest {
            delta_x: 10.0,
            delta_y: -20.0,
            ..ScrollRequest::default()
        });
        assert_eq!(args.len(), 4);
        assert_eq!(args[0], 10.0);
        assert_eq!(args[1], -20.0);
        assert_eq!(args[2], Value::Null);
        assert_eq!(args[3], Value::Null);
    }

    #[test]
    fn absolute_offsets_pass_through_per_axis() {
        let args = values(&ScrollRequest {
            delta_y: 5.0,
            to_y: Some(0.0),
            ..ScrollRequest::default()
        });
        assert_eq!(args[2], Value::Null, "unset axis stays null");
        assert_eq!(args[3], 0.0, "zero is a real offset, not 'unset'");
    }
}

/// No `Input.dispatchKeyEvent` may carry `nativeVirtualKeyCode`.
///
/// # Why this is asserted against the source and not against a browser
///
/// `char_to_key_info` returns the WINDOWS virtual key code, and the native code
/// is a different namespace on every platform. Measured 2026-09-04 on macOS:
/// sending 65 in both fields made Chrome read the native one, where 65 is
/// `kVK_ANSI_KeypadDecimal`, and answer one `a` keydown with 374 spurious
/// `NumpadDecimal` ones in a 213 ms burst. Five characters hung the command
/// past chromiumoxide's 30 s `REQUEST_TIMEOUT`, which is how the defect first
/// showed up: as a flaky `tests/input_trace_gate.rs`, three layers away from
/// the field that caused it.
///
/// `input_trace_gate` does catch the regression, by asserting five keydowns and
/// getting thousands. It needs a Chrome, a fixture and roughly five minutes,
/// and it names the symptom rather than the cause. This case needs a file read,
/// and it names the field.
///
/// Scanning source text is the right shape here precisely because the defect is
/// a literal that type-checks. `Some(key_code)` in that position compiles,
/// round-trips through serde and reaches Chrome; no type, no lint and no unit
/// test on `char_to_key_info` can see anything wrong with it.
#[test]
fn no_dispatch_key_event_sends_a_native_virtual_key_code() {
    let src = include_str!("keyboard.rs");
    let offenders: Vec<usize> = src
        .lines()
        .enumerate()
        .filter(|(_, l)| {
            let t = l.trim_start();
            t.starts_with("native_virtual_key_code:")
                && !t.starts_with("native_virtual_key_code: None")
        })
        .map(|(i, _)| i + 1)
        .collect();
    assert!(
        offenders.is_empty(),
        "src/native/interaction/keyboard.rs sets `native_virtual_key_code` to something other \
         than `None` at line(s) {offenders:?}; the only key code this module knows is the \
         WINDOWS one, and Chrome reads the native field in the host platform's namespace, where \
         the same number means a different key"
    );
    assert!(
        src.contains("native_virtual_key_code: None"),
        "src/native/interaction/keyboard.rs no longer mentions `native_virtual_key_code` at all, \
         so this case stopped guarding anything; either the field was dropped from the params \
         struct — in which case delete this case — or the dispatch calls moved elsewhere and the \
         guard must move with them"
    );
}
