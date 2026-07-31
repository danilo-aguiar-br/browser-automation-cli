// SPDX-License-Identifier: MIT OR Apache-2.0
//! `config get` read path (single key or full dump, secrets redacted).

use serde_json::{json, Value};

use super::super::config_io::load_config;
use super::super::config_model::ProductConfig;
use super::super::paths::config_file;
use super::validate::redacted_secret;
use crate::error::{CliError, ErrorKind};

fn get_one(cfg: &ProductConfig, key: &str) -> Result<Value, CliError> {
    let value: Value = match key {
        "lang" => json!(cfg.lang),
        "timeout" => json!(cfg.timeout),
        "artifacts_dir" => json!(cfg.artifacts_dir),
        "ignore_robots" => json!(cfg.ignore_robots),
        "namespace" => json!(cfg.namespace),
        "encryption_key" => json!(redacted_secret(&cfg.encryption_key)),
        "color" => json!(cfg.color),
        "log_level" => json!(cfg.log_level),
        "log_to_file" => json!(cfg.log_to_file),
        "max_log_files" => json!(cfg.max_log_files),
        "log_rotation" => json!(cfg.log_rotation),
        "chrome_path" => json!(cfg.chrome_path),
        "lighthouse_path" => json!(cfg.lighthouse_path),
        "ffmpeg_path" => json!(cfg.ffmpeg_path),
        "lighthouse_timeout_secs" => json!(cfg.lighthouse_timeout_secs),
        "ffmpeg_timeout_secs" => json!(cfg.ffmpeg_timeout_secs),
        "openrouter_api_key" => json!(redacted_secret(&cfg.openrouter_api_key)),
        "llm_base_url" => json!(cfg.llm_base_url),
        "llm_model" => json!(cfg.llm_model),
        "cache_backend" => json!(cfg.cache_backend),
        "cache_redis_url" => json!(redacted_secret(&cfg.cache_redis_url)),
        "search_base_url" => json!(cfg.search_base_url),
        "lightpanda_startup_timeout_secs" => json!(cfg.lightpanda_startup_timeout_secs),
        "lightpanda_session_timeout_secs" => json!(cfg.lightpanda_session_timeout_secs),
        "max_json_file_bytes" => json!(cfg.max_json_file_bytes),
        "max_ndjson_line_bytes" => json!(cfg.max_ndjson_line_bytes),
        "max_cli_json_payload_bytes" => json!(cfg.max_cli_json_payload_bytes),
        "default_jpeg_quality" => json!(cfg.default_jpeg_quality),
        "event_pump_slice_ms" => json!(cfg.event_pump_slice_ms),
        "screencast_jpeg_quality" => json!(cfg.screencast_jpeg_quality),
        "interact_settle_ms" => json!(cfg.interact_settle_ms),
        "dialog_settle_ms" => json!(cfg.dialog_settle_ms),
        "cdp_connection_probe_timeout_secs" => json!(cfg.cdp_connection_probe_timeout_secs),
        "http_ssrf_mode" => json!(cfg.http_ssrf_mode),
        "http_timeout_secs" => json!(cfg.http_timeout_secs),
        "http_connect_timeout_secs" => json!(cfg.http_connect_timeout_secs),
        "scrape_max_body_bytes" => json!(cfg.scrape_max_body_bytes),
        "llm_http_timeout_secs" => json!(cfg.llm_http_timeout_secs),
        "redis_allow_remote" => json!(cfg.redis_allow_remote),
        "redis_connect_timeout_secs" => json!(cfg.redis_connect_timeout_secs),
        "robots_loopback_exempt" => json!(cfg.robots_loopback_exempt),
        "chrome_search_paths" => json!(cfg.chrome_search_paths),
        "allowed_roots" => json!(cfg.allowed_roots),
        other => match crate::xdg::policy::policy_stored(&cfg.policy, other) {
            Some(stored) => json!(stored),
            None => {
                return Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    format!("unknown config key: {other}"),
                    crate::i18n::suggestion_key("config_list_keys", None),
                ));
            }
        },
    };
    Ok(json!({ "key": key, "value": value }))
}

/// Get one config key (or full dump when key is empty).
pub fn config_get(key: Option<&str>) -> Result<Value, CliError> {
    let cfg = load_config()?;
    match key {
        None | Some("") => Ok(full_dump(&cfg)?),
        Some(k) => get_one(&cfg, k),
    }
}

