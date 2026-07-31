// SPDX-License-Identifier: MIT OR Apache-2.0
//! XDG path resolution (L/M/W via `directories`).

use std::fs;
use std::path::{Path, PathBuf};

use directories::{BaseDirs, ProjectDirs, UserDirs};
use serde_json::{json, Value};

use super::config_io::write_config;
use super::config_model::ProductConfig;
use crate::error::{CliError, ErrorKind};

/// Product qualifier for `ProjectDirs` (reversed DNS style, cross-platform).
const QUALIFIER: &str = "cli";
/// Organization segment.
const ORGANIZATION: &str = "browser-automation";
/// Application name (matches binary / `Cargo.toml` package name at build time).
const APPLICATION: &str = env!("CARGO_PKG_NAME");

/// Resolve platform project directories.
pub fn project_dirs() -> Result<ProjectDirs, CliError> {
    ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION).ok_or_else(|| {
        CliError::with_suggestion(
            ErrorKind::Io,
            "cannot resolve XDG project directories",
            crate::i18n::suggestion_key("xdg_home_required", None),
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

/// Ephemeral Chrome user-data profiles under XDG cache (residual-aware marker prefix).
pub fn chrome_profiles_dir() -> Result<PathBuf, CliError> {
    Ok(cache_dir()?.join("chrome-profiles"))
}

/// Default sessions directory under XDG state.
pub fn sessions_dir() -> Result<PathBuf, CliError> {
    Ok(state_dir()?.join("sessions"))
}

/// Default workflow journal directory.
pub fn workflow_dir() -> Result<PathBuf, CliError> {
    Ok(state_dir()?.join("workflows"))
}

/// Rotated local tracing files under XDG state (`log_to_file`; never remote telemetry).
pub fn log_dir() -> Result<PathBuf, CliError> {
    Ok(state_dir()?.join("log"))
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
    ensure_dir(&chrome_profiles_dir()?)?;
    ensure_dir(&sessions_dir()?)?;
    ensure_dir(&workflow_dir()?)?;
    ensure_dir(&log_dir()?)?;
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
        "chrome_profiles_dir": chrome_profiles_dir()?.display().to_string(),
        "sessions_dir": sessions_dir()?.display().to_string(),
        "workflow_dir": workflow_dir()?.display().to_string(),
        "log_dir": log_dir()?.display().to_string(),
        "mitm_ca_dir": mitm_ca_dir()?.display().to_string(),
    }))
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
        "chrome_profiles_dir": chrome_profiles_dir()?.display().to_string(),
        "sessions_dir": sessions_dir()?.display().to_string(),
        "workflow_dir": workflow_dir()?.display().to_string(),
        "log_dir": log_dir()?.display().to_string(),
        "mitm_ca_dir": mitm_ca_dir()?.display().to_string(),
        "mitm_capture_dir": mitm_capture_dir()?.display().to_string(),
        "home_dir": home.or(user_dirs),
        "layout": "xdg",
    }))
}
