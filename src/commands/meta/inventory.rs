// SPDX-License-Identifier: MIT OR Apache-2.0
//! Command inventory for agents (GAP-017 / GAP-018).
//!
//! # Single source per field
//!
//! | Field | Source |
//! |-------|--------|
//! | `name` | [`COMMANDS`] |
//! | `category` | [`COMMAND_CATEGORIES`] (taxonomy is editorial) |
//! | `description` | clap `about`, else the hand-written schema fragment |
//! | `surfaces` | `schema::derive::surfaces_for` |
//!
//! Descriptions are never duplicated here: they are read back from the parser
//! so `commands` and `--help` cannot disagree.

use serde_json::{json, Value};

pub const COMMANDS: &[&str] = &[
    "doctor",
    "commands",
    "schema",
    "version",
    "locale",
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
    "submit",
    "fill-form",
    "select-option",
    "pick",
    "upload",
    "back",
    "forward",
    "reload",
    "eval",
    "grab",
    "print-pdf",
    "monitor",
    "run",
    "exec",
    "record",
    "extract",
    "text",
    "scroll",
    "cookie",
    "storage",
    "attr",
    "assert",
    "console",
    "net",
    "page",
    "dialog",
    "scrape",
    "batch-scrape",
    "crawl",
    "map",
    "search",
    "parse",
    "qr",
    "image",
    "video",
    "audio",
    "find-paths",
    "sg-scan",
    "sg-rewrite",
    "sheet-write",
    "mitm",
    "workflow",
    "config",
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
    "man",
];

/// Command → category taxonomy for agent grouping (covers every [`COMMANDS`] name).
pub const COMMAND_CATEGORIES: &[(&str, &str)] = &[
    ("doctor", "meta"),
    ("commands", "meta"),
    ("schema", "meta"),
    ("version", "meta"),
    ("locale", "meta"),
    ("config", "meta"),
    ("completions", "meta"),
    ("man", "meta"),
    ("goto", "navigation"),
    ("back", "navigation"),
    ("forward", "navigation"),
    ("reload", "navigation"),
    ("wait", "navigation"),
    ("page", "navigation"),
    ("view", "inspection"),
    ("text", "inspection"),
    ("attr", "inspection"),
    ("extract", "inspection"),
    ("eval", "inspection"),
    ("assert", "inspection"),
    ("press", "interaction"),
    ("click-at", "interaction"),
    ("write", "interaction"),
    ("keys", "interaction"),
    ("type", "interaction"),
    ("hover", "interaction"),
    ("drag", "interaction"),
    ("submit", "interaction"),
    ("fill-form", "interaction"),
    ("select-option", "interaction"),
    ("pick", "interaction"),
    ("upload", "interaction"),
    ("scroll", "interaction"),
    ("grab", "artifact"),
    ("print-pdf", "artifact"),
    ("monitor", "artifact"),
    ("screencast", "artifact"),
    ("qr", "artifact"),
    ("image", "artifact"),
    ("video", "artifact"),
    ("audio", "artifact"),
    ("sheet-write", "artifact"),
    ("run", "orchestration"),
    ("exec", "orchestration"),
    ("workflow", "orchestration"),
    ("record", "orchestration"),
    ("console", "capture"),
    ("net", "capture"),
    ("cookie", "capture"),
    ("storage", "capture"),
    ("dialog", "capture"),
    ("mitm", "capture"),
    ("scrape", "scraping"),
    ("batch-scrape", "scraping"),
    ("crawl", "scraping"),
    ("map", "scraping"),
    ("search", "scraping"),
    ("parse", "scraping"),
    ("emulate", "performance"),
    ("resize", "performance"),
    ("perf", "performance"),
    ("lighthouse", "performance"),
    ("heap", "performance"),
    ("extension", "extensions"),
    ("devtools3p", "extensions"),
    ("webmcp", "extensions"),
    ("find-paths", "filesystem"),
    ("sg-scan", "filesystem"),
    ("sg-rewrite", "filesystem"),
];

/// Category for one command (`"other"` when the taxonomy is missing an entry).
pub fn category_for(cmd: &str) -> &'static str {
    COMMAND_CATEGORIES
        .iter()
        .find(|(name, _)| *name == cmd)
        .map(|(_, category)| *category)
        .unwrap_or("other")
}

/// Description for one command: clap `about` first, schema fragment as fallback.
pub fn description_for(cmd: &str) -> String {
    if let Some(derived) = super::schema::derive::derive_command(cmd) {
        if !derived.about.is_empty() {
            return derived.about;
        }
    }
    super::schema::catalog_schema_for(cmd)
        .and_then(|f| {
            f.get("description")
                .and_then(|d| d.as_str())
                .map(str::to_string)
        })
        .unwrap_or_default()
}

/// Full command inventory as objects (`name`, `description`, `category`, `surfaces`).
pub fn command_objects() -> Vec<Value> {
    COMMANDS
        .iter()
        .map(|cmd| {
            json!({
                "name": cmd,
                "description": description_for(cmd),
                "category": category_for(cmd),
                "surfaces": super::schema::derive::surfaces_for(cmd),
            })
        })
        .collect()
}

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
    "select-option",
    "pick",
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
    "man",
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
    ("get_tab_id", "page tab-id"),
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
