// SPDX-License-Identifier: MIT OR Apache-2.0
//! Fail when a CSS selector flag is accepted on the argv and never consumed.
//!
//! # Why this gate exists
//!
//! `gaps.md` names one class above all others: a flag the parser ACCEPTS and
//! the code never APPLIES, answering `ok: true` with exit 0. It has now been
//! found five times in this crate — `--hosts`, `--mitm-max-body-bytes`,
//! `mitm block`, `--format links --include-selector`, and the selector filter
//! itself. Each fix closed one instance. None of them closed the class, because
//! nothing in the tree compared "declared" against "consumed".
//!
//! `src/commands/run/inventory.rs` already closes that class for run STEPS,
//! with `every_dispatched_cmd_has_a_field_row`. This file is the analogue for
//! FLAGS, and it is deliberately written in the same shape.
//!
//! # What was measured
//!
//! Measured 2026-09-01 against 0.1.9, on `example.com` in `--format text`:
//!
//! - no selector: 127 characters, the whole page
//! - `--include-selector 'h1'`: 14 characters, correctly reduced
//! - `--include-selector 'nav.absent'`: 127 characters, the whole page
//! - `--include-selector 'h1['`: 127 characters, the whole page
//!
//! The last two returned `ok: true` and exit 0. A caller asking for a subset
//! received the full set, and no field in the envelope marked the difference.
//! `batch-scrape` reproduced it identically, 127 against 14.
//!
//! # Why this gate needs no network and no Chrome
//!
//! The malformed-selector cases are decided by clap, before a socket is opened:
//! `Selector::parse` returns a `Result`, so the typo is knowable from the argv
//! alone. Testing that against a live page would add a dependency on the public
//! internet to prove something that never leaves the process, and a gate that
//! can fail for reasons outside the code is a gate people learn to ignore.
//!
//! The behavioural half — that a selector matching nothing yields nothing —
//! lives in `src/scrape_local/html_sanitize.rs` as unit tests over a fixed
//! document, for the same reason.
//!
//! # Why the over-rejection case is here
//!
//! A validator that refuses EVERY selector would satisfy every assertion about
//! refusal in this file while breaking the feature completely. Fail-closed must
//! not quietly become fail-always, so one case asserts that a well-formed
//! selector is NOT refused.

mod common;

use common::{binary_or_skip, isolated_cmd, skip_with_reason};
use std::path::{Path, PathBuf};
use std::process::Command;

const GATE: &str = "selector_scope_gate";

/// Every command that declares the selector flags. Extend this list when a
/// fourth command grows them, and the parity case below will hold it honest.
const COMMANDS_DECLARING_SELECTORS: &[&str] = &["scrape", "batch-scrape", "crawl"];

/// Both halves of the surface under test.
const SELECTOR_FLAGS: &[&str] = &["--include-selector", "--exclude-selector"];

/// Flags DECLARED to belong to the whole family, not to one command.
///
/// This is a decision list and not an automatic diff, because the three
/// commands legitimately differ elsewhere: `--urls-file` belongs only to
/// `batch-scrape`, `--max-depth` only to `crawl`. Diffing every flag would
/// produce noise nobody reads. Naming the ones that MUST travel together
/// produces a failure someone acts on.
///
/// `--only-main-content` is on this list because it is the flag that reached
/// `scrape` and not `batch-scrape`, and a user found the gap instead of the
/// tree. That is precisely the discovery this list converts into a build
/// failure.
const FAMILY_WIDE_FLAGS: &[&str] = &[
    "--include-selector",
    "--exclude-selector",
    "--only-main-content",
    "--redact-pii",
];

/// The target each command needs to get PAST argument-shape checking.
///
/// The three commands do not take the same target: `batch-scrape` reads a file
/// of URLs and refuses a positional one. Passing the wrong shape makes clap
/// answer "unexpected argument" and exit 2 for a reason that has nothing to do
/// with selectors — which would let every assertion here pass while the
/// validator was gone. The target is never dereferenced, because the selector
/// is rejected during parsing, before any file or socket is touched.
fn target_args(cmd: &str) -> Vec<&'static str> {
    match cmd {
        "batch-scrape" => vec!["--urls-file", "urls-never-read-by-this-gate.txt"],
        // `.invalid` is reserved by RFC 6761 and guaranteed never to resolve,
        // so the well-formed case below ends at DNS instead of reaching a real
        // host. This file must not depend on the public internet.
        _ => vec!["https://example.invalid"],
    }
}

