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

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use directories::{BaseDirs, ProjectDirs, UserDirs};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};

/// Product qualifier for `ProjectDirs` (reversed DNS style, cross-platform).
const QUALIFIER: &str = "cli";
/// Organization segment.
const ORGANIZATION: &str = "browser-automation";
/// Application name (matches binary).
const APPLICATION: &str = "browser-automation-cli";

/// Resolve platform project directories or a deterministic fallback under the system temp dir.
pub fn project_dirs() -> Result<ProjectDirs, CliError> {
    ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION).ok_or_else(|| {
        CliError::with_suggestion(
            ErrorKind::Io,
            "cannot resolve XDG project directories",
            "Ensure the home directory is available for this user",
        )
    })
}

/// Config directory (`…/browser-automation-cli`).
pub fn config_dir() -> Result<PathBuf, CliError> {
    Ok(project_dirs()?.config_dir().to_path_buf())
}

/// Data directory (sessions, journals, durable artifacts).
pub fn data_dir() -> Result<PathBuf, CliError> {
    Ok(project_dirs()?.data_dir().to_path_buf())
}

/// Cache directory (lighthouse reports, HTTP scrape cache, browsers cache).
pub fn cache_dir() -> Result<PathBuf, CliError> {
    Ok(project_dirs()?.cache_dir().to_path_buf())
}

/// State directory (runtime state, workflow journal default).
pub fn state_dir() -> Result<PathBuf, CliError> {
    // `directories` 5.x exposes state_dir on Unix via ProjectDirs when available.
    let pd = project_dirs()?;
    #[allow(deprecated)]
    let state = pd
        .state_dir()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| pd.data_dir().join("state"));
    Ok(state)
}

/// Default browsers cache under XDG cache.
pub fn browsers_dir() -> Result<PathBuf, CliError> {
    Ok(cache_dir()?.join("browsers"))
}

/// Default sessions directory under XDG state.
pub fn sessions_dir() -> Result<PathBuf, CliError> {
    Ok(state_dir()?.join("sessions"))
}

/// Default workflow journal directory.
pub fn workflow_dir() -> Result<PathBuf, CliError> {
    Ok(state_dir()?.join("workflows"))
}

/// Default MITM CA directory.
pub fn mitm_ca_dir() -> Result<PathBuf, CliError> {
    Ok(data_dir()?.join("mitm").join("ca"))
}

/// Default MITM capture directory for the invocation artifacts.
pub fn mitm_capture_dir() -> Result<PathBuf, CliError> {
    Ok(state_dir()?.join("mitm"))
}

/// Path to the TOML config file.
pub fn config_file() -> Result<PathBuf, CliError> {
    Ok(config_dir()?.join("config.toml"))
}

/// Ensure a directory exists with restrictive permissions when possible.
pub fn ensure_dir(path: &Path) -> Result<(), CliError> {
    fs::create_dir_all(path).map_err(|e| {
        CliError::new(
            ErrorKind::Io,
            format!("create directory {}: {e}", path.display()),
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o700));
    }
    Ok(())
}

/// Create all standard XDG product directories.
pub fn init_layout() -> Result<Value, CliError> {
    let cfg = config_dir()?;
    let data = data_dir()?;
    let cache = cache_dir()?;
    let state = state_dir()?;
    ensure_dir(&cfg)?;
    ensure_dir(&data)?;
    ensure_dir(&cache)?;
    ensure_dir(&state)?;
    ensure_dir(&browsers_dir()?)?;
    ensure_dir(&sessions_dir()?)?;
    ensure_dir(&workflow_dir()?)?;
    ensure_dir(&mitm_ca_dir()?)?;
    ensure_dir(&mitm_capture_dir()?)?;
    let cfg_file = config_file()?;
    if !cfg_file.exists() {
        let default = ProductConfig::default();
        write_config(&default)?;
    }
    Ok(json!({
        "config_dir": cfg.display().to_string(),
        "data_dir": data.display().to_string(),
        "cache_dir": cache.display().to_string(),
        "state_dir": state.display().to_string(),
        "config_file": cfg_file.display().to_string(),
        "browsers_dir": browsers_dir()?.display().to_string(),
        "sessions_dir": sessions_dir()?.display().to_string(),
        "workflow_dir": workflow_dir()?.display().to_string(),
        "mitm_ca_dir": mitm_ca_dir()?.display().to_string(),
    }))
}

