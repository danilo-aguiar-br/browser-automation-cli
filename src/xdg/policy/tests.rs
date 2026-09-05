// SPDX-License-Identifier: MIT OR Apache-2.0
//! Unit tests for the promoted policy table.

use super::knobs::*;

#[test]
fn every_key_has_a_named_constant_default() {
    for k in POLICY_KEYS {
        assert!(policy_default(k).is_some(), "missing default for {k}");
        // A BUDGET at zero disables what it protects, so zero is a broken
        // default. A CHANCE at zero means "never", which is a legitimate
        // default and the only way an operator can turn the behaviour off.
        // The split lives in `parse_policy_value`, and reading it from there
        // is what keeps the setter and this invariant from drifting apart.
        if !super::validate::is_chance(k) {
            assert!(policy_default(k).unwrap() > 0, "non-positive default {k}");
        }
    }
}

#[test]
fn a_chance_may_be_set_to_never_and_a_budget_may_not() {
    let mut cfg = PolicyConfig::default();
    // Measured 2026-09-04: this was documented behaviour that the validator
    // refused. `docs/CONFIGURATION.md` says `0` removes the long-pause tail,
    // and `config set input_word_pause_permille 0` answered `must be > 0`.
    assert!(
        policy_set(&mut cfg, key::INPUT_WORD_PAUSE_PERMILLE, "0")
            .expect("key exists")
            .is_ok(),
        "a chance must accept zero, because zero is how an operator says never"
    );
    assert!(
        policy_set(&mut cfg, key::REDIS_IO_TIMEOUT_SECS, "0")
            .expect("key exists")
            .is_err(),
        "a budget at zero disables what it protects"
    );
}

#[test]
fn a_chance_set_to_never_survives_the_read_path() {
    // Accepting `0` at the setter is only half the contract: the value has to
    // come back. Measured 2026-09-04 on the shipped 0.1.9 binary it did not.
    // `config set input_word_pause_permille 0` answered `ok` and wrote
    // `input_word_pause_permille = 0` to the file, `config get` answered
    // `value: null`, and the effective value stayed at the default of 120.
    //
    // The sibling test above only exercised `policy_set`, so the read path had
    // no gate at all and the fix to the refusal message shipped with the cause
    // untouched.
    let mut cfg = PolicyConfig::default();

    assert!(policy_apply_raw(
        &mut cfg,
        key::INPUT_WORD_PAUSE_PERMILLE,
        "0"
    ));
    assert_eq!(
        policy_stored(&cfg, key::INPUT_WORD_PAUSE_PERMILLE),
        Some(Some(0)),
        "the loose loader must keep a chance of zero instead of reading it as absent"
    );

    // The other half of the same rule: a budget of zero still means absent,
    // because a budget at zero disables the thing it is there to protect.
    assert!(policy_apply_raw(&mut cfg, key::REDIS_IO_TIMEOUT_SECS, "0"));
    assert_eq!(
        policy_stored(&cfg, key::REDIS_IO_TIMEOUT_SECS),
        Some(None),
        "a budget of zero stays absent so the named default wins"
    );
}

#[test]
fn one_function_decides_zero_for_every_path() {
    // The setter, the loader and the resolver all ask the same question. They
    // used to answer it in three places with two different rules, which is how
    // the halves drifted; asserting the rule here keeps a future caller from
    // reintroducing a bare `n > 0` next to a call that respects the split.
    assert!(super::validate::keeps_stored(
        key::INPUT_WORD_PAUSE_PERMILLE,
        0
    ));
    assert!(super::validate::keeps_stored(
        key::INPUT_WORD_PAUSE_PERMILLE,
        7
    ));
    assert!(!super::validate::keeps_stored(
        key::REDIS_IO_TIMEOUT_SECS,
        0
    ));
    assert!(super::validate::keeps_stored(key::REDIS_IO_TIMEOUT_SECS, 7));
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