/// `["-q", "--json", cmd, flag, value, <target...>]`, as a single owned vec.
fn invocation<'a>(cmd: &'a str, flag: &'a str, value: &'a str) -> Vec<&'a str> {
    let mut args = vec!["-q", "--json", cmd, flag, value];
    args.extend(target_args(cmd));
    args
}

/// True when the host cannot run the gate. Prints why; never silently passes.
fn cannot_run() -> bool {
    binary_or_skip(GATE).is_none()
}

/// The binary, or `None` after the skip has already been reported.
fn bin_or_none() -> Option<PathBuf> {
    binary_or_skip(GATE)
}

/// Long help for `cmd`, which is where clap prints every declared flag.
fn long_help(bin: &Path, cmd: &str) -> String {
    let out = isolated_cmd(bin)
        .args([cmd, "--help"])
        .output()
        .expect("help must run");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// Run one invocation and return `(exit code, stdout + stderr)`.
fn run(bin: &Path, args: &[&str]) -> (i32, String) {
    let out: std::process::Output = {
        let mut c: Command = isolated_cmd(bin);
        c.args(args).output().expect("binary must run")
    };
    let merged = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code().unwrap_or(-1), merged)
}

/// ENVIRONMENT GUARD: this one never skips.
///
/// The other cases return early when the host is not ready, and a test that
/// returns counts as a PASS. That turns the whole file green while it tested
/// nothing, and the honest SKIP lines are easy to lose in `cargo test` output.
///
/// A test that fails the ENVIRONMENT is not a test that fails the CODE.
#[test]
fn the_host_can_actually_run_this_gate() {
    assert!(
        !cannot_run(),
        "host cannot run this gate: every other case in this file skipped, and \
         a skip is NOT a pass. The SKIP line on stderr names the missing \
         precondition."
    );
}

/// A malformed selector must die on the argv, in EVERY command that takes one.
///
/// This is the case that fails on 0.1.9: the filter swallowed the parse error
/// with `continue` and fell through to the untouched document.
#[test]
fn a_malformed_selector_is_refused_by_every_command_that_declares_it() {
    let Some(bin) = bin_or_none() else { return };
    // Two shapes of broken: an unterminated attribute selector, and a string
    // that is not a selector at all.
    for bad in ["h1[", ">>>bad<<<"] {
        for cmd in COMMANDS_DECLARING_SELECTORS {
            for flag in SELECTOR_FLAGS {
                let (code, out) = run(&bin, &invocation(cmd, flag, bad));
                assert_eq!(
                    code, 2,
                    "`{cmd} {flag} {bad}` exited {code}; a selector the CSS \
                     parser rejects is a usage error and must not reach a fetch. \
                     Output was: {out}"
                );
                assert!(
                    out.contains("invalid CSS selector"),
                    "`{cmd} {flag} {bad}` exited 2 for some OTHER reason, so this \
                     case would keep passing after the validator was removed. \
                     Output was: {out}"
                );
            }
        }
    }
}

/// Fail-closed must not become fail-always.
///
/// Without this, a validator that refused every selector would satisfy the
/// refusal case above while destroying the feature it protects.
#[test]
fn a_well_formed_selector_is_not_refused() {
    let Some(bin) = bin_or_none() else { return };
    for cmd in COMMANDS_DECLARING_SELECTORS {
        for flag in SELECTOR_FLAGS {
            // The target never resolves, so the run ends early. What matters
            // here is not that it failed, but WHICH complaint came back.
            let (_, out) = run(&bin, &invocation(cmd, flag, "article.main"));
            assert!(
                !out.contains("invalid CSS selector"),
                "`{cmd} {flag} article.main` was rejected as a bad selector, but \
                 `article.main` is valid CSS; the validator is over-rejecting. \
                 Output was: {out}"
            );
        }
    }
}

/// An empty selector is a typo, not a request for everything.
#[test]
fn an_empty_selector_is_refused() {
    let Some(bin) = bin_or_none() else { return };
    let (code, out) = run(
        &bin,
        &[
            "-q",
            "--json",
            "scrape",
            "--include-selector",
            "   ",
            "https://example.com",
        ],
    );
    assert_eq!(
        code, 2,
        "an all-whitespace selector must be refused rather than silently \
         meaning 'no filter'. Output was: {out}"
    );
}

