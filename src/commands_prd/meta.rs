//! Machine-readable command list and JSON Schema fragments for agents.

use serde_json::{json, Value};

use crate::envelope::print_success_json;
use crate::error::{CliError, ErrorKind};

/// Full CLI command surface registered for agents (`commands --json`).
pub const COMMANDS: &[&str] = &[
    "doctor",
    "commands",
    "schema",
    "version",
    "goto",
    "view",
    "press",
    "click-at",
    "write",
    "keys",
    "type",
    "wait",
    "hover",
    "drag",
    "fill-form",
    "upload",
    "back",
    "forward",
    "reload",
    "eval",
    "grab",
    "run",
    "exec",
    "extract",
    "text",
    "scroll",
    "cookie",
    "attr",
    "assert",
    "console",
    "net",
    "page",
    "dialog",
    "scrape",
    "emulate",
    "resize",
    "perf",
    "lighthouse",
    "screencast",
    "heap",
    "extension",
    "devtools3p",
    "webmcp",
    "completions",
];

/// Default-ON DevTools parity commands that MUST appear in `COMMANDS`.
pub const PARITY_DEFAULT_ON_REQUIRED: &[&str] = &[
    "goto",
    "view",
    "press",
    "write",
    "keys",
    "type",
    "wait",
    "hover",
    "drag",
    "fill-form",
    "upload",
    "back",
    "forward",
    "reload",
    "eval",
    "grab",
    "console",
    "net",
    "page",
    "dialog",
    "emulate",
    "resize",
    "perf",
    "lighthouse",
    "run",
    "exec",
    "doctor",
    "commands",
    "schema",
    "version",
    "completions",
];

/// Official DevTools tool-ref name → CLI subcommand (agent discovery).
pub const DEVTOOLS_TOOL_MAP: &[(&str, &str)] = &[
    ("click", "press"),
    ("drag", "drag"),
    ("fill", "write"),
    ("fill_form", "fill-form"),
    ("handle_dialog", "dialog"),
    ("hover", "hover"),
    ("press_key", "keys"),
    ("type_text", "type"),
    ("upload_file", "upload"),
    ("click_at", "click-at"),
    ("navigate_page", "goto|back|forward|reload"),
    ("new_page", "page new"),
    ("list_pages", "page list"),
    ("select_page", "page select"),
    ("close_page", "page close"),
    ("wait_for", "wait"),
    ("emulate", "emulate"),
    ("resize_page", "resize"),
    ("performance_start_trace", "perf start"),
    ("performance_stop_trace", "perf stop"),
    ("performance_analyze_insight", "perf insight"),
    ("list_network_requests", "net list"),
    ("get_network_request", "net get"),
    ("evaluate_script", "eval"),
    ("list_console_messages", "console list"),
    ("get_console_message", "console get"),
    ("take_screenshot", "grab"),
    ("take_snapshot", "view"),
    ("lighthouse_audit", "lighthouse"),
    ("screencast_start", "screencast start"),
    ("screencast_stop", "screencast stop"),
    ("take_heapsnapshot", "heap take"),
    ("close_heapsnapshot", "heap close"),
    ("compare_heapsnapshots", "heap compare"),
    ("get_heapsnapshot_summary", "heap summary"),
    ("get_heapsnapshot_details", "heap details"),
    ("get_heapsnapshot_class_nodes", "heap class-nodes"),
    ("get_heapsnapshot_dominators", "heap dominators"),
    ("get_heapsnapshot_duplicate_strings", "heap dup-strings"),
    ("get_heapsnapshot_edges", "heap edges"),
    ("get_heapsnapshot_retainers", "heap retainers"),
    ("get_heapsnapshot_retaining_paths", "heap paths"),
    ("get_heapsnapshot_object_details", "heap object-details"),
    ("install_extension", "extension install"),
    ("list_extensions", "extension list"),
    ("reload_extension", "extension reload"),
    ("trigger_extension_action", "extension trigger"),
    ("uninstall_extension", "extension uninstall"),
    ("list_3p_developer_tools", "devtools3p list"),
    ("execute_3p_developer_tool", "devtools3p exec"),
    ("list_webmcp_tools", "webmcp list"),
    ("execute_webmcp_tool", "webmcp exec"),
];

pub fn list_commands(json: bool) -> Result<(), CliError> {
    let map: Vec<Value> = DEVTOOLS_TOOL_MAP
        .iter()
        .map(|(tool, cli)| json!({ "tool": tool, "cli": cli }))
        .collect();
    let data = json!({
        "commands": COMMANDS,
        "schema_version": 1,
        "parity_default_on": PARITY_DEFAULT_ON_REQUIRED,
        "devtools_tool_map": map,
        "binary": "browser-automation-cli",
    });
    if json {
        print_success_json(data)?;
    } else {
        for c in COMMANDS {
            println!("{c}");
        }
    }
    Ok(())
}

