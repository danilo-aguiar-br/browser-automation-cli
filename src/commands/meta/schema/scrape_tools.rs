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
        "image" => schema_object(
            "Local image pipeline one-shot (no Chrome): info/convert/resize/download/exif",
            json!({
                "action": {
                    "type": "string",
                    "enum": ["info", "convert", "resize", "download", "exif"]
                },
                "path": { "type": "string" },
                "stdin": { "type": "boolean" },
                "format": { "type": "string", "enum": ["png", "jpeg", "webp", "gif"] },
                "quality": { "type": "integer", "minimum": 1, "maximum": 100 },
                "out": { "type": "string" },
                "width": { "type": "integer", "minimum": 1 },
                "height": { "type": "integer", "minimum": 1 },
                "keep_aspect": { "type": "boolean" },
                "url": { "type": "string" },
                "max_bytes": { "type": "integer", "minimum": 1 },
                "require_image": { "type": "boolean" },
                "allow_non_image": { "type": "boolean" },
                "include_gps": { "type": "boolean" },
                "select": {
                    "type": "string",
                    "description": "CSV field projection for info (format,width,height,path,sha256,…)"
                },
                "strip_exif": { "type": "boolean" },
                "keep_exif": { "type": "boolean" }
            }),
            &["action"],
        ),
        "audio" => schema_object(
            "Local audio pipeline one-shot (no Chrome): info/download/convert/trim",
            json!({
                "action": {
                    "type": "string",
                    "enum": ["info", "download", "convert", "trim"]
                },
                "path": { "type": "string" },
                "stdin": { "type": "boolean" },
                "url": { "type": "string" },
                "format": {
                    "type": "string",
                    "enum": ["mp3", "m4a", "aac", "ogg", "opus", "flac", "wav"]
                },
                "out": { "type": "string" },
                "codec": { "type": "string" },
                "bitrate": { "type": "string" },
                "sample_rate": { "type": "integer", "minimum": 1 },
                "channels": { "type": "integer", "minimum": 1 },
                "audio_stream": { "type": "integer", "minimum": 0 },
                "strip_metadata": { "type": "boolean" },
                "start": {
                    "type": "number",
                    "description": "Trim start seconds (action=trim)"
                },
                "duration": {
                    "type": "number",
                    "description": "Trim duration seconds (action=trim; exclusive with to)"
                },
                "to": {
                    "type": "number",
                    "description": "Trim end seconds (action=trim; exclusive with duration)"
                },
                "max_bytes": { "type": "integer", "minimum": 1 },
                "require_audio": { "type": "boolean" },
                "allow_non_audio": { "type": "boolean" },
                "select": {
                    "type": "string",
                    "description": "CSV field projection; aliases: format→container|container_out, bytes|size→size_bytes|bytes_out, path→path|path_out, duration→duration_secs, codec→audio_codec"
                }
            }),
            &["action"],
        ),
        "video" => schema_object(
            "Local video pipeline one-shot (no Chrome): info/download/convert/to-mp3/trim/thumbnail/manifest",
            json!({
                "action": {
                    "type": "string",
                    "enum": ["info", "download", "convert", "to-mp3", "trim", "thumbnail", "manifest"]
                },
                "base_url": {
                    "type": "string",
                    "description": "Manifest URL used to absolutise relative URIs (action=manifest)"
                },
                "path": { "type": "string" },
                "stdin": { "type": "boolean" },
                "url": { "type": "string" },
                "format": {
                    "type": "string",
                    "enum": ["mp4", "webm", "mkv", "mov", "avi", "m4v"]
                },
                "out": { "type": "string" },
                "video_codec": { "type": "string" },
                "audio_codec": { "type": "string" },
                "crf": { "type": "integer", "minimum": 1, "maximum": 51 },
                "no_faststart": {
                    "type": "boolean",
                    "description": "Disable MP4-family moov-before-mdat (default: faststart applied)"
                },
                "strip_metadata": { "type": "boolean" },
                "drop_audio": { "type": "boolean" },
                "bitrate": { "type": "string" },
                "audio_stream": { "type": "integer", "minimum": 0 },
                "start": {
                    "type": "number",
                    "description": "Trim start seconds (action=trim)"
                },
                "duration": {
                    "type": "number",
                    "description": "Trim duration seconds (action=trim; exclusive with to)"
                },
                "to": {
                    "type": "number",
                    "description": "Trim end seconds (action=trim; exclusive with duration)"
                },
                "at": {
                    "type": "number",
                    "description": "Thumbnail timestamp seconds (action=thumbnail; default 0)"
                },
                "max_bytes": { "type": "integer", "minimum": 1 },
                "require_video": { "type": "boolean" },
                "allow_non_video": { "type": "boolean" },
                "select": {
                    "type": "string",
                    "description": "CSV field projection (path,container,streams,duration_secs,sha256,…); aliases: format→container|container_out, bytes|size→size_bytes|bytes_out, path→path|path_out, duration→duration_secs"
                }
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
