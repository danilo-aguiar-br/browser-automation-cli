// SPDX-License-Identifier: MIT OR Apache-2.0
//! JSON Schema fragments per command (`schema` CLI).
//!
//! Split by command family (SRP); [`schema_for`] or-chains submodules.
use serde_json::{json, Value};

mod browser_nav;
mod core;
pub(crate) mod derive;
mod ops_tools;
pub(crate) mod output;
mod run_exec;
mod scrape_tools;

/// Build a JSON Schema object fragment (shared helper).
// `needless_pass_by_value` false positive: the value IS consumed, moved into the
// `json!` object below. Macro expansion hides the move from the lint.
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn schema_object(description: &str, properties: Value, required: &[&str]) -> Value {
    json!({
        "type": "object",
        "description": description,
        "properties": properties,
        "required": required,
        "additionalProperties": false,
    })
}

/// Hand-written catalog fragment for a command, or `None` if none is declared.
///
/// The catalog carries what the parser cannot know: NDJSON-only aliases
/// (`dx`/`dy`), tool-ref key spellings, and prose for step-only commands.
pub(crate) fn catalog_schema_for(cmd: &str) -> Option<Value> {
    core::schema_for(cmd)
        .or_else(|| browser_nav::schema_for(cmd))
        .or_else(|| run_exec::schema_for(cmd))
        .or_else(|| scrape_tools::schema_for(cmd))
        .or_else(|| ops_tools::schema_for(cmd))
}

/// Resolve the effective schema: clap projection merged over the catalog.
///
/// Precedence is parser-first. A property present in both keeps the parser's
/// type, argv spelling and surface markers; catalog-only properties are kept
/// and marked `source: "catalog"` so agents can tell them apart.
pub(crate) fn schema_for(cmd: &str) -> Option<Value> {
    let catalog = catalog_schema_for(cmd);
    let derived = derive::derive_command(cmd);
    match (catalog, derived) {
        (None, None) => None,
        (Some(catalog), None) => Some(mark_catalog_only(catalog, cmd, false)),
        (None, Some(d)) => Some(schema_object_from_derived(&d, cmd)),
        (Some(catalog), Some(d)) => Some(merge(catalog, &d, cmd)),
    }
}

/// Schema object built purely from the clap projection.
fn schema_object_from_derived(d: &derive::DerivedCommand, cmd: &str) -> Value {
    let required: Vec<&str> = d.required.iter().map(String::as_str).collect();
    let mut out = schema_object(&d.about, d.properties.clone(), &required);
    if let Some(obj) = out.as_object_mut() {
        obj.insert("surfaces".into(), json!(derive::surfaces_for(cmd)));
    }
    out
}

/// Tag every catalog property with its origin and the command surfaces.
fn mark_catalog_only(mut catalog: Value, cmd: &str, has_parser: bool) -> Value {
    let surfaces = derive::surfaces_for(cmd);
    if let Some(props) = catalog
        .get_mut("properties")
        .and_then(|p| p.as_object_mut())
    {
        for (key, value) in props.iter_mut() {
            annotate_catalog_property(key, value, &surfaces, has_parser);
        }
    }
    if let Some(obj) = catalog.as_object_mut() {
        obj.insert("surfaces".into(), json!(surfaces));
    }
    catalog
}

/// Add `source` / `step_key` / `surfaces` to one catalog property in place.
///
/// `has_parser` drops `argv` from catalog-only properties: when the parser does
/// expose this command, anything the parser did not declare is a step-only key
/// (NDJSON alias or tool-ref spelling) and is not typeable as a flag.
fn annotate_catalog_property(
    key: &str,
    value: &mut Value,
    surfaces: &[&'static str],
    has_parser: bool,
) {
    let Some(obj) = value.as_object_mut() else {
        return;
    };
    let effective: Vec<&&str> = if has_parser {
        surfaces
            .iter()
            .filter(|s| **s != derive::SURFACE_ARGV)
            .collect()
    } else {
        surfaces.iter().collect()
    };
    obj.entry("source").or_insert(json!("catalog"));
    obj.entry("step_key").or_insert(json!(key));
    obj.entry("surfaces").or_insert(json!(effective));
}

/// True when a derived property carries no help text.
fn derived_description_is_empty(value: &Value) -> bool {
    value
        .get("description")
        .and_then(|d| d.as_str())
        .map(str::is_empty)
        .unwrap_or(true)
}

/// Non-empty description already present on a catalog property.
fn catalog_description(value: Option<&Value>) -> Option<String> {
    value
        .and_then(|v| v.get("description"))
        .and_then(|d| d.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Merge the clap projection over a catalog fragment (parser wins per property).
fn merge(catalog: Value, d: &derive::DerivedCommand, cmd: &str) -> Value {
    let surfaces = derive::surfaces_for(cmd);
    let mut merged = mark_catalog_only(catalog, cmd, true);
    let description = if d.about.is_empty() {
        merged
            .get("description")
            .cloned()
            .unwrap_or_else(|| json!(""))
    } else {
        json!(d.about)
    };
    let derived_props = d.properties.as_object().cloned().unwrap_or_default();
    if let Some(props) = merged.get_mut("properties").and_then(|p| p.as_object_mut()) {
        for (key, mut value) in derived_props {
            // A clap arg without help text must not blank the catalog prose.
            if derived_description_is_empty(&value) {
                if let Some(prev) = catalog_description(props.get(&key)) {
                    if let Some(obj) = value.as_object_mut() {
                        obj.insert("description".into(), json!(prev));
                    }
                }
            }
            props.insert(key, value);
        }
    }
    // Required is parser truth; catalog `required` entries for step-only keys
    // are preserved so NDJSON callers keep their contract.
    let mut required: Vec<String> = merged
        .get("required")
        .and_then(|r| r.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    for key in &d.required {
        if !required.contains(key) {
            required.push(key.clone());
        }
    }
    if let Some(obj) = merged.as_object_mut() {
        obj.insert("description".into(), description);
        obj.insert("required".into(), json!(required));
        obj.insert("surfaces".into(), json!(surfaces));
    }
    merged
}
