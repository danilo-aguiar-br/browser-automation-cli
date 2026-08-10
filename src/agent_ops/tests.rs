// SPDX-License-Identifier: MIT OR Apache-2.0
//! Contract tests for the universal envelope operations.

use serde_json::json;

use super::*;

fn ops() -> AgentOps {
    AgentOps::default()
}

/// The shape `doctor` emits: several arrays are NOT present, only `checks` is.
fn doctor_like() -> serde_json::Value {
    json!({
        "checks": [
            {"id": "chrome", "status": "pass", "message": "found"},
            {"id": "residual_disk", "status": "warn", "message": "live siblings only"},
            {"id": "cache_redis", "status": "pass", "message": "sqlite"},
        ],
        "residual": {"cli_marker_dirs": 0, "ghost_marker_processes": 0},
    })
}

#[test]
fn a_noop_leaves_the_envelope_byte_identical() {
    let data = doctor_like();
    let (out, report) = apply(data.clone(), &ops()).expect("noop");
    assert_eq!(out, data);
    assert!(
        report.is_empty(),
        "an untouched envelope grows no new fields"
    );
}

/// The motivating case: 26_277 bytes reduced to the one field the agent wanted.
#[test]
fn select_projects_a_single_nested_field() {
    let mut o = ops();
    o.select = vec!["residual.ghost_marker_processes".into()];
    let (out, _) = apply(doctor_like(), &o).expect("select");
    assert_eq!(out, json!({"residual": {"ghost_marker_processes": 0}}));
}

#[test]
fn filter_narrows_rows_and_reports_both_counts() {
    let mut o = ops();
    o.filter = vec![filter::FilterExpr::parse("status=pass").expect("p")];
    let (out, report) = apply(doctor_like(), &o).expect("filter");
    assert_eq!(out["checks"].as_array().map(Vec::len), Some(2));
    assert_eq!(report.total, Some(3));
    assert_eq!(report.matched, Some(2));
}

/// A filter that matches nothing is an empty list, never an error and never
/// the unfiltered list.
#[test]
fn a_filter_matching_nothing_yields_an_empty_list() {
    let mut o = ops();
    o.filter = vec![filter::FilterExpr::parse("status=nonexistent").expect("p")];
    let (out, report) = apply(doctor_like(), &o).expect("filter");
    assert_eq!(out["checks"], json!([]));
    assert_eq!(report.matched, Some(0));
    assert_eq!(
        out["residual"]["cli_marker_dirs"],
        json!(0),
        "filtering rows must not disturb the surrounding object"
    );
}

#[test]
fn limit_cuts_and_marks_truncated() {
    let mut o = ops();
    o.limit = Some(1);
    let (out, report) = apply(doctor_like(), &o).expect("limit");
    assert_eq!(out["checks"].as_array().map(Vec::len), Some(1));
    assert!(
        report.truncated,
        "a cut the agent cannot see is a silent lie"
    );
}

#[test]
fn count_only_replaces_the_payload_with_a_count() {
    let mut o = ops();
    o.count_only = true;
    o.filter = vec![filter::FilterExpr::parse("status=pass").expect("p")];
    let (out, _) = apply(doctor_like(), &o).expect("count");
    assert_eq!(out, json!({"count": 2}));
}

#[test]
fn sort_and_dedupe_compose_over_the_same_list() {
    let data = json!({"rows": [{"k": "b"}, {"k": "a"}, {"k": "a"}]});
    let mut o = ops();
    o.dedupe_by = Some("k".into());
    o.sort = Some("k".into());
    let (out, report) = apply(data, &o).expect("compose");
    assert_eq!(out["rows"], json!([{"k": "a"}, {"k": "b"}]));
    assert_eq!(report.matched, Some(2));
}

/// `data` that is itself a list needs no field name.
#[test]
fn a_root_level_list_is_operated_on_directly() {
    let data = json!([{"n": 2}, {"n": 1}]);
    let mut o = ops();
    o.sort = Some("n".into());
    let (out, _) = apply(data, &o).expect("root rows");
    assert_eq!(out, json!([{"n": 1}, {"n": 2}]));
}

/// Ambiguity is reported with the candidate names, not guessed.
#[test]
fn two_lists_is_a_usage_error_that_names_both() {
    let data = json!({"a": [1], "b": [2]});
    let mut o = ops();
    o.limit = Some(1);
    let err = apply(data, &o).expect_err("ambiguous");
    assert_eq!(err.kind(), ErrorKind::Usage);
    assert!(err.message().contains('a') && err.message().contains('b'));
}

