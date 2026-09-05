// SPDX-License-Identifier: MIT OR Apache-2.0
//! Explicit Target Designation is auditable from outside the process.
//!
//! # What this gate is for
//!
//! The product rule is that a side-effecting verb receives its target in argv
//! and never inherits one from ambient state. Measured 2026-08-18, that rule was
//! unenforceable: exactly one command (`config unset`) published which target it
//! resolved, so for every other verb an ambient-target violation produced an
//! envelope byte-identical to a compliant one. A rule nothing can observe is not
//! a rule.
//!
//! So this gate asserts the evidence, not the prose:
//!
//! - a side-effecting verb's success envelope carries BOTH `target_resolved` and
//!   `target_source`;
//! - `target_source` is one of the four stable tokens and nothing else;
//! - a verb that ambient-resolves says `ambient` rather than dressing the guess
//!   up as `argv` — hiding it would defeat the whole mechanism;
//! - a verb that refuses without an explicit target still refuses.
//!
//! # Why the split between Chrome-free and Chrome-backed cases
//!
//! The Chrome-free half runs everywhere and therefore carries the load-bearing
//! assertions. The Chrome-backed half needs a real browser and goes through
//! [`common::chrome_not_ready`], which under `--features strict-gates` turns a
//! skip into a failure — so on a host with Chrome it runs for real.
//!
//! # Why the element verbs are exercised through `run --script`
//!
//! `press`, `write`, `hover` and `submit` navigate to `about:blank` in their
//! one-shot form, so no selector can ever match and no success envelope exists
//! to inspect. Inside a script they act on a page a previous step navigated to,
//! which is the only path where their success shape is observable at all. There
//! the target came from a step field rather than process argv, and the token is
//! `step` accordingly.

mod common;

use std::path::PathBuf;

use serde_json::Value;

const GATE: &str = "etd_gate";

/// The four tokens `TargetSource` serialises to. Any other value is a contract
/// break, which is why this list is spelled out rather than pattern-matched.
const SOURCES: [&str; 4] = ["argv", "step", "xdg", "ambient"];

/// A scratch directory plus the XDG roots pointed at it.
///
/// The XDG variables are the operating system's own contract, not product
/// knobs: the product reads no environment variable of its own, and a test that
/// wrote into the operator's real config or state to prove a point would be
/// buying its assertion with someone else's data.
struct Scratch {
    dir: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir =
            std::env::temp_dir().join(format!("etd-gate-{tag}-{}-{nanos}", std::process::id()));
        // `mitm` is created up front: the store writes its rules file with a
        // tmp+rename that does not create the parent, so a fresh XDG root would
        // fail on I/O for a reason unrelated to what this gate measures.
        std::fs::create_dir_all(dir.join("state/browser-automation-cli/mitm"))
            .expect("scratch state dir");
        std::fs::create_dir_all(dir.join("cfg")).expect("scratch config dir");
        Self { dir }
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.dir.join(rel)
    }

    fn cmd(&self, args: &[&str]) -> (i32, Value) {
        let out = common::cmd()
            .env("XDG_STATE_HOME", self.dir.join("state"))
            .env("XDG_DATA_HOME", self.dir.join("data"))
            // `HOME` is what isolates config on macOS: `directories` resolves to
            // ~/Library/Application Support and never reads `XDG_CONFIG_HOME`.
            // Full measurement (2026-09-04) lives in `tests/scrape_wave6_gate.rs`.
            .env("HOME", self.dir.join("cfg"))
            .env("XDG_CONFIG_HOME", self.dir.join("cfg"))
            .env("XDG_CACHE_HOME", self.dir.join("cache"))
            .args(["-q", "--json"])
            .args(args)
            .output()
            .expect("spawn browser-automation-cli");
        let v: Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
            panic!(
                "{GATE}: {args:?} did not emit a JSON envelope ({e}); stdout={}",
                String::from_utf8_lossy(&out.stdout)
            )
        });
        (out.status.code().unwrap_or(-1), v)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Assert one envelope's `data` carries both fields with the expected source.
fn assert_target(label: &str, env: &Value, expected: &str) {
    assert_eq!(env["ok"], true, "{GATE}: {label} did not succeed: {env}");
    let data = &env["data"];
    let resolved = data["target_resolved"]
        .as_str()
        .unwrap_or_else(|| panic!("{GATE}: {label} published no target_resolved: {data}"));
    assert!(
        !resolved.is_empty(),
        "{GATE}: {label} published an empty target_resolved"
    );
    let source = data["target_source"]
        .as_str()
        .unwrap_or_else(|| panic!("{GATE}: {label} published no target_source: {data}"));
    assert!(
        SOURCES.contains(&source),
        "{GATE}: {label} published target_source={source:?}, outside {SOURCES:?}"
    );
    assert_eq!(
        source, expected,
        "{GATE}: {label} published target_source={source:?}, expected {expected:?}"
    );
}

