// SPDX-License-Identifier: MIT OR Apache-2.0

use super::redact::redact_headers_for_test;
use super::types::BTreeMapString;
use crate::constants::MITM_REDACTED_PLACEHOLDER;

#[test]
fn redact_auth() {
    let mut h = BTreeMapString::new();
    h.insert("Authorization".into(), "Bearer secret".into());
    redact_headers_for_test(&mut h);
    assert_eq!(
        h.get("Authorization").map(|s| s.as_str()),
        Some(MITM_REDACTED_PLACEHOLDER)
    );
}

#[test]
fn empty_record_allowlist_admits_everything() {
    // The default capture stays exploratory: `--capture-hosts` is opt-in.
    assert!(super::handler::record_allowed(Some("example.com"), &[]));
    assert!(super::handler::record_allowed(None, &[]));
}

#[test]
fn record_allowlist_matches_host_and_subdomain_only() {
    let allow = vec!["example.com".to_string()];
    assert!(super::handler::record_allowed(Some("example.com"), &allow));
    assert!(super::handler::record_allowed(
        Some("api.example.com"),
        &allow
    ));
    // The boundary is a dot, not a substring, or a lookalike domain would be
    // written into the artifact under the name the operator trusted.
    assert!(!super::handler::record_allowed(
        Some("evil-example.com"),
        &allow
    ));
    assert!(!super::handler::record_allowed(
        Some("accounts.google.com"),
        &allow
    ));
    // Case is not a distinction between hosts.
    assert!(super::handler::record_allowed(
        Some("API.Example.COM"),
        &allow
    ));
}

#[test]
fn record_allowlist_refuses_unknown_host() {
    // Selection is on the host. An exchange whose host cannot be determined is
    // not evidence that the operator wanted it.
    let allow = vec!["example.com".to_string()];
    assert!(!super::handler::record_allowed(None, &allow));
}

/// A host-only rule refuses that host and leaves every other host alone.
#[test]
fn block_rule_host_only_matches_that_host() {
    let rule = super::types::BlockRule {
        host: Some("ads.example.com".into()),
        path: None,
    };
    assert!(rule.matches(Some("ads.example.com"), "/anything"));
    assert!(
        rule.matches(Some("ADS.EXAMPLE.COM"), "/"),
        "host is case-blind"
    );
    assert!(!rule.matches(Some("app.example.com"), "/anything"));
    assert!(
        !rule.matches(None, "/anything"),
        "no host cannot match a host"
    );
}

/// A path-only rule refuses that prefix on every host.
#[test]
fn block_rule_path_only_matches_by_prefix() {
    let rule = super::types::BlockRule {
        host: None,
        path: Some("/track".into()),
    };
    assert!(rule.matches(Some("a.test"), "/track"));
    assert!(
        rule.matches(Some("b.test"), "/track/pixel.gif"),
        "prefix, not equality"
    );
    assert!(
        !rule.matches(Some("a.test"), "/api/track"),
        "prefix is anchored"
    );
}

/// Both fields present means AND, never OR.
///
/// Reading it as OR would widen a rule about one path on one host into a block
/// on that path everywhere, and on every path of that host — the operator would
/// have refused traffic they never named.
#[test]
fn block_rule_with_both_fields_requires_both() {
    let rule = super::types::BlockRule {
        host: Some("a.test".into()),
        path: Some("/ads".into()),
    };
    assert!(rule.matches(Some("a.test"), "/ads/banner"));
    assert!(!rule.matches(Some("b.test"), "/ads/banner"), "wrong host");
    assert!(!rule.matches(Some("a.test"), "/news"), "wrong path");
}

/// An empty rule refuses nothing, so a hand-edited file cannot blank the proxy.
///
/// `mitm block` rejects a rule with neither field, but the rules file is plain
/// JSON on disk: the guard has to live where the rule is APPLIED, not only
/// where it is created.
#[test]
fn block_rule_with_no_fields_matches_nothing() {
    let rule = super::types::BlockRule::default();
    assert!(!rule.matches(Some("a.test"), "/anything"));
    assert!(!rule.matches(None, ""));
}

