// SPDX-License-Identifier: MIT OR Apache-2.0
//! Atomic write of XDG `config.toml` (SRP split from `config_io`).

use std::fs;
use std::io::Write;

use super::config_io::join_path_list;
use super::config_model::ProductConfig;
use super::paths::{config_dir, config_file, ensure_dir};
use crate::error::{CliError, ErrorKind};

/// Write config atomically (temp + rename).
///
/// # Errors
///
/// [`ErrorKind::Io`] from [`config_dir`] / [`config_file`] when no home
/// directory resolves, from [`ensure_dir`] when the config directory cannot be
/// created, and from each step of the atomic write: `File::create` of the
/// `.toml.tmp` sibling, `write_all` of the body, `sync_all`, and the final
/// `fs::rename` into place. Tightening the mode to `0o600` on Unix is
/// best-effort and never turns into an error.
pub fn write_config(cfg: &ProductConfig) -> Result<std::path::PathBuf, CliError> {
    let dir = config_dir()?;
    ensure_dir(&dir)?;
    let path = config_file()?;
    let mut body = format!(
        "# browser-automation-cli XDG config (no .env at runtime)\n\
         # Managed by: browser-automation-cli config set|init\n\
         lang = \"{lang}\"\n\
         timeout = {timeout}\n\
         artifacts_dir = \"{artifacts}\"\n\
         ignore_robots = {ignore}\n\
         namespace = \"{ns}\"\n\
         encryption_key = \"{enc}\"\n\
         color = {color}\n\
         log_level = \"{log_level}\"\n\
         input_profile = \"{input_profile}\"\n\
         input_timing_distribution = \"{input_timing_distribution}\"\n\
         log_to_file = {log_to_file}\n\
         max_log_files = {max_log_files}\n\
         log_rotation = \"{log_rotation}\"\n\
         chrome_path = \"{chrome_path}\"\n\
         lighthouse_path = \"{lighthouse_path}\"\n\
         ffmpeg_path = \"{ffmpeg_path}\"\n\
         lighthouse_timeout_secs = {lighthouse_timeout}\n\
         ffmpeg_timeout_secs = {ffmpeg_timeout}\n\
         openrouter_api_key = \"{openrouter_api_key}\"\n\
         llm_base_url = \"{llm_base_url}\"\n\
         llm_model = \"{llm_model}\"\n\
         cache_backend = \"{cache_backend}\"\n\
         cache_redis_url = \"{cache_redis_url}\"\n\
         search_base_url = \"{search_base_url}\"\n\
         user_data_dir = \"{user_data_dir}\"\n\
         lightpanda_startup_timeout_secs = {lp_startup}\n\
         lightpanda_session_timeout_secs = {lp_session}\n\
         max_json_file_bytes = {max_json_file}\n\
         max_ndjson_line_bytes = {max_ndjson_line}\n\
         max_cli_json_payload_bytes = {max_cli_json}\n\
         default_jpeg_quality = {jpeg_q}\n\
         event_pump_slice_ms = {pump_ms}\n\
         screencast_jpeg_quality = {sc_q}\n\
         interact_settle_ms = {interact_ms}\n\
         dialog_settle_ms = {dialog_ms}\n\
         cdp_connection_probe_timeout_secs = {cdp_probe}\n\
         http_ssrf_mode = \"{http_ssrf_mode}\"\n\
         http_timeout_secs = {http_timeout}\n\
         http_connect_timeout_secs = {http_connect}\n\
         scrape_max_body_bytes = {scrape_max_body}\n\
         scrape_max_text_chars = {scrape_max_text}\n\
         scrape_min_delay_ms = {scrape_min_delay}\n\
         scrape_honor_meta_robots = {scrape_meta_robots}\n\
         scrape_honor_nofollow = {scrape_nofollow}\n\
         scrape_use_sitemap = {scrape_sitemap}\n\
         scrape_default_engine = \"{scrape_default_engine}\"\n\
         scrape_delay_jitter_ratio = {scrape_jitter}\n\
         scrape_summary_chars = {scrape_summary}\n\
         scrape_feed_max_entries = {scrape_feed_max}\n\
         scrape_follow_rel_next = {scrape_rel_next}\n\
         scrape_dedup_similar = {scrape_dedup_similar}\n\
         scrape_no_cache = {scrape_no_cache}\n\
         scrape_dedup_similar_distance = {scrape_dedup_dist}\n\
         scrape_sitemap_max_bytes = {scrape_sitemap_max}\n\
         scrape_charset_peek_bytes = {scrape_charset_peek}\n\
         llm_http_timeout_secs = {llm_http_timeout}\n\
         redis_allow_remote = {redis_allow_remote}\n\
         chrome_legacy_oxide_launch = {chrome_legacy_oxide}\n\
         redis_connect_timeout_secs = {redis_connect}\n\
         robots_loopback_exempt = {robots_loopback_exempt}\n\
         image_max_input_bytes = {image_max_input}\n\
         image_max_pixels = {image_max_pixels}\n\
         image_default_format = \"{image_default_format}\"\n\
         image_default_quality = {image_default_quality}\n\
         image_download_max_bytes = {image_download_max}\n\
         video_max_input_bytes = {video_max_input}\n\
         video_download_max_bytes = {video_download_max}\n\
         video_default_container = \"{video_default_container}\"\n\
         video_default_crf = {video_default_crf}\n\
         video_default_audio_bitrate = \"{video_default_audio_bitrate}\"\n\
         audio_max_input_bytes = {audio_max_input}\n\
         audio_download_max_bytes = {audio_download_max}\n\
         audio_default_format = \"{audio_default_format}\"\n\
         audio_default_bitrate = \"{audio_default_bitrate}\"\n",
        lang = cfg.lang.as_deref().unwrap_or(""),
        timeout = cfg.timeout.unwrap_or(0),
        artifacts = cfg.artifacts_dir.as_deref().unwrap_or(""),
        ignore = cfg.ignore_robots.unwrap_or(false),
        ns = cfg.namespace.as_deref().unwrap_or(""),
        enc = cfg.encryption_key.as_deref().unwrap_or(""),
        color = cfg.color.unwrap_or(false),
        log_level = cfg.log_level.as_deref().unwrap_or(""),
        input_profile = cfg.input_profile.as_deref().unwrap_or(""),
        input_timing_distribution = cfg.input_timing_distribution.as_deref().unwrap_or(""),
        log_to_file = cfg.log_to_file.unwrap_or(false),
        max_log_files = cfg
            .max_log_files
            .unwrap_or(crate::constants::DEFAULT_MAX_LOG_FILES),
        log_rotation = cfg
            .log_rotation
            .as_deref()
            .unwrap_or(crate::constants::DEFAULT_LOG_ROTATION),
        chrome_path = cfg.chrome_path.as_deref().unwrap_or(""),
        lighthouse_path = cfg.lighthouse_path.as_deref().unwrap_or(""),
        ffmpeg_path = cfg.ffmpeg_path.as_deref().unwrap_or(""),
        lighthouse_timeout = cfg.lighthouse_timeout_secs.unwrap_or(0),
        ffmpeg_timeout = cfg.ffmpeg_timeout_secs.unwrap_or(0),
        openrouter_api_key = cfg.openrouter_api_key.as_deref().unwrap_or(""),
        llm_base_url = cfg.llm_base_url.as_deref().unwrap_or(""),
        llm_model = cfg.llm_model.as_deref().unwrap_or(""),
        cache_backend = cfg.cache_backend.as_deref().unwrap_or("sqlite"),
        cache_redis_url = cfg.cache_redis_url.as_deref().unwrap_or(""),
        search_base_url = cfg.search_base_url.as_deref().unwrap_or(""),
        user_data_dir = cfg.user_data_dir.as_deref().unwrap_or(""),
        lp_startup = cfg.lightpanda_startup_timeout_secs.unwrap_or(0),
        lp_session = cfg.lightpanda_session_timeout_secs.unwrap_or(0),
        max_json_file = cfg.max_json_file_bytes.unwrap_or(0),
        max_ndjson_line = cfg.max_ndjson_line_bytes.unwrap_or(0),
        max_cli_json = cfg.max_cli_json_payload_bytes.unwrap_or(0),
        jpeg_q = cfg.default_jpeg_quality.unwrap_or(0),
        pump_ms = cfg.event_pump_slice_ms.unwrap_or(0),
        sc_q = cfg.screencast_jpeg_quality.unwrap_or(0),
        interact_ms = cfg.interact_settle_ms.unwrap_or(0),
        dialog_ms = cfg.dialog_settle_ms.unwrap_or(0),
        cdp_probe = cfg.cdp_connection_probe_timeout_secs.unwrap_or(0),
        http_ssrf_mode = cfg.http_ssrf_mode.as_deref().unwrap_or("strict"),
        http_timeout = cfg.http_timeout_secs.unwrap_or(0),
        http_connect = cfg.http_connect_timeout_secs.unwrap_or(0),
        scrape_max_body = cfg.scrape_max_body_bytes.unwrap_or(0),
        scrape_max_text = cfg
            .scrape_max_text_chars
            .unwrap_or(crate::constants::DEFAULT_SCRAPE_MAX_TEXT_CHARS as u64),
        scrape_min_delay = cfg
            .scrape_min_delay_ms
            .unwrap_or(crate::constants::DEFAULT_SCRAPE_MIN_DELAY_MS),
        scrape_meta_robots = cfg.scrape_honor_meta_robots.unwrap_or(true),
        scrape_nofollow = cfg.scrape_honor_nofollow.unwrap_or(true),
        scrape_sitemap = cfg.scrape_use_sitemap.unwrap_or(true),
        scrape_default_engine = cfg
            .scrape_default_engine
            .as_deref()
            .unwrap_or(crate::constants::DEFAULT_SCRAPE_ENGINE),
        scrape_jitter = cfg
            .scrape_delay_jitter_ratio
            .unwrap_or(crate::constants::DEFAULT_SCRAPE_DELAY_JITTER_RATIO),
        scrape_summary = cfg
            .scrape_summary_chars
            .unwrap_or(crate::constants::DEFAULT_SCRAPE_SUMMARY_CHARS as u64),
        scrape_feed_max = cfg
            .scrape_feed_max_entries
            .unwrap_or(crate::constants::DEFAULT_SCRAPE_FEED_MAX_ENTRIES as u64),
        scrape_rel_next = cfg
            .scrape_follow_rel_next
            .unwrap_or(crate::constants::DEFAULT_SCRAPE_FOLLOW_REL_NEXT),
        scrape_dedup_similar = cfg
            .scrape_dedup_similar
            .unwrap_or(crate::constants::DEFAULT_SCRAPE_DEDUP_SIMILAR),
        scrape_no_cache = cfg
            .scrape_no_cache
            .unwrap_or(crate::constants::DEFAULT_SCRAPE_NO_CACHE),
        scrape_dedup_dist = cfg.scrape_dedup_similar_distance.unwrap_or(u64::from(
            crate::constants::DEFAULT_SCRAPE_DEDUP_SIMILAR_DISTANCE
        )),
        scrape_sitemap_max = cfg
            .scrape_sitemap_max_bytes
            .unwrap_or(crate::constants::DEFAULT_SCRAPE_SITEMAP_MAX_BYTES as u64),
        scrape_charset_peek = cfg
            .scrape_charset_peek_bytes
            .unwrap_or(crate::constants::DEFAULT_SCRAPE_CHARSET_PEEK_BYTES as u64),
        llm_http_timeout = cfg.llm_http_timeout_secs.unwrap_or(0),
        redis_allow_remote = cfg.redis_allow_remote.unwrap_or(false),
        chrome_legacy_oxide = cfg.chrome_legacy_oxide_launch.unwrap_or(false),
        redis_connect = cfg.redis_connect_timeout_secs.unwrap_or(0),
        robots_loopback_exempt = cfg.robots_loopback_exempt.unwrap_or(true),
        image_max_input = cfg
            .image_max_input_bytes
            .unwrap_or(crate::constants::DEFAULT_IMAGE_MAX_INPUT_BYTES),
        image_max_pixels = cfg
            .image_max_pixels
            .unwrap_or(crate::constants::DEFAULT_IMAGE_MAX_PIXELS),
        image_default_format = cfg.image_default_format.as_deref().unwrap_or(""),
        image_default_quality = cfg.image_default_quality.unwrap_or(0),
        image_download_max = cfg
            .image_download_max_bytes
            .unwrap_or(crate::constants::DEFAULT_IMAGE_DOWNLOAD_MAX_BYTES),
        video_max_input = cfg
            .video_max_input_bytes
            .unwrap_or(crate::constants::DEFAULT_VIDEO_MAX_INPUT_BYTES),
        video_download_max = cfg
            .video_download_max_bytes
            .unwrap_or(crate::constants::DEFAULT_VIDEO_DOWNLOAD_MAX_BYTES),
        video_default_container = cfg.video_default_container.as_deref().unwrap_or(""),
        video_default_crf = cfg.video_default_crf.unwrap_or(0),
        video_default_audio_bitrate = cfg.video_default_audio_bitrate.as_deref().unwrap_or(""),
        audio_max_input = cfg
            .audio_max_input_bytes
            .unwrap_or(crate::constants::DEFAULT_AUDIO_MAX_INPUT_BYTES),
        audio_download_max = cfg
            .audio_download_max_bytes
            .unwrap_or(crate::constants::DEFAULT_AUDIO_DOWNLOAD_MAX_BYTES),
        audio_default_format = cfg.audio_default_format.as_deref().unwrap_or(""),
        audio_default_bitrate = cfg.audio_default_bitrate.as_deref().unwrap_or(""),
    );
    // Optional multi-path discovery list (GAP-049); omitted when unset.
    if let Some(paths) = cfg.chrome_search_paths.as_ref().filter(|p| !p.is_empty()) {
        body.push_str(&format!(
            "chrome_search_paths = \"{}\"\n",
            join_path_list(paths)
        ));
    }
    if let Some(roots) = cfg.allowed_roots.as_ref().filter(|r| !r.is_empty()) {
        body.push_str(&format!("allowed_roots = \"{}\"\n", join_path_list(roots)));
    }
    // Keys the template cannot express, because they must be emitted only when
    // set. Their own module: see `config_write_optional` for why the boundary is
    // there and not somewhere convenient.
    super::config_write_optional::append_optional_keys(&mut body, cfg);
    // Promoted policy overrides (GAP-048); unset keys stay on the constant default.
    for (key, value) in super::policy::policy_pairs(&cfg.policy) {
        body.push_str(&format!("{key} = {value}\n"));
    }
    let tmp = path.with_extension("toml.tmp");
    {
        // Born `0600`: the temp file holds `proxy_password` while it is written,
        // and `File::create` would honour the umask and expose it until the chmod
        // that used to follow the rename. Creating it private removes the window
        // entirely instead of shortening it.
        let mut f = crate::platform::create_private_file(&tmp)
            .map_err(|e| CliError::new(ErrorKind::Io, format!("create temp config: {e}")))?;
        f.write_all(body.as_bytes())
            .map_err(|e| CliError::new(ErrorKind::Io, format!("write temp config: {e}")))?;
        f.sync_all()
            .map_err(|e| CliError::new(ErrorKind::Io, format!("fsync temp config: {e}")))?;
    }
    fs::rename(&tmp, &path)
        .map_err(|e| CliError::new(ErrorKind::Io, format!("rename config into place: {e}")))?;
    // The temp file above is already `0600` (see `create_private_file`), and the
    // mode survives the rename, so this call is belt-and-braces for a config
    // file that already existed with looser permissions. It reports failure
    // rather than discarding it: this file holds `proxy_password`, and the
    // product's own key catalog tells operators to keep the credential HERE
    // precisely because argv is visible in the process table. A permission
    // failure that stayed silent would betray that instruction.
    crate::platform::restrict_to_owner(&path, 0o600)
        .map_err(|e| CliError::new(ErrorKind::Io, format!("restrict config permissions: {e}")))?;
    Ok(path)
}