/// Full config dump: hand-written keys plus every promoted policy override.
///
/// Built with explicit `insert` calls rather than one wide `json!` literal:
/// the macro hits its recursion limit as keys accumulate, and every team that
/// added a key kept re-discovering that failure. Insertion has no such ceiling.
fn full_dump(cfg: &ProductConfig) -> Result<Value, CliError> {
    let mut map = serde_json::Map::new();
    let mut put = |key: &str, value: Value| {
        map.insert(key.to_string(), value);
    };

    put("lang", json!(cfg.lang));
    put("timeout", json!(cfg.timeout));
    put("artifacts_dir", json!(cfg.artifacts_dir));
    put("ignore_robots", json!(cfg.ignore_robots));
    put("namespace", json!(cfg.namespace));
    put(
        "encryption_key",
        json!(redacted_secret(&cfg.encryption_key)),
    );
    put("color", json!(cfg.color));
    put("log_level", json!(cfg.log_level));
    put("log_to_file", json!(cfg.log_to_file));
    put("max_log_files", json!(cfg.max_log_files));
    put("log_rotation", json!(cfg.log_rotation));
    put("chrome_path", json!(cfg.chrome_path));
    put("lighthouse_path", json!(cfg.lighthouse_path));
    put("ffmpeg_path", json!(cfg.ffmpeg_path));
    put(
        "lighthouse_timeout_secs",
        json!(cfg.lighthouse_timeout_secs),
    );
    put("ffmpeg_timeout_secs", json!(cfg.ffmpeg_timeout_secs));
    put(
        "openrouter_api_key",
        json!(redacted_secret(&cfg.openrouter_api_key)),
    );
    put("llm_base_url", json!(cfg.llm_base_url));
    put("llm_model", json!(cfg.llm_model));
    put("cache_backend", json!(cfg.cache_backend));
    put(
        "cache_redis_url",
        json!(redacted_secret(&cfg.cache_redis_url)),
    );
    put("search_base_url", json!(cfg.search_base_url));
    put(
        "lightpanda_startup_timeout_secs",
        json!(cfg.lightpanda_startup_timeout_secs),
    );
    put(
        "lightpanda_session_timeout_secs",
        json!(cfg.lightpanda_session_timeout_secs),
    );
    put("max_json_file_bytes", json!(cfg.max_json_file_bytes));
    put("max_ndjson_line_bytes", json!(cfg.max_ndjson_line_bytes));
    put(
        "max_cli_json_payload_bytes",
        json!(cfg.max_cli_json_payload_bytes),
    );
    put("default_jpeg_quality", json!(cfg.default_jpeg_quality));
    put("event_pump_slice_ms", json!(cfg.event_pump_slice_ms));
    put(
        "screencast_jpeg_quality",
        json!(cfg.screencast_jpeg_quality),
    );
    put("interact_settle_ms", json!(cfg.interact_settle_ms));
    put("dialog_settle_ms", json!(cfg.dialog_settle_ms));
    put(
        "cdp_connection_probe_timeout_secs",
        json!(cfg.cdp_connection_probe_timeout_secs),
    );
    put("http_ssrf_mode", json!(cfg.http_ssrf_mode));
    put("http_timeout_secs", json!(cfg.http_timeout_secs));
    put(
        "http_connect_timeout_secs",
        json!(cfg.http_connect_timeout_secs),
    );
    put("scrape_max_body_bytes", json!(cfg.scrape_max_body_bytes));
    put("llm_http_timeout_secs", json!(cfg.llm_http_timeout_secs));
    put("redis_allow_remote", json!(cfg.redis_allow_remote));
    put(
        "redis_connect_timeout_secs",
        json!(cfg.redis_connect_timeout_secs),
    );
    put("robots_loopback_exempt", json!(cfg.robots_loopback_exempt));
    put("chrome_search_paths", json!(cfg.chrome_search_paths));
    put("allowed_roots", json!(cfg.allowed_roots));
    for name in crate::xdg::policy::POLICY_KEYS {
        let stored = crate::xdg::policy::policy_stored(&cfg.policy, name).flatten();
        put(name, json!(stored));
    }
    put("path", json!(config_file()?.display().to_string()));

    Ok(Value::Object(map))
}
