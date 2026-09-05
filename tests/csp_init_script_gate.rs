// SPDX-License-Identifier: MIT OR Apache-2.0
//! Permanent gate: a page CSP must not reach the stealth payload.
//!
//! # Why this gate exists at all
//!
//! `docs/STEALTH_PARITY.md` carried this vector as `ABSENT`, on the evidence
//! that the tree holds no `Page.setBypassCSP` and no CSP header rewrite. That
//! evidence was TRUE and the conclusion was WRONG: the defence is not missing,
//! it is unnecessary. A script installed through
//! `Page.addScriptToEvaluateOnNewDocument` is not page script, so the page's
//! own `Content-Security-Policy` never governs it.
//!
//! Measured 2026-09-04 against a loopback page served with
//! `Content-Security-Policy: script-src 'none'; object-src 'none'`, on a macOS
//! host under `--stealth-profile chrome-linux`:
//!
//! | read | value |
//! |---|---|
//! | `navigator.webdriver` | `false` |
//! | `navigator.platform` | `Linux x86_64` |
//! | `getParameter(37446)` | `ANGLE (NVIDIA, NVIDIA GeForce GTX 1070, OpenGL 4.6)` |
//!
//! The host is an Apple M1 Max, so the renderer above is the mask and not the
//! truth.
//!
//! # Which of those reads actually discriminates
//!
//! Two of the three. Measured 2026-09-04 on the same host with `--no-stealth`,
//! `navigator.webdriver` is STILL `false`, because Chrome only raises that flag
//! under `--enable-automation` and this product never passes it. So the
//! `webdriver` assertion below is a true invariant that would pass against a
//! completely absent payload — it is kept as a regression guard, never read as
//! proof that the mask ran.
//!
//! `platform` and the renderer are the discriminating reads: with the mask off
//! the same page answers `MacIntel`, which is what this gate would fail on.
//!
//! # Why a row that needs no code still needs a gate
//!
//! Every other `COVERED` row cites a line of this tree, and
//! `doc_measured_claims_gate` checks that the citation still resolves. This
//! row cites no line, because there is nothing to cite — it rests on a
//! behaviour of Chrome. A claim with no code behind it has nothing that can
//! fail when the claim stops being true, and that is the only kind of green
//! that rots in silence. This file is what that row cites instead.
//!
//! # What regressing looks like
//!
//! | case | the symptom |
//! |---|---|
//! | Chrome starts applying page CSP to init scripts | every read below falls back to the HOST value, and the page sees a macOS machine claiming to be Linux |
//! | the payload stops being installed at all | same symptom, different cause, and the WebGL gate would fail alongside this one |
//!
//! # Skip policy
//!
//! No binary, or no usable Chrome, means SKIP LOUDLY — never a silent pass.
//! The fixture is served from an in-process loopback listener, so this gate
//! makes NO network call.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::thread;

mod common;
use common::{binary, chrome_mentioned_in_doctor_json, missing_binary};

const GATE: &str = "csp_init_script_gate";

/// The strictest script policy a page can publish.
///
/// `script-src 'none'` blocks inline scripts, external scripts and `eval`.
/// Anything that still runs under it did not arrive as page script.
const POLICY: &str = "script-src 'none'; object-src 'none'";

/// The page carries no script of its own, deliberately.
///
/// A fixture that tried to read the values in page script would be blocked by
/// the very policy under test, and a blocked read is indistinguishable from an
/// absent mask. The read happens through `eval`, which is CDP and not page
/// script.
const PAGE: &str =
    "<!doctype html><html><head><title>csp</title></head><body><h1>csp</h1></body></html>";

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

fn start_fixture_server() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback fixture server");
    let port = listener.local_addr().expect("local addr").port();
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            thread::spawn(move || serve_one(stream));
        }
    });
    port
}

fn serve_one(mut stream: TcpStream) {
    let mut line = String::new();
    if BufReader::new(&stream).read_line(&mut line).is_err() {
        return;
    }
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\
         Content-Security-Policy: {POLICY}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{PAGE}",
        PAGE.len()
    );
    let _ = stream.write_all(response.as_bytes());
}

fn scratch() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("bac-csp-init-")
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

/// One launch against the CSP page, returning the page's own JSON report.
fn launch(dir: &Path, script: &Path) -> serde_json::Value {
    let out = common::isolated_cmd(&binary().expect("binary"))
        .env("HOME", dir)
        .env("XDG_CONFIG_HOME", dir)
        .args([
            "-q",
            "--json",
            "--timeout",
            "120",
            // A foreign profile, so the mask carries a value that cannot be
            // confused with what this host would answer on its own.
            "--stealth-profile",
            "chrome-linux",
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
fn a_strict_page_csp_does_not_reach_the_injected_payload() {
    if cannot_run() {
        return;
    }
    let scratch_dir = scratch();
    let dir = scratch_dir.path().to_path_buf();
    set(&dir, "http_ssrf_mode", "allow_loopback");
    set(&dir, "robots_loopback_exempt", "true");

    let port = start_fixture_server();
    let script = dir.join("steps.jsonl");
    std::fs::write(
        &script,
        format!(
            "{{\"cmd\":\"goto\",\"url\":\"http://127.0.0.1:{port}/\",\"navigation_timeout_ms\":45000}}\n\
             {{\"cmd\":\"eval\",\"expression\":\"(function(){{var c=document.createElement('canvas');var g=c.getContext('webgl');return JSON.stringify({{webdriver:navigator.webdriver,platform:navigator.platform,renderer:g?g.getParameter(37446):'no-webgl'}});}})()\"}}\n"
        ),
    )
    .expect("write script");

    let r = launch(&dir, &script);

    // Kept, but never read as proof: see the module doc. `--no-stealth`
    // answers `false` here too, so this assertion cannot tell a live payload
    // from an absent one.
    assert_eq!(
        r["webdriver"],
        serde_json::json!(false),
        "the automation flag was raised, which this product never does: {r}"
    );
    assert_eq!(
        r["platform"], "Linux x86_64",
        "`--stealth-profile chrome-linux` must reach a page that forbids all script: {r}"
    );
    let renderer = r["renderer"].as_str().unwrap_or("MISSING");
    // Equality against the profile's own answer is not available here, so this
    // asserts the negative that matters: the mask is on, therefore the host's
    // real vendor cannot be what the page reads.
    assert!(
        renderer.starts_with("ANGLE ("),
        "renderer must come through the mask, got {renderer:?}"
    );
    assert!(
        !renderer.contains("Apple") && !renderer.contains("SwiftShader"),
        "the raw host renderer leaked past the mask under CSP: {renderer:?}"
    );
}
