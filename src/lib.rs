//! browser-automation-cli library: one-shot browser automation for agents.
//!
//! Default entry: clap parse → dispatch → FINALIZE (no daemon).

pub mod browser;
pub mod cli;
pub mod color;
pub mod commands_prd;
pub mod constants;
pub mod doctor;
pub mod envelope;
pub mod error;
pub mod i18n;
pub mod install;
pub mod lifecycle;
pub mod native;
pub mod robots;
pub mod validation;

#[cfg(test)]
pub mod test_utils;

use std::process::ExitCode;

use clap::{CommandFactory, Parser};

use crate::cli::Cli;
use crate::error::CliError;
use crate::lifecycle::Lifecycle;

/// Parse argv and run the one-shot CLI. Always attempts FINALIZE.
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

/// Expose command factory for tests and `debug_assert`.
pub fn command_factory_debug_assert() {
    Cli::command().debug_assert();
}

/// Map a raw CliError without running clap (unit tests / library use).
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
