// SPDX-License-Identifier: MIT OR Apache-2.0
//! JSON Schema fragments for crawl / map / search / batch-scrape.

use serde_json::{json, Value};

use super::schema_object;

pub(crate) fn schema_for(cmd: &str) -> Option<Value> {
    Some(match cmd {
        "batch-scrape" => schema_object(
            "Scrape many URLs from a file (HTTP or browser engine, one-shot)",
            json!({
                "urls_file": { "type": "string", "description": "Path to file with one URL per line" },
                "format": {
                    "type": "string",
                    "enum": ["text", "markdown", "html", "links", "metadata", "raw-html", "screenshot", "summary", "product", "branding"],
                    "description": "Single format or CSV multi-format when supported"
                },
                "only_main_content": { "type": "boolean", "description": "Prefer main/article content heuristics (parity with scrape)" },
                "engine": {
                    "type": "string",
                    "enum": ["http", "browser"],
                    "description": "Default http; browser uses CDP per URL (GAP-010)"
                },
                "concurrency": { "type": "integer", "minimum": 1 },
                "webhook_url": {
                    "type": "string",
                    "description": "Optional one-shot operator POST of the collection (not product telemetry)"
                }
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
                "only_main_content": { "type": "boolean", "description": "Prefer main/article content heuristics (parity with scrape); refused under engine=browser, which applies no content reduction" },
                "engine": {
                    "type": "string",
                    "enum": ["http", "browser"],
                    "description": "Default http; browser engine for JS-rendered crawl (GAP-010)"
                },
                "include_regex": { "type": "string", "description": "Include path/URL regex (repeatable on argv)" },
                "exclude_regex": { "type": "string", "description": "Exclude path/URL regex (repeatable on argv)" },
                "webhook_url": {
                    "type": "string",
                    "description": "Optional one-shot operator POST of the collection (not product telemetry)"
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
        _ => return None,
    })
}
