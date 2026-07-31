// SPDX-License-Identifier: MIT OR Apache-2.0
//! Local-only tracing init for the one-shot CLI (rules_rust_logs_com_tracing_e_rotacao).
//!
//! Module name is **`tracing_local`** (not product telemetry). It only installs a
//! process-local `tracing` subscriber (stderr + optional XDG rotated files).
//!
//! # Product law
//!
//! - **No remote telemetry**: no OpenTelemetry, OTLP, Sentry, or log shipping.
//! - **stderr by default**: agent pipelines keep stdout for JSON envelopes.
//! - **Optional XDG file**: `config set log_to_file true` writes rotated JSON under
//!   [`crate::xdg::log_dir`] (never cloud).
//! - **No `RUST_LOG` product path**: filter comes from argv (`-q`/`-v`/`--debug`) or
//!   XDG `log_level` (default [`crate::constants::DEFAULT_LOG_LEVEL`]).
//! - **Knobs via XDG only**: `log_level`, `log_to_file`, `max_log_files`, `log_rotation`.
//!
//! # N/A (one-shot + product ban — do not implement)
//!
//! | Rule (generic) | Why N/A |
//! |----------------|---------|
//! | `reload::Layer` + admin HTTP | BORN→DIE; no daemon / no admin surface |
//! | OTEL / OTLP / metrics export | **PROIBIDO telemetria** |
//! | `RUST_LOG` / `OTEL_*` env | **PROIBIDO env produto** |
//! | journald / Lambda / Docker driver | XDG file + stderr multiplataforma |
//! | tokio-console in product binary | dev-only |
//!
//! # Lifecycle (one-shot)
//!
//! `init_tracing_local` installs the global subscriber **once** (from [`crate::run`])
//! and returns a `TracingLocalGuard`. When file logging is enabled, the guard owns a
//! `tracing_appender` `WorkerGuard` so buffered lines flush on drop at process end.
//! Hold the guard until FINALIZE completes — do not `mem::forget` it.
//!
//! # Memory / parallelism
//!
//! - **Memory:** single XDG `load_config` per init; default filter keeps event volume low.
//! - **Parallelism:** `non_blocking` spawns one writer thread only when `log_to_file`;
//!   application/CDP fan-out is never blocked on log fsync.

use std::io::{self, IsTerminal};
use std::path::PathBuf;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

use crate::constants::{
    DEFAULT_LOG_LEVEL, DEFAULT_LOG_ROTATION, DEFAULT_MAX_LOG_FILES, MAX_LOG_FILES_CAP,
    MAX_LOG_FILES_MIN,
};
use crate::error::{CliError, ErrorKind};

/// Filename prefix for rotated logs (`{pkg}.YYYY-MM-DD` …).
pub const LOG_FILE_PREFIX: &str = env!("CARGO_PKG_NAME");

/// Unix directory mode for the log directory (owner-only).
#[cfg(unix)]
const LOG_DIR_MODE: u32 = 0o700;

/// Process-scoped local tracing handle. Drop flushes the optional non-blocking file worker.
///
/// Named field (not bare `_`) so the guard is never discarded by accident.
#[derive(Debug, Default)]
pub struct TracingLocalGuard {
    /// When `Some`, keeps the appender worker alive and flushes on drop.
    _file_worker: Option<WorkerGuard>,
}

impl TracingLocalGuard {
    /// Empty guard (stderr-only path, or subscriber already installed).
    #[must_use]
    pub fn none() -> Self {
        Self { _file_worker: None }
    }
}

/// Inputs for local tracing (mirrors CLI globals).
///
/// Agent `correlation_id` is read from [`crate::agent_context`] at init time
/// (set in `run()` before this call).
#[derive(Debug, Clone, Copy)]
pub struct TracingLocalOpts {
    /// `--quiet` / `-q` → force `error` only.
    pub quiet: bool,
    /// `--verbose` / `-v` → `info`.
    pub verbose: bool,
    /// `--debug` → `debug`.
    pub debug: bool,
    /// `--plain` / NO_COLOR / agent plain: disable ANSI on stderr.
    pub plain: bool,
}

/// Resolve the EnvFilter directive string (testable pure function).
///
/// Priority: quiet > debug > verbose > non-empty XDG `log_level` > [`DEFAULT_LOG_LEVEL`].
#[must_use]
pub fn resolve_filter_directive(
    quiet: bool,
    verbose: bool,
    debug: bool,
    xdg_level: Option<&str>,
) -> String {
    if quiet {
        return DEFAULT_LOG_LEVEL.to_string();
    }
    if debug {
        return "debug".to_string();
    }
    if verbose {
        return "info".to_string();
    }
    if let Some(level) = xdg_level.map(str::trim).filter(|s| !s.is_empty()) {
        return level.to_string();
    }
    DEFAULT_LOG_LEVEL.to_string()
}

/// Validate an EnvFilter directive for `config set log_level` (strict; product XDG path).
pub fn validate_log_level_directive(value: &str) -> Result<(), CliError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(CliError::new(
            ErrorKind::Usage,
            "log_level must be a non-empty EnvFilter directive (e.g. error, info, debug)",
        ));
    }
    EnvFilter::try_new(trimmed).map_err(|e| {
        CliError::with_suggestion(
            ErrorKind::Usage,
            format!("invalid log_level EnvFilter directive: {e}"),
            crate::i18n::suggestion_key("use_listed_value", None),
        )
    })?;
    Ok(())
}

/// Validate XDG `log_rotation` (`daily` | `hourly` | `never`).
pub fn validate_log_rotation(value: &str) -> Result<(), CliError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "daily" | "hourly" | "never" => Ok(()),
        other => Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("invalid log_rotation: {other}"),
            crate::i18n::suggestion_key("use_listed_value", None),
        )),
    }
}

/// Map XDG rotation string to appender [`Rotation`] (permissive; unknown → daily).
#[must_use]
pub fn parse_log_rotation(value: Option<&str>) -> Rotation {
    match value
        .map(str::trim)
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
    {
        Some("hourly") => Rotation::HOURLY,
        Some("never") => Rotation::NEVER,
        Some("daily") | None | Some("") => Rotation::DAILY,
        Some(_) => Rotation::DAILY,
    }
}

/// Clamp loaded `max_log_files` into the allowed range (permissive load path).
#[must_use]
pub fn clamp_max_log_files(raw: Option<u32>) -> usize {
    let n = raw.unwrap_or(DEFAULT_MAX_LOG_FILES);
    let n = n.clamp(MAX_LOG_FILES_MIN, MAX_LOG_FILES_CAP);
    n as usize
}

mod init;
#[cfg(test)]
mod tests;

pub use init::init_tracing_local;
