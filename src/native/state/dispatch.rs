// SPDX-License-Identifier: MIT OR Apache-2.0

use serde_json::Value;

use super::fs_ops::{state_clean, state_clear, state_list, state_rename, state_show};

/// Dispatch a state management command from its JSON payload.
/// Returns `Some(result)` for recognised state_* actions, `None` otherwise.
///
/// # Errors
///
/// The inner `Result` fails with `"Missing 'path' parameter"` for
/// `state_show` and `state_rename` without a string `path`, with
/// `"Missing 'name' parameter"` for `state_rename` without a string `name`,
/// and otherwise carries the error of the `state_*` operation it dispatched
/// to.
///
/// An unrecognised or absent `action` is not an error: it yields `None`, which
/// is how the caller learns this payload was meant for a different dispatcher.
/// A non-numeric `days` on `state_clean` is likewise not an error; it falls
/// back to 30.
pub fn dispatch_state_command(cmd: &Value) -> Option<Result<Value, String>> {
    let action = cmd.get("action").and_then(|v| v.as_str())?;
    match action {
        "state_list" => Some(state_list()),
        "state_show" => Some(
            cmd.get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'path' parameter".to_string())
                .and_then(state_show),
        ),
        "state_clear" => {
            let path = cmd.get("path").and_then(|v| v.as_str());
            Some(state_clear(path))
        }
        "state_clean" => {
            let days = cmd.get("days").and_then(|v| v.as_u64()).unwrap_or(30);
            Some(state_clean(days))
        }
        "state_rename" => Some(
            cmd.get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'path' parameter".to_string())
                .and_then(|path| {
                    cmd.get("name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| "Missing 'name' parameter".to_string())
                        .and_then(|name| state_rename(path, name))
                }),
        ),
        _ => None,
    }
}
