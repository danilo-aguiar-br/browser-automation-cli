// SPDX-License-Identifier: MIT OR Apache-2.0
//! Output envelope schemas for `schema <cmd>` (GAP-015).
//!
//! Input schema alone tells an agent how to call a command but not how to read
//! the answer. These fragments describe the two stdout shapes produced by
//! [`crate::envelope`], so a single `schema <cmd>` call covers both directions.
use serde_json::{json, Value};

/// Success envelope written to stdout (`ok: true`).
// `needless_pass_by_value` false positive: the value IS consumed, moved into the
// `json!` object below. Macro expansion hides the move from the lint.
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn success_envelope_schema(data: Value) -> Value {
    json!({
        "type": "object",
        "description": "Success envelope on stdout (one compact JSON object)",
        "properties": {
            "schema_version": { "type": "integer", "description": "Envelope schema version" },
            "ok": { "type": "boolean", "description": "Always true for this shape" },
            "correlation_id": {
                "type": "string",
                "description": "Echoed --correlation-id when supplied"
            },
            "data": data,
        },
        "required": ["schema_version", "ok", "data"],
        "additionalProperties": false,
    })
}

/// Error envelope written to stdout (`ok: false`).
pub(crate) fn error_envelope_schema() -> Value {
    json!({
        "type": "object",
        "description": "Error envelope on stdout; check the exit code before parsing",
        "properties": {
            "schema_version": { "type": "integer" },
            "ok": { "type": "boolean", "description": "Always false for this shape" },
            "correlation_id": { "type": "string" },
            "error": {
                "type": "object",
                "properties": {
                    "kind": {
                        "type": "string",
                        "description": "Machine-stable error kind",
                        "enum": [
                            "usage", "data", "no-input", "unavailable", "software",
                            "browser", "protocol", "timeout", "cancelled",
                            "broken-pipe", "config", "io"
                        ]
                    },
                    "message": { "type": "string", "description": "English technical message" },
                    "exit_code": { "type": "integer", "description": "Sysexits-style process exit code" },
                    "suggestion": {
                        "type": "string",
                        "description": "Localized human remediation hint (absent when none)"
                    }
                },
                "required": ["kind", "message", "exit_code"],
                "additionalProperties": false,
            },
            "data": {
                "type": "object",
                "description": "Partial payload on fail-fast paths (e.g. run steps already executed)",
                "additionalProperties": true,
            }
        },
        "required": ["schema_version", "ok", "error"],
        "additionalProperties": false,
    })
}

/// Command-specific `data` payload description, or a permissive object.
///
/// Only shapes with a stable documented contract are narrowed; everything else
/// stays `additionalProperties: true` rather than claiming a shape it does not
/// guarantee.
pub(crate) fn data_schema_for(cmd: &str) -> Value {
    match cmd {
        "commands" => json!({
            "type": "object",
            "description": "Command inventory for agent discovery",
            "properties": {
                "commands": {
                    "type": "array",
                    "description": "Flat command names by default; command objects under --detail (never both)",
                    "items": {
                        "oneOf": [
                            { "type": "string", "description": "Command name (default shape)" },
                            {
                                "type": "object",
                                "description": "Enriched shape emitted only with --detail",
                                "properties": {
                                    "name": { "type": "string" },
                                    "description": { "type": "string" },
                                    "category": { "type": "string" },
                                    "surfaces": { "type": "array", "items": { "type": "string" } }
                                },
                                "required": ["name", "description", "category", "surfaces"],
                            }
                        ]
                    }
                },
                "detail": {
                    "type": "boolean",
                    "description": "Which shape `commands` carries in this envelope"
                },
                "schema_version": { "type": "integer" },
                "parity_default_on": { "type": "array", "items": { "type": "string" } },
                "devtools_tool_map": { "type": "array", "items": { "type": "object" } },
                "binary": { "type": "string" }
            },
            "additionalProperties": true,
        }),
        "schema" => json!({
            "type": "object",
            "description": "Schema fragment for one command, derived from the clap parser",
            "properties": {
                "command": { "type": "string" },
                "schema_version": { "type": "integer" },
                "schema": { "type": "object" },
                "properties": { "type": "object" },
                "required": { "type": "array", "items": { "type": "string" } },
                "surfaces": { "type": "array", "items": { "type": "string" } },
                "output_schema": { "type": "object" },
                "error_schema": { "type": "object" }
            },
            "additionalProperties": true,
        }),
        "version" => json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "version": { "type": "string" }
            },
            "additionalProperties": true,
        }),
        "run" => json!({
            "type": "object",
            "description": "Executed step results in order",
            "properties": {
                "steps": {
                    "type": "array",
                    "description": "One entry per executed step; partial on fail-fast",
                    "items": { "type": "object" }
                },
                "total": { "type": "integer" }
            },
            "additionalProperties": true,
        }),
        _ => json!({
            "type": "object",
            "description": "Command-specific payload",
            "additionalProperties": true,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_envelope_requires_data() {
        let s = success_envelope_schema(data_schema_for("goto"));
        let required = s["required"].as_array().expect("required");
        assert!(required.iter().any(|v| v == "data"));
        assert_eq!(s["properties"]["ok"]["type"], json!("boolean"));
    }

    #[test]
    fn error_envelope_lists_every_error_kind() {
        let s = error_envelope_schema();
        let kinds = s["properties"]["error"]["properties"]["kind"]["enum"]
            .as_array()
            .expect("enum");
        // Must stay in sync with ErrorKind::as_str.
        assert_eq!(kinds.len(), 12, "{kinds:?}");
        assert!(kinds.iter().any(|v| v == "broken-pipe"));
    }

    #[test]
    fn unknown_command_gets_permissive_data() {
        let s = data_schema_for("goto");
        assert_eq!(s["additionalProperties"], json!(true));
    }
}
