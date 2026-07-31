// SPDX-License-Identifier: MIT OR Apache-2.0
//! JSON Schema fragments (browser_nav).
use serde_json::{json, Value};

use super::schema_object;

pub(crate) fn schema_for(cmd: &str) -> Option<Value> {
    Some(match cmd {
        "goto" => schema_object(
            "Navigate to URL and wait for load (one-shot)",
            json!({
                "url": { "type": "string", "description": "Absolute URL or about:blank" },
                "init_script": { "type": "string", "description": "JS to evaluate before navigation (tool-ref initScript)" },
                "handle_before_unload": {
                    "type": "string",
                    "enum": ["accept", "dismiss"],
                    "description": "Auto-handle beforeunload via CDP: accept | dismiss (GAP-003; CLI flag alone defaults to accept; never injects preventDefault)"
                },
                "navigation_timeout_ms": { "type": "integer", "description": "Navigation timeout override in milliseconds" }
            }),
            &["url"],
        ),
        "view" => schema_object(
            "Accessibility snapshot with @eN refs",
            json!({
                "verbose": {
                    "type": "boolean",
                    "description": "Full a11y tree (run/JSON tool-ref). CLI flag is --detailed (avoids shadowing global --verbose)."
                },
                "detailed": {
                    "type": "boolean",
                    "description": "CLI alias of verbose for one-shot argv (maps to verbose in handlers)"
                },
                "path": { "type": "string", "description": "Optional file to write tree text" }
            }),
            &[],
        ),
        "press" => schema_object(
            "Click element by CSS selector or @eN",
            json!({
                "target": { "type": "string" },
                "dblclick": { "type": "boolean" },
                "include_snapshot": { "type": "boolean" }
            }),
            &["target"],
        ),
        "click-at" => schema_object(
            "Click at page CSS coordinates (requires --experimental-vision)",
            json!({
                "x": { "type": "number" },
                "y": { "type": "number" },
                "dblclick": { "type": "boolean" },
                "include_snapshot": { "type": "boolean" }
            }),
            &["x", "y"],
        ),
        "write" => schema_object(
            "Smart fill: text, select option, checkbox/radio true|false",
            json!({
                "target": { "type": "string" },
                "value": { "type": "string" },
                "include_snapshot": { "type": "boolean" }
            }),
            &["target", "value"],
        ),
        "keys" => schema_object(
            "Press a CDP key name",
            json!({ "key": { "type": "string" } }),
            &["key"],
        ),
        "type" => schema_object(
            "Type text into a target",
            json!({
                "target": { "type": "string" },
                "text": { "type": "string" },
                "clear": { "type": "boolean" },
                "submit": { "type": "string", "description": "Optional key after type (e.g. Enter)" },
                "focus_only": { "type": "boolean", "description": "Focus target without typing" },
                "include_snapshot": { "type": "boolean" }
            }),
            &["text"],
        ),
        "wait" => schema_object(
            "Wait ms and/or text and/or CSS selector (comma OR / array) and/or URL and/or load state (GAP-019/024)",
            json!({
                "ms": { "type": "integer", "minimum": 0 },
                "text": {
                    "oneOf": [
                        { "type": "string" },
                        { "type": "array", "items": { "type": "string" } }
                    ],
                    "description": "Repeatable --text values; any match wins (OR)"
                },
                "selector": {
                    "oneOf": [
                        { "type": "string", "description": "CSS selector; comma-separated OR supported (GAP-019)" },
                        { "type": "array", "items": { "type": "string" } }
                    ]
                },
                "selectors": { "type": "array", "items": { "type": "string" }, "description": "OR list of CSS selectors" },
                "url": { "type": "string", "description": "Exact location.href match (GAP-024)" },
                "url_contains": { "type": "string", "description": "Substring match on location.href (GAP-024)" },
                "navigation": { "type": "boolean", "description": "Wait for load lifecycle (GAP-024)" },
                "state": {
                    "type": "string",
                    "enum": ["load", "domcontentloaded", "networkidle", "none"]
                },
                "wait_timeout_ms": { "type": "integer", "minimum": 0 },
                "network_idle_ms": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Quiet window with zero in-flight requests; 0 = built-in default (GAP-032)"
                },
                "min_count": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Minimum nodes the selector condition must match, default 1 (GAP-032)"
                },
                "dom_stable_ms": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Window with no serialized-DOM change; 0 = built-in default (GAP-032)"
                },
                "include_snapshot": { "type": "boolean" }
            }),
            &[],
        ),
        "hover" => schema_object(
            "Hover element by CSS selector or @eN",
            json!({ "target": { "type": "string" } }),
            &["target"],
        ),
        "drag" => schema_object(
            "HTML5 drag-and-drop driven through the page's own dragstart (GAP-030)",
            json!({
                "from": { "type": "string" },
                "to": {
                    "type": "string",
                    "description": "Destination selector or @eN; omit only when using to_x/to_y"
                },
                "to_x": { "type": "number", "description": "Absolute drop X in page CSS pixels" },
                "to_y": { "type": "number", "description": "Absolute drop Y in page CSS pixels" },
                "anchor": {
                    "type": "string",
                    "enum": ["center", "before", "after"],
                    "description": "Where in the destination rect to drop; edge anchors order list insertions"
                },
                "synthetic_payload": {
                    "type": "object",
                    "description": "CDP DragData to inject instead of the page-built DataTransfer; bypasses the page dragstart"
                },
                "include_snapshot": { "type": "boolean" }
            }),
            &["from"],
        ),
        "submit" => schema_object(
            "Submit a form (or the form owning a field) and wait for navigation or a completed request (GAP-036)",
            json!({
                "target": {
                    "type": "string",
                    "description": "Selector or @eN of the <form> or any field inside it"
                },
                "timeout_ms": { "type": "integer", "minimum": 1 },
                "include_snapshot": { "type": "boolean" }
            }),
            &["target"],
        ),
        "fill-form" => schema_object(
            "Fill multiple fields from JSON array",
            json!({
                "json": {
                    "type": "string",
                    "description": "JSON array of {target,value} objects (run/JSON key). CLI flag: --fields-json"
                },
                "fields": {
                    "type": "array",
                    "description": "Preferred run-script form: array of {target|uid,value}"
                },
                "fields_json": {
                    "type": "string",
                    "description": "CLI long name --fields-json (avoids shadowing global --json)"
                }
            }),
            &[],
        ),
        "select-option" | "select_option" | "pick" => schema_object(
            "Pick option from custom select / badge popover / role=option (GAP-023)",
            json!({
                "target": { "type": "string", "description": "Trigger control (badge/button)" },
                "option": { "type": "string", "description": "Option text, CSS selector, or role label" },
                "value": { "type": "string" },
                "include_snapshot": { "type": "boolean" }
            }),
            &["target", "option"],
        ),
        "upload" => schema_object(
            "Upload a regular file to a file input",
            json!({
                "target": { "type": "string" },
                "path": { "type": "string" }
            }),
            &["target", "path"],
        ),
        "back" => schema_object("History back", json!({}), &[]),
        "forward" => schema_object("History forward", json!({}), &[]),
        "reload" => schema_object(
            "Reload page",
            json!({ "ignore_cache": { "type": "boolean" } }),
            &[],
        ),
        "eval" => schema_object(
            "Evaluate JavaScript expression or function declaration",
            json!({
                "expression": { "type": "string" },
                "args": { "type": "string", "description": "JSON array of function args" },
                "dialog_action": { "type": "string", "description": "accept|dismiss during evaluate" },
                "file_path": { "type": "string", "description": "Optional path to write result" },
                "typed": {
                    "type": "boolean",
                    "description": "Emit data.value plus the page-reported data.value_type instead of data.result (GAP-035)"
                }
            }),
            &["expression"],
        ),
        "grab" => schema_object(
            "Screenshot (png/jpeg/webp)",
            json!({
                "path": { "type": "string" },
                "format": { "type": "string", "enum": ["png", "jpeg", "webp"] },
                "full_page": { "type": "boolean" },
                "quality": { "type": "integer" },
                "element": { "type": "string", "description": "CSS selector or @eN" }
            }),
            &[],
        ),
        "print-pdf" => schema_object(
            "Print current page to PDF via CDP Page.printToPDF (one-shot)",
            json!({
                "path": { "type": "string", "description": "Output PDF path" },
                "url": { "type": "string", "description": "Optional URL to navigate before print" }
            }),
            &[],
        ),
        "monitor" => schema_object(
            "One-shot change check against a baseline file (hash/text)",
            json!({
                "action": { "type": "string", "enum": ["check"] },
                "url": { "type": "string" },
                "baseline": { "type": "string", "description": "Baseline file path" },
                "write_baseline": { "type": "boolean" },
                "engine": { "type": "string", "enum": ["http", "browser"] }
            }),
            &["action", "url", "baseline"],
        ),
        "page" => schema_object(
            "Page info or multi-tab list|new|select|close|tab-id",
            json!({
                "action": {
                    "type": "string",
                    "enum": ["info", "list", "new", "select", "close", "tab-id"]
                },
                "url": { "type": "string" },
                "index": { "type": "integer", "minimum": 0 },
                "background": { "type": "boolean", "description": "Open new tab without focusing (page new)" },
                "isolated_context": {
                    "type": "string",
                    "description": "Named isolated browser context for page new (tool-ref isolatedContext; GAP-004; flag alone = default-isolated)"
                },
                "page_id": { "type": "integer", "minimum": 0, "description": "Tool-ref pageId alias for index (select/close)" },
                "bring_to_front": { "type": "boolean", "description": "Bring selected tab to front (page select; default true)" }
            }),
            &[],
        ),
        "dialog" => schema_object(
            "Accept or dismiss open dialog",
            json!({
                "action": { "type": "string", "enum": ["accept", "dismiss"] },
                "text": { "type": "string", "description": "Optional prompt response text (accept only)" },
                "if_present": {
                    "type": "boolean",
                    "description": "Soft-ok when no dialog is showing (GAP-006); envelope dialog_shown:false"
                }
            }),
            &["action"],
        ),
        _ => return None,
    })
}
