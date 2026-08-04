// SPDX-License-Identifier: MIT OR Apache-2.0
//! Row filtering, sorting and deduplication for the agent envelope.

use serde_json::Value;

use super::path::{get_path, scalar_text};
use crate::error::{CliError, ErrorKind};

/// One `key<op>value` term. Multiple terms are ANDed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterExpr {
    /// Dotted path evaluated against each row.
    pub path: String,
    /// Comparison to apply.
    pub op: FilterOp,
    /// Right-hand side, compared as text.
    pub value: String,
}

/// Supported comparisons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterOp {
    /// `key=value` — exact match on the rendered scalar.
    Eq,
    /// `key!=value` — negated exact match.
    Ne,
    /// `key~value` — case-insensitive substring.
    Contains,
}

impl FilterExpr {
    /// Parse one term.
    ///
    /// `!=` is tested before `=` because `a!=b` also contains `=`, and testing in
    /// the other order would parse the path as `a!` and never report the mistake.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Usage`] when no operator is present or the path is empty.
    pub fn parse(raw: &str) -> Result<Self, CliError> {
        for (sep, op) in [
            ("!=", FilterOp::Ne),
            ("~", FilterOp::Contains),
            ("=", FilterOp::Eq),
        ] {
            if let Some(idx) = raw.find(sep) {
                let path = raw[..idx].trim().to_string();
                let value = raw[idx + sep.len()..].trim().to_string();
                if path.is_empty() {
                    return Err(usage(raw, "the left-hand side is empty"));
                }
                return Ok(Self { path, op, value });
            }
        }
        Err(usage(raw, "no operator found"))
    }

    /// True when `row` satisfies this term.
    ///
    /// A missing path never matches, including under `!=`. "This row has no such
    /// field" is not the same claim as "this row's field differs", and answering
    /// the second when asked the first is how a filter quietly returns rows the
    /// agent believed it had excluded.
    #[must_use]
    pub fn matches(&self, row: &Value) -> bool {
        let Some(text) = get_path(row, &self.path).and_then(scalar_text) else {
            return false;
        };
        match self.op {
            FilterOp::Eq => text == self.value,
            FilterOp::Ne => text != self.value,
            FilterOp::Contains => text.to_lowercase().contains(&self.value.to_lowercase()),
        }
    }
}

fn usage(raw: &str, why: &str) -> CliError {
    CliError::with_suggestion(
        ErrorKind::Usage,
        format!("--filter '{raw}': {why}"),
        crate::i18n::suggestion_key("agent_ops_filter_syntax", None),
    )
}

/// Keep the rows that satisfy every term.
#[must_use]
pub fn retain_matching(rows: Vec<Value>, terms: &[FilterExpr]) -> Vec<Value> {
    rows.into_iter()
        .filter(|row| terms.iter().all(|t| t.matches(row)))
        .collect()
}

