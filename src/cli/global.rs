// SPDX-License-Identifier: MIT OR Apache-2.0
//! Global CLI flags flattened into every subcommand.

use clap::{ArgAction, Args, ValueHint};

/// Global options applied to every subcommand.
///
/// Flattened into the root [`Cli`](crate::cli::Cli) via `#[command(flatten)]`.
#[derive(Debug, Clone, Args)]
pub struct GlobalOpts {
    /// Emit machine-readable JSON success/error envelopes on stdout
    #[arg(long, global = true, action = ArgAction::SetTrue, help_heading = "Output")]
    pub json: bool,

    /// GAP-020: stream one NDJSON object per `run` step on stdout (step,cmd,ok,result)
    #[arg(
        long = "json-steps",
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Output"
    )]
    pub json_steps: bool,

    /// Suppress non-error human logs on stderr
    #[arg(
        short = 'q',
        long = "quiet",
        global = true,
        action = ArgAction::SetTrue,
        conflicts_with_all = ["verbose", "debug"],
        help_heading = "Output"
    )]
    pub quiet: bool,

    /// Increase stderr verbosity (`-v` / `--verbose` = info; or `config set log_level debug`)
    #[arg(
        short = 'v',
        long = "verbose",
        global = true,
        action = ArgAction::SetTrue,
        conflicts_with = "quiet",
        help_heading = "Output"
    )]
    pub verbose: bool,

    /// Maximum tracing detail on stderr (debug/trace)
    #[arg(
        long = "debug",
        global = true,
        action = ArgAction::SetTrue,
        conflicts_with = "quiet",
        help_heading = "Output"
    )]
    pub debug: bool,

    /// Force plain stderr (no ANSI colors; accessibility / agent-friendly)
    #[arg(
        long = "plain",
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Output"
    )]
    pub plain: bool,

    /// Global wall-clock timeout in seconds (0 = no override)
    #[arg(
        long,
        global = true,
        default_value_t = 0,
        value_name = "SECS",
        help_heading = "Timeouts"
    )]
    pub timeout: u64,

    /// Max concurrent I/O tasks (batch/crawl/CDP fan-out) and Rayon CPU pool hint
    ///
    /// `0` = auto: `min(cpus, (free_ram_mb×50%)/64, 64)`. Every fan-out is
    /// hard-capped (no unbounded `join_all` / spawn loops).
    #[arg(
        long = "max-concurrency",
        global = true,
        default_value_t = 0,
        value_name = "N",
        help_heading = "Parallelism"
    )]
    pub max_concurrency: usize,

    /// Per-step timeout in seconds for `run` scripts (0 = inherit global timeout)
    #[arg(
        long,
        global = true,
        default_value_t = 0,
        value_name = "SECS",
        help_heading = "Timeouts"
    )]
    pub step_timeout: u64,

    /// Launch Chrome with a visible window (debug; default headless=new)
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Browser"
    )]
    pub headed: bool,

    /// Directory for screenshots, PDFs, and other one-shot artifacts
    #[arg(
        long,
        global = true,
        value_name = "DIR",
        value_hint = ValueHint::DirPath,
        help_heading = "Browser"
    )]
    pub artifacts_dir: Option<std::path::PathBuf>,

    /// Force UI language (`en` / `pt-BR`); default: flag → XDG `lang` → OS → en
    ///
    /// Accepts BCP 47 (`pt-BR`, `en`). Bare `pt` is rejected (use `pt-BR`). Persist with
    /// `config set lang <token>`. No product environment variables. Machine JSON stays English.
    #[arg(long, global = true, value_name = "LANG", help_heading = "Output")]
    pub lang: Option<String>,

    /// Correlation id echoed on JSON envelopes and NDJSON steps (agent join key)
    ///
    /// Optional. Not a secret. Persist workflow ids in the caller; this CLI is one-shot.
    #[arg(
        long = "correlation-id",
        global = true,
        value_name = "ID",
        help_heading = "Output"
    )]
    pub correlation_id: Option<String>,

    /// Capture console messages during browser commands
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Browser"
    )]
    pub capture_console: bool,

    /// Capture network requests during browser commands
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Browser"
    )]
    pub capture_network: bool,

    /// On failure, write captured console/network evidence to the artifacts dir (GAP-039)
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Browser"
    )]
    pub dump_on_failure: bool,

    /// Permit local reads and artifact writes outside the allowed roots (GAP-026 risk acceptance)
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Security"
    )]
    pub allow_outside_roots: bool,

    /// Skip robots.txt policy checks (requires risk acceptance for blocked hosts)
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Robots"
    )]
    pub ignore_robots: bool,

    /// Explicitly accept robots.txt override risk when using --ignore-robots
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Robots"
    )]
    pub i_accept_robots_risk: bool,

    /// Enable deep heap analysis tools (PRD category-memory)
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Categories"
    )]
    pub category_memory: bool,

    /// Enable extension management tools
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Categories"
    )]
    pub category_extensions: bool,

    /// Enable third-party developer tool surface
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Categories"
    )]
    pub category_third_party: bool,

    /// Enable WebMCP-compatible tool surface
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Categories"
    )]
    pub category_webmcp: bool,

    /// Enable experimental screencast (may require ffmpeg for file export)
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Categories"
    )]
    pub experimental_screencast: bool,

    /// Enable coordinate click-at (vision) tools
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Categories"
    )]
    pub experimental_vision: bool,

    /// Enable one-shot local MITM proxy and route Chrome through it (PRD §5E / GAP-019)
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "MITM"
    )]
    pub mitm: bool,

    /// Directory for MITM CA key+cert PEM (default: XDG data)
    #[arg(
        long,
        global = true,
        value_name = "DIR",
        value_hint = ValueHint::DirPath,
        help_heading = "MITM"
    )]
    pub mitm_ca_dir: Option<std::path::PathBuf>,

    /// Write HAR 1.2 to this path on FINALIZE when --mitm is active
    #[arg(
        long,
        global = true,
        value_name = "FILE",
        value_hint = ValueHint::FilePath,
        help_heading = "MITM"
    )]
    pub mitm_har: Option<std::path::PathBuf>,

    /// Comma-separated hosts to decrypt (empty = all via proxy)
    #[arg(long, global = true, value_name = "HOSTS", help_heading = "MITM")]
    pub mitm_hosts: Option<String>,

    /// Capture WebSocket frames in MITM handler
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "MITM"
    )]
    pub mitm_ws: bool,

    /// Max body bytes retained per exchange
    #[arg(long, global = true, value_name = "BYTES", help_heading = "MITM")]
    pub mitm_max_body_bytes: Option<usize>,

    /// Drop image/video/audio bodies from MITM capture
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "MITM"
    )]
    pub mitm_no_media_bodies: bool,

    /// Redact Authorization/Cookie secrets in MITM exports (default on when set)
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "MITM"
    )]
    pub mitm_redact_secrets: bool,

    /// Universal data operations applied to `data` before it reaches stdout.
    ///
    /// Declared in its own module so this file stays under the 300-line ceiling
    /// `scripts/filesize-check.sh` enforces, and because payload reduction is a
    /// responsibility of its own rather than another global knob.
    #[command(flatten)]
    pub agent_ops: super::agent_ops_args::AgentOpsArgs,
}
