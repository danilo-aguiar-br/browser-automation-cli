// SPDX-License-Identifier: MIT OR Apache-2.0
//! JSON Schema fragments (core).
use serde_json::{json, Value};

use super::schema_object;

pub(crate) fn schema_for(cmd: &str) -> Option<Value> {
    Some(match cmd {
        "doctor" => schema_object(
            "Diagnose local Chrome install and one-shot readiness",
            json!({
                "offline": { "type": "boolean", "description": "Skip network probes" },
                "quick": { "type": "boolean", "description": "Skip live launch test" },
                "fix": { "type": "boolean", "description": "Apply safe repairs when possible" },
                "json": {
                    "type": "boolean",
                    "description": "Global envelope flag --json (not a local doctor flag)"
                }
            }),
            &[],
        ),
        "commands" => schema_object(
            "List available commands",
            json!({
                "json": {
                    "type": "boolean",
                    "description": "Global envelope flag --json (not a local commands flag)"
                }
            }),
            &[],
        ),
        "schema" => schema_object(
            "JSON Schema fragment for one command",
            json!({
                "cmd": { "type": "string", "description": "Command name from `commands`" }
            }),
            &["cmd"],
        ),
        "version" => schema_object("Print CLI version (JSON when --json)", json!({}), &[]),
        "locale" => schema_object(
            "Show resolved UI locale diagnostics (suggestions only; JSON machine keys English)",
            json!({}),
            &[],
        ),
        _ => return None,
    })
}
