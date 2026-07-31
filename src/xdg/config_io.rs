// SPDX-License-Identifier: MIT OR Apache-2.0
//! Load / parse / atomic write of XDG `config.toml`.

use std::fs;
use std::io::Write;

use super::config_model::ProductConfig;
use super::paths::{config_dir, config_file, ensure_dir};
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
pub(crate) fn apply_toml_kv(cfg: &mut ProductConfig, k: &str, v: &str) {
    match k {
        "lang" => {
            // Permissive load: invalid tokens dropped (strict reject is `config set`).
            if crate::i18n::UiLocale::parse_token(v).is_some() {
                cfg.lang = Some(v.to_string());
            }
        }
        "timeout" => cfg.timeout = v.parse().ok(),
        "artifacts_dir" => cfg.artifacts_dir = Some(v.to_string()),
        "ignore_robots" => cfg.ignore_robots = Some(v == "true" || v == "1"),
        "namespace" => cfg.namespace = Some(v.to_string()),
        "encryption_key" => cfg.encryption_key = Some(v.to_string()),
        "color" => cfg.color = Some(v == "true" || v == "1"),
        "log_level" => cfg.log_level = Some(v.to_string()),
        "chrome_path" => cfg.chrome_path = Some(v.to_string()),
        "lighthouse_path" => cfg.lighthouse_path = Some(v.to_string()),
        "ffmpeg_path" => cfg.ffmpeg_path = Some(v.to_string()),
        "lighthouse_timeout_secs" => cfg.lighthouse_timeout_secs = v.parse().ok(),
        "ffmpeg_timeout_secs" => cfg.ffmpeg_timeout_secs = v.parse().ok(),
        "openrouter_api_key" => cfg.openrouter_api_key = Some(v.to_string()),
        "llm_base_url" => cfg.llm_base_url = Some(v.to_string()),
        "llm_model" => cfg.llm_model = Some(v.to_string()),
        "log_to_file" => cfg.log_to_file = Some(v == "true" || v == "1"),
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
        "llm_http_timeout_secs" => cfg.llm_http_timeout_secs = v.parse().ok(),
        "redis_allow_remote" => cfg.redis_allow_remote = Some(v == "true" || v == "1"),
        "robots_loopback_exempt" => cfg.robots_loopback_exempt = Some(v == "true" || v == "1"),
        "redis_connect_timeout_secs" => cfg.redis_connect_timeout_secs = v.parse().ok(),
        "chrome_search_paths" => cfg.chrome_search_paths = Some(split_path_list(v)),
        "allowed_roots" => cfg.allowed_roots = Some(split_path_list(v)),
        other => {
            // Promoted policy knobs (GAP-048) share this loose table.
            super::policy::policy_apply_raw(&mut cfg.policy, other, v);
        }
    }
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
        apply_toml_kv(&mut cfg, k, v);
    }
    Ok(cfg)
}

/// Write config atomically (temp + rename).
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
         llm_http_timeout_secs = {llm_http_timeout}\n\
         redis_allow_remote = {redis_allow_remote}\n\
         redis_connect_timeout_secs = {redis_connect}\n\
         robots_loopback_exempt = {robots_loopback_exempt}\n",
        lang = cfg.lang.as_deref().unwrap_or(""),
        timeout = cfg.timeout.unwrap_or(0),
        artifacts = cfg.artifacts_dir.as_deref().unwrap_or(""),
        ignore = cfg.ignore_robots.unwrap_or(false),
        ns = cfg.namespace.as_deref().unwrap_or(""),
        enc = cfg.encryption_key.as_deref().unwrap_or(""),
        color = cfg.color.unwrap_or(false),
        log_level = cfg.log_level.as_deref().unwrap_or(""),
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
        llm_http_timeout = cfg.llm_http_timeout_secs.unwrap_or(0),
        redis_allow_remote = cfg.redis_allow_remote.unwrap_or(false),
        redis_connect = cfg.redis_connect_timeout_secs.unwrap_or(0),
        robots_loopback_exempt = cfg.robots_loopback_exempt.unwrap_or(true),
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
    // Promoted policy overrides (GAP-048); unset keys stay on the constant default.
    for (key, value) in super::policy::policy_pairs(&cfg.policy) {
        body.push_str(&format!("{key} = {value}\n"));
    }
    let tmp = path.with_extension("toml.tmp");
    {
        let mut f = fs::File::create(&tmp)
            .map_err(|e| CliError::new(ErrorKind::Io, format!("create temp config: {e}")))?;
        f.write_all(body.as_bytes())
            .map_err(|e| CliError::new(ErrorKind::Io, format!("write temp config: {e}")))?;
        f.sync_all()
            .map_err(|e| CliError::new(ErrorKind::Io, format!("fsync temp config: {e}")))?;
    }
    fs::rename(&tmp, &path)
        .map_err(|e| CliError::new(ErrorKind::Io, format!("rename config into place: {e}")))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(path)
}