/// And `--fields` is the documented way out of that ambiguity.
#[test]
fn select_disambiguates_two_lists() {
    let data = json!({"a": [{"n": 1}, {"n": 2}], "b": [{"n": 9}]});
    let mut o = ops();
    o.select = vec!["a".into()];
    o.limit = Some(1);
    let (out, _) = apply(data, &o).expect("select then limit");
    assert_eq!(out, json!({"a": [{"n": 1}]}));
}

#[test]
fn row_ops_against_data_with_no_list_are_a_usage_error() {
    let data = json!({"only": {"nested": 1}});
    let mut o = ops();
    o.limit = Some(1);
    let err = apply(data, &o).expect_err("no rows");
    assert_eq!(err.kind(), ErrorKind::Usage);
}

#[test]
fn truncate_content_marks_the_cut() {
    let data = json!({"rows": [{"body": "abcdefghij"}]});
    let mut o = ops();
    o.truncate_content = Some(3);
    let (out, report) = apply(data, &o).expect("truncate");
    assert_eq!(out["rows"][0]["body"], json!("abc"));
    assert!(report.truncated);
}

#[test]
fn max_output_bytes_sheds_rows_and_reports_how_many() {
    let rows: Vec<serde_json::Value> = (0..40)
        .map(|i| json!({"i": i, "pad": "0123456789012345678901234567890123456789"}))
        .collect();
    let data = json!({"rows": rows});
    let mut o = ops();
    o.max_output_bytes = Some(600);
    let (out, report) = apply(data, &o).expect("budget");
    assert!(report.truncated);
    assert!(report.omitted_rows.unwrap_or(0) > 0);
    assert!(serde_json::to_vec(&out).expect("ser").len() <= 600);
}

#[test]
fn a_ceiling_that_cannot_be_met_is_reported_not_mangled() {
    let data = json!({"rows": [], "big": "x".repeat(4096)});
    let mut o = ops();
    o.max_output_bytes = Some(16);
    let err = apply(data, &o).expect_err("impossible budget");
    assert_eq!(err.kind(), ErrorKind::Usage);
}

#[test]
fn is_noop_recognises_an_empty_request() {
    assert!(ops().is_noop());
    let mut o = ops();
    o.count_only = true;
    assert!(!o.is_noop());
}

/// Build an `--expect` request from raw expressions.
fn expecting(raws: &[&str]) -> AgentOps {
    let mut o = ops();
    o.expect = raws
        .iter()
        .map(|r| ((*r).to_string(), FilterExpr::parse(r).expect("parse")))
        .collect();
    o
}

#[test]
fn a_held_expectation_reports_nothing() {
    let unmet = unmet_expectations(&doctor_like(), &expecting(&["id=chrome"]));
    assert!(unmet.is_empty(), "{unmet:?}");
}

#[test]
fn an_unmet_expectation_echoes_the_caller_argument() {
    // Echoed verbatim so the agent matches the string it typed instead of
    // reconstructing one from a normalised parse.
    let unmet = unmet_expectations(&doctor_like(), &expecting(&["id=nosuchcheck"]));
    assert_eq!(unmet, vec!["id=nosuchcheck".to_string()]);
}

#[test]
fn every_unmet_expectation_is_listed_not_just_the_first() {
    // A single-item report would send the agent round the loop once per typo.
    let unmet = unmet_expectations(&doctor_like(), &expecting(&["id=nope", "status=exploded"]));
    assert_eq!(unmet.len(), 2, "{unmet:?}");
}

#[test]
fn one_matching_row_satisfies_an_expectation() {
    // `--expect status=warn` asks "is there a warn in here?", which is the
    // question an agent has. The stricter reading is available by filtering
    // first, so the loose one is the useful default.
    assert!(unmet_expectations(&doctor_like(), &expecting(&["status=warn"])).is_empty());
}

#[test]
fn a_payload_with_no_rows_is_asserted_against_itself() {
    let flat = json!({"ok_detail": true, "engine": "http"});
    assert!(unmet_expectations(&flat, &expecting(&["engine=http"])).is_empty());
    assert_eq!(
        unmet_expectations(&flat, &expecting(&["engine=browser"])).len(),
        1
    );
}

#[test]
fn a_missing_key_never_satisfies_an_expectation() {
    // Consistent with `--filter-rows`: absence is not difference, so `!=`
    // does not turn a missing field into a match.
    let unmet = unmet_expectations(&doctor_like(), &expecting(&["absent!=whatever"]));
    assert_eq!(unmet.len(), 1, "{unmet:?}");
}

#[test]
fn expectations_do_not_alter_the_payload() {
    // The assertion observes; it must never become another reduction step.
    let data = doctor_like();
    let (out, _) = apply(data.clone(), &expecting(&["id=chrome"])).expect("apply");
    assert_eq!(out, data);
}
