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
/// Locale messaging helpers.
pub mod i18n;
/// Install path helpers for doctor and packaging checks.
pub mod install;
/// Local Firecrawl-parity scrape/crawl/map/search/parse (HTTP + files).
pub mod scrape_local;
/// Local MITM capture, CA, and HAR export (one-shot).
pub mod mitm_local;
/// Workflow journal DAG (petgraph + SQLite), one-shot run/resume.
pub mod workflow_local;
/// XDG Base Directory paths and config file (no `.env` at runtime).
pub mod xdg;
/// Cooperative cancel and FINALIZE ledger.
pub mod lifecycle;
/// Native CDP stack (browser, network, snapshot, heap).
pub mod native;
/// robots.txt policy enforcement.
pub mod robots;
/// Input and path validation helpers.
pub mod validation;

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

    init_tracing(cli.globals.quiet, cli.globals.verbose, cli.globals.debug);

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
fn init_tracing(quiet: bool, verbose: bool, debug: bool) {
    use tracing_subscriber::EnvFilter;

    let filter = if quiet {
        EnvFilter::new("error")
    } else if let Ok(from_env) = std::env::var("RUST_LOG") {
        EnvFilter::new(from_env)
    } else if debug {
        EnvFilter::new("debug")
    } else if verbose {
        EnvFilter::new("info")
    } else {
        EnvFilter::new("error")
    };

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .try_init();
}
