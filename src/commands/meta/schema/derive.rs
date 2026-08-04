// SPDX-License-Identifier: MIT OR Apache-2.0
//! Schema derived from the live clap parser (GAP-013 / GAP-014).
//!
//! # Why derive
//!
//! A hand-written schema drifts from the parser the moment a flag is added.
//! This module walks [`crate::cli::Cli::command()`] so the emitted schema is a
//! projection of the parser itself: adding a clap arg changes `schema <cmd>`
//! with no second edit.
//!
//! # Name mapping
//!
//! | Surface | Source | Example |
//! |---------|--------|---------|
//! | argv flag | [`clap::Arg::get_long`] | `--delta-y` |
//! | NDJSON step key | [`clap::Arg::get_id`] | `delta_y` |
//! | positional argv | [`clap::Arg::get_id`] upper-cased | `<TARGET>` |
//!
//! # Type inference
//!
//! JSON type comes from [`clap::Arg::get_action`] first (flags are boolean,
//! `Append` is array, `Count` is integer) and falls back to the arg's value
//! parser type id for `Set` (number vs string).

#[cfg(test)]
use std::collections::BTreeSet;

use clap::builder::ValueParser;
use clap::{Arg, ArgAction, Command, CommandFactory};
use serde_json::{json, Map, Value};

use crate::cli::Cli;
use crate::commands::run::RUN_DISPATCHED_CMDS;

/// Surface tokens an agent can use to supply a property.
pub(crate) const SURFACE_ARGV: &str = "argv";
/// Multi-step `run --script` NDJSON step surface.
pub(crate) const SURFACE_RUN_STEP: &str = "run_step";
/// Single-step `exec` argv surface.
pub(crate) const SURFACE_EXEC: &str = "exec";

/// Schema projected from one clap subcommand.
pub(crate) struct DerivedCommand {
    /// `about` text from the clap subcommand.
    pub about: String,
    /// JSON Schema `properties` object.
    pub properties: Value,
    /// Property names clap marks required.
    pub required: Vec<String>,
}

/// Surfaces a command is reachable on.
///
/// `argv` is claimed only when the parser really exposes a top-level
/// subcommand: step-only commands such as `select-option` are reachable from
/// `run` and `exec` but cannot be typed as `browser-automation-cli select-option`.
pub(crate) fn surfaces_for(cmd: &str) -> Vec<&'static str> {
    surfaces_in(&Cli::command(), cmd)
}

/// [`surfaces_for`] against an already built tree (one build per caller).
fn surfaces_in(root: &Command, cmd: &str) -> Vec<&'static str> {
    let mut out = Vec::new();
    if find_subcommand(root, cmd).is_some() {
        out.push(SURFACE_ARGV);
    }
    if RUN_DISPATCHED_CMDS.contains(&cmd) {
        out.push(SURFACE_RUN_STEP);
        out.push(SURFACE_EXEC);
    }
    out
}

/// Locate a top-level subcommand by its CLI name.
///
/// Borrows out of `root` instead of cloning: `Command` is a large struct and a
/// clone landed a full copy in the caller's frame on every lookup.
fn find_subcommand<'tree>(root: &'tree Command, cmd: &str) -> Option<&'tree Command> {
    root.get_subcommands().find(|s| s.get_name() == cmd)
}

/// True for clap's auto-generated `help` / `version` args.
fn is_builtin(arg: &Arg) -> bool {
    matches!(arg.get_id().as_str(), "help" | "version") || arg.is_hide_set()
}

/// JSON Schema type token for one clap arg.
fn json_type(arg: &Arg) -> &'static str {
    match arg.get_action() {
        ArgAction::SetTrue | ArgAction::SetFalse => "boolean",
        ArgAction::Count => "integer",
        ArgAction::Append => "array",
        _ => scalar_type(arg),
    }
}

