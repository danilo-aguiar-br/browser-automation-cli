// SPDX-License-Identifier: MIT OR Apache-2.0
//! A test that resolves the binary by PATH must never assert on a feature gate.
//!
//! # The two resolvers and why both exist
//!
//! `tests/common/mod.rs` offers two ways to reach the product binary.
//! [`common::bin`] wraps `CARGO_BIN_EXE_browser-automation-cli`, which cargo
//! sets at test-COMPILE time and which therefore names the artifact built with
//! the SAME feature set as the test asking for it. `common::binary()` looks
//! `target/debug/browser-automation-cli` up by path and returns `None` when it
//! is absent, so its callers SKIP instead of failing.
//!
//! The skip is deliberate — those gates are meant to be runnable before a full
//! build — and unifying the two would delete that property. But the path form
//! knows nothing about features: it hands back whatever binary last landed at
//! that path, built with whatever feature set that build happened to use.
//!
//! # What went wrong on 2026-08-25
//!
//! A `cargo run` launched next to the suite rebuilt that shared path with
//! DEFAULT features while the suite was driving it. Four gates in
//! `tests/image_media_cli_e2e.rs` failed with `requires the image-svg Cargo
//! feature`, which reads as a product regression and is in fact a true
//! statement about the wrong artifact.
//!
//! Those four use [`common::bin`], and `CARGO_BIN_EXE_*` did not save them:
//! the macro guarantees which build PRODUCED the artifact, never who
//! overwrites the shared path afterwards. The path form is strictly more
//! exposed, because it does not even carry the first guarantee.
//!
//! # The invariant
//!
//! A test file that resolves through `binary()` must not assert on
//! feature-gated behaviour. Then a stale or differently-featured artifact at
//! that path can still make the gate skip, but can never make it fail with a
//! true sentence about a binary the test did not mean to measure.
//!
//! This is asserted rather than written down because the audit that found the
//! exposure also found the doc of `binary()` claiming 23 callers when there
//! were 24. A number in prose goes stale silently; a scanner does not.

use std::path::Path;

/// Markers that a test exercises behaviour which differs per feature set.
const FEATURE_MARKERS: &[&str] = &["Cargo feature", "cfg(feature", "feature = \""];

/// This file names every marker it looks for, so scanning it would report
/// itself. Same exemption the `sg_local` scanner needed for the same reason.
const SELF: &str = "binary_resolver_boundary_gate.rs";

/// Every `tests/*.rs` that reaches the product binary by path.
fn path_resolving_tests(root: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let dir = root.join("tests");
    let entries = std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("read tests/: {e}"));
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        if name == SELF {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        if text.contains("binary()") {
            out.push((name, text));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

#[test]
fn no_path_resolved_test_asserts_on_a_feature_gate() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let files = path_resolving_tests(root);

    assert!(
        files.len() >= 20,
        "the scanner found only {} path-resolving test files, which means it \
         stopped matching rather than that the family shrank; 24 were measured \
         on 2026-08-25",
        files.len()
    );

    let offenders: Vec<&str> = files
        .iter()
        .filter(|(_, text)| FEATURE_MARKERS.iter().any(|m| text.contains(m)))
        .map(|(name, _)| name.as_str())
        .collect();

    assert!(
        offenders.is_empty(),
        "these tests resolve the binary by PATH and also assert on feature-gated \
         behaviour, so a differently-featured build at `target/debug/` makes them \
         fail with a true sentence about the wrong artifact. Switch them to \
         `common::bin()`, which at least pins the feature set of the build that \
         produced the binary: {offenders:#?}"
    );
}

#[test]
fn the_scanner_recognizes_a_violation_when_there_is_one() {
    // The assertion above is green today and would stay green against a scanner
    // that had stopped matching. This exercises the rule on input whose answer
    // is known, in both directions.
    let clean = "let b = common::binary().unwrap();\nassert!(out.status.success());";
    let dirty = "let b = common::binary().unwrap();\nassert!(err.contains(\"Cargo feature\"));";
    let unrelated = "let b = common::bin();\nassert!(err.contains(\"Cargo feature\"));";

    let hits = |t: &str| FEATURE_MARKERS.iter().any(|m| t.contains(m));

    assert!(
        !hits(clean),
        "a path-resolved test with no feature assertion is clean"
    );
    assert!(hits(dirty), "a feature assertion must be recognized");
    assert!(
        hits(unrelated),
        "the marker check itself is resolver-agnostic; it is the FILE LIST that \
         restricts it to path-resolving tests, and that separation is why the \
         list is built first"
    );
}
