// SPDX-License-Identifier: MIT OR Apache-2.0
//! Shared JSON / NDJSON helpers (RFC 8259 + I-JSON-oriented CLI contracts).
//!
//! Rules (`rules_rust_json_e_ndjson`):
//! - Machine-to-machine interop uses strict RFC 8259 via `serde_json` (not JSON5)
//! - Strip UTF-8 BOM before parse (serde_json rejects `\u{FEFF}` at root)
//! - Bound untrusted file / line size before allocating full buffers
//! - NDJSON = one complete JSON value per LF line; compact (no pretty print)
//! - Prefer typed structs at domain boundaries; `Value` only for dynamic agent steps
//!
//! Content-Type notes (this product is a **CLI**, not an HTTP server):
//! - stdout JSON envelopes are single-line compact objects (`application/json` semantics)
//! - `--json-steps` emits NDJSON (`application/x-ndjson` / `application/jsonl` semantics)
//! - HTTP Content-Type headers are N/A until an HTTP surface exists

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use crate::error::{CliError, ErrorKind};

mod file_io;

pub use file_io::{
    read_json_file, read_json_value_file, read_text_file_limited, read_text_file_limited_lossy,
    write_json_file_atomic,
};

/// Default ceiling for a single JSON / NDJSON **script or manifest** file.
///
/// Prefer [`crate::xdg::resolve_max_json_file_bytes`] at product call sites so
/// operators can raise/lower via `config set max_json_file_bytes`.
pub const MAX_JSON_FILE_BYTES: u64 = crate::constants::DEFAULT_MAX_JSON_FILE_BYTES;

/// Default ceiling for one NDJSON line (DoS / accidental multi-MB line).
///
/// Prefer [`crate::xdg::resolve_max_ndjson_line_bytes`] at product call sites.
pub const MAX_NDJSON_LINE_BYTES: usize = crate::constants::DEFAULT_MAX_NDJSON_LINE_BYTES;

/// Default ceiling for CLI flag payloads (`--fields-json`, cookie JSON, etc.).
///
/// Prefer [`crate::xdg::resolve_max_cli_json_payload_bytes`] at product call sites.
pub const MAX_CLI_JSON_PAYLOAD_BYTES: usize = crate::constants::DEFAULT_MAX_CLI_JSON_PAYLOAD_BYTES;

/// UTF-8 BOM (`U+FEFF`) as bytes.
const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

/// Strip a leading UTF-8 BOM from a string slice (idempotent).
///
/// Windows editors and some HTTP clients still emit BOM; RFC 8259 JSON does not
/// allow it, and `serde_json::from_str` fails with a syntax error at column 1.
#[inline]
pub fn strip_utf8_bom(s: &str) -> &str {
    s.strip_prefix('\u{FEFF}').unwrap_or(s)
}

/// Strip a leading UTF-8 BOM from a byte slice.
#[inline]
pub fn strip_utf8_bom_bytes(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(UTF8_BOM).unwrap_or(bytes)
}

/// Parse JSON from a UTF-8 string after BOM strip.
pub fn from_str<T: DeserializeOwned>(s: &str) -> Result<T, serde_json::Error> {
    serde_json::from_str(strip_utf8_bom(s.trim_start_matches('\u{FEFF}')))
}

/// Parse JSON from bytes after BOM strip (validates UTF-8 when needed via `from_slice`).
pub fn from_slice<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, serde_json::Error> {
    serde_json::from_slice(strip_utf8_bom_bytes(bytes))
}

/// Parse a dynamic [`Value`] from a string (BOM-aware).
pub fn value_from_str(s: &str) -> Result<Value, serde_json::Error> {
    from_str(s)
}

/// Map a `serde_json` parse error into a domain [`CliError`] with context.
///
/// Takes `&serde_json::Error` (Display/line/column only) — never consumes the
/// error value when the caller still needs it (rules: smallest permission).
pub fn map_parse_err(ctx: &str, e: &serde_json::Error) -> CliError {
    CliError::new(
        ErrorKind::Data,
        format!(
            "{ctx}: invalid JSON (line {} column {}): {e}",
            e.line(),
            e.column()
        ),
    )
}

/// Parse CLI flag / inline payload JSON with size guard + BOM strip.
///
/// Ceiling comes from XDG `max_cli_json_payload_bytes` (default
/// [`MAX_CLI_JSON_PAYLOAD_BYTES`]). Use [`parse_cli_json_value_max`] only when
/// the caller already resolved a budget (e.g. tests with a fixed max).
pub fn parse_cli_json_value(raw: &str, ctx: &str) -> Result<Value, CliError> {
    parse_cli_json_value_max(raw, ctx, crate::xdg::resolve_max_cli_json_payload_bytes())
}