/// On-disk product configuration (TOML). Flags override these at parse time.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProductConfig {
    /// Default language override (`en` / `pt-BR`).
    #[serde(default)]
    pub lang: Option<String>,
    /// Default global timeout seconds (0 = none).
    #[serde(default)]
    pub timeout: Option<u64>,
    /// Default artifacts directory.
    #[serde(default)]
    pub artifacts_dir: Option<String>,
    /// Default ignore-robots (requires explicit risk acceptance in flags still).
    #[serde(default)]
    pub ignore_robots: Option<bool>,
    /// Namespace for isolated state trees (optional).
    #[serde(default)]
    pub namespace: Option<String>,
    /// Optional AES key material for encrypted session state (stored in XDG config, mode 0600).
    #[serde(default)]
    pub encryption_key: Option<String>,
    /// Enable ANSI colors on human stderr paths when true.
    #[serde(default)]
    pub color: Option<bool>,
    /// Tracing filter level when flags are quiet/default (`error`/`info`/`debug`).
    #[serde(default)]
    pub log_level: Option<String>,
    /// Absolute path to Chrome/Chromium binary (XDG only; never product env).
    #[serde(default)]
    pub chrome_path: Option<String>,
    /// Absolute path to lighthouse CLI (optional).
    #[serde(default)]
    pub lighthouse_path: Option<String>,
    /// Optional LLM provider API key for extract --llm (stored in XDG config mode 0600).
    #[serde(default)]
    pub openrouter_api_key: Option<String>,
    /// OpenAI-compatible API base URL (no trailing slash).
    #[serde(default)]
    pub llm_base_url: Option<String>,
    /// Default model id for extract --llm.
    #[serde(default)]
    pub llm_model: Option<String>,
}

/// Load config from XDG path; missing file yields defaults.
pub fn load_config() -> Result<ProductConfig, CliError> {
    let path = config_file()?;
    if !path.exists() {
        return Ok(ProductConfig::default());
    }
    let raw = fs::read_to_string(&path).map_err(|e| {
        CliError::new(
            ErrorKind::Io,
            format!("read config {}: {e}", path.display()),
        )
    })?;
    // Minimal TOML subset via serde if we add toml; otherwise JSON fallback.
    // Prefer JSON if file ends with .json — primary is TOML via line parse for keys we need.
    if path.extension().and_then(|e| e.to_str()) == Some("json") {
        return serde_json::from_str(&raw)
            .map_err(|e| CliError::new(ErrorKind::Data, format!("invalid config JSON: {e}")));
    }
    parse_simple_toml(&raw)
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
        match k {
            "lang" => cfg.lang = Some(v.to_string()),
            "timeout" => cfg.timeout = v.parse().ok(),
            "artifacts_dir" => cfg.artifacts_dir = Some(v.to_string()),
            "ignore_robots" => cfg.ignore_robots = Some(v == "true" || v == "1"),
            "namespace" => cfg.namespace = Some(v.to_string()),
            "encryption_key" => cfg.encryption_key = Some(v.to_string()),
            "color" => cfg.color = Some(v == "true" || v == "1"),
            "log_level" => cfg.log_level = Some(v.to_string()),
            "chrome_path" => cfg.chrome_path = Some(v.to_string()),
            "lighthouse_path" => cfg.lighthouse_path = Some(v.to_string()),
            "openrouter_api_key" => cfg.openrouter_api_key = Some(v.to_string()),
            "llm_base_url" => cfg.llm_base_url = Some(v.to_string()),
            "llm_model" => cfg.llm_model = Some(v.to_string()),
            _ => {}
        }
    }
    Ok(cfg)
}

