// SPDX-License-Identifier: MIT OR Apache-2.0
//! JSON Schema fragments (scrape_tools).
use serde_json::{json, Value};

use super::schema_object;

pub(crate) fn schema_for(cmd: &str) -> Option<Value> {
    Some(match cmd {
        "scrape" => schema_object(
            "Navigate and return body text / formats (local HTTP or CDP scrape)",
            json!({
                "url": { "type": "string" },
                "format": {
                    "oneOf": [
                        {
                            "type": "string",
                            "enum": [
                                "text", "markdown", "html", "raw-html", "links", "metadata",
                                "screenshot", "summary", "product", "branding"
                            ]
                        },
                        {
                            "type": "array",
                            "items": {
                                "type": "string",
                                "enum": [
                                    "text", "markdown", "html", "raw-html", "links", "metadata",
                                    "screenshot", "summary", "product", "branding"
                                ]
                            }
                        }
                    ],
                    "description": "Single format, CSV multi-format, or array (GAP-009); browser applies via outerHTML"
                },
                "formats": {
                    "description": "Alias of format for multi-value (GAP-018)",
                    "oneOf": [
                        { "type": "string" },
                        { "type": "array", "items": { "type": "string" } }
                    ]
                },
                "engine": {
                    "type": "string",
                    "enum": ["http", "browser"],
                    "description": "Default browser (CDP)"
                },
                "only_main_content": { "type": "boolean" },
                "webhook_url": {
                    "type": "string",
                    "description": "Optional one-shot operator POST of result data (not product telemetry)"
                }
            }),
            &["url"],
        ),
        "batch-scrape" => schema_object(
            "Scrape many URLs from a file (HTTP or browser engine, one-shot)",
            json!({
                "urls_file": { "type": "string", "description": "Path to file with one URL per line" },
                "format": {
                    "type": "string",
                    "enum": ["text", "markdown", "html", "links", "metadata", "raw-html", "screenshot", "summary", "product", "branding"],
                    "description": "Single format or CSV multi-format when supported"
                },
                "engine": {
                    "type": "string",
                    "enum": ["http", "browser"],
                    "description": "Default http; browser uses CDP per URL (GAP-010)"
                },
                "concurrency": { "type": "integer", "minimum": 1 }
            }),
            &["urls_file"],
        ),
        "crawl" => schema_object(
            "Crawl from a seed URL (HTTP BFS or browser, one-shot)",
            json!({
                "url": { "type": "string" },
                "limit": { "type": "integer", "minimum": 1 },
                "max_pages": { "type": "integer", "minimum": 1, "description": "Alias of limit" },
                "max_depth": { "type": "integer", "minimum": 0 },
                "format": { "type": "string" },
                "same_host": { "type": "boolean" },
                "engine": {
                    "type": "string",
                    "enum": ["http", "browser"],
                    "description": "Default http; browser engine for JS-rendered crawl (GAP-010)"
                }
            }),
            &["url"],
        ),
        "map" => schema_object(
            "Map site URLs from a seed (HTTP)",
            json!({
                "url": { "type": "string" },
                "limit": { "type": "integer", "minimum": 1 },
                "max_depth": { "type": "integer", "minimum": 0 }
            }),
            &["url"],
        ),
        "search" => schema_object(
            "Local search (HTTP SERP links or URL map)",
            json!({
                "query": { "type": "string" },
                "limit": { "type": "integer", "minimum": 1 }
            }),
            &["query"],
        ),
        "parse" => schema_object(
            "Parse a local file (html/md/txt/pdf/docx/xlsx)",
            json!({
                "path": { "type": "string" },
                "redact_pii": { "type": "boolean" }
            }),
            &["path"],
        ),
        "qr" => schema_object(
            "QR encode/decode one-shot (no Chrome)",
            json!({
                "action": { "type": "string", "enum": ["encode", "decode"] },
                "text": { "type": "string" },
                "format": { "type": "string", "enum": ["png", "svg", "terminal"] },
                "path": { "type": "string" }
            }),
            &["action"],
        ),
        "find-paths" => schema_object(
            "Discover filesystem paths (fd-like; no Chrome)",
            json!({
                "pattern": { "type": "string" },
                "paths": { "type": "array", "items": { "type": "string" } },
                "extension": { "type": "string" },
                "hidden": { "type": "boolean" },
                "no_ignore": { "type": "boolean" },
                "max_depth": { "type": "integer" },
                "type": { "type": "string", "enum": ["f", "d"] },
                "limit": { "type": "integer" },
                "glob": { "type": "string", "description": "Shell-style glob e.g. **/*.rs" }
            }),
            &[],
        ),
        "sg-scan" => schema_object(
            "Structural lint scan for forbidden product patterns (one-shot; no Chrome)",
            json!({
                "paths": { "type": "array", "items": { "type": "string" } },
                "limit": { "type": "integer" }
            }),
            &[],
        ),
        "sg-rewrite" => schema_object(
            "Structural rewrite dry-run/apply for safe patterns only (one-shot; no Chrome)",
            json!({
                "paths": { "type": "array", "items": { "type": "string" } },
                "apply": { "type": "boolean" }
            }),
            &[],
        ),
        "sheet-write" => schema_object(
            "Write XLSX from CSV/JSON (write-only; no Chrome)",
            json!({
                "input": { "type": "string" },
                "out": { "type": "string" },
                "sheet": { "type": "string" }
            }),
            &["input", "out"],
        ),
        _ => return None,
    })
}