/// Parse CLI flag / inline payload JSON with an explicit size ceiling + BOM strip.
pub fn parse_cli_json_value_max(raw: &str, ctx: &str, max_bytes: usize) -> Result<Value, CliError> {
    if raw.len() > max_bytes {
        return Err(CliError::with_suggestion(
            ErrorKind::Data,
            format!(
                "{ctx}: JSON payload too large ({} bytes > {max_bytes})",
                raw.len()
            ),
            crate::i18n::suggestion_key("cli_json_payload_too_large", None),
        ));
    }
    value_from_str(raw).map_err(|e| map_parse_err(ctx, &e))
}

/// Project a JSON object to a subset of keys (CSV / space-separated).
///
/// Agent-native anti-token helper shared by local media pipelines (`image`, `video`).
/// Unknown keys are ignored. Always retains `action` for agent routing when present.
///
/// Optional `aliases` maps input select tokens to canonical object keys
/// (e.g. `tags` → `exif` for image EXIF projection).
pub fn project_fields(value: Value, select: Option<&str>, aliases: &[(&str, &str)]) -> Value {
    let Some(sel) = select.map(str::trim).filter(|s| !s.is_empty()) else {
        return value;
    };
    let Some(obj) = value.as_object() else {
        return value;
    };
    let mut out = serde_json::Map::new();
    if let Some(a) = obj.get("action") {
        out.insert("action".into(), a.clone());
    }
    for key in sel.split([',', ' ']) {
        let key = key.trim();
        if key.is_empty() || key == "action" {
            continue;
        }
        let mut canonical = key;
        for (alias, target) in aliases {
            if key == *alias {
                canonical = target;
                break;
            }
        }
        if let Some(v) = obj.get(canonical) {
            out.insert(canonical.to_string(), v.clone());
        }
    }
    Value::Object(out)
}

/// [`project_fields`] with no alias table.
#[inline]
pub fn project_fields_plain(value: Value, select: Option<&str>) -> Value {
    project_fields(value, select, &[])
}

/// Reject an NDJSON line that exceeds the **default** per-line ceiling.
///
/// Prefer [`check_ndjson_line_len_max`] with
/// [`crate::xdg::resolve_max_ndjson_line_bytes`] on product paths.
pub fn check_ndjson_line_len(line: &str, lineno: usize) -> Result<(), CliError> {
    check_ndjson_line_len_max(line, lineno, MAX_NDJSON_LINE_BYTES)
}

/// Reject an NDJSON line that exceeds an explicit per-line ceiling.
pub fn check_ndjson_line_len_max(
    line: &str,
    lineno: usize,
    max_bytes: usize,
) -> Result<(), CliError> {
    if line.len() > max_bytes {
        return Err(CliError::with_suggestion(
            ErrorKind::Data,
            format!(
                "NDJSON line {lineno}: line too large ({} bytes > {max_bytes})",
                line.len()
            ),
            crate::i18n::suggestion_key("ndjson_line_too_large", None),
        ));
    }
    Ok(())
}

/// Serialize `value` as **compact** JSON (machine interop; never pretty).
pub fn to_compact_string<T: Serialize>(value: &T) -> Result<String, CliError> {
    serde_json::to_string(value)
        .map_err(|e| CliError::new(ErrorKind::Software, format!("json encode: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn strips_bom_from_str() {
        let with_bom = "\u{FEFF}{\"ok\":true}";
        let v: Value = from_str(with_bom).expect("bom parse");
        assert_eq!(v, json!({"ok": true}));
    }

    #[test]
    fn strips_bom_from_bytes() {
        let mut bytes = UTF8_BOM.to_vec();
        bytes.extend_from_slice(br#"{"n":1}"#);
        let v: Value = from_slice(&bytes).expect("bom bytes");
        assert_eq!(v["n"], 1);
    }

    #[test]
    fn rejects_oversized_cli_payload() {
        let huge = format!("{{\"x\":\"{}\"}}", "a".repeat(MAX_CLI_JSON_PAYLOAD_BYTES));
        let err = parse_cli_json_value(&huge, "test").unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Data);
    }

    #[test]
    fn ndjson_line_limit() {
        let line = "x".repeat(MAX_NDJSON_LINE_BYTES + 1);
        assert!(check_ndjson_line_len(&line, 1).is_err());
        assert!(check_ndjson_line_len("{}", 1).is_ok());
    }

    #[test]
    fn compact_roundtrip() {
        let v = json!({"schema_version": 1, "ok": true});
        let s = to_compact_string(&v).unwrap();
        assert!(!s.contains('\n'));
        assert!(!s.contains("  "));
        let back: Value = from_str(&s).unwrap();
        assert_eq!(back, v);
    }
}
