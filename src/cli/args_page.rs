// SPDX-License-Identifier: MIT OR Apache-2.0
//! Argument structs for navigation, input and page capture.
//!
//! Split out of `Commands` for the same reason as the other `args_*` modules:
//! the enum kept every field inline and outgrew the file-size gate. The enum
//! shape is unchanged, so the dispatcher's exhaustive `match` still catches a
//! new variant at compile time.

use clap::{ArgAction, Args, ValueHint};

use super::{BeforeUnloadAction, GrabFormat};

/// Navigate to a URL (one-shot)
#[derive(Debug, Clone, Args)]
pub struct GotoArgs {
    /// Absolute URL to navigate to (robots.txt is honoured unless overridden)
    #[arg(value_hint = ValueHint::Url)]
    pub url: String,
    /// JS to evaluate before navigation (tool-ref initScript)
    #[arg(long)]
    pub init_script: Option<String>,
    /// Auto-handle beforeunload: accept | dismiss (GAP-003; flag alone = accept)
    #[arg(long, value_enum, num_args = 0..=1, default_missing_value = "accept")]
    pub handle_before_unload: Option<BeforeUnloadAction>,
    /// Navigation timeout override in milliseconds
    #[arg(long)]
    pub navigation_timeout_ms: Option<u64>,
}

/// Accessibility snapshot with @eN refs
#[derive(Debug, Clone, Args)]
pub struct ViewArgs {
    /// Full a11y tree (tool-ref take_snapshot.verbose / run JSON `"verbose":true`).
    ///
    /// CLI long name is `--detailed` so it does not silently shadow global
    /// `--verbose` (product logging). Multi-step `run` scripts still use the
    /// JSON key `verbose` for DevTools tool-ref parity.
    #[arg(long = "detailed", action = ArgAction::SetTrue)]
    pub verbose: bool,
    /// Write the snapshot to this path instead of stdout
    #[arg(long, value_hint = ValueHint::FilePath)]
    pub path: Option<std::path::PathBuf>,
    /// Allow empty about:blank snapshots (GAP-012)
    #[arg(long, action = ArgAction::SetTrue)]
    pub allow_empty: bool,
}

/// Click at page CSS coordinates (requires --experimental-vision)
#[derive(Debug, Clone, Args)]
pub struct ClickAtArgs {
    /// Horizontal page CSS pixel coordinate
    #[arg(long)]
    pub x: f64,
    /// Vertical page CSS pixel coordinate
    #[arg(long)]
    pub y: f64,
    /// Send a double click instead of a single click
    #[arg(long, action = ArgAction::SetTrue)]
    pub dblclick: bool,
    /// Attach slim a11y snapshot after the click
    #[arg(long, action = ArgAction::SetTrue)]
    pub include_snapshot: bool,
}

/// Type text (tool-ref type_text). Use --target or --focus-only.
#[derive(Debug, Clone, Args)]
pub struct TypeArgs {
    /// Text to type (required positional)
    pub text: String,
    /// CSS selector or @eN (optional; use --focus-only for focused element)
    #[arg(long)]
    pub target: Option<String>,
    /// Clear the existing value before typing
    #[arg(long, action = ArgAction::SetTrue)]
    pub clear: bool,
    /// Optional key to press after typing (e.g. Enter)
    #[arg(long)]
    pub submit: Option<String>,
    /// Type into currently focused element without resolving a target
    #[arg(long, action = ArgAction::SetTrue)]
    pub focus_only: bool,
    /// Attach slim a11y snapshot after typing
    #[arg(long, action = ArgAction::SetTrue)]
    pub include_snapshot: bool,
}

/// Evaluate JavaScript (expression or function declaration)
#[derive(Debug, Clone, Args)]
pub struct EvalArgs {
    /// JS expression or function declaration `() => ...`
    pub expression: String,
    /// Snapshot uids (@eN) passed as function args (JSON array of strings)
    #[arg(long)]
    pub args: Option<String>,
    /// accept | dismiss | prompt response text (default accept)
    #[arg(long)]
    pub dialog_action: Option<String>,
    /// Write evaluate result JSON to this path
    #[arg(long, value_hint = ValueHint::FilePath)]
    pub file_path: Option<std::path::PathBuf>,
    /// Evaluate inside an extension service worker target (tool-ref serviceWorkerId)
    #[arg(long)]
    pub service_worker_id: Option<String>,
    /// Return the deserialized structure under `data.value` plus the
    /// page-reported `data.value_type` instead of the legacy `data.result` (GAP-035)
    #[arg(long, action = ArgAction::SetTrue)]
    pub typed: bool,
}

/// Capture a screenshot
#[derive(Debug, Clone, Args)]
pub struct GrabArgs {
    /// Output path; defaults to a stamped file under the artifacts directory
    #[arg(long, value_hint = ValueHint::FilePath)]
    pub path: Option<std::path::PathBuf>,
    /// Image encoding of the capture
    #[arg(long, value_enum, default_value_t = GrabFormat::Png)]
    pub format: GrabFormat,
    /// Capture the whole scrollable page instead of the viewport
    #[arg(long, action = ArgAction::SetTrue)]
    pub full_page: bool,
    /// Encoder quality 1..=100 (jpeg/webp only)
    #[arg(long)]
    pub quality: Option<i32>,
    /// CSS selector or @eN element to capture
    #[arg(long)]
    pub element: Option<String>,
    /// Opt-in: include raw image base64 in the JSON envelope (agent-native default off)
    #[arg(long, action = ArgAction::SetTrue)]
    pub include_base64: bool,
}

/// Record page interactions as a replayable `run --script` NDJSON file
#[derive(Debug, Clone, Args)]
pub struct RecordArgs {
    /// Absolute URL to open and record
    #[arg(long, value_hint = ValueHint::Url)]
    pub url: String,
    /// Destination NDJSON file for the recorded steps
    #[arg(long, value_hint = ValueHint::FilePath)]
    pub path: std::path::PathBuf,
    /// Wall-clock recording ceiling in seconds (first ceiling reached wins)
    #[arg(long, default_value_t = crate::constants::RECORD_DEFAULT_SECONDS)]
    pub seconds: u64,
    /// Recorded-step ceiling (first ceiling reached wins)
    #[arg(long, default_value_t = crate::constants::RECORD_DEFAULT_MAX_EVENTS as u64)]
    pub max_events: u64,
}

/// Extract text/attribute from a target, or LLM extract with --llm
#[derive(Debug, Clone, Args)]
pub struct ExtractArgs {
    /// Selector, @eN ref, about:blank target, or http(s) URL for LLM/text path
    pub target: String,
    /// Read this attribute instead of the element text
    #[arg(long)]
    pub attr: Option<String>,
    /// Opt-in LLM HTTP extract (requires XDG openrouter_api_key)
    #[arg(long, action = ArgAction::SetTrue)]
    pub llm: bool,
    /// Question for LLM extract
    #[arg(long)]
    pub question: Option<String>,
    /// Path to JSON schema file for structured LLM extract
    #[arg(long, value_hint = ValueHint::FilePath)]
    pub schema_json: Option<std::path::PathBuf>,
}
