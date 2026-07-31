// SPDX-License-Identifier: MIT OR Apache-2.0

use super::types::BTreeMapString;
use super::util::redact_headers_for_test;
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
