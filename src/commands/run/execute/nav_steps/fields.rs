// SPDX-License-Identifier: MIT OR Apache-2.0
//! Shared JSON field accessors for navigation/interaction steps.
//!
//! # Why the readers take a `cmd`
//!
//! Each of these steps used to carry its own inline array of accepted
//! spellings — `&["target", "ref", "selector"]` written out at 25 sites across
//! seven files. Nothing connected those arrays to the validator, so `ref` and
//! `sel` and `trigger` were read by the code, published by no schema, and
//! rejected by nothing. The readers below take the command name and resolve the
//! spellings through [`step_key_reads`], so the list a handler reads IS the
//! list the validator allows.

use serde_json::Value;

use super::super::super::inventory::step_key_reads;

/// First **present** field among `keys`, in order.
///
/// Mirrors the original `get(a).or_else(|| get(b))` chains: a present-but-wrong
/// type value short-circuits the lookup instead of falling through to the next
/// alias.
pub(super) fn first_present<'a>(step: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    for k in keys {
        if let Some(v) = step.get(*k) {
            return Some(v);
        }
    }
    None
}

/// First present spelling of `step_key` for `cmd`, whatever its JSON type.
pub(super) fn step_present<'a>(step: &'a Value, cmd: &str, step_key: &str) -> Option<&'a Value> {
    first_present(step, &step_key_reads(cmd, step_key))
}

/// First present spelling of `step_key` for `cmd`, read as a string.
pub(super) fn step_str<'a>(step: &'a Value, cmd: &str, step_key: &str) -> Option<&'a str> {
    step_present(step, cmd, step_key).and_then(|v| v.as_str())
}

/// First present spelling of `step_key` for `cmd`, read as a bool.
pub(super) fn step_bool(step: &Value, cmd: &str, step_key: &str, default: bool) -> bool {
    step_present(step, cmd, step_key)
        .and_then(|v| v.as_bool())
        .unwrap_or(default)
}

/// Read the `include_snapshot` flag of `cmd` (default false).
pub(super) fn include_snapshot(step: &Value, cmd: &str) -> bool {
    step_bool(step, cmd, "include_snapshot", false)
}
