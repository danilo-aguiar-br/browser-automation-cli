// SPDX-License-Identifier: MIT OR Apache-2.0
//! Process entry surface: argv parsing, BORN/EXECUTE/FINALIZE, exit mapping.
//!
//! Split out of `lib.rs` so the crate root stays a module index plus the
//! crate manual. Every item is re-exported from the crate root, so paths like
//! `browser_automation_cli::run` are unchanged.

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
///
/// # Examples
///
/// ```no_run
/// use std::process::ExitCode;
///
/// fn main() -> ExitCode {
///     browser_automation_cli::run()
/// }
/// ```
///
/// # See also
///
/// - [`run_from_args`] for injectable argv (tests / embedders)
/// - [`scripts/docs-check.sh`](https://github.com/danilo-aguiar-br/browser-automation-cli/blob/main/scripts/docs-check.sh)
///   for local HTML + optional rustdoc JSON validation
#[cfg_attr(all(doc, feature = "docs-mermaid"), aquamarine::aquamarine)]
pub fn run() -> ExitCode {
    run_from_args(std::env::args_os())
}

/// Parse the given argv (including program name as first element) and run.
///
/// Enables tests and embedders to inject argv without mutating process-global
/// state. Stdin/stdout/stderr remain the process streams (Unix pipes / agent
/// contract); full stream injection is reserved for unit tests of
/// [`crate::output`].
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
            // Pass K: resolve UI locale before human suggestion (flag scan → XDG → sys-locale).
            let early_lang = crate::i18n::scan_lang_flag_from_argv(&args);
            let early = crate::i18n::resolve_locale(early_lang.as_deref());
            crate::i18n::set_effective_ui_locale(early);
            let code = e.exit_code();
            let is_help_or_version = matches!(
                e.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            );
            if wants_json && !is_help_or_version && code != 0 {
                let msg = e.to_string();
                let err = crate::error::CliError::with_suggestion(
                    crate::error::ErrorKind::Usage,
                    msg.lines()
                        .next()
                        .unwrap_or("invalid arguments")
                        .to_string(),
                    crate::i18n::UiMessage::UsageSuggestion
                        .text(crate::i18n::effective_ui_locale()),
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

    // Resolve UI locale once (flag > XDG lang > sys-locale > en; no product env).
    // Human suggestions only; machine JSON `error.message` stays English.
    let resolved = crate::i18n::resolve_locale(cli.globals.lang.as_deref());
    crate::i18n::set_effective_ui_locale(resolved);

    // Agent correlation: optional global flag → process-local context (envelopes / NDJSON).
    crate::agent_context::set_correlation_id(cli.globals.correlation_id.clone());

    // Universal data operations (agent CLEAN STDOUT). Parsed and validated HERE,
    // before the command runs, so a malformed `--filter` costs an argv error and
    // not a completed browser session whose output is then rejected.
    match cli.globals.agent_ops.to_ops() {
        Ok(ops) => crate::agent_ops::set_agent_ops(Some(ops)),
        Err(err) => {
            let code = err.exit_code();
            let _ = crate::envelope::print_error_json(&err);
            let _ = crate::output::flush_stdout();
            return ExitCode::from(code);
        }
    }

    // Process-wide concurrency budget (rules_rust_paralelismo): every fan-out
    // reads `concurrency::effective_limit()`. `0` = auto (CPU × free RAM).
    crate::concurrency::install_limit(cli.globals.max_concurrency);
    crate::concurrency::install_rayon_pool_once();

    // Install subscriber once; hold WorkerGuard (file path) until FINALIZE/DIE so
    // non_blocking flushes (rules_rust_logs: never mem::forget the guard).
    let _tracing_local =
        crate::tracing_local::init_tracing_local(crate::tracing_local::TracingLocalOpts {
            quiet: cli.globals.quiet,
            verbose: cli.globals.verbose,
            debug: cli.globals.debug,
            plain: cli.globals.plain,
        });

    // Root span carries agent correlation into nested events (local only; no OTEL).
    let correlation = crate::agent_context::correlation_id();
    let run_span = tracing::info_span!(
        "cli_run",
        correlation_id = correlation.as_deref().unwrap_or("")
    );
    let code = {
        let _run_enter = run_span.enter();
        crate::commands::dispatch(cli, &life)
    };
    // FINALIZE: flush both streams (rules: flush stdout+stderr before exit).
    let _ = crate::output::flush_stdout();
    let _ = crate::output::flush_stderr();
    // finalize is called inside dispatch; call again is idempotent
    life.finalize();
    // Drop `_tracing_local` after flush so file WorkerGuard drains last lines.
    drop(_tracing_local);
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
/// `git_sha` / `source_hash` / `build_timestamp` come from `build.rs`
/// (`cargo:rustc-env`).
///
/// `git_sha` names the last commit; `source_hash` fingerprints the bytes that
/// were actually compiled. They diverge whenever the worktree is modified, and
/// that divergence is the point: an agent reproducing a run checks out
/// `git_sha`, rebuilds, and compares `source_hash` to know whether it holds the
/// same code. There is deliberately no `dirty` flag — see `emit_source_hash` in
/// `build.rs` for why no cheap heuristic can honestly claim a clean tree.
pub fn build_identity() -> serde_json::Value {
    serde_json::json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "git_sha": option_env!("GIT_SHA").unwrap_or("unknown"),
        "source_hash": option_env!("SOURCE_HASH").unwrap_or("unknown"),
        "build_timestamp": option_env!("BUILD_TIMESTAMP").unwrap_or("unknown"),
    })
}
