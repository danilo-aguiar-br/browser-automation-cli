// SPDX-License-Identifier: MIT OR Apache-2.0
//! Load / parse of XDG `config.toml` (atomic write lives in `config_write`).

use std::fs;

use super::config_model::ProductConfig;
use super::config_ops::validate::parse_boolish;
use super::paths::config_file;
use crate::error::{CliError, ErrorKind};

/// Load config from XDG path; missing file yields defaults.
pub fn load_config() -> Result<ProductConfig, CliError> {
    let path = config_file()?;
    if !path.exists() {
        return Ok(ProductConfig::default());
    }
    if path.extension().and_then(|e| e.to_str()) == Some("json") {
        return crate::json_util::read_json_file(
            &path,
            crate::constants::DEFAULT_MAX_JSON_FILE_BYTES,
        )
        .map_err(|e| {
            CliError::new(
                ErrorKind::Data,
                format!("invalid config JSON: {}", e.message()),
            )
        });
    }
    let raw = fs::read_to_string(&path).map_err(|e| {
        CliError::new(
            ErrorKind::Io,
            format!("read config {}: {e}", path.display()),
        )
    })?;
    parse_simple_toml(&raw)
}

/// Apply one loose TOML `key = value` line into `cfg` (unknown keys ignored).
///
/// # Why a bad boolean fails the load instead of falling back
///
/// Unknown KEYS stay ignored: a file written by a newer build must not brick an
/// older one. A known key with an unreadable VALUE is the opposite case — the
/// operator named a knob this build owns and spelled the value wrong, and the
/// only readings available are "what they asked for" or "the default", which is
/// frequently its opposite.
///
/// Seven boolean keys default to `true`, `stealth` and three robots controls
/// among them. Silently reading `stealth = "banana"` as `false` turns off the
/// anti-detection layer on every later run, with `ok: true` and exit 0 on each
/// one. Refusing to load is loud, immediate, and costs one corrected line.
pub(crate) fn apply_toml_kv(cfg: &mut ProductConfig, k: &str, v: &str) -> Result<(), CliError> {
    apply_toml_kv_inner(cfg, k, v).map_err(|e| as_config_error(&e))
}

/// Restate a value-parse failure as a CONFIG failure rather than a USAGE one.
///
/// The parsers are shared with `config set`, where a bad value came from argv
/// and `usage` (exit 2) is the honest answer. Reaching them from here means the
/// argv was fine and the file on disk is not, which is exit 78. Same message,
/// different exit, because the caller fixes a different thing in each case.
fn as_config_error(err: &CliError) -> CliError {
    match err.suggestion() {
        Some(s) => CliError::with_suggestion(ErrorKind::Config, err.message().to_string(), s),
        None => CliError::new(ErrorKind::Config, err.message().to_string()),
    }
}

