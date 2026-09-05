// SPDX-License-Identifier: MIT OR Apache-2.0
//! Fail when one branch of `batch-scrape` drops a status field its siblings emit.
//!
//! # Why this exists
//!
//! `batch-scrape` builds its envelope in three places: `batch_scrape_http`
//! returns one for the single-format HTTP path, and `handle_batch_scrape` hand
//! builds two more for the browser engine and for multi-format. Nothing tied
//! them together, so they drifted, twice, in opposite directions.
//!
//! Defeito 12 was that a nested `ok` collided with the envelope's own. The fix
//! renamed it to `all_succeeded` and added `partial_failure` and `error_count`
//! — in ONE of the three branches. The multi-format branch kept reading the old
//! key through `base.get("ok")` with `unwrap_or(json!(true))`, so the rename
//! turned a real value into a constant `true` and dropped the other two fields;
//! and the browser branch had never carried any of them at all.
//!
//! Measured 2026-09-01, before the fix: the same two-URL batch with one dead
//! host reported `partial_failure: true` under `--engine http`, and nothing at
//! all under `--engine browser` or under `--format text,markdown`. Which of the
//! three the caller got depended on flags that have nothing to do with error
//! reporting.
//!
//! # Why a hand-kept list is right HERE and wrong elsewhere
//!
//! This file's sibling gate, `phantom_flag_gate.rs`, discovers its subjects
//! rather than listing them, because a list of things-that-exist ages behind the
//! code. [`STATUS_KEYS`] is not that: it is the CONTRACT the envelope promises,
//! and a contract that changes should require editing the file that states it.
//! Adding a key here and watching three branches fail is the intended workflow.
//!
//! # What this checks
//!
//! 1. The source of truth emits every key in [`STATUS_KEYS`], so the contract is
//!    not vacuous.
//! 2. Every hand-built envelope in the branches file emits all of them too.
//! 3. No branch reintroduces a nested `ok`, which is the collision Defeito 12
//!    was filed for.

use std::collections::BTreeSet;

mod common;
use common::root;

/// The envelope `batch_scrape_http` returns; the shape the others must match.
const SOURCE_OF_TRUTH: &str = "src/scrape_local/batch.rs";

/// Where the browser and multi-format envelopes are assembled by hand.
const BRANCHES_FILE: &str = "src/commands/scrape/batch.rs";

/// Fields that answer "how did the batch go", as opposed to what it returned.
///
/// `count` and `error_count` are here and `pages`/`errors` are not: a caller
/// branching on partial failure must not have to length-check an array, which is
/// the ergonomic half of what Defeito 12 asked for.
const STATUS_KEYS: &[&str] = &["all_succeeded", "partial_failure", "count", "error_count"];

/// The banned key. Its meaning collides with the envelope's own top-level `ok`.
const BANNED_KEY: &str = "ok";

/// Every `json!({ … })` block in `text`, as (line number, body).
///
/// Braces are counted rather than the block being assumed to end at the first
/// `})`: a nested `json!` inside an envelope is legal and used, and a scanner
/// that stopped early would silently audit half a branch and pass.
fn json_blocks(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    for (idx, line) in lines.iter().enumerate() {
        if !line.contains("json!({") {
            continue;
        }
        let mut depth = 0i32;
        let mut body = String::new();
        for candidate in lines.iter().skip(idx) {
            body.push_str(candidate);
            body.push('\n');
            for ch in candidate.chars() {
                match ch {
                    '{' => depth += 1,
                    '}' => depth -= 1,
                    _ => {}
                }
            }
            if depth <= 0 {
                break;
            }
        }
        out.push((idx + 1, body));
    }
    out
}

/// The keys a `json!` body assigns at any depth, as they appear quoted.
fn keys_in(body: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let bytes = body.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'"' {
            i += 1;
            continue;
        }
        let start = i + 1;
        let mut end = start;
        while end < bytes.len() && bytes[end] != b'"' {
            end += 1;
        }
        if end >= bytes.len() {
            break;
        }
        // A key is a quoted token followed by `:`.
        if body[end + 1..].trim_start().starts_with(':') {
            out.insert(body[start..end].to_string());
        }
        i = end + 1;
    }
    out
}

/// A `json!` block is an envelope when it carries the batch's payload array.
fn is_envelope(keys: &BTreeSet<String>) -> bool {
    keys.contains("pages") || keys.contains("results")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel))
        .unwrap_or_else(|e| panic!("{rel} must be readable to audit the envelope: {e}"))
}

#[test]
fn the_source_of_truth_still_emits_every_contracted_status_key() {
    let text = read(SOURCE_OF_TRUTH);
    let envelopes: Vec<_> = json_blocks(&text)
        .into_iter()
        .map(|(line, body)| (line, keys_in(&body)))
        .filter(|(_, keys)| is_envelope(keys))
        .collect();
    assert!(
        !envelopes.is_empty(),
        "{SOURCE_OF_TRUTH} exposed no envelope to this scan, so every other \
         assertion in this file would pass vacuously"
    );
    for (line, keys) in envelopes {
        let missing: Vec<&str> = STATUS_KEYS
            .iter()
            .copied()
            .filter(|k| !keys.contains(*k))
            .collect();
        assert!(
            missing.is_empty(),
            "{SOURCE_OF_TRUTH}:{line} no longer emits {missing:?}; either the \
             contract moved and STATUS_KEYS is stale, or the fix for Defeito 12 \
             was reverted at its origin"
        );
    }
}

#[test]
fn every_hand_built_branch_matches_the_contract() {
    let text = read(BRANCHES_FILE);
    let envelopes: Vec<_> = json_blocks(&text)
        .into_iter()
        .map(|(line, body)| (line, keys_in(&body)))
        .filter(|(_, keys)| is_envelope(keys))
        .collect();
    assert!(
        envelopes.len() >= 2,
        "{BRANCHES_FILE} exposed only {} envelope(s); the browser and \
         multi-format branches both build one, so the scan failed",
        envelopes.len()
    );
    let mut problems = Vec::new();
    for (line, keys) in envelopes {
        for key in STATUS_KEYS {
            if !keys.contains(*key) {
                problems.push(format!(
                    "{BRANCHES_FILE}:{line} builds a batch envelope without `{key}`"
                ));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "a caller cannot branch on partial failure when the answer depends on \
         which engine or format they picked:\n{}",
        problems.join("\n")
    );
}

#[test]
fn no_branch_reintroduces_the_nested_ok() {
    let mut problems = Vec::new();
    for rel in [SOURCE_OF_TRUTH, BRANCHES_FILE] {
        for (line, body) in json_blocks(&read(rel)) {
            let keys = keys_in(&body);
            if is_envelope(&keys) && keys.contains(BANNED_KEY) {
                problems.push(format!(
                    "{rel}:{line} emits a nested `{BANNED_KEY}` inside the batch envelope"
                ));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "the envelope already carries a top-level `{BANNED_KEY}` meaning \"the \
         command ran\"; a second one meaning \"every URL succeeded\" is the \
         contract bug Defeito 12 was filed for, and both readings are defensible \
         alone, which is exactly why sharing the name is not:\n{}",
        problems.join("\n")
    );
}
