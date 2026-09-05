// SPDX-License-Identifier: MIT OR Apache-2.0
//! Permanent gate: an injected typo must be corrected, never delivered.
//!
//! # Why this costs a browser launch
//!
//! `input_typo_permille` is the one humanisation knob that changes WHAT the
//! page reads instead of WHEN it reads it. The unit tests prove the draw picks
//! a neighbouring key and that the default never fires; neither proves the
//! thing a caller depends on, which is that the field still holds exactly the
//! requested text after the correction ran.
//!
//! A regression here does not look like a crash. It looks like `type search`
//! leaving `sesarch` in the box, with `ok: true` and exit 0 — a wrong answer
//! wearing the shape of a right one.
//!
//! # Why the keydown count is asserted and not only the value
//!
//! Measured 2026-09-04 while writing this gate: the first version compared the
//! field value alone and passed identically at both rates, which read as proof
//! and was proof of nothing — the value is correct whether or not a typo was
//! ever injected. Counting `Backspace` is what separates "the correction works"
//! from "the correction never ran".
//!
//! | rate | keydown events | `Backspace` | field |
//! |---|---|---|---|
//! | default (`0`) | 6 | 0 | `search` |
//! | `1000` | 18 | 6 | `search` |
//!
//! Eighteen is three events per character: the wrong key, the `Backspace`, the
//! intended key.
//!
//! # Why the rate is pinned to 1000
//!
//! One thousand per thousand mistypes every character, so one short word
//! exercises the path once per letter. Any middle value would make this gate a
//! coin flip dressed as an assertion.
//!
//! # Skip policy
//!
//! No binary, or no usable Chrome, means SKIP LOUDLY — never a silent pass.
//! The fixture is a `data:` URL, so this gate makes NO network call.

use std::path::Path;

mod common;
use common::{binary, chrome_mentioned_in_doctor_json, missing_binary};

const GATE: &str = "typo_correction_gate";

/// The text the caller asked for, and the only text that may survive.
const WANTED: &str = "search";

/// A bare input plus a keydown recorder, inline, so no server is involved.
const PAGE: &str = "data:text/html,%3Cinput%20id%3Dq%3E%3Cscript%3Ewindow.K%3D%5B%5D%3Bdocument.getElementById%28%27q%27%29.addEventListener%28%27keydown%27%2Cfunction%28e%29%7Bwindow.K.push%28e.key%29%7D%29%3B%3C/script%3E";

fn cannot_run() -> bool {
    if missing_binary(GATE) {
        return true;
    }
    if !chrome_mentioned_in_doctor_json() {
        common::skip_with_remedy(
            GATE,
            "doctor reports no usable Chrome.",
            "install a system Chrome/Chromium.",
        );
        return true;
    }
    false
}

fn scratch() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("bac-typo-")
        .tempdir()
        .expect("create scratch dir")
}

fn set(cfg: &Path, key: &str, value: &str) {
    let out = common::isolated_cmd(&binary().expect("binary"))
        // `HOME` is what isolates config on macOS: `directories` resolves to
        // ~/Library/Application Support and never reads `XDG_CONFIG_HOME`.
        .env("HOME", cfg)
        .env("XDG_CONFIG_HOME", cfg)
        .args(["-q", "--json", "config", "set", key, value])
        .output()
        .expect("spawn config set");
    assert!(out.status.success(), "config set {key}={value}");
}

/// Type `WANTED` into the fixture and return the page's own report.
fn typed(dir: &Path) -> serde_json::Value {
    let script = dir.join("steps.jsonl");
    std::fs::write(
        &script,
        format!(
            "{{\"cmd\":\"goto\",\"url\":\"{PAGE}\"}}\n\
             {{\"cmd\":\"type\",\"text\":\"{WANTED}\",\"target\":\"#q\"}}\n\
             {{\"cmd\":\"eval\",\"expression\":\"JSON.stringify({{v:document.getElementById('q').value,bs:window.K.filter(function(k){{return k==='Backspace'}}).length,n:window.K.length}})\"}}\n"
        ),
    )
    .expect("write script");
    let out = common::isolated_cmd(&binary().expect("binary"))
        .env("HOME", dir)
        .env("XDG_CONFIG_HOME", dir)
        .args([
            "-q",
            "--json",
            "--timeout",
            "120",
            "run",
            "--script",
            &script.to_string_lossy(),
        ])
        .output()
        .expect("spawn run --script");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout was not JSON ({e}).\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    });
    assert_eq!(v["ok"], serde_json::json!(true), "fixture run failed: {v}");
    let raw = v["data"]["steps"]
        .as_array()
        .expect("steps array")
        .iter()
        .find_map(|s| {
            s.pointer("/data/result")
                .and_then(serde_json::Value::as_str)
        })
        .unwrap_or_default()
        .to_string();
    serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("page report was not JSON ({e}): {raw} / {}", v["data"]))
}

#[test]
fn every_injected_typo_is_erased_before_the_next_character() {
    if cannot_run() {
        return;
    }
    let scratch_dir = scratch();
    let dir = scratch_dir.path().to_path_buf();
    let letters = WANTED.chars().count() as u64;

    let quiet = typed(&dir);
    assert_eq!(
        quiet["v"], WANTED,
        "the default must type the text literally"
    );
    assert_eq!(
        quiet["bs"],
        serde_json::json!(0),
        "the default rate is zero, so no correction may fire: {quiet}"
    );
    assert_eq!(
        quiet["n"],
        serde_json::json!(letters),
        "one keydown per character and nothing more: {quiet}"
    );

    set(&dir, "input_typo_permille", "1000");
    let noisy = typed(&dir);
    assert_eq!(
        noisy["v"], WANTED,
        "every character was mistyped on purpose and every correction had to \
         land; {noisy} means a wrong key reached the caller's data"
    );
    assert_eq!(
        noisy["bs"],
        serde_json::json!(letters),
        "one correction per character was expected: {noisy}"
    );
    assert_eq!(
        noisy["n"],
        serde_json::json!(letters * 3),
        "wrong key, Backspace, intended key — three per character: {noisy}"
    );
}
