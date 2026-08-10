// SPDX-License-Identifier: MIT OR Apache-2.0
//! Argument structs for waiting, pointer input and emulation.
//!
//! Same reason as the scrape family: the fields moved out of `Commands` so the
//! enum fits the file-size gate, and the enum itself is unchanged in shape.

use clap::{ArgAction, Args};

/// Wait for ms and/or text and/or selector and/or load state
#[derive(Debug, Clone, Args)]
pub struct WaitArgs {
    /// Unconditional sleep in milliseconds before evaluating other conditions
    #[arg(long, default_value_t = 0)]
    pub ms: u64,
    /// Text to wait for (repeatable; resolves when any value appears — tool-ref OR)
    #[arg(long = "text", action = clap::ArgAction::Append)]
    pub text: Vec<String>,
    /// CSS selector that must match before the wait resolves
    #[arg(long)]
    pub selector: Option<String>,
    /// Page lifecycle: load | domcontentloaded | networkidle | none
    #[arg(long)]
    pub state: Option<String>,
    /// Max wait time in milliseconds for text/selector/state (0 = default)
    #[arg(long)]
    pub wait_timeout_ms: Option<u64>,
    /// Resolve when no request has been in flight for this many ms.
    /// Bare flag (or 0) uses the built-in window (GAP-032).
    #[arg(
        long = "network-idle",
        value_name = "MS",
        num_args = 0..=1,
        default_missing_value = "0"
    )]
    pub network_idle_ms: Option<u64>,
    /// Minimum number of nodes --selector must match (default 1; GAP-032)
    #[arg(long)]
    pub min_count: Option<u64>,
    /// Resolve when the serialized DOM has not changed for this many ms.
    /// Bare flag (or 0) uses the built-in window (GAP-032).
    #[arg(
        long = "dom-stable",
        value_name = "MS",
        num_args = 0..=1,
        default_missing_value = "0"
    )]
    pub dom_stable_ms: Option<u64>,
    /// Attach slim a11y snapshot after the wait succeeds
    #[arg(long, action = ArgAction::SetTrue)]
    pub include_snapshot: bool,
}

/// Drag from one target to another (HTML5 drag-and-drop; GAP-030)
#[derive(Debug, Clone, Args)]
pub struct DragArgs {
    /// Source CSS selector or `@eN` snapshot ref to drag
    #[arg(long)]
    pub from: String,
    /// Destination CSS selector or @eN (omit only when using --to-x/--to-y)
    #[arg(long)]
    pub to: Option<String>,
    /// Absolute drop X in page CSS pixels (overrides the destination rect)
    #[arg(long)]
    pub to_x: Option<f64>,
    /// Absolute drop Y in page CSS pixels (overrides the destination rect)
    #[arg(long)]
    pub to_y: Option<f64>,
    /// Where in the destination rect to drop: center | before | after.
    /// Edge anchors disambiguate insertion order in a sorted list.
    #[arg(long, default_value = "center")]
    pub anchor: String,
    /// Inject this CDP DragData instead of the DataTransfer the page builds.
    /// Opt-in: it bypasses the page's own dragstart handler.
    #[arg(long = "synthetic-payload", value_name = "JSON")]
    pub synthetic_payload: Option<String>,
    /// Attach slim a11y snapshot after drag
    #[arg(long, action = ArgAction::SetTrue)]
    pub include_snapshot: bool,
}

/// Scroll page or element by delta pixels (PRD §7 `scroll`)
#[derive(Debug, Clone, Args)]
pub struct ScrollArgs {
    /// CSS selector or @eN (optional; omit for window scroll)
    #[arg(long)]
    pub target: Option<String>,
    /// Horizontal scroll delta in page CSS pixels
    #[arg(long, default_value_t = 0.0)]
    pub delta_x: f64,
    /// Vertical scroll delta in page CSS pixels
    #[arg(long, default_value_t = 0.0)]
    pub delta_y: f64,
    /// Absolute horizontal offset; wins over --delta-x on that axis (GAP-031)
    #[arg(long)]
    pub to_x: Option<f64>,
    /// Absolute vertical offset; wins over --delta-y on that axis (GAP-031)
    #[arg(long)]
    pub to_y: Option<f64>,
    /// Attach slim a11y snapshot after the scroll
    #[arg(long, action = ArgAction::SetTrue)]
    pub include_snapshot: bool,
}

/// Emulate device / network / UA / geo / CPU
#[derive(Debug, Clone, Args)]
pub struct EmulateArgs {
    /// Override the browser User-Agent string
    #[arg(long)]
    pub user_agent: Option<String>,
    /// BCP47 locale reported to the page (for example `pt-BR`)
    #[arg(long)]
    pub locale: Option<String>,
    /// IANA timezone reported to the page (for example `America/Sao_Paulo`)
    #[arg(long)]
    pub timezone: Option<String>,
    /// Force the page offline (no network)
    #[arg(long, action = ArgAction::SetTrue)]
    pub offline: bool,
    /// Geolocation latitude in degrees; pair with --longitude
    #[arg(long)]
    pub latitude: Option<f64>,
    /// Geolocation longitude in degrees; pair with --latitude
    #[arg(long)]
    pub longitude: Option<f64>,
    /// CSS media type to emulate (for example `print`)
    #[arg(long)]
    pub media: Option<String>,
    /// Network preset: Offline, No throttling, Slow 3G, Fast 3G, Slow 4G, Fast 4G
    #[arg(long)]
    pub network_conditions: Option<String>,
    /// CPU slowdown factor 1..=20 (1 disables)
    #[arg(long)]
    pub cpu_throttling_rate: Option<f64>,
    /// prefers-color-scheme: dark | light | auto
    #[arg(long)]
    pub color_scheme: Option<String>,
    /// Extra HTTP headers as JSON object string
    #[arg(long)]
    pub extra_headers: Option<String>,
    /// Viewport `WxHxDPR` with optional `,mobile`, `,touch`, `,landscape` flags
    #[arg(long)]
    pub viewport: Option<String>,
}
