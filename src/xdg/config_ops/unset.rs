// SPDX-License-Identifier: MIT OR Apache-2.0
//! `config unset` mutation path (restore a key to its built-in default).
//!
//! # Why this exists
//!
//! `config set` was the only declared way to change configuration, and it had
//! no inverse. Once a key was written there was no supported way to take it
//! back: `config set k ""` stores an empty string, which for an `Option<String>`
//! key is a third state the normal path never produces, and for a numeric key
//! is a parse error. The only remaining route was editing the TOML by hand,
//! which the product tells operators not to do.
//!
//! The gap was not theoretical. `set_network.rs` rejects an empty stealth seed
//! with the message "unset the key to redraw per process" — the product's own
//! error text prescribed an operation the CLI did not expose.
//!
//! # Why this is one function and not a 204-arm match
//!
//! `config set` needs a per-key arm because each key parses and range-checks
//! differently. Unsetting has no value to parse: every key becomes `None`, and
//! `None` is the same shape for all of them. Writing the arms out again would
//! duplicate the key list a third time (after `CONFIG_KEYS` and the writer) and
//! would drift from it silently — which is the exact failure
//! `scripts/config-roundtrip-check.sh` was created to catch.
//!
//! Instead the config round-trips through `serde_json`. `PolicyConfig` is
//! `#[serde(flatten)]` into `ProductConfig`, so the 97 hand-written keys and the
//! 107 macro-generated knobs land in one flat map and clear the same way.
//!
//! # What the operator sees on disk
//!
//! Two shapes, one meaning. Keys emitted by the `config_write` template are
//! rendered unconditionally with `unwrap_or(default)`, so the line stays and
//! shows the default value. Keys emitted by `config_write_optional` appear only
//! when set, so the line disappears. Either way the key is back on the built-in
//! default, which is what the operator asked for.

use serde_json::{json, Map, Value};

use super::super::config_io::{load_config, write_config};
use super::keys::all_config_keys;
use crate::error::{CliError, ErrorKind};

/// Remove one config key, restoring the built-in default.
///
/// Unsetting a key that is already absent succeeds: the requested end state is
/// the observed end state, and reporting an error there would make the command
/// unusable in any script that cannot know the prior state.
///
/// # Errors
///
/// [`ErrorKind::Usage`] when `key` is not in [`all_config_keys`], carrying the
/// `config_list_keys` suggestion.
/// [`ErrorKind::Software`] when [`ProductConfig`](super::super::config_model::ProductConfig)
/// does not serialise to a JSON object, or does not deserialise back after the
/// key is nulled — both are model bugs, never operator input.
/// [`ErrorKind::Config`] propagated from [`load_config`] for an unreadable value
/// already on disk, and [`ErrorKind::Io`] from [`load_config`] or
/// [`write_config`] for path resolution and the atomic rewrite.
pub fn config_unset(key: &str) -> Result<Value, CliError> {
    if !all_config_keys().contains(&key) {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("unknown config key: {key}"),
            crate::i18n::suggestion_key("config_list_keys", None),
        ));
    }

    let cfg = load_config()?;
    let mut map = match serde_json::to_value(&cfg) {
        Ok(Value::Object(m)) => m,
        _ => {
            return Err(CliError::new(
                ErrorKind::Software,
                "config model did not serialise to an object",
            ))
        }
    };

    let was_set = clear_key(&mut map, key);

    let cleared: super::super::config_model::ProductConfig =
        serde_json::from_value(Value::Object(map)).map_err(|e| {
            CliError::new(
                ErrorKind::Software,
                format!("config model did not round-trip after unset: {e}"),
            )
        })?;
    let path = write_config(&cleared)?;

    Ok(json!({
        "key": key,
        "was_set": was_set,
        "target_resolved": key,
        "target_source": "argv",
        "path": path.display().to_string(),
    }))
}

/// Drop `key` from the serialised map, reporting whether it held a value.
///
/// Set to `Null` rather than removed: every field is `Option<_>` with
/// `#[serde(default)]`, so both spellings deserialise to `None`, and keeping the
/// key present makes the round-trip independent of how `flatten` orders fields.
fn clear_key(map: &mut Map<String, Value>, key: &str) -> bool {
    let had_value = map.get(key).is_some_and(|v| !v.is_null());
    map.insert(key.to_string(), Value::Null);
    had_value
}
