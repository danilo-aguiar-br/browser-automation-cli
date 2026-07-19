// SPDX-License-Identifier: MIT OR Apache-2.0
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
//! Cargo features control **optional locale packs** only (MVP always includes `en` + `pt-BR`):
//!
//! | Feature | Purpose |
//! |---------|---------|
//! | `i18n-cjk` | Scaffold for zh-Hans / zh-Hant / ja / ko packs |
//! | `i18n-rtl` | Scaffold for ar / he (RTL) packs |
//! | `i18n-europe` | Scaffold for additional European packs |
//! | `i18n-full` | Enables all optional i18n scaffolds |
//! | `i18n-pseudo` | Pseudolocalization (dev only) |
//!
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
//! ## Graceful shutdown (one-shot)
//!
//! Detect → signal → await, scoped for a **CLI one-shot** (not a long-lived server):
//!
//! | Phase | Mechanism |
//! |-------|-----------|
//! | Detect | [`browser::shutdown_signal`] — SIGINT/SIGTERM (Unix), Ctrl-C/Break (Windows) |
//! | Signal | [`lifecycle::Lifecycle`] `CancellationToken` → exit **130** |
//! | Await | `OneShotSession::shutdown` (Browser.close + wait ≤5s + kill); residual SIGTERM→grace→SIGKILL |
//! | Pipeline | SIGPIPE default + BrokenPipe → exit **141**; dual flush before DIE |
//! | Force | Second OS signal runs residual [`lifecycle::Lifecycle::finalize`] |
//!
//! Daemon-only rules (TaskTracker fleets, SIGHUP reload, readiness probes, `sd_notify`)
//! are **N/A** by product law.
//!
//! ## Safety
//!
//! - No remote telemetry is emitted by this crate (no OTEL/OTLP/Sentry)
//! - Local tracing: stderr by default; optional rotated JSON under XDG state
//!   (`log_to_file`); see [`telemetry`]
//! - Unix paths may call `libc` for signal defaults and last-resort process kill
//! - Windows paths may use Job Objects (`win_job`) for residual-zero process trees
//!
//! ### docs.rs / rustdoc feature gates (nightly)
//!
//! - `docs.rs` builds this crate with `--cfg docsrs` (see `[package.metadata.docs.rs]`)
//! - Under `docsrs`, the crate root enables `#![feature(doc_cfg)]` so
//!   `#[doc(cfg(...))]` and `#[cfg_attr(docsrs, doc(cfg(...)))]` render platform
//!   and feature badges on multi-target docs
//! - **`doc_auto_cfg` is not used**: as of the October 2025 rustdoc consolidation,
//!   automatic cfg labels live under `doc_cfg` only; enabling the removed
//!   `doc_auto_cfg` feature gate risks nightly docs.rs failures
//! - Stable `cargo doc` does not enable `docsrs`; platform items still compile via
//!   normal `#[cfg(unix)]` / `#[cfg(windows)]` without the experimental feature
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
//! - Local validation: `scripts/docs-check.sh` (HTML + optional rustdoc JSON; no CI/GHA)

// docs.rs / nightly: `doc_cfg` only (rules_rust_docsrs — no `doc_auto_cfg`).
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![warn(rustdoc::private_intra_doc_links)]
#![deny(rustdoc::invalid_html_tags)]
#![deny(rustdoc::invalid_rust_codeblocks)]
#![deny(rustdoc::bare_urls)]
#![warn(rustdoc::redundant_explicit_links)]
// Document every `unsafe` block (rules: English + crates.io safety docs).
#![warn(clippy::undocumented_unsafe_blocks)]
#![warn(clippy::multiple_unsafe_ops_per_block)]
#![deny(unsafe_op_in_unsafe_fn)]
// Const / static hygiene (rules_rust_const_static_inicializacao).
#![deny(static_mut_refs)]
#![warn(clippy::declare_interior_mutable_const)]
#![warn(clippy::borrow_interior_mutable_const)]
// Ownership / borrowing hygiene (rules_rust_ownership_borrowing_lifetimes).
#![warn(clippy::redundant_clone)]
#![warn(clippy::needless_pass_by_value)]
#![warn(clippy::ptr_arg)]
#![warn(clippy::implicit_clone)]
#![warn(clippy::unnecessary_to_owned)]
#![warn(clippy::cloned_instead_of_copied)]
#![warn(clippy::map_clone)]
#![warn(clippy::mut_mut)]
#![warn(clippy::needless_lifetimes)]

