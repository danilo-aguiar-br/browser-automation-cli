// SPDX-License-Identifier: MIT OR Apache-2.0
//! `config set` mutation path (strict validation then persist).

use serde_json::{json, Value};

use super::super::config_io::{load_config, write_config};
use super::super::config_model::ProductConfig;
use super::validate::{
    parse_boolish, parse_positive_u64, parse_quality_u8, parse_u64, reject_untransportable_value,
};
use crate::error::{CliError, ErrorKind};

/// Apply a strict `config set` mutation (validates ranges).
///
/// The first check is format-level and applies to every key, present and
/// future: a value carrying a quote or a newline cannot be stored, because the
/// writer interpolates it between quotes and the reader parses line by line.
/// Without it a value can close its own string and declare a key of its
/// choosing, which for a key naming an executable is code execution. It runs
/// here, at the one funnel every key passes through, rather than in the family
/// setters, so a key added later is covered without anyone remembering to.
fn apply_set(cfg: &mut ProductConfig, key: &str, value: &str) -> Result<(), CliError> {
    super::validate::reject_unrepresentable_value(key, value)?;
    if super::set_media::apply_media_set(cfg, key, value)? {
        return Ok(());
    }
    if super::set_scrape::apply_scrape_set(cfg, key, value)? {
        return Ok(());
    }
    if super::set_network::apply_network_set(cfg, key, value)? {
        return Ok(());
    }
    match key {
        "lang" => {
            crate::i18n::validate_lang_token(value)?;
            cfg.lang = Some(value.to_string());
        }
        "timeout" => {
            cfg.timeout = Some(parse_u64(value, "timeout").map_err(|_| {
                CliError::new(ErrorKind::Usage, "timeout must be an integer seconds")
            })?);
        }
        "artifacts_dir" => cfg.artifacts_dir = Some(value.to_string()),
        "ignore_robots" => cfg.ignore_robots = Some(parse_boolish(value, key)?),
        "namespace" => cfg.namespace = Some(value.to_string()),
        "encryption_key" => cfg.encryption_key = Some(value.to_string()),
        "color" => cfg.color = Some(parse_boolish(value, key)?),
        "log_level" => {
            crate::tracing_local::validate_log_level_directive(value)?;
            cfg.log_level = Some(value.to_string());
        }
        "chrome_path" => cfg.chrome_path = Some(value.to_string()),
        "lighthouse_path" => cfg.lighthouse_path = Some(value.to_string()),
        "ffmpeg_path" => cfg.ffmpeg_path = Some(value.to_string()),
        "lighthouse_timeout_secs" => {
            let secs = parse_positive_u64(value, "lighthouse_timeout_secs")?;
            if secs > crate::constants::EXTERNAL_PROCESS_TIMEOUT_CAP_SECS {
                return Err(CliError::new(
                    ErrorKind::Usage,
                    format!(
                        "lighthouse_timeout_secs must be 1..={}",
                        crate::constants::EXTERNAL_PROCESS_TIMEOUT_CAP_SECS
                    ),
                ));
            }
            cfg.lighthouse_timeout_secs = Some(secs);
        }
        "ffmpeg_timeout_secs" => {
            let secs = parse_positive_u64(value, "ffmpeg_timeout_secs")?;
            if secs > crate::constants::EXTERNAL_PROCESS_TIMEOUT_CAP_SECS {
                return Err(CliError::new(
                    ErrorKind::Usage,
                    format!(
                        "ffmpeg_timeout_secs must be 1..={}",
                        crate::constants::EXTERNAL_PROCESS_TIMEOUT_CAP_SECS
                    ),
                ));
            }
            cfg.ffmpeg_timeout_secs = Some(secs);
        }
        "openrouter_api_key" => cfg.openrouter_api_key = Some(value.to_string()),
        "llm_base_url" => cfg.llm_base_url = Some(value.to_string()),
        "llm_model" => cfg.llm_model = Some(value.to_string()),
        "log_to_file" => cfg.log_to_file = Some(parse_boolish(value, key)?),
        "max_log_files" => {
            let n: u32 = value
                .parse()
                .map_err(|_| CliError::new(ErrorKind::Usage, "max_log_files must be an integer"))?;
            if !(crate::constants::MAX_LOG_FILES_MIN..=crate::constants::MAX_LOG_FILES_CAP)
                .contains(&n)
            {
                return Err(CliError::new(
                    ErrorKind::Usage,
                    format!(
                        "max_log_files must be {}..={}",
                        crate::constants::MAX_LOG_FILES_MIN,
                        crate::constants::MAX_LOG_FILES_CAP
                    ),
                ));
            }
            cfg.max_log_files = Some(n);
        }
        "log_rotation" => {
            crate::tracing_local::validate_log_rotation(value)?;
            cfg.log_rotation = Some(value.trim().to_ascii_lowercase());
        }
        "input_profile" => {
            // Validated here, not on read. An unparsable value would otherwise
            // fall back to the default at interaction time and steer nothing,
            // so the operator would set `directt`, see `ok: true`, and never
            // learn that every run stayed on `human`.
            let normalized = value.trim().to_ascii_lowercase();
            if crate::native::interaction::InputProfile::parse(&normalized).is_none() {
                return Err(CliError::new(
                    ErrorKind::Usage,
                    "input_profile must be `human` or `direct`",
                ));
            }
            cfg.input_profile = Some(normalized);
        }
        "input_timing_distribution" => {
            // Validated on write for the same reason as input_profile, and the
            // stakes are higher: an unparsable token falls back to `lognormal`
            // at interaction time, so an operator asking for `uniform` to
            // reproduce an old trace would get the new shape and never be told.
            let normalized = value.trim().to_ascii_lowercase();
            if crate::native::interaction::TimingDistribution::parse(&normalized).is_none() {
                return Err(CliError::new(
                    ErrorKind::Usage,
                    "input_timing_distribution must be `lognormal`, `normal` or `uniform`",
                ));
            }
            cfg.input_timing_distribution = Some(normalized);
        }
        "browser_mode" => {
            // Same reason input_profile validates on write: a bad token would
            // otherwise resolve to the default at launch, so the operator sets
            // `headles`, reads `ok: true`, and never learns the mode is auto.
            let normalized = value.trim().to_ascii_lowercase();
            if crate::browser_policy::BrowserMode::parse(&normalized).is_none() {
                return Err(CliError::new(
                    ErrorKind::Usage,
                    "browser_mode must be `auto`, `headed` or `headless`",
                ));
            }
            cfg.browser_mode = Some(normalized);
        }
        "stealth" => {
            cfg.stealth = Some(parse_boolish(value, key)?);
        }
        "stealth_profile" => {
            let normalized = value.trim().to_ascii_lowercase();
            if crate::browser_policy::StealthProfile::parse(&normalized).is_none() {
                return Err(CliError::new(
                    ErrorKind::Usage,
                    "stealth_profile must be `auto`, `chrome-linux`, `chrome-win` or `chrome-mac`",
                ));
            }
            cfg.stealth_profile = Some(normalized);
        }
        "screen" => {
            let trimmed = value.trim();
            crate::native::stealth::parse_screen_spec(trimmed).map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Usage,
                    e,
                    crate::i18n::suggestion_key("screen_spec_format", None),
                )
            })?;
            cfg.screen = Some(trimmed.to_string());
        }
        "robots_user_agent" => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Err(CliError::new(
                    ErrorKind::Usage,
                    "robots_user_agent must not be empty",
                ));
            }
            cfg.robots_user_agent = Some(trimmed.to_string());
        }
        "cache_backend" => cfg.cache_backend = Some(value.to_string()),
        "cache_redis_url" => cfg.cache_redis_url = Some(value.to_string()),
        "search_base_url" => cfg.search_base_url = Some(value.to_string()),
        "user_data_dir" => cfg.user_data_dir = Some(value.to_string()),
        "lightpanda_startup_timeout_secs" => {
            cfg.lightpanda_startup_timeout_secs = Some(value.parse().map_err(|_| {
                CliError::new(
                    ErrorKind::Usage,
                    "lightpanda_startup_timeout_secs must be an integer seconds",
                )
            })?);
        }
        "lightpanda_session_timeout_secs" => {
            let secs: u64 = value.parse().map_err(|_| {
                CliError::new(
                    ErrorKind::Usage,
                    "lightpanda_session_timeout_secs must be an integer seconds",
                )
            })?;
            if secs == 0 || secs > crate::constants::LIGHTPANDA_SESSION_TIMEOUT_SECS {
                return Err(CliError::new(
                    ErrorKind::Usage,
                    format!(
                        "lightpanda_session_timeout_secs must be 1..={}",
                        crate::constants::LIGHTPANDA_SESSION_TIMEOUT_SECS
                    ),
                ));
            }
            cfg.lightpanda_session_timeout_secs = Some(secs);
        }
        "max_json_file_bytes" => {
            cfg.max_json_file_bytes = Some(parse_positive_u64(value, "max_json_file_bytes")?);
        }
        "max_ndjson_line_bytes" => {
            cfg.max_ndjson_line_bytes = Some(parse_positive_u64(value, "max_ndjson_line_bytes")?);
        }
        "max_cli_json_payload_bytes" => {
            cfg.max_cli_json_payload_bytes =
                Some(parse_positive_u64(value, "max_cli_json_payload_bytes")?);
        }
        "default_jpeg_quality" => {
            cfg.default_jpeg_quality = Some(parse_quality_u8(value, "default_jpeg_quality")?);
        }
        "event_pump_slice_ms" => {
            cfg.event_pump_slice_ms = Some(parse_positive_u64(value, "event_pump_slice_ms")?);
        }
        "screencast_jpeg_quality" => {
            cfg.screencast_jpeg_quality = Some(parse_quality_u8(value, "screencast_jpeg_quality")?);
        }
        "interact_settle_ms" => {
            cfg.interact_settle_ms = Some(parse_positive_u64(value, "interact_settle_ms")?);
        }
        "dialog_settle_ms" => {
            cfg.dialog_settle_ms = Some(parse_positive_u64(value, "dialog_settle_ms")?);
        }
        "cdp_connection_probe_timeout_secs" => {
            cfg.cdp_connection_probe_timeout_secs = Some(parse_positive_u64(
                value,
                "cdp_connection_probe_timeout_secs",
            )?);
        }
        "http_ssrf_mode" => {
            let mode = value.trim().to_ascii_lowercase();
            if !matches!(mode.as_str(), "strict" | "allow_loopback" | "off") {
                return Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    format!("invalid http_ssrf_mode: {value}"),
                    crate::i18n::suggestion_key("use_listed_value", None),
                ));
            }
            cfg.http_ssrf_mode = Some(mode);
        }
        "http_timeout_secs" => {
            cfg.http_timeout_secs = Some(parse_positive_u64(value, "http_timeout_secs")?);
        }
        "http_connect_timeout_secs" => {
            cfg.http_connect_timeout_secs =
                Some(parse_positive_u64(value, "http_connect_timeout_secs")?);
        }
        "llm_http_timeout_secs" => {
            cfg.llm_http_timeout_secs = Some(parse_positive_u64(value, "llm_http_timeout_secs")?);
        }
        "redis_allow_remote" => cfg.redis_allow_remote = Some(parse_boolish(value, key)?),
        "chrome_legacy_oxide_launch" => {
            cfg.chrome_legacy_oxide_launch = Some(parse_boolish(value, key)?);
        }
        "robots_loopback_exempt" => cfg.robots_loopback_exempt = Some(parse_boolish(value, key)?),
        "redis_connect_timeout_secs" => {
            cfg.redis_connect_timeout_secs =
                Some(parse_positive_u64(value, "redis_connect_timeout_secs")?);
        }
        "allowed_roots" => {
            let roots = super::super::config_io::split_path_list(value);
            if roots.is_empty() {
                return Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    "allowed_roots must list at least one path",
                    crate::i18n::suggestion_key("chrome_search_paths_format", None),
                ));
            }
            cfg.allowed_roots = Some(roots);
        }
        "chrome_search_paths" => {
            let paths = super::super::config_io::split_path_list(value);
            if paths.is_empty() {
                return Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    "chrome_search_paths must list at least one path",
                    crate::i18n::suggestion_key("chrome_search_paths_format", None),
                ));
            }
            cfg.chrome_search_paths = Some(paths);
        }
        other => {
            // Promoted policy knobs (GAP-048) validate through the shared table.
            match crate::xdg::policy::policy_set(&mut cfg.policy, other, value) {
                Some(result) => result?,
                None => {
                    return Err(CliError::with_suggestion(
                        ErrorKind::Usage,
                        format!("unknown config key: {other}"),
                        crate::i18n::suggestion_key("config_list_keys", None),
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Set one string key in config and persist.
///
/// # Errors
///
/// [`ErrorKind::Usage`] when the value carries a character that cannot survive
/// the config round-trip (see `reject_untransportable_value`), when `key` is
/// outside the catalog, or when the value fails its family validator
/// (`apply_media_set`, `apply_network_set` and `apply_scrape_set`).
/// [`ErrorKind::Config`] propagated from [`load_config`] when the file already
/// on disk carries an unreadable value for a known key.
/// [`ErrorKind::Io`] from [`load_config`] or [`write_config`] when the config
/// path cannot be resolved, read, or atomically rewritten.
pub fn config_set(key: &str, value: &str) -> Result<Value, CliError> {
    reject_untransportable_value(key, value)?;
    let mut cfg = load_config()?;
    apply_set(&mut cfg, key, value)?;
    let path = write_config(&cfg)?;
    Ok(json!({
        "key": key,
        "value": value,
        "path": path.display().to_string(),
    }))
}
