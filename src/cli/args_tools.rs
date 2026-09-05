// SPDX-License-Identifier: MIT OR Apache-2.0
//! Argument structs for the local tooling verbs.
//!
//! These verbs touch the filesystem or an external binary rather than a page,
//! which is why they are their own family rather than folded into `args_page`.

use clap::{ArgAction, Args, ValueHint};

/// Discover filesystem paths (fd-like UX; binary remains browser-automation-cli)
#[derive(Debug, Clone, Args)]
pub struct FindPathsArgs {
    /// Regex pattern on name/path (optional)
    pub pattern: Option<String>,
    /// Root paths to search
    #[arg(num_args = 0..)]
    pub paths: Vec<String>,
    /// Filter by extension (e.g. rs, html)
    #[arg(long)]
    pub extension: Option<String>,
    /// Include hidden files
    #[arg(long, action = ArgAction::SetTrue)]
    pub hidden: bool,
    /// Do not respect .gitignore
    #[arg(long, action = ArgAction::SetTrue)]
    pub no_ignore: bool,
    /// Max directory depth
    #[arg(long)]
    pub max_depth: Option<usize>,
    /// Entry type: f|d
    #[arg(long = "type")]
    pub entry_type: Option<String>,
    /// Max results
    #[arg(long, default_value_t = crate::constants::FIND_PATHS_LIMIT)]
    pub limit: usize,
    /// Shell-style glob filter (e.g. `**/*.rs`) — GAP-A011 / §5AE
    #[arg(long)]
    pub glob: Option<String>,
}

/// Run Lighthouse audit (external binary)
#[derive(Debug, Clone, Args)]
pub struct LighthouseArgs {
    /// Absolute URL to audit
    #[arg(value_hint = ValueHint::Url)]
    pub url: String,
    /// Directory for the generated report files
    #[arg(long, value_hint = ValueHint::DirPath)]
    pub out_dir: Option<std::path::PathBuf>,
    /// Form factor preset: desktop | mobile
    #[arg(long, default_value = "desktop")]
    pub device: String,
    /// navigation (default) or snapshot (maps to navigation in one-shot CLI)
    #[arg(long, default_value = "navigation")]
    pub mode: String,
    /// Absolute path to the lighthouse binary; overrides PATH and XDG
    #[arg(long, value_hint = ValueHint::FilePath)]
    pub lighthouse_path: Option<std::path::PathBuf>,
}

/// Resize page viewport
#[derive(Debug, Clone, Args)]
pub struct ResizeArgs {
    /// Viewport width in CSS pixels
    #[arg(long)]
    pub width: i32,
    /// Viewport height in CSS pixels
    #[arg(long)]
    pub height: i32,
    /// Device pixel ratio
    #[arg(long, default_value_t = crate::constants::EMULATE_DEFAULT_SCALE)]
    pub scale: f64,
    /// Enable mobile metrics emulation
    #[arg(long, action = ArgAction::SetTrue)]
    pub mobile: bool,
    /// Screen size `WxH`. Defaults to the viewport so screen cannot stay 800x600.
    #[arg(long, value_name = "WxH")]
    pub screen: Option<String>,
}

/// Write a simple XLSX workbook from CSV/JSON (one-shot; §5Z / GAP-A011)
#[derive(Debug, Clone, Args)]
pub struct SheetWriteArgs {
    /// Input path (.csv or .json array-of-objects)
    #[arg(value_hint = ValueHint::FilePath)]
    pub input: std::path::PathBuf,
    /// Output .xlsx path
    #[arg(long, short = 'o', value_hint = ValueHint::FilePath)]
    pub out: std::path::PathBuf,
    /// Worksheet name
    #[arg(long, default_value = "Sheet1")]
    pub sheet: String,
    /// Overwrite `--out` when it already exists
    ///
    /// Without this, an existing destination is refused rather than replaced.
    /// A workbook is a deliverable, not a scratch file, and losing one to a
    /// re-run with a stale `-o` is not recoverable from the envelope.
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub force: bool,
}