/// Scalar type for `ArgAction::Set` args, read from the value parser type id.
///
/// `AnyValueId` is not re-exported by clap, so the comparison is made against
/// parsers built here for the same Rust types.
fn scalar_type(arg: &Arg) -> &'static str {
    let id = arg.get_value_parser().type_id();
    // `AnyValueId` is not re-exported by clap, so the probe values come from
    // parsers built here for known Rust types. The probe is a bare
    // `ValueParser` (not a whole `Arg`) and lives in a closure, so the eleven
    // comparisons below reuse one small slot instead of stacking eleven `Arg`
    // temporaries in this frame.
    let is_type = |probe: ValueParser| probe.type_id() == id;
    // `value_parser!(bool)` already yields a `ValueParser`; the numeric ones
    // yield typed parsers that still need the conversion.
    if is_type(clap::value_parser!(bool)) {
        "boolean"
    } else if is_type(clap::value_parser!(f64).into()) || is_type(clap::value_parser!(f32).into()) {
        "number"
    } else if is_type(clap::value_parser!(u64).into())
        || is_type(clap::value_parser!(i64).into())
        || is_type(clap::value_parser!(u32).into())
        || is_type(clap::value_parser!(i32).into())
        || is_type(clap::value_parser!(usize).into())
        || is_type(clap::value_parser!(u8).into())
    {
        "integer"
    } else {
        "string"
    }
}

/// Argv spelling: `--long` for flags, `<ID>` for positionals.
fn argv_name(arg: &Arg) -> String {
    match arg.get_long() {
        Some(long) => format!("--{long}"),
        None => format!("<{}>", arg.get_id().as_str().to_ascii_uppercase()),
    }
}

/// Enum values declared by a clap `ValueEnum` or explicit possible values.
fn possible_values(arg: &Arg) -> Option<Value> {
    let values: Vec<String> = arg
        .get_possible_values()
        .iter()
        .map(|p| p.get_name().to_string())
        .collect();
    if values.is_empty() {
        None
    } else {
        Some(json!(values))
    }
}

/// Default value rendered as a JSON scalar matching the declared type.
fn default_value(arg: &Arg, ty: &str) -> Option<Value> {
    let raw = arg
        .get_default_values()
        .first()?
        .to_string_lossy()
        .into_owned();
    Some(match ty {
        "boolean" => json!(raw == "true"),
        "integer" => raw.parse::<i64>().map(Value::from).unwrap_or(json!(raw)),
        "number" => raw.parse::<f64>().map(Value::from).unwrap_or(json!(raw)),
        _ => json!(raw),
    })
}

