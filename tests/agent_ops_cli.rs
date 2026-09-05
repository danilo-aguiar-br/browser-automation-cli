// SPDX-License-Identifier: MIT OR Apache-2.0
//! Integration tests for the eight universal agent-ops flags.
//!
//! # Why these exist
//!
//! `src/agent_ops/tests.rs` covers `apply()` in-process and passes. It passed
//! while `doctor` discarded the over-budget error those very tests assert on,
//! answering exit 0 with an empty stdout — the unit test and the defect lived in
//! different files and nothing crossed the boundary.
//!
//! Before this file, `tests/` had zero coverage of these flags: the only
//! `--fields` match in the directory was `--fields-json` from `fill-form`. Every
//! assertion here spawns the real binary and reads the real exit code, because
//! that is the contract an agent depends on.

use tempfile::TempDir;

mod common;

/// Spawn the CLI with `HOME` pointed at a throwaway directory.
fn run(tmp: &TempDir, args: &[&str]) -> (i32, String, String) {
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).expect("create temp home");

    let out = common::cmd()
        .args(args)
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn browser-automation-cli");

    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn an_impossible_ceiling_is_an_error_not_an_empty_success() {
    let tmp = TempDir::new().unwrap();
    let (code, stdout, stderr) = run(
        &tmp,
        &[
            "-q",
            "--json",
            "--max-output-bytes",
            "10",
            "doctor",
            "--offline",
            "--quick",
        ],
    );

    // The regression this pins: exit 0 with an empty stdout reads to an agent as
    // "the doctor ran and everything is fine".
    assert_ne!(
        code, 0,
        "an unmeetable ceiling must not report success\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        !stdout.trim().is_empty(),
        "an unmeetable ceiling must say something\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("\"ok\":false"),
        "expected an error envelope, got:\n{stdout}"
    );
}

#[test]
fn an_operationally_plausible_ceiling_never_goes_silent() {
    // 4000 bytes is a value an agent would reasonably pass; the doctor payload
    // is ~26 KiB, so this is the realistic shape of the failure, not a 10-byte
    // edge case.
    let tmp = TempDir::new().unwrap();
    let (_code, stdout, stderr) = run(
        &tmp,
        &[
            "-q",
            "--json",
            "--max-output-bytes",
            "4000",
            "doctor",
            "--offline",
            "--quick",
        ],
    );
    assert!(
        !stdout.trim().is_empty(),
        "doctor emitted nothing at a 4000-byte ceiling\nstderr:\n{stderr}"
    );
}

#[test]
fn other_commands_already_reported_the_ceiling_correctly() {
    // Control: `version` always behaved. If this ever fails, the regression is
    // in `agent_ops` itself and not in one command's error handling.
    let tmp = TempDir::new().unwrap();
    let (code, stdout, _) = run(
        &tmp,
        &["-q", "--json", "--max-output-bytes", "10", "version"],
    );
    assert_eq!(code, 2, "want usage exit, got {code}\n{stdout}");
    assert!(stdout.contains("\"ok\":false"), "{stdout}");
}

#[test]
fn fields_with_a_missing_path_names_it() {
    let tmp = TempDir::new().unwrap();
    let (_code, stdout, _) = run(
        &tmp,
        &[
            "-q",
            "--json",
            "--fields",
            "nao.existe.mesmo",
            "doctor",
            "--offline",
            "--quick",
        ],
    );
    // Used to be `{"schema_version":1,"ok":true,"data":{}}` — indistinguishable
    // from a field that exists and is empty.
    assert!(
        stdout.contains("unresolved_paths"),
        "a misspelled path must be named, got:\n{stdout}"
    );
    assert!(
        stdout.contains("nao.existe.mesmo"),
        "the report must echo the path as typed, got:\n{stdout}"
    );
}

#[test]
fn fields_that_resolve_leave_the_envelope_quiet() {
    let tmp = TempDir::new().unwrap();
    let (_code, stdout, _) = run(
        &tmp,
        &[
            "-q",
            "--json",
            "--fields",
            "checks",
            "doctor",
            "--offline",
            "--quick",
        ],
    );
    assert!(
        !stdout.contains("unresolved_paths"),
        "a clean projection must not add noise, got:\n{stdout}"
    );
}

#[test]
fn sort_rows_with_a_missing_key_is_no_longer_indistinguishable() {
    // `sort_rows` falls into `(None, None) => Ordering::Equal` and `sort_by` is
    // stable, so a missing key produced a perfect no-op with matched == total.
    let tmp = TempDir::new().unwrap();
    let (_code, stdout, _) = run(
        &tmp,
        &[
            "-q",
            "--json",
            "--sort-rows",
            "chave_que_nao_existe",
            "doctor",
            "--offline",
            "--quick",
        ],
    );
    assert!(
        stdout.contains("unresolved_paths"),
        "a sort key nobody carries must be reported, got:\n{stdout}"
    );
}

#[test]
fn dedupe_by_with_a_missing_key_is_no_longer_read_as_all_unique() {
    // A missing dedupe key returned matched == total, which reads as "every row
    // was unique" rather than "I could not find that key".
    let tmp = TempDir::new().unwrap();
    let (_code, stdout, _) = run(
        &tmp,
        &[
            "-q",
            "--json",
            "--dedupe-by",
            "chave_que_nao_existe",
            "doctor",
            "--offline",
            "--quick",
        ],
    );
    assert!(
        stdout.contains("unresolved_paths"),
        "a dedupe key nobody carries must be reported, got:\n{stdout}"
    );
}

#[test]
fn agent_ops_suggestions_never_cite_a_flag_outside_their_scope() {
    // The `agent-ops-*` messages are emitted for ANY of the 71 commands, so they
    // may only mention GLOBAL flags. They used to suggest `--select`, which is a
    // per-command flag on scrape/crawl/map/search and the media `info` verbs, so
    // following the advice anywhere else produced `unexpected argument`.
    let tmp = TempDir::new().unwrap();
    let (_code, help, _) = run(&tmp, &["--help"]);

    // A command with no list triggers the `no rows` suggestion.
    let (_code, stdout, _) = run(&tmp, &["-q", "--json", "--limit-rows", "1", "version"]);
    let suggestion = stdout
        .split("\"suggestion\":\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .unwrap_or_default()
        .to_string();
    assert!(
        !suggestion.is_empty(),
        "expected a suggestion in:\n{stdout}"
    );

    for flag in suggestion
        .split_whitespace()
        .filter(|w| w.starts_with("--"))
    {
        let flag = flag.trim_end_matches([',', '.', ';']);
        assert!(
            help.contains(flag),
            "suggestion cites {flag}, which is absent from the global help.\n\
             suggestion: {suggestion}"
        );
    }
}

#[test]
fn count_only_replaces_the_payload_with_a_count() {
    let tmp = TempDir::new().unwrap();
    let (code, stdout, _) = run(
        &tmp,
        &[
            "-q",
            "--json",
            "--fields",
            "checks",
            "--count-only",
            "doctor",
            "--offline",
            "--quick",
        ],
    );
    assert!(code == 0 || code == 1, "unexpected exit {code}\n{stdout}");
    assert!(stdout.contains("\"count\""), "{stdout}");
}

#[test]
fn truncate_content_marks_the_cut() {
    let tmp = TempDir::new().unwrap();
    let (_code, stdout, _) = run(
        &tmp,
        &[
            "-q",
            "--json",
            "--truncate-content",
            "5",
            "doctor",
            "--offline",
            "--quick",
        ],
    );
    // `truncated` is the only way to tell a short payload from a cut one.
    assert!(stdout.contains("\"truncated\":true"), "{stdout}");
}