// --------------------------------------------------------------------------
// Chrome-free verbs. These carry the assertions that must hold on every host.
// --------------------------------------------------------------------------

#[test]
fn sheet_write_publishes_the_workbook_it_overwrote() {
    let s = Scratch::new("sheet");
    let csv = s.path("rows.csv");
    std::fs::write(&csv, "a,b\n1,2\n").expect("write csv");
    let out = s.path("out.xlsx");
    let (code, env) = s.cmd(&[
        "sheet-write",
        csv.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--force",
    ]);
    assert_eq!(code, 0, "{GATE}: sheet-write exit {code}: {env}");
    assert_target("sheet-write", &env, "argv");
    assert_eq!(env["data"]["target_resolved"], out.display().to_string());
}

#[test]
fn sg_rewrite_distinguishes_a_named_root_from_a_defaulted_one() {
    let s = Scratch::new("sg");
    let tree = s.path("tree");
    std::fs::create_dir_all(&tree).expect("tree");
    std::fs::write(tree.join("a.rs"), "fn main() {}\n").expect("write rs");

    let (code, named) = s.cmd(&["sg-rewrite", tree.to_str().unwrap()]);
    assert_eq!(code, 0, "{GATE}: sg-rewrite exit {code}: {named}");
    assert_target("sg-rewrite <root>", &named, "argv");

    // The reporting path still defaults to `.`, and the point of the field is
    // that it admits so instead of presenting the default as a caller choice.
    let out = common::cmd()
        .current_dir(&tree)
        .args(["-q", "--json", "sg-rewrite"])
        .output()
        .expect("spawn");
    let bare: Value = serde_json::from_slice(&out.stdout).expect("envelope");
    assert_target("sg-rewrite (no root)", &bare, "ambient");
}

#[test]
fn mitm_writers_publish_the_rule_they_persisted() {
    let s = Scratch::new("mitm");
    let (code, allow) = s.cmd(&["mitm", "allow", "--host", "example.com"]);
    assert_eq!(code, 0, "{GATE}: mitm allow exit {code}: {allow}");
    assert_target("mitm allow", &allow, "argv");

    let (code, block) = s.cmd(&["mitm", "block", "--host", "example.com", "--path", "/ads"]);
    assert_eq!(code, 0, "{GATE}: mitm block exit {code}: {block}");
    assert_target("mitm block", &block, "argv");
    assert_eq!(block["data"]["target_resolved"], "example.com/ads");
}

#[test]
fn config_unset_still_publishes_the_key_it_cleared() {
    let s = Scratch::new("cfg");
    let (code, env) = s.cmd(&["config", "unset", "timeout"]);
    assert_eq!(code, 0, "{GATE}: config unset exit {code}: {env}");
    assert_target("config unset", &env, "argv");
    assert_eq!(env["data"]["target_resolved"], "timeout");
}

#[test]
fn a_verb_that_requires_an_explicit_target_refuses_without_one() {
    let s = Scratch::new("refuse");
    // `mitm block` with neither --host nor --path.
    let (code, env) = s.cmd(&["mitm", "block"]);
    assert_eq!(code, 2, "{GATE}: bare `mitm block` exit {code}: {env}");
    assert_eq!(env["ok"], false);
    assert_eq!(env["error"]["kind"], "usage");

    // `sg-rewrite --apply` must not fall back to the current directory.
    let (code, env) = s.cmd(&["sg-rewrite", "--apply"]);
    assert_eq!(code, 2, "{GATE}: `sg-rewrite --apply` exit {code}: {env}");
    assert_eq!(env["ok"], false);
    assert_eq!(env["error"]["kind"], "usage");

    // `cookie clear` must not infer "the whole jar" from an absent argument.
    //
    // This one needs no browser to prove: the refusal happens in the parser,
    // before anything is launched, which is also why a destructive verb is the
    // right place to spend a required flag.
    let (code, env) = s.cmd(&["cookie", "clear"]);
    assert_eq!(code, 2, "{GATE}: bare `cookie clear` exit {code}: {env}");
    assert_eq!(env["ok"], false);
    assert_eq!(env["error"]["kind"], "usage");
}

// --------------------------------------------------------------------------
// Chrome-backed verbs.
// --------------------------------------------------------------------------

