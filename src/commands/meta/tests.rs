// SPDX-License-Identifier: MIT OR Apache-2.0
//! Meta inventory/schema unit tests.
use super::inventory::{category_for, command_objects, COMMANDS, PARITY_DEFAULT_ON_REQUIRED};
use super::schema::derive::{parser_arg_keys, parser_command_names, surfaces_for};
use super::schema::schema_for;

#[test]
fn parity_default_on_subset_of_commands() {
    for req in PARITY_DEFAULT_ON_REQUIRED {
        assert!(
            COMMANDS.contains(req),
            "parity command missing from COMMANDS: {req}"
        );
    }
}

#[test]
fn commands_unique() {
    let mut seen = std::collections::BTreeSet::new();
    for c in COMMANDS {
        assert!(seen.insert(*c), "duplicate command: {c}");
    }
}

#[test]
fn config_schema_includes_list_keys_and_cache_keys() {
    let frag = schema_for("config").expect("config schema");
    let action_enum = frag
        .pointer("/properties/action/enum")
        .and_then(|v| v.as_array())
        .expect("action.enum");
    let actions: Vec<&str> = action_enum.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        actions.contains(&"list-keys"),
        "config action enum must include list-keys: {actions:?}"
    );
    for required in ["path", "init", "show", "set", "get", "list-keys"] {
        assert!(
            actions.contains(&required),
            "missing config action {required} in {actions:?}"
        );
    }
    let key_desc = frag
        .pointer("/properties/key/description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    for key in [
        "lang",
        "timeout",
        "artifacts_dir",
        "ignore_robots",
        "namespace",
        "encryption_key",
        "color",
        "log_level",
        "log_to_file",
        "chrome_path",
        "lighthouse_path",
        "lighthouse_timeout_secs",
        "ffmpeg_timeout_secs",
        "openrouter_api_key",
        "llm_base_url",
        "llm_model",
        "cache_backend",
        "cache_redis_url",
        "search_base_url",
    ] {
        assert!(
            key_desc.contains(key),
            "config key description missing {key}: {key_desc}"
        );
    }
}

#[test]
fn run_schema_documents_ndjson_and_array() {
    let frag = schema_for("run").expect("run schema");
    let desc = frag
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let script_desc = frag
        .pointer("/properties/script/description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        desc.to_ascii_lowercase().contains("array")
            || script_desc.to_ascii_lowercase().contains("array"),
        "run schema must document JSON array scripts: desc={desc} script={script_desc}"
    );
    assert!(
        desc.to_ascii_lowercase().contains("ndjson")
            || script_desc.to_ascii_lowercase().contains("ndjson")
            || script_desc.contains("jsonl"),
        "run schema must document NDJSON scripts: desc={desc} script={script_desc}"
    );
}

// ── Conformance: emitted schema vs live clap parser (GAP-013 / GAP-014) ──

#[test]
fn every_parser_subcommand_is_in_the_inventory() {
    for name in parser_command_names() {
        assert!(
            COMMANDS.contains(&name.as_str()),
            "clap subcommand missing from COMMANDS: {name}"
        );
    }
}

#[test]
fn every_parser_arg_appears_in_the_emitted_schema() {
    for name in parser_command_names() {
        let Some(keys) = parser_arg_keys(&name) else {
            continue;
        };
        let frag = schema_for(&name).unwrap_or_else(|| panic!("no schema for {name}"));
        let props = frag
            .get("properties")
            .and_then(|p| p.as_object())
            .unwrap_or_else(|| panic!("no properties for {name}"));
        for key in keys {
            assert!(
                props.contains_key(&key),
                "schema for `{name}` is missing parser arg `{key}`"
            );
        }
    }
}

#[test]
fn parser_derived_properties_carry_argv_and_step_keys() {
    for name in parser_command_names() {
        let frag = schema_for(&name).unwrap_or_else(|| panic!("no schema for {name}"));
        let props = frag
            .get("properties")
            .and_then(|p| p.as_object())
            .unwrap_or_else(|| panic!("no properties for {name}"));
        for (key, value) in props {
            if value.get("source").and_then(|s| s.as_str()) != Some("parser") {
                continue;
            }
            assert!(
                value.get("argv").and_then(|v| v.as_str()).is_some(),
                "{name}.{key} has no argv spelling"
            );
            assert_eq!(
                value.get("step_key").and_then(|v| v.as_str()),
                Some(key.as_str()),
                "{name}.{key} step_key mismatch"
            );
            assert!(
                value
                    .get("surfaces")
                    .and_then(|v| v.as_array())
                    .map(|a| !a.is_empty())
                    .unwrap_or(false),
                "{name}.{key} has no surface marker"
            );
        }
    }
}

#[test]
fn required_properties_match_the_parser() {
    for name in parser_command_names() {
        let Some(sub_required) = parser_arg_keys(&name) else {
            continue;
        };
        let frag = schema_for(&name).unwrap_or_else(|| panic!("no schema for {name}"));
        let required: Vec<&str> = frag
            .get("required")
            .and_then(|r| r.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();
        for key in required {
            // Required keys are either parser args or catalog step-only keys.
            let props = frag.get("properties").and_then(|p| p.as_object());
            let known =
                sub_required.contains(key) || props.map(|p| p.contains_key(key)).unwrap_or(false);
            assert!(known, "schema for `{name}` requires unknown key `{key}`");
        }
    }
}

#[test]
fn every_command_exposes_output_and_error_schema_surface() {
    // `schema <cmd>` payload assembly lives in handlers; here we assert the
    // pieces it composes exist for every command.
    for cmd in COMMANDS {
        let success = super::schema::output::success_envelope_schema(
            super::schema::output::data_schema_for(cmd),
        );
        assert_eq!(success["type"], serde_json::json!("object"), "{cmd}");
        assert!(success["properties"]["data"].is_object(), "{cmd}");
    }
    let err = super::schema::output::error_envelope_schema();
    assert!(err["properties"]["error"]["properties"]["kind"]["enum"].is_array());
}

// ── Inventory objects (GAP-017 / GAP-018) ──────────────────────────────

#[test]
fn every_command_has_a_category_and_surfaces() {
    for cmd in COMMANDS {
        assert_ne!(
            category_for(cmd),
            "other",
            "command `{cmd}` has no category in COMMAND_CATEGORIES"
        );
        assert!(
            !surfaces_for(cmd).is_empty(),
            "command `{cmd}` has no surfaces"
        );
    }
}

#[test]
fn command_objects_cover_every_command_with_a_description() {
    let objects = command_objects();
    assert_eq!(objects.len(), COMMANDS.len());
    for obj in objects {
        let name = obj["name"].as_str().expect("name");
        assert!(
            obj["description"]
                .as_str()
                .map(|d| !d.is_empty())
                .unwrap_or(false),
            "command `{name}` has an empty description"
        );
        assert!(obj["category"].is_string(), "{name}");
        assert!(obj["surfaces"].is_array(), "{name}");
    }
}

#[test]
fn include_snapshot_is_declared_for_type_select_option_and_scroll() {
    for cmd in ["type", "select-option", "scroll"] {
        let frag = schema_for(cmd).unwrap_or_else(|| panic!("no schema for {cmd}"));
        assert!(
            frag.pointer("/properties/include_snapshot").is_some(),
            "`{cmd}` must declare include_snapshot"
        );
    }
}
