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
///
/// # Errors
///
/// [`ErrorKind::Io`] when `ProjectDirs::from` cannot locate a home directory
/// (no `$HOME` on Unix, no known-folder path on Windows). The suggestion points
/// at the `xdg_home_required` remediation.
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
///
/// # Errors
///
/// [`ErrorKind::Io`] propagated from [`project_dirs`] when no home directory
/// can be resolved; the config directory itself is derived infallibly.
pub fn config_dir() -> Result<PathBuf, CliError> {
    Ok(project_dirs()?.config_dir().to_path_buf())
}

/// Data directory (sessions, journals, durable artifacts).
///
/// # Errors
///
/// [`ErrorKind::Io`] propagated from [`project_dirs`] when no home directory
/// can be resolved; the data directory itself is derived infallibly.
pub fn data_dir() -> Result<PathBuf, CliError> {
    Ok(project_dirs()?.data_dir().to_path_buf())
}

/// Cache directory (lighthouse reports, HTTP scrape cache, browsers cache).
///
/// # Errors
///
/// [`ErrorKind::Io`] propagated from [`project_dirs`] when no home directory
/// can be resolved; the cache directory itself is derived infallibly.
pub fn cache_dir() -> Result<PathBuf, CliError> {
    Ok(project_dirs()?.cache_dir().to_path_buf())
}

/// State directory (runtime state, workflow journal default).
///
/// # Errors
///
/// [`ErrorKind::Io`] propagated from [`project_dirs`]. A platform without a
/// dedicated state directory is not an error: it falls back to
/// `<data_dir>/state`.
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
///
/// # Errors
///
/// [`ErrorKind::Io`] propagated from [`cache_dir`], itself propagated from
/// [`project_dirs`] when no home directory can be resolved.
pub fn browsers_dir() -> Result<PathBuf, CliError> {
    Ok(cache_dir()?.join("browsers"))
}

/// Ephemeral Chrome user-data profiles under XDG cache (residual-aware marker prefix).
///
/// # Errors
///
/// [`ErrorKind::Io`] propagated from [`cache_dir`], itself propagated from
/// [`project_dirs`] when no home directory can be resolved.
pub fn chrome_profiles_dir() -> Result<PathBuf, CliError> {
    Ok(cache_dir()?.join("chrome-profiles"))
}

/// Default sessions directory under XDG state.
///
/// # Errors
///
/// [`ErrorKind::Io`] propagated from [`state_dir`], itself propagated from
/// [`project_dirs`] when no home directory can be resolved.
pub fn sessions_dir() -> Result<PathBuf, CliError> {
    Ok(state_dir()?.join("sessions"))
}

/// Default workflow journal directory.
///
/// # Errors
///
/// [`ErrorKind::Io`] propagated from [`state_dir`], itself propagated from
/// [`project_dirs`] when no home directory can be resolved.
pub fn workflow_dir() -> Result<PathBuf, CliError> {
    Ok(state_dir()?.join("workflows"))
}

/// Rotated local tracing files under XDG state (`log_to_file`; never remote telemetry).
///
/// # Errors
///
/// [`ErrorKind::Io`] propagated from [`state_dir`], itself propagated from
/// [`project_dirs`] when no home directory can be resolved.
pub fn log_dir() -> Result<PathBuf, CliError> {
    Ok(state_dir()?.join("log"))
}

/// Default MITM CA directory.
///
/// # Errors
///
/// [`ErrorKind::Io`] propagated from [`data_dir`], itself propagated from
/// [`project_dirs`] when no home directory can be resolved.
pub fn mitm_ca_dir() -> Result<PathBuf, CliError> {
    Ok(data_dir()?.join("mitm").join("ca"))
}

/// Default MITM capture directory for the invocation artifacts.
///
/// # Errors
///
/// [`ErrorKind::Io`] propagated from [`state_dir`], itself propagated from
/// [`project_dirs`] when no home directory can be resolved.
pub fn mitm_capture_dir() -> Result<PathBuf, CliError> {
    Ok(state_dir()?.join("mitm"))
}

/// Path to the TOML config file.
///
/// # Errors
///
/// [`ErrorKind::Io`] propagated from [`config_dir`], itself propagated from
/// [`project_dirs`] when no home directory can be resolved.
pub fn config_file() -> Result<PathBuf, CliError> {
    Ok(config_dir()?.join("config.toml"))
}

/// Ensure a directory exists with restrictive permissions when possible.
///
/// # Errors
///
/// [`ErrorKind::Io`] when `std::fs::create_dir_all` fails — a missing parent
/// that cannot be created, a permission denial, or a non-directory component in
/// the path. Tightening the mode to `0o700` on Unix is best-effort and never
/// turns into an error.
pub fn ensure_dir(path: &Path) -> Result<(), CliError> {
    fs::create_dir_all(path).map_err(|e| {
        CliError::new(
            ErrorKind::Io,
            format!("create directory {}: {e}", path.display()),
        )
    })?;
    // `0700` and not best-effort: this directory holds the MITM CA private key,
    // the config file with `proxy_password`, and the stealth seed cache. A
    // failure here used to be discarded, leaving the whole tree at whatever the
    // umask allowed while the product reported success.
    crate::platform::restrict_to_owner(path, 0o700).map_err(|e| {
        CliError::new(
            ErrorKind::Io,
            format!("restrict directory {}: {e}", path.display()),
        )
    })?;
    Ok(())
}

/// Create all standard XDG product directories.
///
/// # Errors
///
/// [`ErrorKind::Io`] from [`project_dirs`] when no home directory resolves, or
/// from [`ensure_dir`] when any of the product directories cannot be created.
/// [`ErrorKind::Io`] also propagates from
/// [`write_config`] when the default
/// `config.toml` is materialized on first run.
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
///
/// # Errors
///
/// [`ErrorKind::Io`] propagated from [`project_dirs`] via the individual path
/// accessors when no home directory can be resolved. Nothing is created or
/// written here, so no other failure mode exists.
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
