// SPDX-License-Identifier: MIT OR Apache-2.0
//! Canonical config key catalog and `config list-keys` payload.

use serde_json::{json, Value};

use super::super::paths::config_file;
use crate::error::CliError;

/// Canonical key catalog (DRY: list-keys + schema description source).
pub const CONFIG_KEYS: &[&str] = &[
    "lang",
    "timeout",
    "artifacts_dir",
    "ignore_robots",
    "namespace",
    "encryption_key",
    "color",
    "log_level",
    "log_to_file",
    "max_log_files",
    "log_rotation",
    "chrome_path",
    "lighthouse_path",
    "ffmpeg_path",
    "lighthouse_timeout_secs",
    "ffmpeg_timeout_secs",
    "openrouter_api_key",
    "llm_base_url",
    "llm_model",
    "cache_backend",
    "cache_redis_url",
    "search_base_url",
    "lightpanda_startup_timeout_secs",
    "lightpanda_session_timeout_secs",
    "max_json_file_bytes",
    "max_ndjson_line_bytes",
    "max_cli_json_payload_bytes",
    "default_jpeg_quality",
    "event_pump_slice_ms",
    "screencast_jpeg_quality",
    "interact_settle_ms",
    "dialog_settle_ms",
    "cdp_connection_probe_timeout_secs",
    "http_ssrf_mode",
    "http_timeout_secs",
    "http_connect_timeout_secs",
    "scrape_max_body_bytes",
    "llm_http_timeout_secs",
    "redis_allow_remote",
    "redis_connect_timeout_secs",
    "chrome_search_paths",
    "allowed_roots",
    "robots_loopback_exempt",
];

/// Every supported key: the hand-written catalog plus promoted policy knobs.
pub fn all_config_keys() -> Vec<&'static str> {
    let mut keys: Vec<&'static str> = CONFIG_KEYS.to_vec();
    keys.extend_from_slice(crate::xdg::policy::POLICY_KEYS);
    keys
}

/// Pipe-joined key list for schema / agent descriptions.
pub fn config_keys_description() -> String {
    all_config_keys().join("|")
}

/// List supported XDG config keys (GAP-018 catalog + GAP-048 policy knobs).
pub fn config_list_keys() -> Result<Value, CliError> {
    let mut keys: Vec<Value> = base_key_entries();
    keys.extend(crate::xdg::policy::policy_list_entries());
    Ok(json!({
        "keys": keys,
        "path": config_file()?.display().to_string(),
    }))
}

