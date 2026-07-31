// SPDX-License-Identifier: MIT OR Apache-2.0
//! Unit tests for the promoted policy table.

use super::knobs::*;

#[test]
fn every_key_has_a_named_constant_default() {
    for k in POLICY_KEYS {
        assert!(policy_default(k).is_some(), "missing default for {k}");
        assert!(policy_default(k).unwrap() > 0, "non-positive default {k}");
    }
}

#[test]
fn key_names_are_unique_and_snake_case() {
    let mut sorted: Vec<&str> = POLICY_KEYS.to_vec();
    sorted.sort_unstable();
    let before = sorted.len();
    sorted.dedup();
    assert_eq!(before, sorted.len(), "duplicate policy key");
    for k in POLICY_KEYS {
        assert!(
            k.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
            "non snake_case key: {k}"
        );
    }
}

#[test]
fn unset_key_resolves_to_the_constant_default() {
    let cfg = PolicyConfig::default();
    for k in POLICY_KEYS {
        assert_eq!(policy_stored(&cfg, k), Some(None), "{k}");
    }
}

#[test]
fn set_then_stored_roundtrip() {
    let mut cfg = PolicyConfig::default();
    assert!(policy_set(&mut cfg, key::REDIS_IO_TIMEOUT_SECS, "11").is_some());
    assert_eq!(
        policy_stored(&cfg, key::REDIS_IO_TIMEOUT_SECS),
        Some(Some(11))
    );
    assert_eq!(policy_pairs(&cfg), vec![("redis_io_timeout_secs", 11)]);
}

#[test]
fn zero_and_garbage_are_rejected() {
    let mut cfg = PolicyConfig::default();
    assert!(policy_set(&mut cfg, key::RETRY_BUDGET_SECS, "0")
        .unwrap()
        .is_err());
    assert!(policy_set(&mut cfg, key::RETRY_BUDGET_SECS, "abc")
        .unwrap()
        .is_err());
    assert!(policy_set(&mut cfg, "not_a_key", "1").is_none());
}

#[test]
fn list_entries_cover_every_key() {
    assert_eq!(policy_list_entries().len(), POLICY_KEYS.len());
}