/// Build the schema property object for one clap arg.
fn property_for(arg: &Arg, surfaces: &[&'static str]) -> (String, Value) {
    let step_key = arg.get_id().as_str().to_string();
    let ty = json_type(arg);
    let mut obj = Map::new();
    obj.insert("type".into(), json!(ty));
    if ty == "array" {
        obj.insert("items".into(), json!({ "type": scalar_type(arg) }));
    }
    let description = arg
        .get_help()
        .or_else(|| arg.get_long_help())
        .map(|h| h.to_string())
        .unwrap_or_default();
    obj.insert("description".into(), json!(description));
    obj.insert("argv".into(), json!(argv_name(arg)));
    obj.insert("step_key".into(), json!(step_key));
    obj.insert("surfaces".into(), json!(surfaces));
    obj.insert("source".into(), json!("parser"));
    obj.insert("required".into(), json!(arg.is_required_set()));
    // clap reports `true`/`false` possible values for flags; that is noise
    // once the property is already typed as boolean.
    if ty != "boolean" {
        if let Some(values) = possible_values(arg) {
            obj.insert("enum".into(), values);
        }
    }
    if let Some(default) = default_value(arg, ty) {
        obj.insert("default".into(), default);
    }
    (step_key, Value::Object(obj))
}

/// Nested action property for subcommands that dispatch on an action word.
fn action_property(sub: &Command, surfaces: &[&'static str]) -> Option<(String, Value)> {
    let names: Vec<String> = sub
        .get_subcommands()
        .map(|s| s.get_name().to_string())
        .collect();
    if names.is_empty() {
        return None;
    }
    let actions: Map<String, Value> = sub
        .get_subcommands()
        .map(|s| {
            let mut props = Map::new();
            for arg in s.get_arguments().filter(|a| !is_builtin(a)) {
                let (key, value) = property_for(arg, surfaces);
                props.insert(key, value);
            }
            (
                s.get_name().to_string(),
                json!({
                    "description": s.get_about().map(|a| a.to_string()).unwrap_or_default(),
                    "properties": Value::Object(props),
                }),
            )
        })
        .collect();
    Some((
        "action".to_string(),
        json!({
            "type": "string",
            "description": "Subcommand action word",
            "argv": "<ACTION>",
            "step_key": "action",
            "surfaces": surfaces,
            "source": "parser",
            "required": sub.is_subcommand_required_set(),
            "enum": names,
            "actions": Value::Object(actions),
        }),
    ))
}

/// Project one CLI command into schema properties, or `None` when the command
/// has no clap subcommand (step-only surfaces such as `select-option`).
pub(crate) fn derive_command(cmd: &str) -> Option<DerivedCommand> {
    let root = Cli::command();
    let sub = find_subcommand(&root, cmd)?;
    let surfaces = surfaces_in(&root, cmd);
    let mut properties = Map::new();
    let mut required = Vec::new();

    for arg in sub.get_arguments().filter(|a| !is_builtin(a)) {
        if arg.is_required_set() {
            required.push(arg.get_id().as_str().to_string());
        }
        let (key, value) = property_for(arg, &surfaces);
        properties.insert(key, value);
    }
    if let Some((key, value)) = action_property(sub, &surfaces) {
        if sub.is_subcommand_required_set() {
            required.push(key.clone());
        }
        properties.insert(key, value);
    }

    Some(DerivedCommand {
        about: sub.get_about().map(|a| a.to_string()).unwrap_or_default(),
        properties: Value::Object(properties),
        required,
    })
}

/// Every top-level clap subcommand name (parser truth for conformance tests).
#[cfg(test)]
pub(crate) fn parser_command_names() -> BTreeSet<String> {
    Cli::command()
        .get_subcommands()
        .map(|s| s.get_name().to_string())
        .collect()
}

/// Argv long flags declared by one clap subcommand (conformance helper).
#[cfg(test)]
pub(crate) fn parser_arg_keys(cmd: &str) -> Option<BTreeSet<String>> {
    let root = Cli::command();
    let sub = find_subcommand(&root, cmd)?;
    Some(
        sub.get_arguments()
            .filter(|a| !is_builtin(a))
            .map(|a| a.get_id().as_str().to_string())
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_scroll_with_include_snapshot() {
        crate::cli::on_clap_stack(|| {
            let d = derive_command("scroll").expect("scroll subcommand");
            let props = d.properties.as_object().expect("object");
            assert!(props.contains_key("include_snapshot"), "{props:?}");
            assert_eq!(props["include_snapshot"]["type"], json!("boolean"));
            assert_eq!(
                props["include_snapshot"]["argv"],
                json!("--include-snapshot")
            );
            assert_eq!(props["delta_y"]["type"], json!("number"));
        });
    }

    #[test]
    fn positional_args_keep_step_key_and_argv_form() {
        crate::cli::on_clap_stack(|| {
            let d = derive_command("goto").expect("goto subcommand");
            let props = d.properties.as_object().expect("object");
            assert_eq!(props["url"]["argv"], json!("<URL>"));
            assert_eq!(props["url"]["step_key"], json!("url"));
            assert!(d.required.contains(&"url".to_string()));
        });
    }

    #[test]
    fn surfaces_split_meta_from_step_commands() {
        crate::cli::on_clap_stack(|| {
            assert_eq!(surfaces_for("doctor"), vec![SURFACE_ARGV]);
            assert_eq!(
                surfaces_for("goto"),
                vec![SURFACE_ARGV, SURFACE_RUN_STEP, SURFACE_EXEC]
            );
        });
    }

    #[test]
    fn value_enum_args_expose_enum_values() {
        crate::cli::on_clap_stack(|| {
            let d = derive_command("grab").expect("grab subcommand");
            let props = d.properties.as_object().expect("object");
            let values = props["format"]["enum"].as_array().expect("enum");
            assert!(values.iter().any(|v| v == "png"), "{values:?}");
        });
    }

    #[test]
    fn subcommand_actions_are_projected() {
        crate::cli::on_clap_stack(|| {
            let d = derive_command("console").expect("console subcommand");
            let props = d.properties.as_object().expect("object");
            let action = &props["action"];
            let names = action["enum"].as_array().expect("enum");
            assert!(names.iter().any(|v| v == "list"), "{names:?}");
            assert!(action["actions"]["list"]["properties"]["page_idx"].is_object());
        });
    }

    #[test]
    fn step_only_command_has_no_parser_projection() {
        crate::cli::on_clap_stack(|| {
            assert!(derive_command("select-option").is_none());
        });
    }
}
