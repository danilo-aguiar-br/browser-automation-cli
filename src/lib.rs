//! # browser-automation-cli
//!
//! One-shot Chrome CDP automation library and CLI for AI agents.
//!
//! Lifecycle is always **BORN → EXECUTE → FINALIZE → DIE** in a single process.
//! There is no daemon, no npm runtime, and no remote telemetry.
//!
//! ## Overview
//!
//! - Parse argv with clap ([`cli`])
//! - Dispatch one command or a multi-step `run --script` session
//! - Launch system Chrome/Chromium through chromiumoxide CDP
//! - Always attempt FINALIZE (Browser.close, wait, kill fallback)
//!
//! ## Quick Start
//!
//! ```bash
//! cargo install --path . --locked
//! browser-automation-cli doctor --offline --quick --json
//! browser-automation-cli goto https://example.com --json
//! ```
//!
//! Library entry for embedding or tests:
//!
//! ```no_run
//! use std::process::ExitCode;
//!
//! fn main() -> ExitCode {
//!     browser_automation_cli::run()
//! }
//! ```
//!
//! ## Features
//!
//! This crate has no Cargo feature flags.
//! Rustdoc embeds Mermaid lifecycle diagrams via `aquamarine` on documented entry points.
//! Optional CLI categories are process flags:
//!
//! - `--category-memory` — deep heap tools
//! - `--category-extensions` — extension tools
//! - `--category-third-party` — third-party DevTools helpers
//! - `--category-webmcp` — webmcp tools
//! - `--experimental-vision` — coordinate click
//! - `--experimental-screencast` — screencast export (needs ffmpeg)
//!
//! ## Targets
//!
//! Documented and tested for:
//!
//! - `x86_64-unknown-linux-gnu`
//! - `x86_64-apple-darwin`
//! - `aarch64-apple-darwin`
//! - `x86_64-pc-windows-msvc`
//! - `aarch64-unknown-linux-musl`
//!
//! Chrome automation is not supported on `wasm32-unknown-unknown`.
//!
//! ## MSRV
//!
//! Minimum Supported Rust Version is **1.88.0** (`rust-version` in `Cargo.toml`).
//!
//! ## Safety
//!
//! - No remote telemetry is emitted by this crate
//! - Local tracing stays on stderr only
//! - `docs.rs` builds pass `--cfg docsrs` and may enable `doc_cfg` on nightly
//! - Unix paths may call `libc` for signal defaults and last-resort process kill
//!
//! ## Error handling
//!
//! Public errors use [`error::CliError`] with sysexits-style exit codes.
//! JSON agents should parse the envelope from [`envelope`].
//!
//! ## Examples
//!
//! ```no_run
//! use browser_automation_cli::error::{CliError, ErrorKind};
//! use browser_automation_cli::exit_code_for;
//!
//! let err = CliError::new(ErrorKind::Unavailable, "chrome not found");
//! assert_eq!(exit_code_for(&err), 69);
//! ```
//!
//! ## See also
//!
//! - Crate README and `docs/HOW_TO_USE.md`
//! - `docs/schemas/` for JSON contracts
//! - `skill/browser-automation-cli-en/SKILL.md` for agent skill surface

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![warn(rustdoc::private_intra_doc_links)]
#![deny(rustdoc::invalid_html_tags)]
#![deny(rustdoc::invalid_rust_codeblocks)]
#![deny(rustdoc::bare_urls)]
#![warn(rustdoc::redundant_explicit_links)]

/// Chrome one-shot session: launch, actions, reap.
pub mod browser;
/// HTTP/parse cache under XDG (one-shot L1 + SQLite L2).
pub mod cache;
/// Clap derive surface and global flags.
pub mod cli;
/// ANSI color helpers for human stderr diagnostics.
pub mod color;
/// PRD command dispatch (meta paths and browser one-shot).
pub mod commands_prd;
/// Shared constants (schema version, product name).
pub mod constants;
/// Local install diagnostics (`doctor`).
pub mod doctor;
/// JSON success/error envelopes for agents.
pub mod envelope;
/// Typed CLI errors and exit codes.
pub mod error;
/// One-shot filesystem path discovery (`find-paths`).
pub mod find_paths;
/// Locale messaging helpers.
pub mod i18n;
/// Install path helpers for doctor and packaging checks.
pub mod install;
/// Cooperative cancel and FINALIZE ledger.
pub mod lifecycle;
/// Optional one-shot LLM HTTP extract (XDG key only).
pub mod llm_local;
/// Local MITM capture, CA, and HAR export (one-shot).
pub mod mitm_local;
/// Native CDP stack (browser, network, snapshot, heap).
pub mod native;
/// One-shot QR encode/decode (no Chrome).
pub mod qr_local;
/// Owned residual path discovery (CLI marker + chromium tmp).
pub mod residual;
/// Named retry policies with backoff and jitter.
pub mod retry;
/// robots.txt policy enforcement.
pub mod robots;
/// Local scrape/crawl/map/search/parse (HTTP + files; one-shot).
pub mod scrape_local;
/// Structural lint scan/rewrite one-shot (§5AC).
pub mod sg_local;
/// XLSX write-only path via rust_xlsxwriter (§5Z).
pub mod sheet_local;
/// Input and path validation helpers.
pub mod validation;
/// Windows Job Object helpers (stubs on non-Windows).
pub mod win_job;
/// Workflow journal DAG (petgraph + SQLite), one-shot run/resume.
pub mod workflow_local;
/// XDG Base Directory paths and config file (no `.env` at runtime).
pub mod xdg;

