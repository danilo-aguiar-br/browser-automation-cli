// SPDX-License-Identifier: MIT OR Apache-2.0
//! JSON Schema fragments (ops_tools).
use serde_json::{json, Value};

use super::schema_object;

pub(crate) fn schema_for(cmd: &str) -> Option<Value> {
    Some(match cmd {
        "mitm" => schema_object(
            "MITM capture / CA / HAR (one-shot local 127.0.0.1)",
            json!({
                "action": {
                    "type": "string",
                    "enum": [
                        "status", "list", "get", "har", "export",
                        "domains", "apis", "init-ca", "start"
                    ]
                },
                "id": { "type": "string" },
                "out": { "type": "string" },
                "seconds": { "type": "integer", "minimum": 1 },
                "limit": { "type": "integer", "minimum": 1 }
            }),
            &["action"],
        ),
        "workflow" => schema_object(
            "Workflow journal DAG (petgraph + SQLite under XDG state)",
            json!({
                "action": { "type": "string", "enum": ["run", "resume", "status"] },
                "manifest": { "type": "string", "description": "JSON workflow manifest path" },
                "journal": { "type": "string" },
                "name": { "type": "string" }
            }),
            &["action"],
        ),
        "config" => schema_object(
            "XDG config and path management (no product env at runtime)",
            json!({
                "action": {
                    "type": "string",
                    "enum": ["path", "init", "show", "set", "get", "list-keys"]
                },
                "key": {
                    "type": "string",
                    "description": format!("For set/get: {}", crate::xdg::config_keys_description())
                },
                "value": { "type": "string" }
            }),
            &["action"],
        ),
        "emulate" => schema_object(
            "Emulate UA locale timezone network geo media CPU viewport headers",
            json!({
                "user_agent": { "type": "string" },
                "locale": { "type": "string" },
                "timezone": { "type": "string" },
                "offline": { "type": "boolean" },
                "latitude": { "type": "number" },
                "longitude": { "type": "number" },
                "media": { "type": "string" },
                "network_conditions": { "type": "string" },
                "cpu_throttling_rate": { "type": "number" },
                "color_scheme": { "type": "string" },
                "extra_headers": { "type": "string" },
                "viewport": { "type": "string" },
                "screen": { "type": "string", "description": "Screen size WxH (never smaller than the viewport)" }
            }),
            &[],
        ),
        "resize" => schema_object(
            "Resize viewport",
            json!({
                "width": { "type": "integer" },
                "height": { "type": "integer" },
                "scale": { "type": "number" },
                "mobile": { "type": "boolean" },
                "screen": { "type": "string", "description": "Screen size WxH (never smaller than the viewport)" }
            }),
            &["width", "height"],
        ),
        "perf" => schema_object(
            "Performance start|stop|insight",
            json!({
                "action": { "type": "string", "enum": ["start", "stop", "insight"] },
                "path": { "type": "string" },
                "reload": { "type": "boolean" },
                "name": { "type": "string" }
            }),
            &["action"],
        ),
        "lighthouse" => schema_object(
            "External lighthouse audit with JSON scores",
            json!({
                "url": { "type": "string" },
                "out_dir": { "type": "string" },
                "device": { "type": "string" },
                "mode": { "type": "string" },
                "lighthouse_path": { "type": "string" }
            }),
            &["url"],
        ),
        "screencast" => schema_object(
            "Screencast start|stop (requires --experimental-screencast)",
            json!({
                "action": { "type": "string", "enum": ["start", "stop"] },
                "path": { "type": "string" }
            }),
            &["action"],
        ),
        "heap" => schema_object(
            "Heap snapshot tools (deep ops need --category-memory)",
            json!({
                "action": { "type": "string" },
                "path": { "type": "string" },
                "base": { "type": "string" },
                "current": { "type": "string" },
                "id": { "type": "integer" },
                "node": { "type": "integer" }
            }),
            &["action"],
        ),
        "extension" => schema_object(
            "Extension tools (requires --category-extensions)",
            json!({
                "action": { "type": "string" },
                "path": { "type": "string" },
                "id": { "type": "string" }
            }),
            &["action"],
        ),
        "devtools3p" => schema_object(
            "Third-party tools surface (requires --category-third-party)",
            json!({
                "action": { "type": "string", "enum": ["list", "exec"] },
                "name": { "type": "string" },
                "params": { "type": "string" },
                "url": { "type": "string" }
            }),
            &["action"],
        ),
        "webmcp" => schema_object(
            "Web surface tools (requires --category-webmcp)",
            json!({
                "action": { "type": "string", "enum": ["list", "exec"] },
                "name": { "type": "string" },
                "input": { "type": "string" },
                "url": { "type": "string" }
            }),
            &["action"],
        ),
        "completions" => schema_object(
            "Generate shell completions (no Chrome)",
            json!({
                "shell": {
                    "type": "string",
                    "enum": ["bash", "zsh", "fish", "elvish", "powershell"]
                }
            }),
            &["shell"],
        ),
        "man" => schema_object(
            "Generate man page (roff) via clap_mangen (no Chrome)",
            json!({
                "out": {
                    "type": "string",
                    "description": "Optional output path; default stdout"
                }
            }),
            &[],
        ),
        _ => return None,
    })
}