fn schema_object(description: &str, properties: Value, required: &[&str]) -> Value {
    json!({
        "type": "object",
        "description": description,
        "properties": properties,
        "required": required,
        "additionalProperties": false,
    })
}

fn schema_for(cmd: &str) -> Option<Value> {
    let props = match cmd {
        "doctor" => schema_object(
            "Diagnose local Chrome install and one-shot readiness",
            json!({
                "offline": { "type": "boolean", "description": "Skip network probes" },
                "quick": { "type": "boolean", "description": "Skip live launch test" },
                "fix": { "type": "boolean", "description": "Apply safe repairs when possible" },
                "json": { "type": "boolean" }
            }),
            &[],
        ),
        "commands" => schema_object(
            "List available commands",
            json!({ "json": { "type": "boolean" } }),
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
        "goto" => schema_object(
            "Navigate to URL and wait for load (one-shot)",
            json!({
                "url": { "type": "string", "description": "Absolute URL or about:blank" }
            }),
            &["url"],
        ),
        "view" => schema_object(
            "Accessibility snapshot with @eN refs",
            json!({
                "verbose": { "type": "boolean" },
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
                "submit": { "type": "string", "description": "Optional key after type (e.g. Enter)" }
            }),
            &["target", "text"],
        ),
        "wait" => schema_object(
            "Wait ms and/or text and/or CSS selector and/or load state",
            json!({
                "ms": { "type": "integer", "minimum": 0 },
                "text": { "type": "string" },
                "selector": { "type": "string" },
                "state": {
                    "type": "string",
                    "enum": ["load", "domcontentloaded", "networkidle", "none"]
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
            "Drag from one target to another",
            json!({
                "from": { "type": "string" },
                "to": { "type": "string" }
            }),
            &["from", "to"],
        ),
        "fill-form" => schema_object(
            "Fill multiple fields from JSON array",
            json!({
                "json": {
                    "type": "string",
                    "description": "JSON array of {target,value} objects"
                }
            }),
            &["json"],
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
            "Evaluate JavaScript expression",
            json!({ "expression": { "type": "string" } }),
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
        "run" => schema_object(
            "Execute NDJSON multi-step script in one process",
            json!({ "script": { "type": "string", "description": "Path to .jsonl script" } }),
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
            "Extract text or attribute from target",
            json!({
                "target": { "type": "string" },
                "attr": { "type": "string" }
            }),
            &["target"],
        ),
        "text" => schema_object(
            "Extract visible text from target (PRD §7)",
            json!({
                "target": { "type": "string" }
            }),
            &["target"],
        ),
        "scroll" => schema_object(
            "Scroll window or element by pixel deltas",
            json!({
                "target": { "type": "string", "description": "Optional CSS/@eN" },
                "delta_x": { "type": "number" },
                "delta_y": { "type": "number" }
            }),
            &[],
        ),
        "cookie" => schema_object(
            "Cookie list/set/clear for the one-shot browser process",
            json!({
                "action": { "type": "string", "enum": ["list", "set", "clear"] },
                "url": { "type": "string" },
                "json": { "type": "string", "description": "JSON array for set" }
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
                "kind": { "type": "string", "enum": ["url", "text", "console"] }
            }),
            &["kind"],
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
        "page" => schema_object(
            "Page info or multi-tab list|new|select|close",
            json!({
                "action": {
                    "type": "string",
                    "enum": ["info", "list", "new", "select", "close"]
                },
                "url": { "type": "string" },
                "index": { "type": "integer", "minimum": 0 }
            }),
            &[],
        ),
        "dialog" => schema_object(
            "Accept or dismiss open dialog",
            json!({
                "action": { "type": "string", "enum": ["accept", "dismiss"] },
                "text": { "type": "string" }
            }),
            &["action"],
        ),
        "scrape" => schema_object(
            "Navigate and return body text",
            json!({ "url": { "type": "string" } }),
            &["url"],
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
                "viewport": { "type": "string" }
            }),
            &[],
        ),
        "resize" => schema_object(
            "Resize viewport",
            json!({
                "width": { "type": "integer" },
                "height": { "type": "integer" },
                "scale": { "type": "number" },
                "mobile": { "type": "boolean" }
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
        _ => return None,
    };
    Some(props)
}

pub fn schema_for_cmd(cmd: &str, json: bool) -> Result<(), CliError> {
    if !COMMANDS.contains(&cmd) {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("unknown command for schema: {cmd}"),
            "use `browser-automation-cli commands --json` to list commands",
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
    });
    if json {
        print_success_json(data)?;
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&data).unwrap_or_default()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parity_default_on_subset_of_commands() {
        for req in PARITY_DEFAULT_ON_REQUIRED {
            assert!(
                COMMANDS.contains(req),
                "parity command missing from COMMANDS: {req}"
            );
        }
    }

    #[test]
    fn commands_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for c in COMMANDS {
            assert!(seen.insert(*c), "duplicate command: {c}");
        }
    }
}