/// Stable sort by a dotted path.
///
/// Numbers compare numerically and everything else compares as text, so
/// `--sort size` does not order 10 before 9. Rows whose path is missing or is a
/// container sort last, keeping their relative order: an unsortable row is not
/// an error, but it must not be allowed to claim first place either.
pub fn sort_rows(rows: &mut [Value], path: &str) {
    rows.sort_by(|a, b| {
        let ka = get_path(a, path);
        let kb = get_path(b, path);
        if let (Some(x), Some(y)) = (ka.and_then(as_number), kb.and_then(as_number)) {
            return x.total_cmp(&y);
        }
        match (ka.and_then(scalar_text), kb.and_then(scalar_text)) {
            (Some(x), Some(y)) => x.cmp(&y),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });
}

/// Count rows that actually carry `path` as a comparable scalar.
///
/// Sorting and deduplication are silent about a key nobody has: `sort_rows`
/// falls into `(None, None) => Ordering::Equal` and, because `sort_by` is
/// stable, leaves the rows exactly as they were — a perfect no-op that reads
/// as success. This is the predicate both of them already apply internally,
/// exposed so the report can say the key was never there.
///
/// A `null` counts as present (the agent asked for a key that exists and holds
/// null); an object or array does not, because neither sorts nor dedupes.
#[must_use]
pub fn rows_with_key(rows: &[Value], path: &str) -> usize {
    rows.iter()
        .filter(|row| get_path(row, path).and_then(scalar_text).is_some())
        .count()
}

fn as_number(v: &Value) -> Option<f64> {
    v.as_f64()
}

/// Drop rows whose key repeats, keeping the first occurrence.
///
/// Rows with no value at `path` are all kept: they are not proven duplicates of
/// each other, and collapsing them would delete data on the strength of a field
/// that is not there.
#[must_use]
pub fn dedupe_rows(rows: Vec<Value>, path: &str) -> Vec<Value> {
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    rows.into_iter()
        .filter(|row| match get_path(row, path).and_then(scalar_text) {
            Some(key) => seen.insert(key),
            None => true,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_the_three_operators() {
        assert_eq!(FilterExpr::parse("a=b").expect("eq").op, FilterOp::Eq);
        assert_eq!(FilterExpr::parse("a!=b").expect("ne").op, FilterOp::Ne);
        assert_eq!(
            FilterExpr::parse("a~b").expect("has").op,
            FilterOp::Contains
        );
    }

    #[test]
    fn ne_is_parsed_before_eq() {
        let f = FilterExpr::parse("status!=pass").expect("parse");
        assert_eq!(f.path, "status", "`a!` would be a silently wrong path");
        assert_eq!(f.value, "pass");
    }

    #[test]
    fn a_term_without_an_operator_is_a_usage_error() {
        let err = FilterExpr::parse("justakey").expect_err("must reject");
        assert_eq!(err.kind(), ErrorKind::Usage);
    }

    #[test]
    fn a_missing_path_never_matches_even_under_ne() {
        let row = json!({"other": 1});
        assert!(!FilterExpr::parse("k=v").expect("p").matches(&row));
        assert!(
            !FilterExpr::parse("k!=v").expect("p").matches(&row),
            "absence is not difference"
        );
    }

    #[test]
    fn contains_is_case_insensitive() {
        let row = json!({"title": "Residual Disk"});
        assert!(FilterExpr::parse("title~residual")
            .expect("p")
            .matches(&row));
    }

    #[test]
    fn terms_are_anded() {
        let rows = vec![
            json!({"id": "a", "status": "pass"}),
            json!({"id": "b", "status": "fail"}),
        ];
        let terms = vec![
            FilterExpr::parse("status=pass").expect("p"),
            FilterExpr::parse("id=a").expect("p"),
        ];
        assert_eq!(retain_matching(rows, &terms).len(), 1);
    }

    #[test]
    fn sort_orders_numbers_numerically() {
        let mut rows = vec![json!({"n": 10}), json!({"n": 9}), json!({"n": 100})];
        sort_rows(&mut rows, "n");
        let got: Vec<i64> = rows.iter().map(|r| r["n"].as_i64().unwrap_or(0)).collect();
        assert_eq!(got, vec![9, 10, 100], "text sort would give 10, 100, 9");
    }

    #[test]
    fn sort_puts_unsortable_rows_last() {
        let mut rows = vec![json!({}), json!({"n": "a"})];
        sort_rows(&mut rows, "n");
        assert!(rows[0].get("n").is_some());
    }

    #[test]
    fn dedupe_keeps_first_and_spares_keyless_rows() {
        let rows = vec![
            json!({"u": "x", "tag": 1}),
            json!({"u": "x", "tag": 2}),
            json!({}),
            json!({}),
        ];
        let out = dedupe_rows(rows, "u");
        assert_eq!(out.len(), 3);
        assert_eq!(out[0]["tag"], json!(1));
    }
}
