// SPDX-License-Identifier: MIT OR Apache-2.0
//! Network module unit tests.
use super::domain_filter::parse_domain_list;
use super::domain_script::domain_filter_script;
use super::*;
use serde_json::json;

#[test]
fn test_domain_filter_exact() {
    let filter = DomainFilter::new("example.com");
    assert!(filter.is_allowed("example.com"));
    assert!(!filter.is_allowed("other.com"));
}

#[test]
fn test_domain_filter_wildcard() {
    let filter = DomainFilter::new("*.example.com");
    assert!(filter.is_allowed("example.com"));
    assert!(filter.is_allowed("api.example.com"));
    assert!(filter.is_allowed("sub.api.example.com"));
    assert!(!filter.is_allowed("other.com"));
}

#[test]
fn test_domain_filter_empty() {
    let filter = DomainFilter::new("");
    assert!(filter.is_allowed("anything.com"));
}

#[test]
fn test_domain_filter_multiple() {
    let filter = DomainFilter::new("example.com, *.api.io");
    assert!(filter.is_allowed("example.com"));
    assert!(filter.is_allowed("api.io"));
    assert!(filter.is_allowed("v1.api.io"));
    assert!(!filter.is_allowed("other.com"));
}

#[test]
fn test_parse_domain_list() {
    let domains = parse_domain_list("A.com, B.com , *.C.com");
    assert_eq!(domains, vec!["a.com", "b.com", "*.c.com"]);
}

#[test]
fn test_domain_filter_script_blocks_peer_connection_constructors() {
    let script = domain_filter_script(&["example.com".to_string()]);
    assert!(script.contains("_blockPeerConnection('RTCPeerConnection')"));
    assert!(script.contains("_blockPeerConnection('webkitRTCPeerConnection')"));
    assert!(script.contains("RTCPeerConnection blocked while domain filtering is active"));
    assert!(script.contains("configurable: false"));
}

#[test]
fn test_domain_filter_script_fails_closed_when_worker_blob_is_csp_blocked() {
    let script = domain_filter_script(&["example.com".to_string()]);
    assert!(script.contains("createObjectURL(new Blob"));
    assert!(script.contains("'await import(' + JSON.stringify(absolute)"));
    assert!(script.contains("const worker = new OrigCtor(bootstrapUrl, options)"));
    assert!(script.contains("return worker"));
    assert!(!script.contains("_wrapWorkerWithCspFallback"));
    assert!(!script.contains("return new OrigCtor(checkedUrl, options)"));
}

#[test]
fn test_event_tracker() {
    let mut tracker = EventTracker::new();
    tracker.add_console("log", "hello", vec![]);
    tracker.add_error("oops", Some("test.js"), Some(1), Some(5));

    assert_eq!(tracker.console_entries.len(), 1);
    assert_eq!(tracker.error_entries.len(), 1);
}

#[test]
fn test_console_json_includes_args() {
    let mut tracker = EventTracker::new();
    let raw_args = vec![
        json!({"type": "string", "value": "hello"}),
        json!({"type": "number", "value": 42}),
    ];
    tracker.add_console("log", "hello 42", raw_args);

    let result = tracker.get_console_json();
    let messages = result.get("messages").unwrap().as_array().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].get("text").unwrap(), "hello 42");
    let args = messages[0].get("args").unwrap().as_array().unwrap();
    assert_eq!(args.len(), 2);
    assert_eq!(args[0], json!({"type": "string", "value": "hello"}));
    assert_eq!(args[1], json!({"type": "number", "value": 42}));
}

#[test]
fn test_console_json_empty_args_omits_field() {
    let mut tracker = EventTracker::new();
    tracker.add_console("log", "text only", vec![]);

    let result = tracker.get_console_json();
    let messages = result.get("messages").unwrap().as_array().unwrap();
    assert!(messages[0].get("args").is_none());
}

// -- format_console_arg: primitives --

#[test]
fn test_format_arg_string() {
    let arg = json!({"type": "string", "value": "hello"});
    assert_eq!(format_console_arg(&arg), Some("hello".to_string()));
}

#[test]
fn test_format_arg_number() {
    let arg = json!({"type": "number", "value": 42});
    assert_eq!(format_console_arg(&arg), Some("42".to_string()));
}

#[test]
fn test_format_arg_null() {
    let arg = json!({"type": "object", "subtype": "null", "value": null});
    assert_eq!(format_console_arg(&arg), Some("null".to_string()));
}

#[test]
fn test_format_arg_undefined() {
    let arg = json!({"type": "undefined"});
    assert_eq!(format_console_arg(&arg), Some("undefined".to_string()));
}

// -- format_console_arg: objects with preview --

#[test]
fn test_format_arg_object_preview() {
    let arg = json!({
        "type": "object",
        "preview": {
            "properties": [
                {"name": "userId", "type": "string", "value": "abc123"},
                {"name": "count", "type": "number", "value": "42"}
            ],
            "overflow": false
        }
    });
    assert_eq!(
        format_console_arg(&arg),
        Some("{userId: \"abc123\", count: 42}".to_string())
    );
}

#[test]
fn test_format_arg_object_preview_overflow() {
    let arg = json!({
        "type": "object",
        "preview": {
            "properties": [
                {"name": "a", "type": "number", "value": "1"}
            ],
            "overflow": true
        }
    });
    assert_eq!(format_console_arg(&arg), Some("{a: 1, ...}".to_string()));
}

// -- format_console_arg: arrays with preview --

#[test]
fn test_format_arg_array_preview() {
    let arg = json!({
        "type": "object",
        "subtype": "array",
        "preview": {
            "subtype": "array",
            "properties": [
                {"name": "0", "type": "number", "value": "1"},
                {"name": "1", "type": "number", "value": "2"},
                {"name": "2", "type": "number", "value": "3"}
            ],
            "overflow": false
        }
    });
    assert_eq!(format_console_arg(&arg), Some("[1, 2, 3]".to_string()));
}

// -- format_console_arg: map/set use description --

#[test]
fn test_format_arg_map_uses_description() {
    let arg = json!({
        "type": "object",
        "subtype": "map",
        "description": "Map(1)",
        "preview": {
            "subtype": "map",
            "properties": [{"name": "size", "type": "number", "value": "1"}]
        }
    });
    assert_eq!(format_console_arg(&arg), Some("Map(1)".to_string()));
}

// -- format_console_arg: fallback --

#[test]
fn test_format_arg_description_fallback() {
    let arg = json!({"type": "object", "description": "RegExp"});
    assert_eq!(format_console_arg(&arg), Some("RegExp".to_string()));
}

#[test]
fn test_format_arg_no_value_no_preview_no_description() {
    let arg = json!({"type": "object"});
    assert_eq!(format_console_arg(&arg), None);
}

// -- format_console_args --

#[test]
fn test_format_console_args_join() {
    let args = vec![
        json!({"type": "string", "value": "user"}),
        json!({
            "type": "object",
            "preview": {
                "properties": [{"name": "id", "type": "number", "value": "1"}],
                "overflow": false
            }
        }),
    ];
    assert_eq!(format_console_args(&args), "user {id: 1}");
}

#[test]
fn test_format_console_args_filters_none() {
    // An arg that returns None should be skipped, not produce empty string
    let args = vec![
        json!({"type": "string", "value": "before"}),
        json!({"type": "object"}), // no value, preview, or description → None
        json!({"type": "string", "value": "after"}),
    ];
    assert_eq!(format_console_args(&args), "before after");
}
