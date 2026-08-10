// SPDX-License-Identifier: MIT OR Apache-2.0
//! XDG Base Directory layout for browser-automation-cli (no `.env` at runtime).
//!
//! Canonical product paths use the `directories` crate:
//! - config: `$XDG_CONFIG_HOME/browser-automation-cli` (Linux)
//! - data:   `$XDG_DATA_HOME/browser-automation-cli`
//! - cache:  `$XDG_CACHE_HOME/browser-automation-cli`
//! - state:  `$XDG_STATE_HOME/browser-automation-cli` (when available) or data/state
//!
//! Flags on the CLI override file config. Environment variables are **not** used for
//! product settings; system paths (`PATH`, locale) remain OS concerns.
//!
//! # Module map (Tier-4 SRP)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `paths` | ProjectDirs, `*_dir`, ensure_dir, init_layout, paths_snapshot |
//! | `config_model` | `ProductConfig` |
//! | `config_io` | load / parse |
//! | `config_write` | atomic write_config |
//! | `config_ops` | config_set / config_unset / config_get / config_list_keys |
//! | `policy` | Promoted operation-policy knobs (GAP-048) |
//! | `resolve` | resolve_* knobs + non-secret path helpers |
//! | `secrets` | Zeroizing secret accessors |

mod config_io;
mod config_model;
mod config_ops;
mod config_write;
mod config_write_optional;
mod paths;
pub mod policy;
mod resolve;
mod resolve_media;
mod resolve_scrape;
mod secrets;

#[cfg(test)]
mod tests;

pub use config_io::load_config;
pub use config_model::ProductConfig;
pub use config_ops::{
    all_config_keys, config_get, config_keys_description, config_list_keys, config_set,
    config_unset, CONFIG_KEYS,
};
pub use config_write::write_config;
pub use paths::{
    browsers_dir, cache_dir, chrome_profiles_dir, config_dir, config_file, data_dir, ensure_dir,
    init_layout, log_dir, mitm_ca_dir, mitm_capture_dir, paths_snapshot, project_dirs,
    sessions_dir, state_dir, workflow_dir,
};
pub use resolve::{
    chrome_path_from_config, ffmpeg_path_from_config, lighthouse_path_from_config, llm_base_url,
    llm_model, resolve_allowed_roots, resolve_audio_default_bitrate, resolve_audio_default_format,
    resolve_audio_download_max_bytes, resolve_audio_max_input_bytes,
    resolve_cdp_connection_probe_timeout_secs, resolve_cdp_proxy_bypass_loopback,
    resolve_chrome_legacy_oxide_launch, resolve_chrome_search_paths, resolve_default_jpeg_quality,
    resolve_dialog_settle_ms, resolve_eval_drain_slice_ms, resolve_event_pump_slice_ms,
    resolve_ffmpeg_timeout_secs, resolve_gif_max_frames, resolve_http2_adaptive_window,
    resolve_http2_enabled, resolve_http2_initial_connection_window_size,
    resolve_http2_initial_stream_window_size, resolve_http2_max_frame_size,
    resolve_http2_max_header_list_size, resolve_http_connect_timeout_secs, resolve_http_ssrf_mode,
    resolve_http_timeout_secs, resolve_image_avif_speed, resolve_image_default_format,
    resolve_image_default_quality, resolve_image_download_max_bytes, resolve_image_max_input_bytes,
    resolve_image_max_pixels, resolve_interact_settle_ms, resolve_lighthouse_timeout_secs,
    resolve_lightpanda_session_timeout_secs, resolve_lightpanda_startup_timeout_secs,
    resolve_llm_http_timeout_secs, resolve_manifest_max_bytes, resolve_manifest_max_variants,
    resolve_max_cli_json_payload_bytes, resolve_max_json_file_bytes, resolve_max_ndjson_line_bytes,
    resolve_monitor_diff_max_bytes, resolve_proxy_credentials, resolve_redis_allow_remote,
    resolve_redis_connect_timeout_secs, resolve_robots_loopback_exempt, resolve_robots_user_agent,
    resolve_screencast_jpeg_quality, resolve_stealth_seed, resolve_svg_max_bytes,
    resolve_svg_max_depth, resolve_svg_max_entities, resolve_video_default_audio_bitrate,
    resolve_video_default_container, resolve_video_default_crf, resolve_video_download_max_bytes,
    resolve_video_max_input_bytes, search_base_url,
};
pub use resolve_scrape::{
    resolve_scrape_charset_peek_bytes, resolve_scrape_dedup_similar,
    resolve_scrape_dedup_similar_distance, resolve_scrape_default_engine,
    resolve_scrape_delay_jitter_ratio, resolve_scrape_feed_max_entries,
    resolve_scrape_follow_rel_next, resolve_scrape_honor_meta_robots,
    resolve_scrape_honor_nofollow, resolve_scrape_max_body_bytes, resolve_scrape_max_text_chars,
    resolve_scrape_min_delay_ms, resolve_scrape_no_cache, resolve_scrape_sitemap_max_bytes,
    resolve_scrape_summary_chars, resolve_scrape_use_sitemap,
};
pub use secrets::{encryption_key, openrouter_api_key};
