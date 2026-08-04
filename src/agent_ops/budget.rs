// SPDX-License-Identifier: MIT OR Apache-2.0
//! Output budgets: string truncation and a hard ceiling on emitted bytes.

use serde_json::Value;

/// Truncate every string longer than `max` characters, in place.
///
/// Returns `true` when anything was cut, so the caller can say so in the
/// envelope. Silent truncation is the failure mode this whole module exists to
/// avoid: an agent that cannot tell a short page from a cut page will happily
/// conclude the page was short.
///
/// Counts characters, not bytes, and cuts on a character boundary — slicing a
/// byte range out of UTF-8 would panic on the first accented word.
pub fn truncate_strings(value: &mut Value, max: usize) -> bool {
    match value {
        Value::String(s) => {
            if s.chars().count() > max {
                *s = s.chars().take(max).collect();
                true
            } else {
                false
            }
        }
        Value::Array(items) => {
            let mut hit = false;
            for item in items {
                hit |= truncate_strings(item, max);
            }
            hit
        }
        Value::Object(map) => {
            let mut hit = false;
            for (_, v) in map.iter_mut() {
                hit |= truncate_strings(v, max);
            }
            hit
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => false,
    }
}

/// Outcome of enforcing a byte ceiling.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BudgetOutcome {
    /// Rows dropped from the end to fit.
    pub omitted_rows: usize,
    /// True when anything was dropped.
    pub truncated: bool,
    /// True when the payload is still over budget with zero rows left.
    pub still_over: bool,
}

/// Drop rows from the end until the serialized payload fits `max_bytes`.
///
/// Rows go last-first because a list an agent asked to be sorted or limited has
/// its most relevant entries at the front; discarding from the head would throw
/// away exactly what the flags were used to surface.
///
/// When no rows are left and the payload still does not fit, this reports
/// `still_over` rather than mangling the object. A half-serialized envelope is
/// not smaller in any useful sense — it is unparseable, which costs the agent a
/// retry instead of saving it bytes.
pub fn fit_rows_to_budget(
    rows: &mut Vec<Value>,
    max_bytes: usize,
    overhead: usize,
) -> BudgetOutcome {
    let mut out = BudgetOutcome::default();
    loop {
        let size = serialized_len(rows) + overhead;
        if size <= max_bytes {
            return out;
        }
        if rows.pop().is_none() {
            out.still_over = true;
            out.truncated = true;
            return out;
        }
        out.omitted_rows += 1;
        out.truncated = true;
    }
}

/// Compact serialized length of `value`, or `usize::MAX` when it cannot serialize.
///
/// A value that will not serialize is treated as infinitely large so the caller
/// keeps shrinking instead of concluding it fits.
#[must_use]
pub fn serialized_len<T: serde::Serialize>(value: &T) -> usize {
    serde_json::to_vec(value).map_or(usize::MAX, |v| v.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn truncate_cuts_only_long_strings() {
        let mut v = json!({"a": "abcdef", "b": "xy", "n": 5});
        assert!(truncate_strings(&mut v, 3));
        assert_eq!(v, json!({"a": "abc", "b": "xy", "n": 5}));
    }

    #[test]
    fn truncate_walks_arrays_and_nested_objects() {
        let mut v = json!({"rows": [{"t": "aaaa"}, {"t": "b"}]});
        assert!(truncate_strings(&mut v, 2));
        assert_eq!(v, json!({"rows": [{"t": "aa"}, {"t": "b"}]}));
    }

    #[test]
    fn truncate_reports_false_when_nothing_was_cut() {
        let mut v = json!({"a": "ab"});
        assert!(!truncate_strings(&mut v, 5));
    }

    /// Byte slicing would panic here; character slicing must not.
    #[test]
    fn truncate_cuts_on_a_character_boundary() {
        let mut v = json!({"a": "ação não"});
        assert!(truncate_strings(&mut v, 4));
        assert_eq!(v, json!({"a": "ação"}));
    }

    #[test]
    fn budget_drops_rows_from_the_end() {
        let mut rows: Vec<Value> = (0..10)
            .map(|i| json!({"i": i, "pad": "xxxxxxxxxx"}))
            .collect();
        let out = fit_rows_to_budget(&mut rows, 120, 0);
        assert!(out.truncated && out.omitted_rows > 0);
        assert!(!out.still_over);
        assert_eq!(
            rows[0]["i"],
            json!(0),
            "the head is what the agent asked for"
        );
    }

    #[test]
    fn budget_reports_still_over_instead_of_mangling() {
        let mut rows: Vec<Value> = vec![];
        let out = fit_rows_to_budget(&mut rows, 1, 500);
        assert!(out.still_over, "overhead alone exceeds the ceiling");
    }

    #[test]
    fn budget_is_a_no_op_when_it_already_fits() {
        let mut rows = vec![json!({"i": 1})];
        let out = fit_rows_to_budget(&mut rows, 10_000, 0);
        assert_eq!(out, BudgetOutcome::default());
        assert_eq!(rows.len(), 1);
    }
}
