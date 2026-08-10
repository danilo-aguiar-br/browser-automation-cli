// SPDX-License-Identifier: MIT OR Apache-2.0
//! Clap derive surface for browser-automation-cli (PRD Layer L).
//!
//! Help text on flags is the primary documentation for this module.
//! Item-level rustdoc is intentionally light: clap help strings power `--help`
//! and man pages; agent skills cover recipes (audit D-02/D-11).
//!
//! # Module map (Pass 31 SRP-02)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | global | GlobalOpts flattened into root CLI |
//! | commands | Top-level Commands enum |
//! | actions_browser | Page/cookie/console/net/dialog/assert/grab |
//! | actions_media | Perf/screencast/heap/monitor |
//! | actions_tools | Extension/devtools/webmcp/mitm/workflow/config/qr/image |
//! | actions_local_media | Local video pipeline (no Chrome) |

use clap::Parser;

mod actions_browser;
mod actions_local_media;
mod actions_media;
mod actions_tools;
pub mod agent_ops_args;
mod args_interact;
mod args_page;
mod args_scrape;
mod args_small;
mod args_tools;
mod commands;
mod global;

pub use actions_browser::*;
pub use actions_local_media::*;
pub use actions_media::*;
pub use actions_tools::*;
pub use commands::Commands;
pub use global::GlobalOpts;

/// One-shot browser automation CLI for AI agents.
#[derive(Debug, Parser)]
#[command(
    name = env!("CARGO_PKG_NAME"),
    version,
    author,
    about = "One-shot browser automation CLI (Chrome CDP). BORN, EXECUTE, FINALIZE, DIE.",
    long_about = None,
    propagate_version = true,
    after_help = "Examples:\n  \
browser-automation-cli doctor --json\n  \
browser-automation-cli goto https://example.com --json\n  \
browser-automation-cli schema run\n  \
browser-automation-cli run --script steps.ndjson --json-steps\n  \
browser-automation-cli config path\n\n\
Exit codes follow sysexits-style mapping (2 usage, 69 unavailable, 70 software, 124 timeout).\n\
Config is XDG-only (config set); product settings do not read process environment variables."
)]
pub struct Cli {
    /// Global flags shared by all subcommands
    #[command(flatten)]
    pub globals: GlobalOpts,

    /// Subcommand to execute (one-shot)
    #[command(subcommand)]
    pub command: Commands,
}

/// Stack bytes reserved for building the clap command tree off the main thread.
///
/// `Cli::command()` is one enormous generated function: every one of the
/// top-level subcommands and their nested action enums is constructed inline,
/// so the frame is a few MiB in the unoptimized `test` profile (measured need:
/// just over 2 MiB, and it grows with each new flag). The process main thread
/// has 8 MiB, so production argv parsing is unaffected, but libtest runs each
/// test on a worker thread with a 2 MiB stack, which aborts the whole test
/// binary with `has overflowed its stack`.
///
/// 16 MiB leaves ~8x headroom over the current measurement so the gate does not
/// have to be retuned whenever a subcommand is added.
pub const CLAP_TREE_STACK_BYTES: usize = 16 * 1024 * 1024;

/// Run `f` on a worker thread sized by [`CLAP_TREE_STACK_BYTES`].
///
/// Use this in tests (and any other non-main thread) that touch
/// `Cli::command()`, `Cli::try_parse_from`, or anything that rebuilds the clap
/// tree, so `cargo test` passes with no `RUST_MIN_STACK` in the environment.
///
/// Panics raised by `f` are re-raised on the calling thread, so `assert!`
/// inside `f` still fails the test normally.
///
/// ```no_run
/// use clap::CommandFactory;
/// use browser_automation_cli::cli::{on_clap_stack, Cli};
///
/// let names: Vec<String> = on_clap_stack(|| {
///     Cli::command()
///         .get_subcommands()
///         .map(|s| s.get_name().to_string())
///         .collect()
/// });
/// assert!(!names.is_empty());
/// ```
pub fn on_clap_stack<T, F>(f: F) -> T
where
    F: FnOnce() -> T + Send,
    T: Send,
{
    std::thread::scope(|scope| {
        let handle = std::thread::Builder::new()
            .stack_size(CLAP_TREE_STACK_BYTES)
            .spawn_scoped(scope, f)
            .expect("spawn clap-tree thread");
        match handle.join() {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    })
}
