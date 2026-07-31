// SPDX-License-Identifier: MIT OR Apache-2.0
//! Step and capability validation performed before BORN.

use super::super::execute::{is_dispatchable_cmd, reject_unknown_step_fields, unknown_cmd_error};
use super::super::flags::RunFlags;
use super::include::step_cmd;
use super::INCLUDE_CMD;
use crate::browser::CaptureOpts;
use crate::capability::Capability;
use crate::error::{CliError, ErrorKind};
use serde_json::Value;
use std::collections::BTreeSet;

/// Reject unknown commands and unknown fields across the whole script.
pub(super) fn validate_steps(steps: &[Value]) -> Result<(), CliError> {
    for (idx, step) in steps.iter().enumerate() {
        // Expansion removed every include, so any left here is a bug, not input.
        let cmd = step_cmd(step);
        debug_assert_ne!(cmd, INCLUDE_CMD, "include survived expansion");
        if !is_dispatchable_cmd(cmd) {
            return Err(prefix_step(idx, &unknown_cmd_error(cmd)));
        }
        reject_unknown_step_fields(cmd, step).map_err(|e| prefix_step(idx, &e))?;
    }
    Ok(())
}

/// Refuse before BORN when the script needs a flag this invocation lacks.
pub(super) fn validate_capabilities(
    steps: &[Value],
    flags: RunFlags,
    capture: CaptureOpts,
) -> Result<(), CliError> {
    let mut missing: BTreeSet<Capability> = BTreeSet::new();
    for step in steps {
        let cmd = step_cmd(step);
        let action = step
            .get("action")
            .or_else(|| step.get("kind"))
            .and_then(|v| v.as_str());
        for cap in crate::capability::required_capabilities(cmd, action) {
            if !capability_enabled(*cap, flags, capture) {
                missing.insert(*cap);
            }
        }
    }
    if missing.is_empty() {
        return Ok(());
    }
    let flags_list = missing
        .iter()
        .map(|c| c.flag())
        .collect::<Vec<_>>()
        .join(" ");
    Err(CliError::with_suggestion(
        ErrorKind::CapabilityDisabled,
        format!("script requires flags not enabled for this invocation: {flags_list}"),
        format!("Re-run with: {flags_list}"),
    ))
}

/// True when `cap` is enabled by this invocation's flags / capture options.
fn capability_enabled(cap: Capability, flags: RunFlags, capture: CaptureOpts) -> bool {
    match cap {
        Capability::Memory => flags.category_memory,
        Capability::Extensions => flags.category_extensions,
        Capability::ThirdParty => flags.category_third_party,
        Capability::Webmcp => flags.category_webmcp,
        Capability::Vision => flags.experimental_vision,
        Capability::Screencast => flags.experimental_screencast,
        Capability::CaptureConsole => capture.console,
        Capability::CaptureNetwork => capture.network,
    }
}

/// Prefix an error message with the step index that produced it.
fn prefix_step(idx: usize, e: &CliError) -> CliError {
    let mut out = CliError::new(e.kind(), format!("script step {idx}: {}", e.message()));
    if let Some(s) = e.suggestion() {
        out = CliError::with_suggestion(e.kind(), format!("script step {idx}: {}", e.message()), s);
    }
    out
}