#[cfg(test)]
#[allow(missing_docs)]
pub mod test_utils;

use std::process::ExitCode;

use clap::{CommandFactory, Parser};

use crate::cli::Cli;
use crate::error::CliError;
use crate::lifecycle::Lifecycle;

/// Parse argv and run the one-shot CLI.
///
/// Always attempts FINALIZE before returning, including on clap help/version paths.
///
/// # Lifecycle
///
/// ```mermaid
/// flowchart LR
///   BORN → EXECUTE → FINALIZE → DIE
/// ```
///
/// # Returns
///
/// Process [`ExitCode`] mapped from sysexits-style CLI codes.
#[cfg_attr(doc, aquamarine::aquamarine)]
pub fn run() -> ExitCode {
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    #[cfg(windows)]
    {
        std::env::set_var("MSYS_NO_PATHCONV", "1");
        std::env::set_var("MSYS2_ARG_CONV_EXCL", "*");
    }

    let life = Lifecycle::new();

    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e) => {
            // clap: DisplayHelp/DisplayVersion → exit 0; usage errors → 2
            let code = e.exit_code();
            let _ = e.print();
            life.finalize();
            return ExitCode::from(code as u8);
        }
    };

    // Resolve language once (CLI flag > XDG config > OS locale). Used for human suggestions.
    let lang = crate::i18n::resolve_lang(cli.globals.lang.as_deref());
    crate::i18n::set_effective_lang(lang);

    init_tracing(
        cli.globals.quiet,
        cli.globals.verbose,
        cli.globals.debug,
        cli.globals.lang.as_deref(),
    );

    let code = commands_prd::dispatch(cli, &life);
    // finalize is called inside dispatch; call again is idempotent
    life.finalize();
    if code <= 0 {
        ExitCode::SUCCESS
    } else if code >= 256 {
        ExitCode::from(255)
    } else {
        ExitCode::from(code as u8)
    }
}

/// Run clap `debug_assert` on the command tree (tests and diagnostics).
pub fn command_factory_debug_assert() {
    Cli::command().debug_assert();
}

/// Map a [`CliError`] to its process exit code without parsing argv.
///
/// Useful for unit tests and library callers that already hold a typed error.
pub fn exit_code_for(err: &CliError) -> u8 {
    err.exit_code()
}

/// Local-only tracing on stderr (zero remote telemetry).
///
/// Level order: `--quiet` / `--debug` / `--verbose` / XDG `log_level` / default `error`.
/// Product settings never read `RUST_LOG` (XDG + argv only).
fn init_tracing(quiet: bool, verbose: bool, debug: bool, _cli_lang: Option<&str>) {
    use tracing_subscriber::EnvFilter;

    let xdg_level = crate::xdg::load_config()
        .ok()
        .and_then(|c| c.log_level)
        .filter(|s| !s.trim().is_empty());

    let filter = if quiet {
        EnvFilter::new("error")
    } else if debug {
        EnvFilter::new("debug")
    } else if verbose {
        EnvFilter::new("info")
    } else if let Some(level) = xdg_level {
        EnvFilter::new(level)
    } else {
        EnvFilter::new("error")
    };

    let log_to_file = crate::xdg::load_config()
        .ok()
        .and_then(|c| c.log_to_file)
        .unwrap_or(false);

    if log_to_file {
        // Optional rotated local file under XDG state (GAP-012). Never remote telemetry.
        if let Ok(state) = crate::xdg::state_dir() {
            let log_dir = state.join("log");
            let _ = std::fs::create_dir_all(&log_dir);
            let file_appender =
                tracing_appender::rolling::daily(&log_dir, "browser-automation-cli");
            let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
            // Leak guard for process lifetime (one-shot DIE frees it).
            std::mem::forget(_guard);
            let _ = tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_writer(non_blocking)
                .with_target(false)
                .try_init();
            return;
        }
    }

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .try_init();
}
