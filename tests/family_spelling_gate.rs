// SPDX-License-Identifier: MIT OR Apache-2.0
//! Fail when an advertised dedicated command spelling cannot be dispatched.
//!
//! # The defect this exists for
//!
//! `RUN_DISPATCHED_CMDS` advertises `devtools3p-list`, `devtools3p-exec`,
//! `webmcp-list` and `webmcp-exec`, `canonical_step_cmd` maps all four, and the
//! dispatcher lists them. Their arms in `ext_steps` nevertheless guarded on the
//! `action` ALONE:
//!
//! ```text
//! "webmcp-list" | "webmcp" if step.get("action") == Some("list")
//! ```
//!
//! so the dedicated spelling had to repeat the action it already names. Without
//! that repetition no arm matched and the step fell to the family catch-all,
//! which is written for a routing bug. Measured 2026-09-01:
//! `{"cmd":"webmcp-list","url":"..."}` under `--category-webmcp` answered
//! `internal: unexpected cmd in this family: webmcp-list`. Four advertised
//! commands were unreachable.
//!
//! # Why `parity_run_inventory` could not catch it, and still cannot
//!
//! That gate's probe appends a sentinel step whose whole job is to be rejected
//! by PREFLIGHT, which is what proves the command under test resolved. The run
//! therefore dies before any step EXECUTES, so the probe never reaches the
//! dispatcher and no dispatch-layer phantom is visible to it.
//!
//! Measured while writing this file: adding the `internal:` string to that
//! gate's assertion left it green against the broken build, because the string
//! it was now looking for could not appear in the output it collects. A check
//! whose probe cannot reach the layer it judges is decorative, and that edit
//! was reverted rather than shipped.
//!
//! This gate pays for a real launch per spelling instead, which is the price of
//! asking a question about dispatch.

mod common;
use common::{binary_or_skip, chrome_not_ready, isolated_cmd};

const GATE: &str = "family_spelling_gate";

/// Advertised dedicated spellings, with the capability each one needs.
///
/// The capability flag is not optional detail: without it the run exits on the
/// capability gate BEFORE the dispatcher, which returns the expected failure
/// for the wrong reason and would let this gate pass on a broken build.
const SPELLINGS: &[(&str, &str)] = &[
    ("devtools3p-list", "--category-third-party"),
    ("devtools3p-exec", "--category-third-party"),
    ("webmcp-list", "--category-webmcp"),
    ("webmcp-exec", "--category-webmcp"),
];

#[test]
fn every_dedicated_family_spelling_reaches_its_arm() {
    let Some(bin) = binary_or_skip(GATE) else {
        return;
    };
    if chrome_not_ready(GATE, &bin) {
        return;
    }
    let dir = std::env::temp_dir().join(format!("ba-{GATE}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create work dir");

    let mut unreachable = Vec::new();
    for (cmd, capability) in SPELLINGS {
        let script = dir.join(format!("{cmd}.ndjson"));
        // `name` is present so `exec` gets past its own argument check and the
        // only thing left to fail on is routing.
        std::fs::write(
            &script,
            format!("{{\"cmd\":\"{cmd}\",\"url\":\"about:blank\",\"name\":\"probe\"}}\n"),
        )
        .expect("write probe script");

        let out = isolated_cmd(&bin)
            .args([
                capability,
                "--timeout",
                "60",
                "--json",
                "run",
                "--script",
                script.to_str().expect("script path is utf-8"),
            ])
            .output()
            .expect("spawn run --script");

        // The step is allowed to FAIL — there may be no third-party tool to
        // list on a blank page. What it may not do is fail as a routing bug,
        // because that answer means no arm claimed an advertised command.
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        if text.contains("internal: unexpected cmd in this family") {
            unreachable.push(*cmd);
        }
    }

    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        unreachable.is_empty(),
        "these spellings are advertised in RUN_DISPATCHED_CMDS and reach no dispatcher arm, \
         answering the family catch-all written for routing bugs: {unreachable:?}"
    );
}