#[test]
fn ambient_targets_are_reported_as_ambient() {
    let bin = common::bin();
    if common::chrome_not_ready(GATE, &bin) {
        return;
    }
    let s = Scratch::new("ambient");
    // `cookie clear` used to be listed here, and that was the bug rather than
    // the contract: it is destructive, its scope is the entire jar, and nothing
    // in argv chose it. It now requires `--all` and reports `argv`, so it lives
    // in the argv test below and in the fail-closed test above.
    for (label, args) in [
        ("keys", vec!["keys", "Enter"]),
        ("scroll", vec!["scroll", "--delta-y", "100"]),
    ] {
        let (code, env) = s.cmd(&args);
        assert_eq!(code, 0, "{GATE}: {label} exit {code}: {env}");
        assert_target(label, &env, "ambient");
    }
}

#[test]
fn argv_targets_are_reported_as_argv() {
    let bin = common::bin();
    if common::chrome_not_ready(GATE, &bin) {
        return;
    }
    let s = Scratch::new("argv");
    let png = s.path("p.png");
    let auth = s.path("auth.json");
    for (label, args, expect) in [
        (
            "grab",
            vec!["grab", "--path", png.to_str().unwrap(), "--format", "png"],
            png.display().to_string(),
        ),
        (
            "storage export",
            vec!["storage", "export", "--path", auth.to_str().unwrap()],
            auth.display().to_string(),
        ),
        (
            "page new",
            vec!["page", "new", "--url", "about:blank"],
            "about:blank".to_string(),
        ),
        // The scope is still the whole jar — CDP offers no partial clear — but
        // `--all` is what puts it in argv. This asserts the source flipped from
        // `ambient` to `argv`, which is the whole point of requiring the flag.
        (
            "cookie clear --all",
            vec!["cookie", "clear", "--all"],
            "all".to_string(),
        ),
    ] {
        let (code, env) = s.cmd(&args);
        assert_eq!(code, 0, "{GATE}: {label} exit {code}: {env}");
        assert_target(label, &env, "argv");
        assert_eq!(env["data"]["target_resolved"], expect, "{GATE}: {label}");
    }
}

#[test]
fn element_verbs_report_the_step_field_they_read() {
    let bin = common::bin();
    if common::chrome_not_ready(GATE, &bin) {
        return;
    }
    let s = Scratch::new("steps");
    let page = s.path("f.html");
    std::fs::write(
        &page,
        "<html><body><form id=\"f\"><input id=\"u\" name=\"u\">\
         <button id=\"b\" type=\"button\">go</button></form></body></html>",
    )
    .expect("write fixture");
    let script = s.path("s.jsonl");
    let url = format!("file://{}", page.display());
    std::fs::write(
        &script,
        format!(
            "{}\n{}\n{}\n{}\n{}\n",
            serde_json::json!({"cmd": "goto", "url": url}),
            serde_json::json!({"cmd": "write", "target": "#u", "value": "hello"}),
            serde_json::json!({"cmd": "press", "target": "#b"}),
            serde_json::json!({"cmd": "hover", "target": "#b"}),
            serde_json::json!({"cmd": "submit", "target": "#u"}),
        ),
    )
    .expect("write script");

    let (code, env) = s.cmd(&["run", "--script", script.to_str().unwrap()]);
    assert_eq!(code, 0, "{GATE}: run exit {code}: {env}");
    let steps = env["data"]["steps"].as_array().expect("steps array");
    let mut seen = 0;
    for step in steps {
        let cmd = step["cmd"].as_str().unwrap_or_default();
        if matches!(cmd, "write" | "press" | "hover" | "submit") {
            assert_target(cmd, step, "step");
            seen += 1;
        }
    }
    assert_eq!(seen, 4, "{GATE}: expected four element steps, saw {seen}");
}

/// A `goto` step is navigation, not a target designation, so it deliberately
/// carries no annotation. Asserting that keeps the field meaningful: if every
/// step grew one, its presence would stop distinguishing anything.
#[test]
fn a_pure_navigation_step_is_left_unannotated() {
    let bin = common::bin();
    if common::chrome_not_ready(GATE, &bin) {
        return;
    }
    let s = Scratch::new("nav");
    let script = s.path("n.jsonl");
    std::fs::write(
        &script,
        format!(
            "{}\n",
            serde_json::json!({"cmd": "goto", "url": "about:blank"})
        ),
    )
    .expect("write script");
    let (code, env) = s.cmd(&["run", "--script", script.to_str().unwrap()]);
    assert_eq!(code, 0, "{GATE}: run exit {code}: {env}");
    let goto = &env["data"]["steps"][0];
    assert_eq!(goto["cmd"], "goto");
    assert!(
        goto["data"]["target_source"].is_null(),
        "{GATE}: goto grew a target_source: {goto}"
    );
}
