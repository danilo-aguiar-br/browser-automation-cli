//! `run` dispatch inventory: every browser-side top-level command is either in
//! `run` dispatch or in the intentional-exclude list.
//!
//! Scope note: this gate reads command NAMES. It does not look at the help text
//! attached to those commands, so a green run here is not evidence that every
//! subcommand carries a description.

use browser_automation_cli::commands::run::{INTENTIONAL_RUN_EXCLUDE, RUN_DISPATCHED_CMDS};

/// Browser-side commands that agents may expect inside multi-step `run`.
const BROWSER_SIDE_TOP_LEVEL: &[&str] = &[
    "goto",
    "view",
    "press",
    "write",
    "keys",
    "type",
    "wait",
    "hover",
    "drag",
    "fill-form",
    "upload",
    "back",
    "forward",
    "reload",
    "eval",
    "grab",
    "print-pdf",
    "extract",
    "text",
    "scroll",
    "cookie",
    "attr",
    "assert",
    "console",
    "net",
    "page",
    "dialog",
    "scrape",
    "emulate",
    "resize",
    "perf",
    "lighthouse",
    "screencast",
    "heap",
    "extension",
    "click-at",
];

#[test]
fn print_pdf_is_dispatched_in_run() {
    assert!(
        RUN_DISPATCHED_CMDS.contains(&"print-pdf") || RUN_DISPATCHED_CMDS.contains(&"print_pdf"),
        "print-pdf must be in RUN_DISPATCHED_CMDS"
    );
}

#[test]
fn browser_side_top_level_covered_by_run_or_exclude() {
    let dispatched: std::collections::BTreeSet<&str> =
        RUN_DISPATCHED_CMDS.iter().copied().collect();
    let excluded: std::collections::BTreeSet<&str> =
        INTENTIONAL_RUN_EXCLUDE.iter().map(|(c, _)| *c).collect();
    let mut missing = Vec::new();
    for cmd in BROWSER_SIDE_TOP_LEVEL {
        let underscored = cmd.replace('-', "_");
        let ok = dispatched.contains(cmd)
            || dispatched.contains(underscored.as_str())
            || excluded.contains(cmd)
            || excluded.iter().any(|e| e.starts_with(cmd));
        if !ok {
            missing.push(*cmd);
        }
    }
    assert!(
        missing.is_empty(),
        "browser-side cmds missing from run dispatch and intentional exclude: {missing:?}"
    );
}

#[test]
fn intentional_exclude_has_reasons() {
    for (cmd, reason) in INTENTIONAL_RUN_EXCLUDE {
        assert!(!cmd.is_empty());
        assert!(!reason.is_empty(), "exclude {cmd} must document a reason");
    }
}

// ---------------------------------------------------------------------------
// The inventory against the RUNNING dispatcher.
//
// Everything above compares two lists in the same crate. What follows asks the
// binary, because the drift this file now locks was invisible to a list-vs-list
// check: `RUN_DISPATCHED_CMDS` and the dispatcher slices are different files
// with no compiler relationship, so both could be internally consistent and
// still disagree about which commands exist.
// ---------------------------------------------------------------------------

mod common;

/// A trailing step that always fails preflight, so nothing is dispatched.
///
/// A command that IS recognised would otherwise run — and a `goto` that runs
/// launches Chrome, which would make a question about the command inventory
/// depend on a browser. `validate_steps` walks every step before BORN and stops
/// at the first error, so the walk still reaches the step under test.
const PREFLIGHT_SENTINEL: &str = r#"{"cmd":"cmd_inexistente_sentinela_parity"}"#;

fn validate_cmd(cmd: &str) -> String {
    let dir = tempfile::tempdir().expect("tempdir");
    let script = dir.path().join("s.jsonl");
    std::fs::write(
        &script,
        format!("{{\"cmd\":\"{cmd}\"}}\n{PREFLIGHT_SENTINEL}\n"),
    )
    .expect("write script");
    let out = common::cmd()
        .args(["--json", "run", "--script"])
        .arg(&script)
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn run");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// Nothing this list advertises may be refused as unknown.
///
/// Measured 2026-08-31 before the fix: `devtools3p_list`, `devtools3p_exec`,
/// `webmcp_list` and `webmcp_exec` were all answered with `unknown script cmd`,
/// and the suggestion attached to that refusal was `run_supported_suggestion`,
/// which had just listed them. A caller reading the error was told the command
/// does not exist by a sentence naming it.
#[test]
fn every_advertised_cmd_is_recognised_by_the_dispatcher() {
    if common::binary_or_skip("parity_run_inventory").is_none() {
        return;
    }
    let mut phantom = Vec::new();
    for cmd in RUN_DISPATCHED_CMDS {
        let out = validate_cmd(cmd);
        // The sentinel is what the run must die on. Seeing the sentinel's own
        // name proves the step under test got past command resolution.
        if out.contains(&format!("unknown script cmd: {cmd}")) {
            phantom.push(*cmd);
        }
    }
    assert!(
        phantom.is_empty(),
        "RUN_DISPATCHED_CMDS advertises commands the dispatcher refuses: {phantom:?}"
    );
}

/// Spellings the dispatcher has always run and the inventory never named.
///
/// `press | click` and `write | fill` were arms of the `match`; `submit`,
/// `screenshot`, `devtools3p` and `webmcp` were entries in the family slices.
/// All six worked, and `commands` did not list any of them, so an agent
/// discovering the surface through the published inventory could not find them.
#[test]
fn alias_spellings_the_dispatcher_runs_are_advertised() {
    for cmd in [
        "submit",
        "click",
        "fill",
        "screenshot",
        "devtools3p",
        "webmcp",
    ] {
        assert!(
            RUN_DISPATCHED_CMDS.contains(&cmd),
            "{cmd} is dispatched and must be advertised"
        );
    }
}

/// The four spellings that were advertised and never existed.
///
/// Kept as an explicit denial rather than as an absence: the underscore forms
/// are a plausible guess for anyone normalising `devtools3p-list`, and a list
/// that merely stops containing them does not record that they were WRONG.
#[test]
fn phantom_underscore_spellings_are_gone() {
    for cmd in [
        "devtools3p_list",
        "devtools3p_exec",
        "webmcp_list",
        "webmcp_exec",
    ] {
        assert!(
            !RUN_DISPATCHED_CMDS.contains(&cmd),
            "{cmd} has no dispatch arm and must not be advertised"
        );
    }
}
