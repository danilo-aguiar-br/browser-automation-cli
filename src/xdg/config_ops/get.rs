// SPDX-License-Identifier: MIT OR Apache-2.0
//! `config get` read path (single key or full dump, secrets redacted).
//!
//! Both surfaces read [`super::get_table::config_entries`]. Neither one
//! transcribes the key list, which is what used to make adding a key a
//! two-file edit with no gate on the second half.

use serde_json::{json, Value};

use super::super::config_io::load_config;
use super::super::config_model::ProductConfig;
use super::super::paths::config_file;
use super::get_table::config_entries;
use crate::error::{CliError, ErrorKind};

/// Read one key, falling back to the promoted policy knobs.
///
/// A linear scan over roughly a hundred entries: this is a one-shot CLI
/// answering one question, and a map would cost a build of the whole table to
/// save a scan of it.
fn get_one(cfg: &ProductConfig, key: &str) -> Result<Value, CliError> {
    let value = match config_entries(cfg)
        .into_iter()
        .find(|(name, _)| *name == key)
    {
        Some((_, value)) => value,
        None => match crate::xdg::policy::policy_stored(&cfg.policy, key) {
            Some(stored) => json!(stored),
            None => {
                return Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    format!("unknown config key: {key}"),
                    crate::i18n::suggestion_key("config_list_keys", None),
                ));
            }
        },
    };
    Ok(json!({ "key": key, "value": value }))
}

/// Get one config key (or full dump when key is empty).
///
/// # Errors
///
/// [`crate::error::ErrorKind::Usage`] for an unknown key, or an I/O error when
/// the config file cannot be read.
pub fn config_get(key: Option<&str>) -> Result<Value, CliError> {
    let cfg = load_config()?;
    match key {
        None | Some("") => full_dump(&cfg),
        Some(k) => get_one(&cfg, k),
    }
}

/// Full config dump: every hand-written key plus every promoted policy override.
///
/// Built with explicit `insert` calls rather than one wide `json!` literal:
/// the macro hits its recursion limit as keys accumulate, and every team that
/// added a key kept re-discovering that failure. Insertion has no such ceiling.
fn full_dump(cfg: &ProductConfig) -> Result<Value, CliError> {
    let mut map = serde_json::Map::new();
    // Agent-native CLEAN STDOUT: omit keys whose value is JSON null.
    let mut put = |key: &str, value: Value| {
        if !value.is_null() {
            map.insert(key.to_string(), value);
        }
    };

    for (key, value) in config_entries(cfg) {
        put(key, value);
    }
    for name in crate::xdg::policy::POLICY_KEYS {
        let stored = crate::xdg::policy::policy_stored(&cfg.policy, name).flatten();
        put(name, json!(stored));
    }
    // Dump-only, and derived rather than stored: there is no `path` key to
    // set, so it has no row in the table.
    put("path", json!(config_file()?.display().to_string()));

    Ok(Value::Object(map))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unknown_key_is_a_usage_error_not_a_null() {
        // Returning null would be indistinguishable from a key that exists and
        // is unset, which is the difference the caller is asking about.
        let err = get_one(&ProductConfig::default(), "no_such_key_at_all")
            .expect_err("unknown key must fail");
        assert_eq!(err.kind(), ErrorKind::Usage);
    }

    #[test]
    fn the_dump_and_the_single_key_path_agree() {
        // The regression this guards: two hand-maintained lists that drift, so
        // `config get <key>` answers a key the full dump never mentions.
        let cfg = ProductConfig::default();
        let dump = full_dump(&cfg).expect("dump");
        let object = dump.as_object().expect("object");
        for (key, value) in config_entries(&cfg) {
            if value.is_null() {
                // Omitted from the dump by the CLEAN STDOUT rule, but the
                // single-key path must still recognise it as a known key.
                assert!(get_one(&cfg, key).is_ok(), "{key} is not gettable");
                continue;
            }
            assert!(object.contains_key(key), "{key} missing from the full dump");
        }
    }

    #[test]
    fn the_dump_reports_the_config_path() {
        let dump = full_dump(&ProductConfig::default()).expect("dump");
        assert!(dump.get("path").is_some());
    }
}
