// SPDX-License-Identifier: MIT OR Apache-2.0
//! Dispatch context and exit-code mapping.
//!
//! Split out of `mod.rs` so that file holds only the exhaustive `route` match.

use crate::browser::CaptureOpts;
use crate::error::CliError;
use crate::lifecycle::Lifecycle;
use crate::robots::RobotsPolicy;

use super::super::common::emit_err;

/// Shared dispatch context (one-shot process locals).
pub(crate) struct DispatchCtx<'a> {
    pub life: &'a Lifecycle,
    pub json: bool,
    pub capture: CaptureOpts,
    pub timeout_secs: u64,
    pub step_timeout_secs: u64,
    pub robots: RobotsPolicy,
    pub category_memory: bool,
    pub category_extensions: bool,
    pub category_third_party: bool,
    pub category_webmcp: bool,
    pub experimental_screencast: bool,
    pub experimental_vision: bool,
    pub json_steps: bool,
    /// Global `--artifacts-dir` for grab/output layout.
    pub artifacts: Option<std::path::PathBuf>,
}

/// Map handler `Result` to process exit code.
pub(crate) fn result_code(r: Result<(), CliError>, json: bool) -> i32 {
    match r {
        Ok(()) => 0,
        Err(e) => emit_err(&e, json),
    }
}