/// Write config atomically (temp + rename).
pub fn write_config(cfg: &ProductConfig) -> Result<PathBuf, CliError> {
    let dir = config_dir()?;
    ensure_dir(&dir)?;
    let path = config_file()?;
    let body = format!(
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
         chrome_path = \"{chrome_path}\"\n\
         lighthouse_path = \"{lighthouse_path}\"\n\
         openrouter_api_key = \"{openrouter_api_key}\"\n\
         llm_base_url = \"{llm_base_url}\"\n\
         llm_model = \"{llm_model}\"\n",
        lang = cfg.lang.as_deref().unwrap_or(""),
        timeout = cfg.timeout.unwrap_or(0),
        artifacts = cfg.artifacts_dir.as_deref().unwrap_or(""),
        ignore = cfg.ignore_robots.unwrap_or(false),
        ns = cfg.namespace.as_deref().unwrap_or(""),
        enc = cfg.encryption_key.as_deref().unwrap_or(""),
        color = cfg.color.unwrap_or(false),
        log_level = cfg.log_level.as_deref().unwrap_or(""),
        chrome_path = cfg.chrome_path.as_deref().unwrap_or(""),
        lighthouse_path = cfg.lighthouse_path.as_deref().unwrap_or(""),
        openrouter_api_key = cfg.openrouter_api_key.as_deref().unwrap_or(""),
        llm_base_url = cfg.llm_base_url.as_deref().unwrap_or(""),
        llm_model = cfg.llm_model.as_deref().unwrap_or(""),
    );
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

/// Set one string key in config and persist.
pub fn config_set(key: &str, value: &str) -> Result<Value, CliError> {
    let mut cfg = load_config()?;
    match key {
        "lang" => cfg.lang = Some(value.to_string()),
        "timeout" => {
            cfg.timeout = Some(value.parse().map_err(|_| {
                CliError::new(ErrorKind::Usage, "timeout must be an integer seconds")
            })?);
        }
        "artifacts_dir" => cfg.artifacts_dir = Some(value.to_string()),
        "ignore_robots" => {
            cfg.ignore_robots = Some(matches!(value, "true" | "1" | "yes"));
        }
        "namespace" => cfg.namespace = Some(value.to_string()),
        "encryption_key" => cfg.encryption_key = Some(value.to_string()),
        "color" => {
            cfg.color = Some(matches!(value, "true" | "1" | "yes"));
        }
        "log_level" => cfg.log_level = Some(value.to_string()),
        "chrome_path" => cfg.chrome_path = Some(value.to_string()),
        "lighthouse_path" => cfg.lighthouse_path = Some(value.to_string()),
        "openrouter_api_key" => cfg.openrouter_api_key = Some(value.to_string()),
        "llm_base_url" => cfg.llm_base_url = Some(value.to_string()),
        "llm_model" => cfg.llm_model = Some(value.to_string()),
        other => {
            return Err(CliError::with_suggestion(
                ErrorKind::Usage,
                format!("unknown config key: {other}"),
                "Supported keys: lang, timeout, artifacts_dir, ignore_robots, namespace, encryption_key, color, log_level, chrome_path, lighthouse_path, openrouter_api_key, llm_base_url, llm_model",
            ));
        }
    }
    let path = write_config(&cfg)?;
    Ok(json!({
        "key": key,
        "value": value,
        "path": path.display().to_string(),
    }))
}

/// Get one config key (or full dump when key is empty).
pub fn config_get(key: Option<&str>) -> Result<Value, CliError> {
    let cfg = load_config()?;
    match key {
        None | Some("") => Ok(json!({
            "lang": cfg.lang,
            "timeout": cfg.timeout,
            "artifacts_dir": cfg.artifacts_dir,
            "ignore_robots": cfg.ignore_robots,
            "namespace": cfg.namespace,
            "path": config_file()?.display().to_string(),
        })),
        Some("lang") => Ok(json!({ "key": "lang", "value": cfg.lang })),
        Some("timeout") => Ok(json!({ "key": "timeout", "value": cfg.timeout })),
        Some("artifacts_dir") => Ok(json!({ "key": "artifacts_dir", "value": cfg.artifacts_dir })),
        Some("ignore_robots") => Ok(json!({ "key": "ignore_robots", "value": cfg.ignore_robots })),
        Some("namespace") => Ok(json!({ "key": "namespace", "value": cfg.namespace })),
        Some("log_level") => Ok(json!({ "key": "log_level", "value": cfg.log_level })),
        Some("chrome_path") => Ok(json!({ "key": "chrome_path", "value": cfg.chrome_path })),
        Some("lighthouse_path") => {
            Ok(json!({ "key": "lighthouse_path", "value": cfg.lighthouse_path }))
        }
        Some("openrouter_api_key") => Ok(json!({
            "key": "openrouter_api_key",
            "value": if cfg.openrouter_api_key.as_ref().map(|s| !s.is_empty()).unwrap_or(false) {
                "[set]"
            } else {
                ""
            }
        })),
        Some("llm_base_url") => Ok(json!({ "key": "llm_base_url", "value": cfg.llm_base_url })),
        Some("llm_model") => Ok(json!({ "key": "llm_model", "value": cfg.llm_model })),
        Some(other) => Err(CliError::new(
            ErrorKind::Usage,
            format!("unknown config key: {other}"),
        )),
    }
}

/// Load optional encryption key from XDG config only (never product env vars).
pub fn encryption_key() -> Option<String> {
    load_config()
        .ok()
        .and_then(|c| c.encryption_key)
        .filter(|s| !s.is_empty())
}

/// Chrome/Chromium binary from XDG config only.
pub fn chrome_path_from_config() -> Option<String> {
    load_config()
        .ok()
        .and_then(|c| c.chrome_path)
        .filter(|s| !s.is_empty())
}

/// Lighthouse binary path from XDG config only.
pub fn lighthouse_path_from_config() -> Option<String> {
    load_config()
        .ok()
        .and_then(|c| c.lighthouse_path)
        .filter(|s| !s.is_empty())
}

/// Optional LLM API key from XDG config only (never product env vars).
pub fn openrouter_api_key() -> Option<String> {
    load_config()
        .ok()
        .and_then(|c| c.openrouter_api_key)
        .filter(|s| !s.is_empty())
}

/// Optional LLM base URL from XDG config only.
pub fn llm_base_url() -> Option<String> {
    load_config()
        .ok()
        .and_then(|c| c.llm_base_url)
        .filter(|s| !s.is_empty())
}

/// Optional LLM model id from XDG config only.
pub fn llm_model() -> Option<String> {
    load_config()
        .ok()
        .and_then(|c| c.llm_model)
        .filter(|s| !s.is_empty())
}

/// JSON snapshot of all resolved paths (for `config path` / doctor).
pub fn paths_snapshot() -> Result<Value, CliError> {
    let home = BaseDirs::new().map(|b| b.home_dir().display().to_string());
    let user_dirs = UserDirs::new().map(|u| u.home_dir().display().to_string());
    Ok(json!({
        "config_dir": config_dir()?.display().to_string(),
        "data_dir": data_dir()?.display().to_string(),
        "cache_dir": cache_dir()?.display().to_string(),
        "state_dir": state_dir()?.display().to_string(),
        "config_file": config_file()?.display().to_string(),
        "browsers_dir": browsers_dir()?.display().to_string(),
        "sessions_dir": sessions_dir()?.display().to_string(),
        "workflow_dir": workflow_dir()?.display().to_string(),
        "mitm_ca_dir": mitm_ca_dir()?.display().to_string(),
        "mitm_capture_dir": mitm_capture_dir()?.display().to_string(),
        "home_dir": home.or(user_dirs),
        "layout": "xdg",
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_dirs_resolve() {
        assert!(project_dirs().is_ok());
        assert!(config_dir().unwrap().components().count() > 1);
    }
}
