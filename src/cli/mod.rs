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
//! | actions_tools | Extension/devtools/webmcp/mitm/workflow/config/qr |

use clap::Parser;

mod actions_browser;
mod actions_media;
mod actions_tools;
mod commands;
mod global;

pub use actions_browser::*;
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
