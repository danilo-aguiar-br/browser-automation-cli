// SPDX-License-Identifier: MIT OR Apache-2.0
//! Canonical config key catalog and `config list-keys` payload.

use serde_json::{json, Value};

use super::super::paths::config_file;
use super::key_entries::base_key_entries;
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
    "input_profile",
    "input_timing_distribution",
    "browser_mode",
    "stealth",
    "stealth_profile",
    "proxy_url",
    "proxy_bypass",
    "cdp_proxy_bypass_loopback",
    "proxy_username",
    "proxy_password",
    "stealth_seed",
    "screen",
    "http2_enabled",
    "http2_initial_stream_window_size",
    "http2_initial_connection_window_size",
    "http2_max_header_list_size",
    "http2_max_frame_size",
    "http2_adaptive_window",
    "robots_user_agent",
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
    "user_data_dir",
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
    "scrape_max_text_chars",
    "scrape_min_delay_ms",
    "scrape_honor_meta_robots",
    "scrape_honor_nofollow",
    "scrape_use_sitemap",
    "scrape_default_engine",
    "scrape_delay_jitter_ratio",
    "scrape_summary_chars",
    "scrape_feed_max_entries",
    "scrape_follow_rel_next",
    "scrape_dedup_similar",
    "scrape_no_cache",
    "scrape_dedup_similar_distance",
    "scrape_sitemap_max_bytes",
    "scrape_charset_peek_bytes",
    "llm_http_timeout_secs",
    "redis_allow_remote",
    "chrome_legacy_oxide_launch",
    "redis_connect_timeout_secs",
    "chrome_search_paths",
    "allowed_roots",
    "robots_loopback_exempt",
    "image_max_input_bytes",
    "image_max_pixels",
    "image_default_format",
    "image_default_quality",
    "image_download_max_bytes",
    "image_avif_speed",
    "svg_max_bytes",
    "svg_max_depth",
    "svg_max_entities",
    "gif_max_frames",
    "manifest_max_bytes",
    "manifest_max_variants",
    "video_max_input_bytes",
    "video_download_max_bytes",
    "video_default_container",
    "video_default_crf",
    "video_default_audio_bitrate",
    "audio_max_input_bytes",
    "audio_download_max_bytes",
    "audio_default_format",
    "audio_default_bitrate",
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
///
/// # Errors
///
/// [`crate::error::ErrorKind::Io`] propagated from [`config_file`] when no home directory can
/// be resolved. Building the key list itself is infallible.
pub fn config_list_keys() -> Result<Value, CliError> {
    let mut keys: Vec<Value> = base_key_entries();
    keys.extend(crate::xdg::policy::policy_list_entries());
    Ok(json!({
        "keys": keys,
        "path": config_file()?.display().to_string(),
    }))
}
