// SPDX-License-Identifier: MIT OR Apache-2.0
//! The single list of hand-written config keys and where each value lives.
//!
//! # Why this is one list and not two
//!
//! `config get <key>` and `config get` (full dump) answer the same question at
//! different widths, and each used to carry its own transcription of every
//! key: a `match` arm in one function and a `put(...)` call in the other.
//!
//! Measured on 2026-08-08 by extracting both lists and running `comm`: 96 keys
//! each, identical apart from `path`, which only the dump reports. So the
//! duplication had not diverged YET — it was scheduled divergence. Adding a key
//! meant editing two places, and forgetting the second one produced a key that
//! `config get <key>` answers and `config get` omits, with nothing to catch it.
//!
//! One list removes the schedule. A new key is one row, and both surfaces get
//! it or neither does.
//!
//! # Redaction lives here too
//!
//! Secrets are wrapped in [`redacted_secret`] in this table rather than at the
//! two call sites, so a key cannot be redacted in the dump and printed in
//! plain by the single-key path.

use serde_json::{json, Value};

use super::super::config_model::ProductConfig;
use super::validate::redacted_secret;

/// Every hand-written key, paired with its current value.
///
/// Excludes `path` (dump-only, and derived rather than stored) and the
/// promoted policy knobs, which come from `policy::POLICY_KEYS` and already
/// have a single source of their own.
pub(super) fn config_entries(cfg: &ProductConfig) -> Vec<(&'static str, Value)> {
    vec![
        ("lang", json!(cfg.lang)),
        ("timeout", json!(cfg.timeout)),
        ("artifacts_dir", json!(cfg.artifacts_dir)),
        ("ignore_robots", json!(cfg.ignore_robots)),
        ("namespace", json!(cfg.namespace)),
        (
            "encryption_key",
            json!(redacted_secret(&cfg.encryption_key)),
        ),
        ("color", json!(cfg.color)),
        ("log_level", json!(cfg.log_level)),
        ("input_profile", json!(cfg.input_profile)),
        ("browser_mode", json!(cfg.browser_mode)),
        ("stealth", json!(cfg.stealth)),
        ("stealth_profile", json!(cfg.stealth_profile)),
        ("proxy_url", json!(redacted_secret(&cfg.proxy_url))),
        ("proxy_bypass", json!(cfg.proxy_bypass)),
        (
            "cdp_proxy_bypass_loopback",
            json!(cfg.cdp_proxy_bypass_loopback),
        ),
        (
            "proxy_username",
            json!(redacted_secret(&cfg.proxy_username)),
        ),
        (
            "proxy_password",
            json!(redacted_secret(&cfg.proxy_password)),
        ),
        ("stealth_seed", json!(cfg.stealth_seed)),
        ("http2_enabled", json!(cfg.http2_enabled)),
        (
            "http2_initial_stream_window_size",
            json!(cfg.http2_initial_stream_window_size),
        ),
        (
            "http2_initial_connection_window_size",
            json!(cfg.http2_initial_connection_window_size),
        ),
        (
            "http2_max_header_list_size",
            json!(cfg.http2_max_header_list_size),
        ),
        ("http2_max_frame_size", json!(cfg.http2_max_frame_size)),
        ("http2_adaptive_window", json!(cfg.http2_adaptive_window)),
        ("robots_user_agent", json!(cfg.robots_user_agent)),
        ("log_to_file", json!(cfg.log_to_file)),
        ("max_log_files", json!(cfg.max_log_files)),
        ("log_rotation", json!(cfg.log_rotation)),
        ("chrome_path", json!(cfg.chrome_path)),
        ("lighthouse_path", json!(cfg.lighthouse_path)),
        ("ffmpeg_path", json!(cfg.ffmpeg_path)),
        (
            "lighthouse_timeout_secs",
            json!(cfg.lighthouse_timeout_secs),
        ),
        ("ffmpeg_timeout_secs", json!(cfg.ffmpeg_timeout_secs)),
        (
            "openrouter_api_key",
            json!(redacted_secret(&cfg.openrouter_api_key)),
        ),
        ("llm_base_url", json!(cfg.llm_base_url)),
        ("llm_model", json!(cfg.llm_model)),
        ("cache_backend", json!(cfg.cache_backend)),
        (
            "cache_redis_url",
            json!(redacted_secret(&cfg.cache_redis_url)),
        ),
        ("search_base_url", json!(cfg.search_base_url)),
        (
            "lightpanda_startup_timeout_secs",
            json!(cfg.lightpanda_startup_timeout_secs),
        ),
        (
            "lightpanda_session_timeout_secs",
            json!(cfg.lightpanda_session_timeout_secs),
        ),
        ("max_json_file_bytes", json!(cfg.max_json_file_bytes)),
        ("max_ndjson_line_bytes", json!(cfg.max_ndjson_line_bytes)),
        (
            "max_cli_json_payload_bytes",
            json!(cfg.max_cli_json_payload_bytes),
        ),
        ("default_jpeg_quality", json!(cfg.default_jpeg_quality)),
        ("event_pump_slice_ms", json!(cfg.event_pump_slice_ms)),
        (
            "screencast_jpeg_quality",
            json!(cfg.screencast_jpeg_quality),
        ),
        ("interact_settle_ms", json!(cfg.interact_settle_ms)),
        ("dialog_settle_ms", json!(cfg.dialog_settle_ms)),
        (
            "cdp_connection_probe_timeout_secs",
            json!(cfg.cdp_connection_probe_timeout_secs),
        ),
        ("http_ssrf_mode", json!(cfg.http_ssrf_mode)),
        ("http_timeout_secs", json!(cfg.http_timeout_secs)),
        (
            "http_connect_timeout_secs",
            json!(cfg.http_connect_timeout_secs),
        ),
        ("scrape_max_body_bytes", json!(cfg.scrape_max_body_bytes)),
        ("scrape_max_text_chars", json!(cfg.scrape_max_text_chars)),
        ("scrape_min_delay_ms", json!(cfg.scrape_min_delay_ms)),
        (
            "scrape_honor_meta_robots",
            json!(cfg.scrape_honor_meta_robots),
        ),
        ("scrape_honor_nofollow", json!(cfg.scrape_honor_nofollow)),
        ("scrape_use_sitemap", json!(cfg.scrape_use_sitemap)),
        ("scrape_default_engine", json!(cfg.scrape_default_engine)),
        (
            "scrape_delay_jitter_ratio",
            json!(cfg.scrape_delay_jitter_ratio),
        ),
        ("scrape_summary_chars", json!(cfg.scrape_summary_chars)),
        (
            "scrape_feed_max_entries",
            json!(cfg.scrape_feed_max_entries),
        ),
        ("scrape_follow_rel_next", json!(cfg.scrape_follow_rel_next)),
        ("scrape_dedup_similar", json!(cfg.scrape_dedup_similar)),
        ("scrape_no_cache", json!(cfg.scrape_no_cache)),
        (
            "scrape_dedup_similar_distance",
            json!(cfg.scrape_dedup_similar_distance),
        ),
        (
            "scrape_sitemap_max_bytes",
            json!(cfg.scrape_sitemap_max_bytes),
        ),
        (
            "scrape_charset_peek_bytes",
            json!(cfg.scrape_charset_peek_bytes),
        ),
        ("llm_http_timeout_secs", json!(cfg.llm_http_timeout_secs)),
        ("redis_allow_remote", json!(cfg.redis_allow_remote)),
        (
            "chrome_legacy_oxide_launch",
            json!(cfg.chrome_legacy_oxide_launch),
        ),
        (
            "redis_connect_timeout_secs",
            json!(cfg.redis_connect_timeout_secs),
        ),
        ("robots_loopback_exempt", json!(cfg.robots_loopback_exempt)),
        ("chrome_search_paths", json!(cfg.chrome_search_paths)),
        ("allowed_roots", json!(cfg.allowed_roots)),
        ("image_max_input_bytes", json!(cfg.image_max_input_bytes)),
        ("image_max_pixels", json!(cfg.image_max_pixels)),
        ("image_default_format", json!(cfg.image_default_format)),
        ("image_default_quality", json!(cfg.image_default_quality)),
        (
            "image_download_max_bytes",
            json!(cfg.image_download_max_bytes),
        ),
        ("image_avif_speed", json!(cfg.image_avif_speed)),
        ("svg_max_bytes", json!(cfg.svg_max_bytes)),
        ("svg_max_depth", json!(cfg.svg_max_depth)),
        ("svg_max_entities", json!(cfg.svg_max_entities)),
        ("gif_max_frames", json!(cfg.gif_max_frames)),
        ("manifest_max_bytes", json!(cfg.manifest_max_bytes)),
        ("manifest_max_variants", json!(cfg.manifest_max_variants)),
        ("video_max_input_bytes", json!(cfg.video_max_input_bytes)),
        (
            "video_download_max_bytes",
            json!(cfg.video_download_max_bytes),
        ),
        (
            "video_default_container",
            json!(cfg.video_default_container),
        ),
        ("video_default_crf", json!(cfg.video_default_crf)),
        (
            "video_default_audio_bitrate",
            json!(cfg.video_default_audio_bitrate),
        ),
        ("audio_max_input_bytes", json!(cfg.audio_max_input_bytes)),
        (
            "audio_download_max_bytes",
            json!(cfg.audio_download_max_bytes),
        ),
        ("audio_default_format", json!(cfg.audio_default_format)),
        ("audio_default_bitrate", json!(cfg.audio_default_bitrate)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_appears_exactly_once() {
        // A duplicated key would make the dump emit one value and the
        // single-key lookup return the first match, silently disagreeing.
        let cfg = ProductConfig::default();
        let mut names: Vec<&str> = config_entries(&cfg).into_iter().map(|(k, _)| k).collect();
        let before = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(before, names.len(), "duplicate key in the config table");
    }

    #[test]
    fn the_table_is_not_accidentally_empty() {
        assert!(config_entries(&ProductConfig::default()).len() > 80);
    }

    #[test]
    fn secrets_are_redacted_in_the_table_itself() {
        // Redacting at the call sites is how a secret ends up masked in one
        // surface and printed in the other.
        let cfg = ProductConfig {
            openrouter_api_key: Some("sk-live-should-never-appear".to_string()),
            proxy_password: Some("hunter2".to_string()),
            ..ProductConfig::default()
        };
        let dumped = serde_json::to_string(&config_entries(&cfg)).expect("serialize");
        assert!(!dumped.contains("sk-live-should-never-appear"), "{dumped}");
        assert!(!dumped.contains("hunter2"), "{dumped}");
    }
}
