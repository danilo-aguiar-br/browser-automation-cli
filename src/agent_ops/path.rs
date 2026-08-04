// SPDX-License-Identifier: MIT OR Apache-2.0
//! Dotted-path lookup and projection over `serde_json::Value`.

use serde_json::{Map, Value};

/// Resolve a dotted path such as `checks.residual_disk.status` inside `value`.
///
/// Returns `None` when any segment is missing. Array indices are not supported
/// on purpose: an agent that needs element three of a list wants `--limit`, and
/// positional indexing into a list whose order it did not choose is a bug
/// waiting to happen.
#[must_use]
pub fn get_path<'v>(value: &'v Value, path: &str) -> Option<&'v Value> {
    let mut cur = value;
    for seg in path.split('.') {
        cur = cur.get(seg)?;
    }
    Some(cur)
}

/// Render a scalar for comparison and sorting.
///
/// Objects and arrays deliberately have no rendering: comparing them as text
/// would silently make `--filter` and `--sort` answer questions about JSON
/// formatting rather than about data.
#[must_use]
pub fn scalar_text(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Null => Some(String::new()),
        Value::Array(_) | Value::Object(_) => None,
    }
}

/// Project `value` down to `paths`, rebuilding the nesting each path implies.
///
/// `--fields a.b,c` over `{"a":{"b":1,"z":2},"c":3,"d":4}` yields
/// `{"a":{"b":1},"c":3}`: the shape the agent already knows how to read, minus
/// everything it did not ask for. Flattening to `{"a.b":1}` would have been
/// cheaper to implement and would have forced every caller to learn a second
/// shape for the same data.
///
/// Returns the projection and the paths that did not resolve. Dropping a
/// missing path silently is what let `--fields typo` answer `{}` with exit 0,
/// which an agent reads as "the field is empty" rather than "you misspelled it".
#[must_use]
pub fn project(value: &Value, paths: &[String]) -> (Value, Vec<String>) {
    let mut out = Value::Object(Map::new());
    let mut unresolved = Vec::new();
    for path in paths {
        if let Some(found) = get_path(value, path) {
            insert_path(&mut out, path, found.clone());
        } else {
            unresolved.push(path.clone());
        }
    }
    (out, unresolved)
}

/// Insert `leaf` at `path`, creating intermediate objects as needed.
fn insert_path(root: &mut Value, path: &str, leaf: Value) {
    let segments: Vec<&str> = path.split('.').collect();
    let mut cur = root;
    for (i, seg) in segments.iter().enumerate() {
        let last = i + 1 == segments.len();
        // A non-object on the way down means two selected paths disagree about
        // the shape (`--fields a,a.b`). The wider one already carries the
        // narrower, so keeping it and skipping is the answer that loses nothing.
        let Some(map) = cur.as_object_mut() else {
            return;
        };
        if last {
            map.insert((*seg).to_string(), leaf);
            return;
        }
        cur = map
            .entry((*seg).to_string())
            .or_insert_with(|| Value::Object(Map::new()));
    }
}

/// Split a comma-separated flag value into trimmed, non-empty parts.
#[must_use]
pub fn split_csv(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn get_path_walks_nested_objects() {
        let v = json!({"a": {"b": {"c": 7}}});
        assert_eq!(get_path(&v, "a.b.c"), Some(&json!(7)));
        assert_eq!(get_path(&v, "a.b.missing"), None);
        assert_eq!(get_path(&v, "nope"), None);
    }

    #[test]
    fn project_rebuilds_the_nesting_it_selected() {
        let v = json!({"a": {"b": 1, "z": 2}, "c": 3, "d": 4});
        let (out, unresolved) = project(&v, &["a.b".into(), "c".into()]);
        assert_eq!(out, json!({"a": {"b": 1}, "c": 3}));
        assert!(unresolved.is_empty(), "every path resolved");
    }

    #[test]
    fn project_names_the_paths_that_do_not_exist() {
        let v = json!({"a": 1});
        let (out, unresolved) = project(&v, &["missing".into()]);
        assert_eq!(out, json!({}));
        // The empty projection alone is indistinguishable from an empty field.
        assert_eq!(unresolved, vec!["missing".to_string()]);
    }

    #[test]
    fn project_separates_the_resolved_from_the_missing() {
        let v = json!({"a": 1, "b": 2});
        let (out, unresolved) = project(&v, &["a".into(), "typo".into()]);
        assert_eq!(out, json!({"a": 1}));
        assert_eq!(unresolved, vec!["typo".to_string()]);
    }

    #[test]
    fn overlapping_paths_keep_the_wider_one() {
        let v = json!({"a": {"b": 1}});
        // `a` is selected whole, so `a.b` adds nothing and must not corrupt it.
        let (out, unresolved) = project(&v, &["a".into(), "a.b".into()]);
        assert_eq!(out, json!({"a": {"b": 1}}));
        assert!(unresolved.is_empty(), "both paths resolve");
    }

    #[test]
    fn scalar_text_refuses_containers() {
        assert_eq!(scalar_text(&json!("x")).as_deref(), Some("x"));
        assert_eq!(scalar_text(&json!(3)).as_deref(), Some("3"));
        assert_eq!(scalar_text(&json!(true)).as_deref(), Some("true"));
        assert_eq!(scalar_text(&json!([1])), None);
        assert_eq!(scalar_text(&json!({"a": 1})), None);
    }

    #[test]
    fn split_csv_drops_blanks_and_trims() {
        assert_eq!(
            split_csv(" a , ,b "),
            vec!["a".to_string(), "b".to_string()]
        );
    }
}
