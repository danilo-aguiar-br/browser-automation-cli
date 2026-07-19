// SPDX-License-Identifier: MIT OR Apache-2.0
//! Local-only tracing init for the one-shot CLI (rules_rust_logs_com_tracing_e_rotacao).
//!
//! # Product law
//!
//! - **No remote telemetry**: no OpenTelemetry, OTLP, Sentry, or log shipping.
//! - **stderr by default**: agent pipelines keep stdout for JSON envelopes.
//! - **Optional XDG file**: `config set log_to_file true` writes rotated JSON under
//!   `$XDG_STATE_HOME/browser-automation-cli/log/` (never cloud).
//! - **No `RUST_LOG` product path**: filter comes from argv (`-q`/`-v`/`--debug`) or
//!   XDG `log_level` (default `error`).
//! - **Daemon-only rules N/A**: `reload::Layer` admin HTTP, OTEL sampling, journald,
//!   Lambda flush, encrypted-at-rest pipelines — not applicable to BORN→DIE.
//!
//! # Targets emitted
//!
//! Prefer `browser_automation_cli::<module>` (crate name with underscores). The
//! telemetry module itself logs `browser_automation_cli::telemetry` on successful init.
//!
//! # Lifecycle
//!
//! [`init_telemetry`] installs the global subscriber **once** (from [`crate::run`]) and
//! returns a [`TelemetryGuard`]. When file logging is enabled, the guard owns a
//! `tracing_appender` [`WorkerGuard`] so buffered lines flush on drop at process end.
//! Hold the guard until FINALIZE completes — do not `mem::forget` it.

use std::io::{self, IsTerminal};
use std::path::PathBuf;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

/// Maximum retained daily log files under XDG state (retention ≈ 2 weeks).
pub const MAX_LOG_FILES: usize = 14;

/// Filename prefix for rotated logs (`browser-automation-cli.YYYY-MM-DD`).
pub const LOG_FILE_PREFIX: &str = "browser-automation-cli";

/// Process-scoped telemetry handle. Drop flushes the optional non-blocking file worker.
///
/// Named field (not bare `_`) so the guard is never discarded by accident.
#[derive(Debug, Default)]
pub struct TelemetryGuard {
    /// When `Some`, keeps the appender worker alive and flushes on drop.
    _file_worker: Option<WorkerGuard>,
}

impl TelemetryGuard {
    /// Empty guard (stderr-only path, or subscriber already installed).
    #[must_use]
    pub fn none() -> Self {
        Self {
            _file_worker: None,
        }
    }
}

