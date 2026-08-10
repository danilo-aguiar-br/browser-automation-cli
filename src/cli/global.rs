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

    /// Show the browser window on your own display (debugging)
    ///
    /// The default `browser_mode = auto` currently launches headless. Stealth
    /// does not depend on that: the anti-detection patches, the launch switches
    /// and the identity all apply either way, and `navigator.webdriver` is
    /// `undefined` in both. What headless still costs is the window itself —
    /// `window.outerHeight` and `outerWidth` read 0 on a raw headless Chrome,
    /// which this product patches, and WebGL falls back to a software
    /// rasteriser. Persist a choice with `config set browser_mode headed`.
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Browser"
    )]
    pub headed: bool,

    /// Skip the private virtual display on Linux (use the current display)
    ///
    /// Only meaningful with a headed mode on Linux. Without a virtual display a
    /// headed launch puts a real window on your desktop, because GNOME/Mutter
    /// clamps off-screen window positions back into view.
    #[arg(
        long = "no-xvfb",
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Browser"
    )]
    pub no_xvfb: bool,

    /// Turn off anti-detection patches for this run
    ///
    /// Stealth is ON by default. It masks the automation markers a real Chrome
    /// never exposes: `navigator.webdriver`, an empty plugin array, a missing
    /// `chrome.runtime`, and a Canvas hash that changes on every read. Turn it
    /// off when you are testing your OWN front end and want the browser
    /// untouched. Persist the choice with `config set stealth false`.
    #[arg(
        long = "no-stealth",
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Browser"
    )]
    pub no_stealth: bool,

    /// Impersonated identity: `auto`, `chrome-linux`, `chrome-win`, `chrome-mac`
    ///
    /// `auto` follows the host and is almost always right. Claiming a foreign
    /// platform contradicts the Canvas hash, the WebGL renderer and the TLS
    /// fingerprint, which this product does not impersonate — the mismatch is a
    /// stronger signal than the honest platform.
    #[arg(
        long = "stealth-profile",
        global = true,
        value_name = "PROFILE",
        value_parser = ["auto", "chrome-linux", "chrome-win", "chrome-mac"],
        help_heading = "Browser"
    )]
    pub stealth_profile: Option<String>,

    /// Pin the stealth identity across processes with a seed
    ///
    /// Without it every run draws a fresh identity, so a 50-URL crawl — 50
    /// one-shot processes — presents 50 different machines from one address.
    /// No real user does that, and it is a stronger signal than any single
    /// marker this product masks. With a seed the generated patch script is
    /// cached under XDG state and reused, so the N runs look like one browser.
    /// Persist it with `config set stealth_seed <value>`.
    #[arg(
        long = "stealth-seed",
        global = true,
        value_name = "SEED",
        help_heading = "Browser"
    )]
    pub stealth_seed: Option<String>,

    /// Visit the origin root before the target URL so the session carries cookies
    ///
    /// Some challenge systems expect a session that already has cookies and a
    /// referrer chain. A cold hit straight at a deep URL has neither.
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Browser"
    )]
    pub warmup: bool,

    /// Warm this URL instead of the target's origin root
    ///
    /// The default warm-up lands on the root because that is where a browser
    /// lands. When the edge hands out the session somewhere else — a login
    /// page, a locale splash, a region redirector — warming the root buys a
    /// cookie the target will not accept. Naming the real entry point is
    /// cheaper than giving up on the warm-up.
    ///
    /// Implies `--warmup`; passing it alone is enough.
    #[arg(long, global = true, value_name = "URL", help_heading = "Browser")]
    pub warmup_url: Option<String>,

    /// Egress proxy for Chrome and the HTTP engine (`http`, `https`, `socks5`)
    ///
    /// Applies to both engines, so a blocked address changes for the whole run
    /// rather than for one of them. Credentials belong in the XDG file via
    /// `config set proxy_url`, never in argv where the process table shows them.
    #[arg(long, global = true, value_name = "URL", help_heading = "Network")]
    pub proxy: Option<String>,

    /// Hosts that bypass the proxy, in Chrome's bypass-list syntax
    #[arg(
        long = "proxy-bypass",
        global = true,
        value_name = "HOSTS",
        help_heading = "Network"
    )]
    pub proxy_bypass: Option<String>,

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

    /// Input shaping: `human` (default) or `direct`
    ///
    /// `human` interpolates pointer trajectories, dwells between press and release,
    /// paces typing, and scrolls with real `mouseWheel` ticks, so a page that listens
    /// for `wheel` or `keydown` reacts. `direct` emits one event per action, exactly
    /// as before 0.1.8: faster and exactly deterministic, but a scroll produces no
    /// `wheel` event and printable typing produces no `keydown`.
    #[arg(
        long,
        global = true,
        value_name = "PROFILE",
        value_parser = ["human", "direct"],
        help_heading = "Browser"
    )]
    pub input_profile: Option<String>,

    /// Seed the input jitter so a `human` run reproduces exactly
    ///
    /// Without it the jitter is drawn from the OS and two runs differ. Set it in CI
    /// and in tests that assert on the event trace.
    #[arg(long, global = true, value_name = "SEED", help_heading = "Browser")]
    pub input_seed: Option<u64>,

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

    /// Redact Authorization/Cookie secrets in MITM captures (already the default)
    ///
    /// Kept because it reads as an intent, and passing it changes nothing:
    /// redaction is on unless `--mitm-no-redact-secrets` turns it off.
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "MITM"
    )]
    pub mitm_redact_secrets: bool,

    /// Keep Authorization/Cookie values readable in the MITM capture
    ///
    /// The capture is written to disk and read back by an agent, so masking is
    /// the default: forgetting the flag costs a missing header, while the
    /// opposite default would make forgetting it cost a leaked session cookie.
    /// Turn it off only when the secret itself is what you are debugging.
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "MITM"
    )]
    pub mitm_no_redact_secrets: bool,

    /// Universal data operations applied to `data` before it reaches stdout.
    ///
    /// Declared in its own module so this file stays under the 300-line ceiling
    /// `scripts/filesize-check.sh` enforces, and because payload reduction is a
    /// responsibility of its own rather than another global knob.
    #[command(flatten)]
    pub agent_ops: super::agent_ops_args::AgentOpsArgs,
}
