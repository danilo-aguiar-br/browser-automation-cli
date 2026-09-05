// SPDX-License-Identifier: MIT OR Apache-2.0
//! What the scanner must DECLINE, which is the half a positive fixture cannot prove.
//!
//! Every case here drives [`super::sg_scan`] over a temporary tree, so the file
//! carries fixtures and table-driven assertions rather than logic. It sits in
//! its own module for the reason `heap_snapshot` already does the same: a
//! `#[cfg(test)]` block long enough to describe four separate exemptions stops
//! being an appendix to the scanner and starts hiding it.

use super::*;
use std::io::Write;

/// Write `body` at `rel` under `root`, creating parent directories.
fn write_source(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("fixture parent dir");
    }
    let mut file = fs::File::create(&path).expect("fixture file");
    writeln!(file, "{body}").expect("write fixture");
}

/// Paths reported for one rule, so an assertion can name the rule it means.
fn paths_for(v: &Value, rule: &str) -> Vec<String> {
    v.get("findings")
        .and_then(|f| f.as_array())
        .map(|items| {
            items
                .iter()
                .filter(|f| f.get("rule").and_then(|r| r.as_str()) == Some(rule))
                .map(|f| {
                    f.get("path")
                        .and_then(|p| p.as_str())
                        .unwrap_or_default()
                        .to_string()
                })
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn scan_finds_unwrap_in_temp_rs() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_source(
        dir.path(),
        "prod.rs",
        "fn x() { let _ = \"a\".parse::<u32>().unwrap(); }",
    );
    let v = sg_scan(&[dir.path().to_path_buf()], 50).expect("scan");
    assert_eq!(
        paths_for(&v, "unwrap_prod").len(),
        1,
        "a naked unwrap in a production file is the whole point of the rule"
    );
}

/// The negative half: what the rule must let through.
///
/// # Why this test has to exist
///
/// A scanner is only as good as the cases it declines. Asserting `count >= 1`
/// on a positive fixture passes against a function that reports every line,
/// which is exactly the failure mode measured on 2026-08-25: 323 test
/// `unwrap()` calls reported as production violations, with the single test
/// of this module green throughout.
#[test]
fn the_unwrap_rule_exempts_every_shape_of_test_code() {
    let dir = tempfile::tempdir().expect("tempdir");
    let unwrap_line = "fn x() { let _ = \"a\".parse::<u32>().unwrap(); }";
    // Integration test directory.
    write_source(dir.path(), "tests/it.rs", unwrap_line);
    // In-crate module whose `#[cfg(test)]` lives in the PARENT file.
    write_source(dir.path(), "residual/tests.rs", unwrap_line);
    // The shared helper.
    write_source(dir.path(), "test_utils.rs", unwrap_line);
    // Inline block inside an otherwise production file.
    write_source(
        dir.path(),
        "inline.rs",
        "pub fn prod() -> u32 { 1 }\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() { let _ = \"a\".parse::<u32>().unwrap(); }\n}",
    );
    let v = sg_scan(&[dir.path().to_path_buf()], 50).expect("scan");
    assert_eq!(
        paths_for(&v, "unwrap_prod"),
        Vec::<String>::new(),
        "test code is exempt from the production unwrap rule"
    );
}

/// `Command::env` is the product's own isolation call, not a dotenv read.
#[test]
fn the_dotenv_rule_separates_the_env_file_from_the_env_method() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_source(
        dir.path(),
        "cmd.rs",
        "pub fn spawn(c: &mut Command) { c.env(\"NO_COLOR\", \"1\"); }",
    );
    write_source(
        dir.path(),
        "loader.rs",
        "pub fn load() { let _ = std::fs::read(\".env\"); }",
    );
    let v = sg_scan(&[dir.path().to_path_buf()], 50).expect("scan");
    let hits = paths_for(&v, "dotenv");
    assert_eq!(
        hits.len(),
        1,
        "exactly the dotenv file must match, never the builder method: {hits:?}"
    );
    assert!(
        hits[0].ends_with("loader.rs"),
        "the match must be the file that reads .env, got {hits:?}"
    );
}

/// Prose that describes a rule must not be reported as breaking it.
///
/// # Why this test has to exist
///
/// A scanner with no notion of comment has a perverse incentive attached:
/// the better a rule is documented, the more violations it reports.
/// Measured on 2026-08-25, five of sixteen findings over `src/` and
/// `tests/` were sentences asserting the conformance, `src/lib.rs`
/// promising "no `.env` at runtime" among them.
#[test]
fn a_comment_that_names_a_forbidden_pattern_is_not_a_violation() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_source(
        dir.path(),
        "documented.rs",
        "//! This crate reads no .env at runtime and ships no datadog client.\n\
         /// Never calls `.unwrap()` in production.\n\
         // std::env::var(\"API_KEY\") is forbidden here.\n\
         pub fn prod() -> u32 { 1 }",
    );
    let v = sg_scan(&[dir.path().to_path_buf()], 50).expect("scan");
    assert_eq!(
        v.get("count").and_then(|c| c.as_u64()),
        Some(0),
        "documentation of a rule is not a breach of it: {v}"
    );
}

/// The file that defines the rules is exempt from them.
///
/// # Why this test has to exist
///
/// A rule's pattern is an instance of what the rule hunts, so the definer
/// always breaks every rule it declares. Measured on 2026-08-25, seven of
/// sixteen findings were this module scanning itself. The fixture uses a
/// telemetry string on purpose: that rule grants no test exemption, so a
/// pass here can only come from the path exemption under test.
#[test]
fn the_file_that_defines_the_rules_is_exempt_from_them() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_source(
        dir.path(),
        "src/sg_local.rs",
        "pub fn r() -> &'static str { \"datadog\" }",
    );
    write_source(
        dir.path(),
        "src/other.rs",
        "pub fn r() -> &'static str { \"datadog\" }",
    );
    let v = sg_scan(&[dir.path().to_path_buf()], 50).expect("scan");
    let hits = paths_for(&v, "telemetry_string");
    assert_eq!(
        hits.len(),
        1,
        "only the definer is exempt, never its neighbours: {hits:?}"
    );
    assert!(
        hits[0].ends_with("other.rs"),
        "the surviving hit must be the ordinary file, got {hits:?}"
    );
}

/// The test exemption belongs to one rule, not to test files in general.
///
/// # Why this test has to exist
///
/// The exemption used to be `if *rule == "unwrap_prod"` inside the scan
/// loop. Turning it into a field risks the opposite error — granting every
/// rule the pass that only one has earned — and nothing else in this module
/// would catch that. A telemetry string in a fixture is still a telemetry
/// string in the shipped test binary.
#[test]
fn only_the_unwrap_rule_is_exempt_inside_test_code() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_source(
        dir.path(),
        "tests/it.rs",
        "fn t() { let _ = \"a\".parse::<u32>().unwrap(); let _ = \"datadog\"; }",
    );
    let v = sg_scan(&[dir.path().to_path_buf()], 50).expect("scan");
    assert_eq!(
        paths_for(&v, "unwrap_prod"),
        Vec::<String>::new(),
        "the unwrap rule exempts test code"
    );
    assert_eq!(
        paths_for(&v, "telemetry_string").len(),
        1,
        "the telemetry rule does not: {v}"
    );
}
