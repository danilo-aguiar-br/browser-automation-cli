// SPDX-License-Identifier: MIT OR Apache-2.0
//! Shared JSON field accessors for navigation/interaction steps.

use serde_json::Value;

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

/// First present field among `keys`, read as a string.
pub(super) fn first_str<'a>(step: &'a Value, keys: &[&str]) -> Option<&'a str> {
    first_present(step, keys).and_then(|v| v.as_str())
}

/// First present field among `keys`, read as a bool with `default` fallback.
pub(super) fn first_bool(step: &Value, keys: &[&str], default: bool) -> bool {
    first_present(step, keys)
        .and_then(|v| v.as_bool())
        .unwrap_or(default)
}

/// Read the `include_snapshot` / `includeSnapshot` flag (default false).
pub(super) fn include_snapshot(step: &Value) -> bool {
    first_bool(step, &["include_snapshot", "includeSnapshot"], false)
}