/// Inputs for local tracing (mirrors CLI globals + XDG).
#[derive(Debug, Clone, Copy)]
pub struct TelemetryOpts {
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
/// Priority: quiet > debug > verbose > non-empty XDG `log_level` > `error`.
#[must_use]
pub fn resolve_filter_directive(
    quiet: bool,
    verbose: bool,
    debug: bool,
    xdg_level: Option<&str>,
) -> String {
    if quiet {
        return "error".to_string();
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
    "error".to_string()
}

/// Install the global tracing subscriber (once) and return a process-lifetime guard.
///
/// Safe to call when a subscriber is already installed (tests / re-entry): returns
/// [`TelemetryGuard::none`] without replacing the existing subscriber.
///
/// # Panic hook
///
/// After a successful install, chains a hook that emits a `tracing` `error` event
/// (target `panic`) then calls the previous hook (e.g. `human_panic` from `main`).
#[must_use]
pub fn init_telemetry(opts: TelemetryOpts) -> TelemetryGuard {
    let xdg_level = crate::xdg::load_config()
        .ok()
        .and_then(|c| c.log_level)
        .filter(|s| !s.trim().is_empty());
    let directive = resolve_filter_directive(
        opts.quiet,
        opts.verbose,
        opts.debug,
        xdg_level.as_deref(),
    );
    let log_to_file = crate::xdg::load_config()
        .ok()
        .and_then(|c| c.log_to_file)
        .unwrap_or(false);

    let use_ansi = !opts.plain && crate::color::is_enabled() && io::stderr().is_terminal();

    // Invalid XDG directive must not abort the CLI; fall back to safe default.
    let (filter, effective) = match EnvFilter::try_new(&directive) {
        Ok(f) => (f, directive),
        Err(_) => (EnvFilter::new("error"), "error".to_string()),
    };

    let error_layer = tracing_error::ErrorLayer::default();

    let stderr_layer = fmt::layer()
        .with_writer(io::stderr)
        .with_ansi(use_ansi)
        .with_target(true)
        .with_thread_names(false)
        .with_level(true);

    let mut file_guard: Option<WorkerGuard> = None;
    let mut file_path: Option<PathBuf> = None;

    let init_result = if log_to_file {
        match build_file_appender() {
            Ok((appender, path)) => {
                file_path = Some(path);
                let (non_blocking, guard) = tracing_appender::non_blocking(appender);
                file_guard = Some(guard);
                // File: structured JSON, no ANSI, non-blocking writer (rules: production file).
                let file_layer = fmt::layer()
                    .json()
                    .with_writer(non_blocking)
                    .with_ansi(false)
                    .with_target(true)
                    .with_thread_names(false)
                    .with_current_span(true)
                    .with_span_list(false);
                // Dual sink: stderr (human/agent diagnostics) + rotated JSON file.
                tracing_subscriber::registry()
                    .with(filter)
                    .with(error_layer)
                    .with(stderr_layer)
                    .with(file_layer)
                    .try_init()
            }
            Err(_) => {
                // Fallback: stderr only if XDG state / open failed.
                file_guard = None;
                tracing_subscriber::registry()
                    .with(filter)
                    .with(error_layer)
                    .with(stderr_layer)
                    .try_init()
            }
        }
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(error_layer)
            .with(stderr_layer)
            .try_init()
    };

    match init_result {
        Ok(()) => {
            install_panic_tracing_bridge();
            tracing::info!(
                target: "browser_automation_cli::telemetry",
                filter = %effective,
                log_to_file,
                file = ?file_path.as_ref().map(|p| p.display().to_string()),
                ansi = use_ansi,
                "tracing initialized (local only; no remote export)"
            );
            TelemetryGuard {
                _file_worker: file_guard,
            }
        }
        Err(_) => {
            // Subscriber already set (integration tests calling run more than once, etc.).
            // Do not drop a new WorkerGuard onto a dead writer path: discard file worker.
            drop(file_guard);
            TelemetryGuard::none()
        }
    }
}

/// Build daily rolling appender with retention cap under XDG state.
fn build_file_appender() -> io::Result<(tracing_appender::rolling::RollingFileAppender, PathBuf)> {
    let state = crate::xdg::state_dir().map_err(|e| io::Error::other(e.to_string()))?;
    let log_dir = state.join("log");
    create_log_dir(&log_dir)?;
    let appender = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix(LOG_FILE_PREFIX)
        .max_log_files(MAX_LOG_FILES)
        .build(&log_dir)
        .map_err(|e| io::Error::other(e.to_string()))?;
    Ok((appender, log_dir))
}

/// Create log directory with restricted mode on Unix (owner-only).
fn create_log_dir(log_dir: &std::path::Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(log_dir)?;
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(log_dir)?;
    }
    Ok(())
}

/// After the subscriber is live: log panics as structured events, then chain prior hook.
fn install_panic_tracing_bridge() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown".to_string());
        let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Box<dyn Any>".to_string()
        };
        // Best-effort: may not reach disk if the panic is inside the writer thread.
        tracing::error!(
            target: "panic",
            %location,
            %message,
            "process panic"
        );
        previous(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_priority_quiet_wins() {
        assert_eq!(
            resolve_filter_directive(true, true, true, Some("trace")),
            "error"
        );
    }

    #[test]
    fn filter_debug_before_verbose() {
        assert_eq!(
            resolve_filter_directive(false, true, true, None),
            "debug"
        );
    }

    #[test]
    fn filter_verbose_info() {
        assert_eq!(
            resolve_filter_directive(false, true, false, None),
            "info"
        );
    }

    #[test]
    fn filter_xdg_when_no_flags() {
        assert_eq!(
            resolve_filter_directive(false, false, false, Some("warn")),
            "warn"
        );
    }

    #[test]
    fn filter_default_error() {
        assert_eq!(
            resolve_filter_directive(false, false, false, Some("  ")),
            "error"
        );
        assert_eq!(
            resolve_filter_directive(false, false, false, None),
            "error"
        );
    }

    #[test]
    fn max_log_files_is_positive_retention() {
        assert!(MAX_LOG_FILES >= 7);
        assert!(MAX_LOG_FILES <= 90);
    }
}
