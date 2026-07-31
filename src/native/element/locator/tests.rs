// SPDX-License-Identifier: MIT OR Apache-2.0
//! Unit tests for durable locators.

use super::*;

#[test]
fn wire_round_trip_without_nth() {
    let l = DurableLocator::new("button", "Sign in", None);
    assert_eq!(l.to_wire(), r#"role=button[name="Sign in"]"#);
    assert_eq!(DurableLocator::parse(&l.to_wire()), Some(l));
}

#[test]
fn wire_round_trip_with_nth() {
    let l = DurableLocator::new("textbox", "Email", Some(2));
    assert_eq!(l.to_wire(), r#"role=textbox[name="Email"][2]"#);
    assert_eq!(DurableLocator::parse(&l.to_wire()), Some(l));
}

#[test]
fn nth_is_the_callers_decision_not_a_heuristic() {
    // A unique pair should be built with None, not Some(1)...
    let unique = DurableLocator::new("link", "Home", None);
    assert_eq!(unique.to_wire(), r#"role=link[name="Home"]"#);
    // ...but the first of a repeated pair keeps [1], or it would collide
    // with the bare form that means "the only one".
    let first_of_many = DurableLocator::new("link", "Home", Some(1));
    assert_eq!(first_of_many.to_wire(), r#"role=link[name="Home"][1]"#);
    assert_ne!(unique.to_wire(), first_of_many.to_wire());
}

#[test]
fn names_with_quotes_survive_the_round_trip() {
    let l = DurableLocator::new("button", r#"Say "hi""#, None);
    let wire = l.to_wire();
    assert_eq!(DurableLocator::parse(&wire), Some(l));
}

#[test]
fn role_only_locator_matches_any_name() {
    let l = DurableLocator::parse("role=button").expect("parse");
    assert_eq!(l.role, "button");
    assert!(l.name.is_empty());
    assert!(l.matches("button", "anything"));
    assert!(!l.matches("link", "anything"));
}

#[test]
fn looks_like_separates_from_css_and_at_refs() {
    assert!(DurableLocator::looks_like(r#"role=button[name="X"]"#));
    assert!(!DurableLocator::looks_like("@e7"));
    assert!(!DurableLocator::looks_like("#submit"));
    assert!(!DurableLocator::looks_like("div.role=button"));
}

#[test]
fn parse_rejects_malformed_input() {
    assert_eq!(DurableLocator::parse("button"), None);
    assert_eq!(DurableLocator::parse("role="), None);
    assert_eq!(DurableLocator::parse(r#"role=button[name="X""#), None);
    assert_eq!(DurableLocator::parse(r#"role=button[bogus=1]"#), None);
}

#[test]
fn matching_is_case_insensitive_on_role_only() {
    let l = DurableLocator::new("Button", "Sign in", None);
    assert!(l.matches("button", "Sign in"));
    assert!(!l.matches("button", "sign in"), "name must match exactly");
}

#[test]
fn assign_locators_marks_only_repeated_pairs() {
    let pairs = vec![
        ("button".to_string(), "Save".to_string()),
        ("textbox".to_string(), "Email".to_string()),
        ("button".to_string(), "Save".to_string()),
    ];
    let out = assign_locators(&pairs);
    assert_eq!(out[0].nth, Some(1), "first of a repeated pair");
    assert_eq!(out[1].nth, None, "unique pair carries no index");
    assert_eq!(out[2].nth, Some(2), "second of a repeated pair");
    assert_eq!(out[0].to_wire(), r#"role=button[name="Save"][1]"#);
}

#[test]
fn assignment_is_deterministic_for_the_same_snapshot() {
    // Cross-process reuse only works if the same page yields the same
    // strings every time. Same input order must give byte-identical output.
    let pairs = vec![
        ("button".to_string(), "Sign in".to_string()),
        ("button".to_string(), "Save".to_string()),
        ("textbox".to_string(), "Email".to_string()),
        ("button".to_string(), "Save".to_string()),
    ];
    let first: Vec<String> = assign_locators(&pairs)
        .iter()
        .map(|l| l.to_wire())
        .collect();
    let second: Vec<String> = assign_locators(&pairs)
        .iter()
        .map(|l| l.to_wire())
        .collect();
    assert_eq!(first, second);
    assert_eq!(
        first,
        vec![
            r#"role=button[name="Sign in"]"#,
            r#"role=button[name="Save"][1]"#,
            r#"role=textbox[name="Email"]"#,
            r#"role=button[name="Save"][2]"#,
        ]
    );
}

#[test]
fn every_assigned_locator_round_trips() {
    let pairs = vec![
        ("button".to_string(), r#"Say "hi""#.to_string()),
        ("link".to_string(), String::new()),
        ("button".to_string(), r#"Say "hi""#.to_string()),
    ];
    for locator in assign_locators(&pairs) {
        let wire = locator.to_wire();
        assert_eq!(
            DurableLocator::parse(&wire),
            Some(locator.clone()),
            "round trip broke for {wire}"
        );
    }
}

#[test]
fn candidate_index_is_zero_based() {
    // The wire form counts from 1 because humans read it; the accessibility
    // walker counts from 0. Passing `nth` straight through made `[1]` click
    // the SECOND match and `[2]` fall off the end.
    assert_eq!(DurableLocator::new("b", "n", None).candidate_index(), 0);
    assert_eq!(DurableLocator::new("b", "n", Some(1)).candidate_index(), 0);
    assert_eq!(DurableLocator::new("b", "n", Some(2)).candidate_index(), 1);
    assert_eq!(DurableLocator::new("b", "n", Some(3)).candidate_index(), 2);
}
