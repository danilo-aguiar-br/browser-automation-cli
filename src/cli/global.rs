// SPDX-License-Identifier: MIT OR Apache-2.0
//! Global CLI flags flattened into every subcommand.

use clap::{ArgAction, Args, ValueHint};

/// Parse a timeout expressed in seconds, with an OPTIONAL unit suffix.
///
/// # Why a suffix is accepted at all
///
/// The unit used to live only in the flag NAME, and `wait` and
/// `navigation_timeout_ms` next to it take milliseconds. A caller who read
/// `--step-timeout 150000` as milliseconds was asking for forty-one hours and
/// had no way to say otherwise in the argv. `150s`, `5m` and `2h` let the
/// caller state the unit, so the reading can no longer be silently wrong.
///
/// A bare number stays seconds, so every existing invocation keeps its meaning.
///
/// # Why the ceiling lives here and not in `range()`
///
/// `clap::value_parser!(u64).range(..)` cannot see the suffix, so `5m` would
/// have to be rejected before it was understood. Doing both in one function is
/// what keeps `2h` legal and `100h` refused with the reason.
pub fn parse_timeout_secs(raw: &str) -> Result<u64, String> {
    const MAX: u64 = crate::constants::MAX_GLOBAL_TIMEOUT_SECS;
    let text = raw.trim();
    if text.is_empty() {
        return Err("empty timeout: write a whole number of seconds".to_string());
    }
    let (digits, multiplier) = match text.as_bytes().last() {
        Some(b's' | b'S') => (&text[..text.len() - 1], 1_u64),
        Some(b'm' | b'M') => (&text[..text.len() - 1], 60_u64),
        Some(b'h' | b'H') => (&text[..text.len() - 1], 3_600_u64),
        _ => (text, 1_u64),
    };
    let count: u64 = digits.trim().parse().map_err(|_| {
        format!(
            "`{text}` is not a timeout: write whole seconds, optionally suffixed \
             with s, m or h (for example 90, 90s, 5m, 2h)"
        )
    })?;
    let seconds = count
        .checked_mul(multiplier)
        .ok_or_else(|| format!("`{text}` overflows: the ceiling is {MAX} seconds (24h)"))?;
    if seconds > MAX {
        return Err(format!(
            "`{text}` is {seconds} seconds and the ceiling is {MAX} (24h); a \
             one-shot invocation cannot mean a longer budget"
        ));
    }
    Ok(seconds)
}

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

    /// Global wall-clock timeout in seconds (0 = no override, max 86400)
    ///
    /// Bounded so a typo cannot ask a one-shot process to wait for years; see
    /// [`crate::constants::MAX_GLOBAL_TIMEOUT_SECS`] for why the bound is not
    /// configurable. `0` still means "no override", and the per-operation
    /// budgets keep applying either way.
    #[arg(
        long,
        global = true,
        default_value_t = 0,
        value_name = "SECS",
        value_parser = parse_timeout_secs,
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

    /// Per-step timeout in seconds for `run` scripts (0 = inherit global timeout, max 86400)
    ///
    /// # Why this carries the same bound as `--timeout`
    ///
    /// It did not until 0.1.9, and the asymmetry was the whole defect. Its
    /// sibling above has had `range(0..=MAX_GLOBAL_TIMEOUT_SECS)` all along;
    /// this one accepted any `u64`. A caller who read `150000` as milliseconds
    /// — a reasonable misreading, since `wait` takes `ms` and
    /// `navigation_timeout_ms` takes `ms` — was silently granted a ceiling of
    /// forty-one HOURS, and the mistake surfaced later disguised as a different
    /// problem.
    ///
    /// A value no one-shot invocation can mean is refused at the parser, which
    /// costs an argv error instead of a browser launch. Since 0.1.9 the unit no
    /// longer lives only in the name either: `--step-timeout 5m` says what it
    /// means, and a caller who writes the unit cannot be misread.
    #[arg(
        long,
        global = true,
        default_value_t = 0,
        value_name = "SECS",
        value_parser = parse_timeout_secs,
        help_heading = "Timeouts"
    )]
    pub step_timeout: u64,

    /// Show the browser window on your own display (debugging)
    ///
    /// The default `browser_mode = auto` currently launches headless. Stealth
    /// does not depend on that: the anti-detection patches, the launch switches
    /// and the identity all apply either way, and `navigator.webdriver` is
    /// present and `false` in both. What headless still costs is the window itself —
    /// `window.outerHeight` and `outerWidth` read 0 on a raw headless Chrome,
    /// which this product patches, and WebGL falls back to a software
    /// rasteriser. Persist a choice with `config set browser_mode headed`.
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        conflicts_with_all = ["headless", "browser_mode"],
        help_heading = "Browser"
    )]
    pub headed: bool,

    /// Require a headless browser for this run, overriding any persisted mode
    ///
    /// # Why the symmetric flag had to exist
    ///
    /// Until 0.1.9 only `--headed` existed, so headless was reachable ONLY by
    /// not asking for anything. That makes "I require headless" and "I said
    /// nothing" the same argv, and a requirement that cannot be expressed
    /// cannot be verified. Worse, with no flag for it, a `config set
    /// browser_mode headed` run for an unrelated debugging task silently won
    /// for every automated caller on the machine.
    ///
    /// Callers worked around it by stripping `DISPLAY` from the environment.
    /// That is a defence by absence: it protects only the paths someone
    /// remembered to strip, and fails silently on the first new one.
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        conflicts_with_all = ["headed", "browser_mode"],
        help_heading = "Browser"
    )]
    pub headless: bool,

    /// Window mode for this run: `auto`, `headless` or `headed`
    ///
    /// The canonical spelling; `--headed` and `--headless` are shorthands for
    /// two of its values. Whichever is passed WINS over the XDG
    /// `browser_mode`, and the envelope reports which step decided, so the
    /// choice is provable after the fact rather than assumed.
    #[arg(
        long = "browser-mode",
        global = true,
        value_name = "MODE",
        value_parser = ["auto", "headless", "headed"],
        conflicts_with_all = ["headed", "headless"],
        help_heading = "Browser"
    )]
    pub browser_mode: Option<String>,

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
    /// Stealth is ON by default. It keeps `navigator.webdriver` present with
    /// value `false` (a real Chrome always defines the property), fills the
    /// plugin array, restores `chrome.runtime`, and pins a Canvas hash. Turn
    /// it off when you are testing your OWN front end and want the browser
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
        value_parser = ["auto", "chrome-linux", "chrome-win", "chrome-mac", "list"],
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
    /// The seed varies `hardwareConcurrency`, `deviceMemory`, GPU vendor and
    /// renderer, `history.length` and the Chrome build number. It does not
    /// vary User-Agent, `navigator.platform`, languages, timezone, screen or
    /// `plugins.length`. Persist it with `config set stealth_seed <value>`.
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

    /// Minimum delay between same-origin requests, in milliseconds.
    ///
    /// A per-invocation floor for the same courtesy budget XDG
    /// `scrape_min_delay_ms` sets for the whole host. The effective wait is the
    /// MAXIMUM of this, the XDG floor, and the site's own `Crawl-delay`: a flag
    /// that could lower `Crawl-delay` would be a way to ignore the site rather
    /// than a way to be polite to it.
    #[arg(long, global = true, value_name = "MS", help_heading = "Scrape")]
    pub min_delay_ms: Option<u64>,

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

    /// One-shot local MITM proxy options.
    ///
    /// Flattened, so every flag keeps its exact argv spelling; see
    /// `super::mitm_args` for why the group lives in its own file.
    #[command(flatten)]
    pub mitm_args: super::mitm_args::MitmArgs,

    /// Universal data operations applied to `data` before it reaches stdout.
    ///
    /// Declared in its own module so this file stays under the 300-line ceiling
    /// `scripts/filesize-check.sh` enforces, and because payload reduction is a
    /// responsibility of its own rather than another global knob.
    #[command(flatten)]
    pub agent_ops: super::agent_ops_args::AgentOpsArgs,
}