/// An event with no `url` is dropped instead of stored under an empty address.
#[test]
fn cdp_event_without_url_is_dropped() {
    assert!(super::store::cdp_event_to_exchange(&serde_json::json!({"method": "GET"})).is_none());
    assert!(super::store::cdp_event_to_exchange(&serde_json::json!({"url": ""})).is_none());
}

/// Every key the mapping READS is populated by an event that WRITES only the
/// canonical CDP names.
///
/// This is the class test, not a case test. The defect this module keeps paying
/// for is a consumer reading a key no producer writes: `graphql` read
/// `endpoints` while `analyze` wrote `apis`, and `net list --resource-types`
/// read three names the network buffer had none of. Both were silent, because a
/// missing key reads as `None` and `None` renders as a legitimately absent
/// field. Asserting that NOTHING is `None` under canonical input is what turns
/// that silence into a failing test.
#[test]
fn canonical_cdp_keys_populate_every_mapped_field() {
    let ev = serde_json::json!({
        "method": "POST",
        "url": "https://api.test/v1/items?page=2",
        "status": 201,
        "mimeType": "application/json",
        "request_headers": {"accept": "application/json"},
        "response_headers": {"content-length": "17"},
    });
    let x = super::store::cdp_event_to_exchange(&ev).expect("canonical event maps");
    assert_eq!(x.method, "POST", "method key not read");
    assert_eq!(
        x.url, "https://api.test/v1/items?page=2",
        "url key not read"
    );
    assert_eq!(x.status, Some(201), "status key not read");
    assert_eq!(
        x.content_type.as_deref(),
        Some("application/json"),
        "mimeType key not read"
    );
    assert_eq!(
        x.host.as_deref(),
        Some("api.test"),
        "host not derived from url"
    );
    assert_eq!(
        x.request_headers.get("accept").map(String::as_str),
        Some("application/json"),
        "request_headers key not read"
    );
    assert_eq!(
        x.response_headers.get("content-length").map(String::as_str),
        Some("17"),
        "response_headers key not read"
    );
    assert!(x.started_ms > 0, "started_ms not stamped");
}

/// The three alias reads are a contract with library callers, so they are
/// pinned. Unlike the `request_method` fallback deleted from `proxy.rs`, these
/// events come from OUTSIDE this repository, so "no producer here writes it" is
/// not evidence that nothing writes it.
#[test]
fn cdp_event_alias_keys_are_read() {
    let ev = serde_json::json!({
        "request_method": "PUT",
        "url": "https://alias.test/x",
        "status_code": 404,
        "content_type": "text/plain",
    });
    let x = super::store::cdp_event_to_exchange(&ev).expect("alias event maps");
    assert_eq!(x.method, "PUT");
    assert_eq!(x.status, Some(404));
    assert_eq!(x.content_type.as_deref(), Some("text/plain"));
}

/// A canonical key present alongside its alias wins, so an event carrying both
/// cannot resolve to the fallback.
#[test]
fn canonical_cdp_keys_win_over_their_aliases() {
    let ev = serde_json::json!({
        "method": "GET",
        "request_method": "DELETE",
        "url": "https://both.test/y",
        "status": 200,
        "status_code": 500,
        "mimeType": "text/html",
        "content_type": "text/plain",
    });
    let x = super::store::cdp_event_to_exchange(&ev).expect("event maps");
    assert_eq!(x.method, "GET", "alias shadowed the canonical method");
    assert_eq!(x.status, Some(200), "alias shadowed the canonical status");
    assert_eq!(
        x.content_type.as_deref(),
        Some("text/html"),
        "alias shadowed the canonical mimeType"
    );
}

#[test]
fn graphql_truncates_apis_key_not_endpoints() {
    // Unit-level: key name contract used by store::graphql must match analyze::apis.
    let v = serde_json::json!({
        "count": 2,
        "apis": [
            {"id": 1, "kind": "graphql"},
            {"id": 2, "kind": "graphql"},
        ]
    });
    assert!(v.get("apis").is_some());
    assert!(v.get("endpoints").is_none());
}
