// SPDX-License-Identifier: MIT OR Apache-2.0
//! One envelope shape for `scrape`, whatever `--format` was asked for.
//!
//! # The defect this closes
//!
//! `scrape` used to answer in two different shapes depending on how many values
//! `--format` carried. Measured on 2026-08-10 against the same URL:
//!
//! - `--format markdown` returned twenty top-level keys: the content plus the
//!   whole diagnosis — `status_code`, `http_error`, `cache_hit`, `robots_policy`,
//!   `charset`, `http_version`, `stealth`, `tls_impersonation`, `http2_profile`,
//!   `header_order_controlled`, `change_status`.
//! - `--format markdown,links` returned four: `engine`, `format_list`,
//!   `formats`, `source_url`. The content moved under `formats` and every
//!   diagnostic field was gone.
//!
//! Two consequences, both bad for a caller that cannot see the source.
//!
//! Asking for MORE data returned LESS: a caller that added a second format
//! silently lost the fields it used to branch on, including whether the request
//! was even served (`status_code`) and whether robots was honoured.
//!
//! And `--fields markdown` worked in the first case and returned an empty
//! `data` with `ok: true`, no `agent_ops`, and exit 0 in the second. A silent
//! wrong answer, which is the failure class this repository exists to hunt.
//!
//! # Why the shape is the union rather than the smaller of the two
//!
//! The product's own rule says arity must never carry semantics: a token cannot
//! change role because another flag value appeared beside it. `formats` is
//! always present, holding one entry when one format was asked for. Each format
//! is ALSO promoted to the top level, so the single-format spelling that
//! existing callers already parse keeps working — and starts working in the
//! multi-format case, where it never did.

use serde_json::{Map, Value};

/// Merge the derived formats into the base envelope, keeping one shape.
///
/// `base` is whatever the transport reported, diagnosis included. `formats_out`
/// is the per-format content map. Every format is written under `formats` and
/// mirrored at the top level.
///
/// # Why the mirror does not overwrite
///
/// A transport field wins over a derived one of the same name. The single-format
/// path already puts `html` at the top level as the fetched body; a derived
/// `html` entry describes the same bytes, so overwriting would be a no-op at
/// best and a re-render at worst. Preferring the transport keeps the field
/// meaning "what came back on the wire".
pub fn unify_scrape_shape(
    base: Value,
    formats_out: Map<String, Value>,
    format_list: &[&str],
) -> Value {
    let mut obj = match base {
        Value::Object(m) => m,
        // A transport that answered with a non-object has nothing to merge
        // into; wrapping it would invent a shape the caller cannot rely on.
        other => return other,
    };

    mirror_formats_to_top_level(&mut obj, &formats_out);
    obj.insert("formats".into(), Value::Object(formats_out));
    obj.insert("format_list".into(), Value::from(format_list.to_vec()));
    Value::Object(obj)
}

/// Give a single-format envelope the same `formats` / `format_list` pair.
///
/// The single-format path never built a formats map: it asked the transport for
/// one representation and returned it at the top level. That is half of the
/// shape — enough for a caller that always asks for one format, and a trap for
/// one that sometimes asks for two, because the reading code has to branch on
/// something it cannot see in the data.
///
/// The content is not duplicated blindly: `formats` points at whichever
/// top-level key the format produced, so `formats.markdown` and `markdown` are
/// the same bytes under both spellings.
pub fn unify_single_format_shape(base: Value, format_name: &str) -> Value {
    let mut obj = match base {
        Value::Object(m) => m,
        other => return other,
    };

    let mut formats = Map::new();
    if let Some(v) = obj.get(format_name) {
        formats.insert(format_name.to_string(), v.clone());
    }
    obj.insert("formats".into(), Value::Object(formats));
    obj.insert("format_list".into(), Value::from(vec![format_name]));
    Value::Object(obj)
}

/// Copy each derived format to the top level, never clobbering the transport.
fn mirror_formats_to_top_level(obj: &mut Map<String, Value>, formats: &Map<String, Value>) {
    for (name, value) in formats {
        if !obj.contains_key(name) {
            obj.insert(name.clone(), value.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Unwrap a literal into the map shape `build_formats_map` returns.
    fn obj(v: Value) -> Map<String, Value> {
        match v {
            Value::Object(m) => m,
            other => panic!("test fixture must be an object, got {other}"),
        }
    }

    #[test]
    fn diagnosis_survives_the_merge() {
        let base = json!({"source_url": "https://e.test", "status_code": 200, "stealth": true});
        let out = unify_scrape_shape(base, obj(json!({"markdown": "# t"})), &["markdown"]);
        assert_eq!(out["status_code"], 200, "transport diagnosis must survive");
        assert_eq!(out["stealth"], true);
        assert_eq!(out["source_url"], "https://e.test");
    }

    #[test]
    fn every_format_is_reachable_both_ways() {
        let out = unify_scrape_shape(
            json!({"source_url": "https://e.test"}),
            obj(json!({"markdown": "# t", "links": ["https://a.test"]})),
            &["markdown", "links"],
        );
        // Under `formats`, which is where a multi-format caller looks.
        assert_eq!(out["formats"]["markdown"], "# t");
        // And at the top level, which is what `--fields markdown` projects.
        assert_eq!(out["markdown"], "# t");
        assert_eq!(out["links"][0], "https://a.test");
        assert_eq!(out["format_list"], json!(["markdown", "links"]));
    }

    #[test]
    fn transport_field_wins_over_derived_field() {
        let out = unify_scrape_shape(
            json!({"html": "<from-wire>"}),
            obj(json!({"html": "<derived>"})),
            &["html"],
        );
        assert_eq!(
            out["html"], "<from-wire>",
            "top level must keep meaning 'what came back on the wire'"
        );
        assert_eq!(
            out["formats"]["html"], "<derived>",
            "the derived value stays reachable under formats"
        );
    }

    #[test]
    fn one_format_and_many_produce_the_same_key_set_shape() {
        let one = unify_scrape_shape(
            json!({"source_url": "u", "status_code": 200}),
            obj(json!({"markdown": "m"})),
            &["markdown"],
        );
        let many = unify_scrape_shape(
            json!({"source_url": "u", "status_code": 200}),
            obj(json!({"markdown": "m", "links": []})),
            &["markdown", "links"],
        );
        for key in [
            "source_url",
            "status_code",
            "formats",
            "format_list",
            "markdown",
        ] {
            assert!(one.get(key).is_some(), "single-format lost {key}");
            assert!(many.get(key).is_some(), "multi-format lost {key}");
        }
    }

    #[test]
    fn non_object_base_is_returned_untouched() {
        let out = unify_scrape_shape(json!("plain"), obj(json!({"markdown": "m"})), &["markdown"]);
        assert_eq!(out, json!("plain"));
    }
}
