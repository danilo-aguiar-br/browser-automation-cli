// SPDX-License-Identifier: MIT OR Apache-2.0
//! Run/exec command inventory (GAP-001 / GAP-017 single source of truth).

/// Commands dispatched by `run`/`exec` (GAP-001 / GAP-017 single source of truth).
pub const RUN_DISPATCHED_CMDS: &[&str] = &[
    "goto",
    "wait",
    "hover",
    "drag",
    "fill-form",
    "fill_form",
    "select-option",
    "select_option",
    "pick",
    "upload",
    "back",
    "forward",
    "reload",
    "view",
    "press",
    "write",
    "keys",
    "type",
    "click-at",
    "click_at",
    "eval",
    "grab",
    "print-pdf",
    "print_pdf",
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
    "devtools3p-list",
    "devtools3p-exec",
    "devtools3p_list",
    "devtools3p_exec",
    "webmcp-list",
    "webmcp-exec",
    "webmcp_list",
    "webmcp_exec",
];

/// Top-level browser-adjacent commands intentionally excluded from `run` (GAP-007 / GAP-017).
/// Each entry is `(cmd, reason)`.
pub const INTENTIONAL_RUN_EXCLUDE: &[(&str, &str)] = &[
    (
        "extension-install",
        "install requires Chrome relaunch with --load-extension; use top-level extension install",
    ),
    (
        "extension-uninstall",
        "uninstall is top-level one-shot; use extension uninstall outside run",
    ),
    ("doctor", "meta command; not a browser step"),
    ("commands", "meta discovery; not a browser step"),
    ("schema", "meta discovery; not a browser step"),
    ("version", "meta; not a browser step"),
    ("config", "XDG config; not a browser step"),
    ("completions", "shell completions; not a browser step"),
    ("man", "man page generation; not a browser step"),
    (
        "mitm",
        "MITM is a separate one-shot surface; use mitm capture-url or --mitm with browser cmds",
    ),
    (
        "workflow",
        "workflow journal is top-level; not an in-session browser step",
    ),
    (
        "batch-scrape",
        "batch-scrape is top-level HTTP/browser pool; use scrape steps or top-level batch-scrape",
    ),
    (
        "crawl",
        "crawl is top-level; use top-level crawl or multi-step goto/scrape",
    ),
    ("map", "map is top-level discovery; not an in-session step"),
    ("search", "search is top-level; not an in-session step"),
    ("parse", "path-light parse; not a browser session step"),
    ("qr", "path-light QR; not a browser session step"),
    (
        "image",
        "path-light image pipeline; not a browser session step",
    ),
    (
        "video",
        "path-light video pipeline; not a browser session step",
    ),
    (
        "audio",
        "path-light audio pipeline; not a browser session step",
    ),
    (
        "find-paths",
        "path-light discovery; not a browser session step",
    ),
    (
        "sg-scan",
        "path-light structural scan; not a browser session step",
    ),
    (
        "sg-rewrite",
        "path-light rewrite; not a browser session step",
    ),
    (
        "sheet-write",
        "path-light sheet; not a browser session step",
    ),
    ("monitor", "monitor check is top-level one-shot"),
    (
        "record",
        "record owns the whole session for its recording window; use it top-level and replay its NDJSON with run --script",
    ),
    ("run", "nested run is not supported"),
    ("exec", "nested exec is not supported"),
];

/// Human-readable list of dispatched cmds for suggestions (GAP-017).
pub fn run_supported_suggestion() -> String {
    format!("Supported: {}", RUN_DISPATCHED_CMDS.join(" "))
}

/// Suggestion for an unknown / excluded `run` script cmd (agent-native).
///
/// When `cmd` is in [`INTENTIONAL_RUN_EXCLUDE`], explain why and point to
/// top-level use; otherwise list dispatched cmds.
pub fn run_unknown_cmd_suggestion(cmd: &str) -> String {
    if let Some((_, reason)) = INTENTIONAL_RUN_EXCLUDE.iter().find(|(c, _)| *c == cmd) {
        return format!(
            "{cmd} is intentionally excluded from run: {reason}. {}",
            run_supported_suggestion()
        );
    }
    run_supported_suggestion()
}
