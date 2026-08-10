// SPDX-License-Identifier: MIT OR Apache-2.0
//! Argument structs for the small verbs: meta queries and single-target input.
//!
//! Small individually, and collectively the last thing keeping both
//! `src/cli/commands.rs` and `src/commands/dispatch/mod.rs` over the file-size
//! gate. Moving them makes every dispatcher arm one line, which is the shape
//! `scripts/filesize-check.sh` argues for.

use clap::{ArgAction, Args, ValueHint};

use super::BeforeUnloadAction;

/// Diagnose Chrome install and one-shot readiness
///
/// Use global `--json` for machine-readable envelopes (no local `--json` —
/// avoids silent shadowing of the global flag).
#[derive(Debug, Clone, Args)]
pub struct DoctorArgs {
    /// Skip network probes and record `offline` mode in the report
    #[arg(long, action = ArgAction::SetTrue)]
    pub offline: bool,
    /// Skip the Chrome launch probe (fastest readiness signal)
    #[arg(long, action = ArgAction::SetTrue)]
    pub quick: bool,
    /// Attach a suggested repair command to each failing check
    #[arg(long, action = ArgAction::SetTrue)]
    pub fix: bool,
}

/// JSON Schema fragment for a command (agent discovery)
/// GAP-022: accepts `schema run` or `schema --cmd run`.
#[derive(Debug, Clone, Args)]
pub struct SchemaArgs {
    /// Command name via --cmd
    #[arg(long = "cmd", value_name = "CMD")]
    pub cmd: Option<String>,
    /// Command name as positional (preferred agent UX)
    #[arg(value_name = "CMD")]
    pub cmd_positional: Option<String>,
}

/// Click an element (selector or @eN)
#[derive(Debug, Clone, Args)]
pub struct PressArgs {
    /// CSS selector or `@eN` snapshot ref to click
    pub target: String,
    /// Send a double click instead of a single click
    #[arg(long, action = ArgAction::SetTrue)]
    pub dblclick: bool,
    /// Attach slim a11y snapshot in the same process after the action
    #[arg(long, action = ArgAction::SetTrue)]
    pub include_snapshot: bool,
}

/// Fill an input value (select/checkbox/radio/text smart fill)
#[derive(Debug, Clone, Args)]
pub struct WriteArgs {
    /// CSS selector or `@eN` snapshot ref of the field to fill
    pub target: String,
    /// Value to set (select option, checkbox state, or text)
    pub value: String,
    /// Attach slim a11y snapshot after fill
    #[arg(long, action = ArgAction::SetTrue)]
    pub include_snapshot: bool,
}

/// Press a keyboard key
#[derive(Debug, Clone, Args)]
pub struct KeysArgs {
    /// Key name to press (for example `Enter`, `Escape`, `Control+a`)
    pub key: String,
    /// Attach slim a11y snapshot after the key press
    #[arg(long, action = ArgAction::SetTrue)]
    pub include_snapshot: bool,
}

/// Hover an element
#[derive(Debug, Clone, Args)]
pub struct HoverArgs {
    /// CSS selector or `@eN` snapshot ref to hover
    pub target: String,
    /// Attach slim a11y snapshot after hover
    #[arg(long, action = ArgAction::SetTrue)]
    pub include_snapshot: bool,
}

/// Submit a form, or the form owning a field, and wait for its outcome (GAP-036)
#[derive(Debug, Clone, Args)]
pub struct SubmitArgs {
    /// CSS selector or @eN of the `<form>` or of any field inside it
    pub target: String,
    /// Max time to wait for navigation or a completed request
    #[arg(long)]
    pub timeout_ms: Option<u64>,
    /// Attach slim a11y snapshot after the outcome
    #[arg(long, action = ArgAction::SetTrue)]
    pub include_snapshot: bool,
}

/// Fill multiple form fields from JSON `[{target|uid,value},...]`
#[derive(Debug, Clone, Args)]
pub struct FillFormArgs {
    /// JSON array payload (not the global envelope flag `--json`)
    #[arg(long = "fields-json", value_name = "JSON")]
    pub fields_json: String,
    /// Attach slim a11y snapshot after fill-form
    #[arg(long, action = ArgAction::SetTrue)]
    pub include_snapshot: bool,
}

/// Upload a file to a file input
#[derive(Debug, Clone, Args)]
pub struct UploadArgs {
    /// CSS selector or `@eN` snapshot ref of the file input
    pub target: String,
    /// Local file to attach to the input
    #[arg(value_hint = ValueHint::FilePath)]
    pub path: std::path::PathBuf,
    /// Attach slim a11y snapshot after upload
    #[arg(long, action = ArgAction::SetTrue)]
    pub include_snapshot: bool,
}

/// Reload current page
#[derive(Debug, Clone, Args)]
pub struct ReloadArgs {
    /// Bypass the HTTP cache (hard reload)
    #[arg(long, action = ArgAction::SetTrue)]
    pub ignore_cache: bool,
    /// JS to run before navigation/reload (tool-ref initScript)
    #[arg(long)]
    pub init_script: Option<String>,
    /// Auto-handle beforeunload: accept | dismiss (GAP-003; flag alone = accept)
    #[arg(long, value_enum, num_args = 0..=1, default_missing_value = "accept")]
    pub handle_before_unload: Option<BeforeUnloadAction>,
}
