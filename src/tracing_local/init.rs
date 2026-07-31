// SPDX-License-Identifier: MIT OR Apache-2.0
//! Subscriber construction, file appender and panic bridge.
//!
//! Types, validators and constants stay in the parent module and arrive
//! through `use super::*`, so the import block is not duplicated here.

use super::*;

/// Install the global tracing subscriber (once) and return a process-lifetime guard.
///
/// Safe to call when a subscriber is already installed (tests / re-entry): returns
/// [`TracingLocalGuard::none`] without replacing the existing subscriber.
///
/// # Panic hook
///
/// After a successful install, chains a hook that emits a `tracing` `error` event
/// (target `panic`) then calls the previous hook (e.g. `human_panic` from `main`).
#[must_use]
pub fn init_tracing_local(opts: TracingLocalOpts) -> TracingLocalGuard {
    // Single XDG load (memory: one TOML parse per process boot).
    let cfg = crate::xdg::load_config().ok();
    let xdg_level = cfg
        .as_ref()
        .and_then(|c| c.log_level.as_deref())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let log_to_file = cfg.as_ref().and_then(|c| c.log_to_file).unwrap_or(false);
    let max_log_files = clamp_max_log_files(cfg.as_ref().and_then(|c| c.max_log_files));
    let rotation = parse_log_rotation(cfg.as_ref().and_then(|c| c.log_rotation.as_deref()));
    let rotation_label = cfg
        .as_ref()
        .and_then(|c| c.log_rotation.as_deref())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_LOG_ROTATION);

    let requested =
        resolve_filter_directive(opts.quiet, opts.verbose, opts.debug, xdg_level.as_deref());
    let use_ansi = !opts.plain && crate::color::is_enabled() && io::stderr().is_terminal();

    // Invalid XDG directive must not abort the CLI; fall back to safe default.
    let (filter, effective, filter_fallback) = match EnvFilter::try_new(&requested) {
        Ok(f) => (f, requested.clone(), false),
        Err(_) => (
            EnvFilter::new(DEFAULT_LOG_LEVEL),
            DEFAULT_LOG_LEVEL.to_string(),
            true,
        ),
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
        match build_file_appender(rotation, max_log_files) {
            Ok((appender, path)) => {
                file_path = Some(path);
                // Parallelism: dedicated non-blocking writer thread (only when file on).
                let (non_blocking, guard) = tracing_appender::non_blocking(appender);
                file_guard = Some(guard);
                let file_layer = fmt::layer()
                    .json()
                    .with_writer(non_blocking)
                    .with_ansi(false)
                    .with_target(true)
                    .with_thread_names(false)
                    .with_current_span(true)
                    .with_span_list(false);
                tracing_subscriber::registry()
                    .with(filter)
                    .with(error_layer)
                    .with(stderr_layer)
                    .with(file_layer)
                    .try_init()
            }
            Err(_) => {
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
            let correlation = crate::agent_context::correlation_id();
            tracing::info!(
                target: "browser_automation_cli::tracing_local",
                requested_filter = %requested,
                effective_filter = %effective,
                filter_fallback,
                log_to_file,
                max_log_files,
                log_rotation = %rotation_label,
                file = ?file_path.as_ref().map(|p| p.display().to_string()),
                ansi = use_ansi,
                correlation_id = correlation.as_deref(),
                "tracing initialized (local only; no remote export)"
            );
            TracingLocalGuard {
                _file_worker: file_guard,
            }
        }
        Err(_) => {
            // Subscriber already set (integration tests calling run more than once).
            drop(file_guard);
            TracingLocalGuard::none()
        }
    }
}

/// Build rolling appender with retention cap under XDG state log dir.
fn build_file_appender(
    rotation: Rotation,
    max_log_files: usize,
) -> io::Result<(RollingFileAppender, PathBuf)> {
    let log_dir = crate::xdg::log_dir().map_err(|e| io::Error::other(e.to_string()))?;
    create_log_dir(&log_dir)?;
    let appender = RollingFileAppender::builder()
        .rotation(rotation)
        .filename_prefix(LOG_FILE_PREFIX)
        .max_log_files(max_log_files)
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
            .mode(LOG_DIR_MODE)
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
        tracing::error!(
            target: "panic",
            %location,
            %message,
            "process panic"
        );
        previous(info);
    }));
}