/// THE CLASS GATE: a flag one command declares, its siblings declare too.
///
/// `--only-main-content` reached `scrape` and not `batch-scrape`, and the gap
/// was found by a user rather than by the tree. Enumerating the commands and
/// comparing them is what turns that from a discovery into a build failure.
#[test]
fn every_command_in_the_family_declares_the_same_selector_flags() {
    let Some(bin) = bin_or_none() else { return };
    let mut missing: Vec<String> = Vec::new();
    for cmd in COMMANDS_DECLARING_SELECTORS {
        let help = long_help(&bin, cmd);
        for flag in FAMILY_WIDE_FLAGS {
            if !help.contains(flag) {
                missing.push(format!("{cmd} lacks {flag}"));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "the selector family diverged across sibling commands: {missing:?}. \
         Either add the flag, or remove the command from \
         COMMANDS_DECLARING_SELECTORS and say in this file why it differs"
    );
}

/// The declared flags must still be REACHABLE, not merely printed in help.
///
/// A flag can survive in `--help` after its handler is deleted. Asserting that
/// each one still parses a good value keeps help and behaviour tied together.
///
/// # Why the message assertion is here and not only the exit code
///
/// A blind review of this file found that the first shape asserted `code == 2`
/// alone. Exit 2 is clap's code for EVERY usage error, so an unrelated argument
/// mistake — a renamed flag, a missing target — satisfied it just as well as the
/// validator firing. The case then passed for a reason its name does not claim,
/// which is the false-pass this whole file exists to hunt. Pinning the
/// validator's own words is what ties the exit code to the cause.
///
/// This overlaps `a_malformed_selector_is_refused_by_every_command_that_declares_it`
/// on purpose and the overlap is stated rather than hidden: that case sweeps two
/// shapes of broken selector, this one sweeps the FLAG SURFACE and would survive
/// a future rewrite that narrowed the other to a single command.
#[test]
fn each_declared_selector_flag_is_still_wired() {
    let Some(bin) = bin_or_none() else {
        skip_with_reason(GATE, "binary unavailable");
        return;
    };

    for cmd in COMMANDS_DECLARING_SELECTORS {
        for flag in SELECTOR_FLAGS {
            let (code, out) = run(&bin, &invocation(cmd, flag, "h1["));
            assert_eq!(
                code, 2,
                "`{cmd}` printed {flag} in its help but did not run it through \
                 the selector validator, so the flag is declared and unwired — \
                 the exact class this gate exists for. Output was: {out}"
            );
            assert!(
                out.contains("invalid CSS selector"),
                "`{cmd} {flag}` exited 2 without the validator's message, so this \
                 case would keep passing on ANY usage error and prove nothing \
                 about the flag being wired. Output was: {out}"
            );
        }
    }
}

/// The behavioural half must EXIST, and this case refuses to take a comment's
/// word for it.
///
/// # Why the behavioural half is not tested here
///
/// The defect that opened this file is a VALID selector matching nothing and
/// yielding the whole document. Proving that end-to-end needs a page with known
/// content, and `scrape` refuses `file://` by design — `src/scrape_local/scheme.rs`
/// answers "HTTP engine cannot fetch file:// URL". The only end-to-end route is
/// the public internet, and a gate that fails when a third party is down is a
/// gate people learn to skip.
///
/// So the coverage lives in `FILTER_MODULE` as unit tests over a fixed document.
///
/// # Why a comment saying that is not enough
///
/// A blind review of this file made the point that has to be answered rather
/// than explained away: nothing in this file FAILS if the fail-open bug returns.
/// The comment pointing at the other module is prose, and prose has no compiler
/// — the exact defect `GATE_FILE`'s sibling in `docs/` exists to catch.
///
/// This case turns the pointer into an assertion. If someone deletes the
/// behavioural tests, or renames them past recognition, the pointer becomes a
/// lie and THIS file goes red — which is the only way a delegation of coverage
/// can be honest.
#[test]
fn the_behavioural_half_this_file_delegates_still_exists() {
    let module =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/scrape_local/html_sanitize.rs");
    let src = std::fs::read_to_string(&module).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}. This file DELEGATES its behavioural coverage \
             there, so the module vanishing means the coverage vanished with it",
            module.display()
        )
    });
    // Named individually rather than counted: a count passes while the ONE case
    // that matters is replaced by two trivial ones.
    const DELEGATED: &[&str] = &[
        "an_include_selector_that_matches_nothing_returns_empty_not_the_document",
        "an_include_selector_that_matches_reduces_and_counts",
        "a_malformed_selector_is_refused_rather_than_swallowed",
    ];
    let missing: Vec<&str> = DELEGATED
        .iter()
        .copied()
        .filter(|name| !src.contains(name))
        .collect();
    assert!(
        missing.is_empty(),
        "this gate delegates its behavioural coverage to unit tests in {} and \
         these are gone: {missing:?}. Either restore them, or move the coverage \
         here and delete this case — but do NOT leave a comment claiming a \
         guarantee no test provides",
        module.display()
    );
}
