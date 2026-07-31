// SPDX-License-Identifier: MIT OR Apache-2.0
//! Agent-facing `commands` / `schema` handlers.
use serde_json::{json, Value};

use crate::envelope::print_success_json;
use crate::error::{CliError, ErrorKind};

use super::inventory::{command_objects, COMMANDS, DEVTOOLS_TOOL_MAP, PARITY_DEFAULT_ON_REQUIRED};
use super::schema::derive::surfaces_for;
use super::schema::output::{data_schema_for, error_envelope_schema, success_envelope_schema};
use super::schema::{schema_for, schema_object};

pub fn list_commands(detail: bool, json: bool) -> Result<(), CliError> {
    let map: Vec<Value> = DEVTOOLS_TOOL_MAP
        .iter()
        .map(|(tool, cli)| json!({ "tool": tool, "cli": cli }))
        .collect();
    // Contract: `commands` holds exactly one shape. The flat list is the
    // default because most agents only need names, and emitting both shapes
    // would inflate the envelope the way GAP-019 describes.
    let commands = if detail {
        json!(command_objects())
    } else {
        json!(COMMANDS)
    };
    let data = json!({
        "commands": commands,
        "detail": detail,
        "schema_version": 1,
        "parity_default_on": PARITY_DEFAULT_ON_REQUIRED,
        "devtools_tool_map": map,
        "binary": "browser-automation-cli",
    });
    if json {
        print_success_json(data)?;
    } else if detail {
        for c in command_objects() {
            let name = c.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let category = c.get("category").and_then(|v| v.as_str()).unwrap_or("");
            let description = c.get("description").and_then(|v| v.as_str()).unwrap_or("");
            crate::output::writeln_stdout(format!("{name}\t{category}\t{description}"))?;
        }
        crate::output::flush_stdout()?;
    } else {
        for c in COMMANDS {
            crate::output::writeln_stdout(*c)?;
        }
        crate::output::flush_stdout()?;
    }
    Ok(())
}

pub fn schema_for_cmd(cmd: &str, json: bool) -> Result<(), CliError> {
    if !COMMANDS.contains(&cmd) {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("unknown command for schema: {cmd}"),
            crate::i18n::suggestion_key("commands_discovery", None),
        ));
    }
    let fragment = schema_for(cmd)
        .unwrap_or_else(|| schema_object(&format!("Schema fragment for `{cmd}`"), json!({}), &[]));
    let data = json!({
        "command": cmd,
        "schema_version": 1,
        "schema": fragment,
        "type": fragment.get("type").cloned().unwrap_or(json!("object")),
        "description": fragment.get("description").cloned().unwrap_or(json!("")),
        "properties": fragment.get("properties").cloned().unwrap_or(json!({})),
        "required": fragment.get("required").cloned().unwrap_or(json!([])),
        // GAP-013/014: which surfaces accept this command.
        "surfaces": surfaces_for(cmd),
        // GAP-015: how to read the answer, not just how to call.
        "output_schema": success_envelope_schema(data_schema_for(cmd)),
        "error_schema": error_envelope_schema(),
    });
    if json {
        print_success_json(data)?;
    } else {
        let pretty = serde_json::to_string_pretty(&data).unwrap_or_default();
        crate::output::writeln_stdout(pretty)?;
    }
    Ok(())
}
