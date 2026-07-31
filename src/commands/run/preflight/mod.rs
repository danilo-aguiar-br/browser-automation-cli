// SPDX-License-Identifier: MIT OR Apache-2.0
//! Whole-script validation before BORN (GAP-012 / GAP-029).
//!
//! The engine used to parse each step at the moment it ran, so a typo in step
//! five was discovered only after steps one to four had navigated, clicked and
//! typed on the live target. The browser launch was paid for, and the remote
//! system was left half-mutated, to learn about a misspelled command.
//!
//! Preflight moves every check that does not need a live page to before the
//! launch:
//!
//! | Check | Failure |
//! |-------|---------|
//! | include cycle / depth / missing file | `data` (65) / `no-input` (66) |
//! | unknown step command | `usage` (2) |
//! | unknown field on a step | `usage` (2) |
//! | capability flag missing for some step | `capability-disabled` (64) |
//!
//! Capability failures are reported **together**, so one run reveals every flag
//! the script needs instead of one per launch.
//!
//! # Module map (GAP-051 SRP split)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | include | `include` expansion, cycle and depth limits |
//! | validate | step command / field / capability checks |

mod include;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_include;
mod validate;

use super::flags::RunFlags;
use crate::browser::CaptureOpts;
use crate::error::{CliError, ErrorKind};
use serde_json::Value;
use std::path::{Path, PathBuf};

use include::load_expanded;
use validate::{validate_capabilities, validate_steps};

/// Maximum nesting depth for `include` steps.
///
/// A cycle is caught exactly by the path stack; this is the backstop for a deep
/// but acyclic tree that would otherwise exhaust the stack.
pub(super) const MAX_INCLUDE_DEPTH: usize = 16;

/// Step command that splices another script file in place.
pub(super) const INCLUDE_CMD: &str = "include";

/// Load, expand and validate a script without launching a browser.
///
/// Returns the fully expanded step list on success.
pub fn preflight_script(
    script_path: &Path,
    flags: RunFlags,
    capture: CaptureOpts,
) -> Result<Vec<Value>, CliError> {
    let mut stack: Vec<PathBuf> = Vec::new();
    let steps = load_expanded(script_path, &mut stack, 0)?;

    if steps.is_empty() {
        return Err(CliError::with_suggestion(
            ErrorKind::Data,
            "script has no steps",
            "Add at least one NDJSON line or a JSON array of objects with a cmd field",
        ));
    }

    validate_steps(&steps)?;
    validate_capabilities(&steps, flags, capture)?;
    Ok(steps)
}