fn apply_toml_kv_inner(cfg: &mut ProductConfig, k: &str, v: &str) -> Result<(), CliError> {
    match k {
        "lang" => {
            // Permissive load: invalid tokens dropped (strict reject is `config set`).
            if crate::i18n::UiLocale::parse_token(v).is_some() {
                cfg.lang = Some(v.to_string());
            }
        }
        "timeout" => cfg.timeout = v.parse().ok(),
        "artifacts_dir" => cfg.artifacts_dir = Some(v.to_string()),
        "ignore_robots" => cfg.ignore_robots = Some(parse_boolish(v, k)?),
        "namespace" => cfg.namespace = Some(v.to_string()),
        "encryption_key" => cfg.encryption_key = Some(v.to_string()),
        // Declared in CONFIG_KEYS, settable and gettable, but absent from both
        // this reader and the writer until 2026-08-10 — so `config set` answered
        // ok and the next process read back null. See `config_write.rs`.
        "proxy_url" => cfg.proxy_url = Some(v.to_string()),
        "proxy_bypass" => cfg.proxy_bypass = Some(v.to_string()),
        "proxy_username" => cfg.proxy_username = Some(v.to_string()),
        "proxy_password" => cfg.proxy_password = Some(v.to_string()),
        "stealth_seed" => cfg.stealth_seed = Some(v.to_string()),
        "robots_user_agent" => cfg.robots_user_agent = Some(v.to_string()),
        "color" => cfg.color = Some(parse_boolish(v, k)?),
        "log_level" => cfg.log_level = Some(v.to_string()),
        "input_profile" => cfg.input_profile = Some(v.to_string()),
        "chrome_path" => cfg.chrome_path = Some(v.to_string()),
        "lighthouse_path" => cfg.lighthouse_path = Some(v.to_string()),
        "ffmpeg_path" => cfg.ffmpeg_path = Some(v.to_string()),
        "lighthouse_timeout_secs" => cfg.lighthouse_timeout_secs = v.parse().ok(),
        "ffmpeg_timeout_secs" => cfg.ffmpeg_timeout_secs = v.parse().ok(),
        "openrouter_api_key" => cfg.openrouter_api_key = Some(v.to_string()),
        "llm_base_url" => cfg.llm_base_url = Some(v.to_string()),
        "llm_model" => cfg.llm_model = Some(v.to_string()),
        "log_to_file" => cfg.log_to_file = Some(parse_boolish(v, k)?),
        "max_log_files" => cfg.max_log_files = v.parse().ok(),
        "log_rotation" => cfg.log_rotation = Some(v.to_string()),
        "cache_backend" => cfg.cache_backend = Some(v.to_string()),
        "cache_redis_url" => cfg.cache_redis_url = Some(v.to_string()),
        "search_base_url" => cfg.search_base_url = Some(v.to_string()),
        "lightpanda_startup_timeout_secs" => cfg.lightpanda_startup_timeout_secs = v.parse().ok(),
        "lightpanda_session_timeout_secs" => cfg.lightpanda_session_timeout_secs = v.parse().ok(),
        "max_json_file_bytes" => cfg.max_json_file_bytes = v.parse().ok(),
        "max_ndjson_line_bytes" => cfg.max_ndjson_line_bytes = v.parse().ok(),
        "max_cli_json_payload_bytes" => cfg.max_cli_json_payload_bytes = v.parse().ok(),
        "default_jpeg_quality" => cfg.default_jpeg_quality = v.parse().ok(),
        "event_pump_slice_ms" => cfg.event_pump_slice_ms = v.parse().ok(),
        "screencast_jpeg_quality" => cfg.screencast_jpeg_quality = v.parse().ok(),
        "interact_settle_ms" => cfg.interact_settle_ms = v.parse().ok(),
        "dialog_settle_ms" => cfg.dialog_settle_ms = v.parse().ok(),
        "cdp_connection_probe_timeout_secs" => {
            cfg.cdp_connection_probe_timeout_secs = v.parse().ok()
        }
        "http_ssrf_mode" => cfg.http_ssrf_mode = Some(v.to_string()),
        "http_timeout_secs" => cfg.http_timeout_secs = v.parse().ok(),
        "http_connect_timeout_secs" => cfg.http_connect_timeout_secs = v.parse().ok(),
        "scrape_max_body_bytes" => cfg.scrape_max_body_bytes = v.parse().ok(),
        "scrape_max_text_chars" => cfg.scrape_max_text_chars = v.parse().ok(),
        "scrape_min_delay_ms" => cfg.scrape_min_delay_ms = v.parse().ok(),
        "scrape_honor_meta_robots" => cfg.scrape_honor_meta_robots = Some(parse_boolish(v, k)?),
        "scrape_honor_nofollow" => cfg.scrape_honor_nofollow = Some(parse_boolish(v, k)?),
        "scrape_use_sitemap" => {
            cfg.scrape_use_sitemap = Some(parse_boolish(v, k)?);
        }
        "scrape_default_engine" => cfg.scrape_default_engine = Some(v.to_string()),
        "scrape_delay_jitter_ratio" => cfg.scrape_delay_jitter_ratio = v.parse().ok(),
        "scrape_summary_chars" => cfg.scrape_summary_chars = v.parse().ok(),
        "scrape_feed_max_entries" => cfg.scrape_feed_max_entries = v.parse().ok(),
        "scrape_follow_rel_next" => cfg.scrape_follow_rel_next = Some(parse_boolish(v, k)?),
        "scrape_dedup_similar" => cfg.scrape_dedup_similar = Some(parse_boolish(v, k)?),
        "scrape_no_cache" => cfg.scrape_no_cache = Some(parse_boolish(v, k)?),
        "browser_mode" => cfg.browser_mode = Some(v.to_string()),
        "stealth_profile" => cfg.stealth_profile = Some(v.to_string()),
        "stealth" => cfg.stealth = Some(parse_boolish(v, k)?),
        "cdp_proxy_bypass_loopback" => cfg.cdp_proxy_bypass_loopback = Some(parse_boolish(v, k)?),
        "http2_enabled" => cfg.http2_enabled = Some(parse_boolish(v, k)?),
        "http2_adaptive_window" => cfg.http2_adaptive_window = Some(parse_boolish(v, k)?),
        "http2_initial_stream_window_size" => cfg.http2_initial_stream_window_size = v.parse().ok(),
        "http2_initial_connection_window_size" => {
            cfg.http2_initial_connection_window_size = v.parse().ok()
        }
        "http2_max_header_list_size" => cfg.http2_max_header_list_size = v.parse().ok(),
        "http2_max_frame_size" => cfg.http2_max_frame_size = v.parse().ok(),
        "image_avif_speed" => cfg.image_avif_speed = v.parse().ok(),
        "svg_max_bytes" => cfg.svg_max_bytes = v.parse().ok(),
        "svg_max_depth" => cfg.svg_max_depth = v.parse().ok(),
        "svg_max_entities" => cfg.svg_max_entities = v.parse().ok(),
        "gif_max_frames" => cfg.gif_max_frames = v.parse().ok(),
        "manifest_max_bytes" => cfg.manifest_max_bytes = v.parse().ok(),
        "manifest_max_variants" => cfg.manifest_max_variants = v.parse().ok(),
        "scrape_dedup_similar_distance" => cfg.scrape_dedup_similar_distance = v.parse().ok(),
        "scrape_sitemap_max_bytes" => cfg.scrape_sitemap_max_bytes = v.parse().ok(),
        "scrape_charset_peek_bytes" => cfg.scrape_charset_peek_bytes = v.parse().ok(),
        "llm_http_timeout_secs" => cfg.llm_http_timeout_secs = v.parse().ok(),
        "redis_allow_remote" => cfg.redis_allow_remote = Some(parse_boolish(v, k)?),
        "chrome_legacy_oxide_launch" => {
            cfg.chrome_legacy_oxide_launch = Some(parse_boolish(v, k)?);
        }
        "robots_loopback_exempt" => cfg.robots_loopback_exempt = Some(parse_boolish(v, k)?),
        "redis_connect_timeout_secs" => cfg.redis_connect_timeout_secs = v.parse().ok(),
        "chrome_search_paths" => cfg.chrome_search_paths = Some(split_path_list(v)),
        "allowed_roots" => cfg.allowed_roots = Some(split_path_list(v)),
        "image_max_input_bytes" => cfg.image_max_input_bytes = v.parse().ok().filter(|&n| n > 0),
        "image_max_pixels" => cfg.image_max_pixels = v.parse().ok().filter(|&n| n > 0),
        "image_default_format" => cfg.image_default_format = Some(v.to_string()),
        "image_default_quality" => cfg.image_default_quality = v.parse().ok(),
        "image_download_max_bytes" => {
            cfg.image_download_max_bytes = v.parse().ok().filter(|&n| n > 0)
        }
        "video_max_input_bytes" => cfg.video_max_input_bytes = v.parse().ok().filter(|&n| n > 0),
        "video_download_max_bytes" => {
            cfg.video_download_max_bytes = v.parse().ok().filter(|&n| n > 0)
        }
        "video_default_container" => cfg.video_default_container = Some(v.to_string()),
        "video_default_crf" => cfg.video_default_crf = v.parse().ok(),
        "video_default_audio_bitrate" => cfg.video_default_audio_bitrate = Some(v.to_string()),
        "audio_max_input_bytes" => cfg.audio_max_input_bytes = v.parse().ok().filter(|&n| n > 0),
        "audio_download_max_bytes" => {
            cfg.audio_download_max_bytes = v.parse().ok().filter(|&n| n > 0)
        }
        "audio_default_format" => cfg.audio_default_format = Some(v.to_string()),
        "audio_default_bitrate" => cfg.audio_default_bitrate = Some(v.to_string()),
        other => {
            // Promoted policy knobs (GAP-048) share this loose table.
            super::policy::policy_apply_raw(&mut cfg.policy, other, v);
        }
    }
    Ok(())
}

/// Platform separator for multi-path config values (`;` on Windows, `:` elsewhere).
pub(crate) const PATH_LIST_SEPARATOR: char = if cfg!(windows) { ';' } else { ':' };

/// Split a config path list on [`PATH_LIST_SEPARATOR`], dropping empty entries.
pub(crate) fn split_path_list(raw: &str) -> Vec<String> {
    raw.split(PATH_LIST_SEPARATOR)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// Join a config path list with [`PATH_LIST_SEPARATOR`].
pub(crate) fn join_path_list(items: &[String]) -> String {
    items.join(&PATH_LIST_SEPARATOR.to_string())
}

fn parse_simple_toml(raw: &str) -> Result<ProductConfig, CliError> {
    let mut cfg = ProductConfig::default();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let k = k.trim();
        let v = v.trim().trim_matches('"').trim_matches('\'');
        apply_toml_kv(&mut cfg, k, v)?;
    }
    Ok(cfg)
}

/// Re-export atomic write (SRP: implementation in `config_write`).
pub use super::config_write::write_config;