/// Hand-written key catalog (non-policy knobs and secrets).
fn base_key_entries() -> Vec<Value> {
    let entries = json!([
            {"key": "lang", "default": null, "description": "Message locale override (en|pt-BR; bare pt rejected)"},
            {"key": "timeout", "default": 0, "description": "Global timeout seconds"},
            {"key": "artifacts_dir", "default": null, "description": "Artifacts output directory"},
            {"key": "ignore_robots", "default": false, "description": "Default robots ignore (flags still required)"},
            {"key": "namespace", "default": null, "description": "Isolated state namespace"},
            {"key": "encryption_key", "default": null, "description": "Session encryption key material"},
            {"key": "color", "default": null, "description": "ANSI colors on human stderr"},
            {"key": "log_level", "default": crate::constants::DEFAULT_LOG_LEVEL, "description": "Tracing EnvFilter when argv flags quiet (no RUST_LOG)"},
            {"key": "log_to_file", "default": false, "description": "Rotated local JSON logs under XDG state (never remote)"},
            {"key": "max_log_files", "default": crate::constants::DEFAULT_MAX_LOG_FILES, "description": "Retained rotated log files (1..=90)"},
            {"key": "log_rotation", "default": crate::constants::DEFAULT_LOG_ROTATION, "description": "Rolling policy: daily|hourly|never"},
            {"key": "chrome_path", "default": null, "description": "Absolute Chrome/Chromium path"},
            {"key": "lighthouse_path", "default": null, "description": "Absolute lighthouse CLI path"},
            {"key": "ffmpeg_path", "default": null, "description": "Absolute ffmpeg path (optional screencast encode)"},
            {"key": "lighthouse_timeout_secs", "default": crate::constants::DEFAULT_LIGHTHOUSE_TIMEOUT_SECS, "description": "Wall-clock lighthouse CLI timeout (seconds, 1..=3600)"},
            {"key": "ffmpeg_timeout_secs", "default": crate::constants::DEFAULT_FFMPEG_ENCODE_TIMEOUT_SECS, "description": "Wall-clock ffmpeg encode timeout (seconds, 1..=3600)"},
            {"key": "openrouter_api_key", "default": null, "description": "LLM API key (stored 0600)"},
            {"key": "llm_base_url", "default": null, "description": "OpenAI-compatible base URL"},
            {"key": "llm_model", "default": null, "description": "Default LLM model id"},
            {"key": "cache_backend", "default": "sqlite", "description": "sqlite|memory|redis"},
            {"key": "cache_redis_url", "default": null, "description": "Redis URL when backend=redis"},
            {"key": "search_base_url", "default": crate::constants::DEFAULT_SEARCH_BASE_URL, "description": "HTML search endpoint base (?q= appended)"},
            {"key": "lightpanda_startup_timeout_secs", "default": crate::constants::LIGHTPANDA_STARTUP_TIMEOUT_SECS, "description": "Lightpanda process startup wait (seconds)"},
            {"key": "lightpanda_session_timeout_secs", "default": crate::constants::LIGHTPANDA_SESSION_TIMEOUT_SECS, "description": "Lightpanda --timeout session max (seconds, 1..=604800)"},
            {"key": "max_json_file_bytes", "default": crate::constants::DEFAULT_MAX_JSON_FILE_BYTES, "description": "Max bytes for JSON/NDJSON script or manifest files"},
            {"key": "max_ndjson_line_bytes", "default": crate::constants::DEFAULT_MAX_NDJSON_LINE_BYTES, "description": "Max bytes for one NDJSON line (run scripts / traces)"},
            {"key": "max_cli_json_payload_bytes", "default": crate::constants::DEFAULT_MAX_CLI_JSON_PAYLOAD_BYTES, "description": "Max bytes for CLI flag JSON payloads"},
            {"key": "default_jpeg_quality", "default": crate::constants::DEFAULT_JPEG_QUALITY, "description": "JPEG quality 1..=100 when grab omits --quality"},
            {"key": "event_pump_slice_ms", "default": crate::constants::DEFAULT_EVENT_PUMP_SLICE_MS, "description": "Wait/eval event pump slice (milliseconds)"},
            {"key": "screencast_jpeg_quality", "default": crate::constants::DEFAULT_SCREENCAST_JPEG_QUALITY, "description": "Screencast CDP JPEG quality 1..=100"},
            {"key": "interact_settle_ms", "default": crate::constants::DEFAULT_INTERACT_SETTLE_MS, "description": "UI settle delay after click/type/extension (ms)"},
            {"key": "dialog_settle_ms", "default": crate::constants::DEFAULT_DIALOG_SETTLE_MS, "description": "Max wait after JS dialog answer for javascriptDialogClosed (ms, GAP-054)"},
            {"key": "cdp_connection_probe_timeout_secs", "default": crate::constants::CDP_CONNECTION_PROBE_TIMEOUT_SECS, "description": "CDP Browser.getVersion liveness probe timeout (seconds)"},
            {"key": "http_ssrf_mode", "default": "strict", "description": "HTTP SSRF policy: strict|allow_loopback|off"},
            {"key": "http_timeout_secs", "default": crate::constants::DEFAULT_HTTP_TIMEOUT_SECS, "description": "Shared HTTP client total timeout (seconds)"},
            {"key": "http_connect_timeout_secs", "default": crate::constants::DEFAULT_HTTP_CONNECT_TIMEOUT_SECS, "description": "HTTP connect-phase timeout (seconds)"},
            {"key": "scrape_max_body_bytes", "default": crate::constants::DEFAULT_SCRAPE_MAX_BODY_BYTES, "description": "Max HTTP scrape body bytes"},
            {"key": "llm_http_timeout_secs", "default": crate::constants::DEFAULT_LLM_HTTP_TIMEOUT_SECS, "description": "LLM/webhook blocking HTTP timeout (seconds)"},
            {"key": "redis_allow_remote", "default": false, "description": "Allow non-loopback Redis hosts (default false)"},
            {"key": "redis_connect_timeout_secs", "default": crate::constants::REDIS_CONNECT_TIMEOUT_SECS, "description": "Redis TCP connect timeout (seconds)"},
            {"key": "robots_loopback_exempt", "default": true, "description": "Loopback hosts skip robots.txt (set false to enforce against localhost)"},
            {"key": "allowed_roots", "default": null, "description": "Extra allowed roots for local reads and artifact writes (platform-separated); defaults cover cwd, XDG dirs and temp"},
            {"key": "chrome_search_paths", "default": null, "description": "Ordered Chrome/Chromium discovery paths (platform-separated); empty uses the built-in per-OS layout"},
    ]);
    entries.as_array().cloned().unwrap_or_default()
}