/// Chrome one-shot session: launch, actions, reap.
pub mod browser;
/// HTTP/parse cache under XDG (one-shot L1 + SQLite L2).
pub mod cache;
/// Clap derive surface and global flags.
pub mod cli;
/// Injectable wall clock for deterministic tests.
pub mod clock;
/// ANSI color helpers for human stderr diagnostics.
pub mod color;
/// PRD command dispatch (meta paths and browser one-shot).
pub mod commands_prd;
/// Config surface (re-export of XDG; layout name for clap rules).
pub mod config;
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
/// Shared JSON / NDJSON helpers (BOM strip, size ceilings, compact encode).
pub mod json_util;
/// Cooperative cancel and FINALIZE ledger.
pub mod lifecycle;
/// Optional one-shot LLM HTTP extract (XDG key only).
pub mod llm_local;
/// Local MITM capture, CA, and HAR export (one-shot).
pub mod mitm_local;
/// Native CDP stack (browser, network, snapshot, heap).
pub mod native;
/// Canonical stdout/stderr writers (BrokenPipe → 141, explicit flush).
pub mod output;
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
/// Cross-platform host helpers (PATH, console UTF-8/VT, sandbox, WSL/container).
pub mod platform;
/// Input and path validation helpers.
pub mod validation;
/// Windows Job Object helpers (stubs on non-Windows).
pub mod win_job;
/// Workflow journal DAG (petgraph + SQLite), one-shot run/resume.
pub mod workflow_local;
/// Bounded parallelism budget (`--max-concurrency`, Semaphore, Rayon, join_bounded).
pub mod concurrency;
/// Tokio runtime builders (budgeted multi-thread workers for CDP + HTTP fan-out).
pub mod runtime_util;
/// Local tracing init (stderr + optional XDG rotated JSON; no remote export).
pub mod telemetry;
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

/// Parse process argv and run the one-shot CLI.
///
/// Thin wrapper over [`run_from_args`] with `std::env::args_os()`.
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
    run_from_args(std::env::args_os())
}

/// Parse the given argv (including program name as first element) and run.
///
/// Enables tests and embedders to inject argv without mutating process-global
/// state. Stdin/stdout/stderr remain the process streams (Unix pipes / agent
/// contract); full stream injection is reserved for unit tests of [`output`].
///
/// # Returns
///
/// Process [`ExitCode`] mapped from sysexits-style CLI codes.
pub fn run_from_args<I, T>(args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    // Phase 1 multiplatform: Windows console UTF-8 + VT before any user-facing I/O.
    crate::platform::configure_console();

    // SAFETY:
    // - Contract: restore default SIGPIPE so BrokenPipe becomes EPIPE (exit 141 path).
    // - Invariant: `signal` is async-signal-safe and only replaces the disposition.
    // - Caller/callee: process owns its signal table; no other handler is required at BORN.
    // - See: `man 2 signal`, POSIX SIG_DFL; product maps EPIPE via `output::map_io_error`.
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
    let args: Vec<std::ffi::OsString> = args.into_iter().map(Into::into).collect();
    let wants_json = args.iter().any(|a| a == "--json");

    let cli = match Cli::try_parse_from(&args) {
        Ok(c) => c,
        Err(e) => {
            // clap: DisplayHelp/DisplayVersion → exit 0; usage errors → 2
            // GAP-002: when `--json` is on argv, emit agent envelope on stdout (not human clap only).
            let code = e.exit_code();
            let is_help_or_version = matches!(
                e.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            );
            if wants_json && !is_help_or_version && code != 0 {
                let msg = e.to_string();
                let err = crate::error::CliError::with_suggestion(
                    crate::error::ErrorKind::Usage,
                    msg.lines().next().unwrap_or("invalid arguments").to_string(),
                    "Check --help or schema --cmd <name>; pass valid argv",
                );
                let _ = crate::envelope::print_error_json(&err);
            } else {
                let _ = e.print();
            }
            let _ = crate::output::flush_stdout();
            let _ = crate::output::flush_stderr();
            life.finalize();
            return ExitCode::from(code as u8);
        }
    };

    // Accessibility / agent plain stderr (also honors NO_COLOR / CLICOLOR / TERM=dumb).
    // Phase 2 of i18n boot: TTY/plain before any colored human text.
    crate::color::set_plain(cli.globals.plain);

    // Resolve UI locale once (flag > BROWSER_AUTOMATION_CLI_LANG > XDG > sys-locale > en).
    // Human suggestions only; machine JSON `error.message` stays English.
    let resolved = crate::i18n::resolve_locale(cli.globals.lang.as_deref());
    crate::i18n::set_effective_idioma(resolved);

    // Process-wide concurrency budget (rules_rust_paralelismo): every fan-out
    // reads `concurrency::effective_limit()`. `0` = auto (CPU × free RAM).
    crate::concurrency::install_limit(cli.globals.max_concurrency);
    crate::concurrency::install_rayon_pool_once();

    // Install subscriber once; hold WorkerGuard (file path) until FINALIZE/DIE so
    // non_blocking flushes (rules_rust_logs: never mem::forget the guard).
    let _telemetry = crate::telemetry::init_telemetry(crate::telemetry::TelemetryOpts {
        quiet: cli.globals.quiet,
        verbose: cli.globals.verbose,
        debug: cli.globals.debug,
        plain: cli.globals.plain,
    });

    let code = commands_prd::dispatch(cli, &life);
    // FINALIZE: flush both streams (rules: flush stdout+stderr before exit).
    let _ = crate::output::flush_stdout();
    let _ = crate::output::flush_stderr();
    // finalize is called inside dispatch; call again is idempotent
    life.finalize();
    // Drop `_telemetry` after flush so file WorkerGuard drains last lines.
    drop(_telemetry);
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

/// Build identity for `version` and packaging diagnostics.
///
/// `git_sha` / `build_timestamp` come from `build.rs` (`cargo:rustc-env`).
pub fn build_identity() -> serde_json::Value {
    serde_json::json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "git_sha": option_env!("GIT_SHA").unwrap_or("unknown"),
        "build_timestamp": option_env!("BUILD_TIMESTAMP").unwrap_or("unknown"),
    })
}


