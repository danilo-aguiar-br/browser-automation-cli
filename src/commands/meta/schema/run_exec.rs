// SPDX-License-Identifier: MIT OR Apache-2.0
//! JSON Schema fragments (run_exec).
use serde_json::{json, Value};

use super::schema_object;

pub(crate) fn schema_for(cmd: &str) -> Option<Value> {
    Some(match cmd {
        "run" => schema_object(
            "Execute multi-step script in one process; script file is NDJSON (one object per line) or a top-level JSON array of step objects",
            json!({ "script": { "type": "string", "description": "Path to script file (.jsonl NDJSON or .json array of steps)" } }),
            &["script"],
        ),
        "exec" => schema_object(
            "Single-step inline command (same surface as run steps)",
            json!({
                "args": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "e.g. [\"goto\", \"about:blank\"] or [\"wait\", \"--ms\", \"100\"]"
                }
            }),
            &["args"],
        ),
        "extract" => schema_object(
            "Extract text or attribute from target, or LLM extract with --llm",
            json!({
                "target": { "type": "string", "description": "CSS/@eN, http(s) URL, or file path for --llm (positional; or pass --url)" },
                "url": { "type": "string", "description": "Alias for the positional target; pass one form or the other" },
                "attr": { "type": "string" },
                "llm": { "type": "boolean" },
                "question": { "type": "string" },
                "schema_json": { "type": "string", "description": "Path to JSON schema file" }
            }),
            // Neither name is required alone: clap accepts the target
            // positionally OR as `--url`, and refuses both at once.
            &[],
        ),
        "text" => schema_object(
            "Extract visible text from target (PRD §7)",
            json!({
                "target": { "type": "string" }
            }),
            &["target"],
        ),
        "scroll" => schema_object(
            "Scroll the viewport or a container with its own scrollbar, by delta or absolute offset (GAP-031)",
            json!({
                "target": { "type": "string", "description": "Optional CSS/@eN" },
                "delta_x": { "type": "number" },
                "delta_y": { "type": "number" },
                "dx": { "type": "number", "description": "Alias for delta_x" },
                "dy": { "type": "number", "description": "Alias for delta_y" },
                "to_x": { "type": "number", "description": "Absolute horizontal offset; wins over delta_x (GAP-031)" },
                "to_y": { "type": "number", "description": "Absolute vertical offset; wins over delta_y (GAP-031)" }
            }),
            &[],
        ),
        "cookie" => schema_object(
            "Cookie list/set/clear for the one-shot browser process",
            json!({
                "action": { "type": "string", "enum": ["list", "set", "clear"] },
                "url": { "type": "string" },
                "json": {
                    "type": "string",
                    "description": "JSON array for set (run/JSON key). CLI flag: --cookies-json"
                },
                "cookies": {
                    "type": "array",
                    "description": "Preferred run-script form for set"
                },
                "cookies_json": {
                    "type": "string",
                    "description": "CLI long name --cookies-json (avoids shadowing global --json)"
                }
            }),
            &["action"],
        ),
        "attr" => schema_object(
            "Read one attribute from target",
            json!({
                "target": { "type": "string" },
                "name": { "type": "string" }
            }),
            &["target", "name"],
        ),
        "assert" => schema_object(
            "Assertion helpers (url/text/console)",
            json!({
                "kind": { "type": "string", "enum": ["url", "text", "console", "console_empty", "console_no_match"] },
                "pattern": { "type": "string", "description": "For console_no_match (GAP-025)" },
                "value": { "type": "string" },
                "url": { "type": "string" },
                "url_contains": { "type": "string" },
                "text": { "type": "string" },
                "text_contains": { "type": "string" },
                "contains": { "type": "boolean" },
                "target": { "type": "string" },
                "level": { "type": "string" },
                "max": { "type": "integer" }
            }),
            &[],
        ),
        "console" => schema_object(
            "List/get/clear/dump captured console messages (needs --capture-console)",
            json!({
                "action": { "type": "string", "enum": ["list", "get", "clear", "dump"] },
                "id": { "type": "integer", "minimum": 0 },
                "path": { "type": "string" }
            }),
            &["action"],
        ),
        "net" => schema_object(
            "List or get captured network requests (needs --capture-network)",
            json!({
                "action": { "type": "string", "enum": ["list", "get"] },
                "id": { "type": "integer", "minimum": 0 },
                "request_path": { "type": "string" },
                "response_path": { "type": "string" }
            }),
            &["action"],
        ),
        _ => return None,
    })
}
