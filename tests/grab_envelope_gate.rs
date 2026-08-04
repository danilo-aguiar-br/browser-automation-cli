// SPDX-License-Identifier: MIT OR Apache-2.0
//! `grab` must report the dimensions of the image it just wrote.
//!
//! # The gap this closes
//!
//! The capture envelope carried `byte_size`, `format`, `full_page`, `magic_ok`,
//! `path`, `quality` and `written` — everything except how big the picture is.
//! The dimensions are known at write time, because the encoder had just
//! produced the file, and the only way for a caller to learn them was to spawn
//! a second process running `image info` on a path this command had produced
//! moments earlier.
//!
//! That is the exact shape the agent-native contract rules out: the binary is
//! supposed to do the work and hand back the actionable result, not hand back a
//! path and make the model pay another round trip to finish the thought. The
//! question "how tall did `--full-page` come out" is the most common thing
//! anyone asks a screenshot, and it was the one thing the envelope withheld.
//!
//! # Why a local fixture
//!
//! The interesting assertion is that `--full-page` exceeds the viewport, which
//! needs a page taller than the viewport and a height that is known in advance.
//! A `file://` fixture with fixed-height blocks gives an exact expected value;
//! a live site would make the test depend on someone else's layout.

use std::path::PathBuf;
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_browser-automation-cli"))
}

/// Five 600px blocks: full page is 3000px tall, well past any viewport.
const TALL_PAGE: &str = "<!doctype html><html><head><meta charset=\"utf-8\"><title>Tall</title>\
<style>body{margin:0}section{height:600px}</style></head><body>\
<section style=\"background:#e63946\"></section>\
<section style=\"background:#457b9d\"></section>\
<section style=\"background:#2a9d8f\"></section>\
<section style=\"background:#e9c46a\"></section>\
<section id=\"last\" style=\"background:#264653\"></section></body></html>";

const BLOCK_PX: u64 = 600;
const BLOCKS: u64 = 5;

fn chrome_available() -> bool {
    Command::new(bin())
        .args(["--json", "doctor", "--offline", "--quick"])
        .output()
        .ok()
        .and_then(|o| serde_json::from_slice::<serde_json::Value>(&o.stdout).ok())
        .and_then(|v| v.pointer("/data/chrome_found").and_then(|c| c.as_bool()))
        .unwrap_or(true)
}

/// Run a `run --script` sequence and return the per-step NDJSON lines.
///
/// `slug` names the script file. Cargo runs `#[test]` functions in parallel
/// threads of one process, so a shared filename would let one test overwrite
/// the script the other is about to execute — which is exactly how this file
/// first reported three captures for a two-step script.
fn run_steps(slug: &str, steps: &[String]) -> Vec<serde_json::Value> {
    let dir = std::env::temp_dir().join("bac-grab-envelope-gate");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let script = dir.join(format!("{slug}.jsonl"));
    std::fs::write(&script, format!("{}\n", steps.join("\n"))).expect("write script");

    let out = Command::new(bin())
        .args([
            "--json",
            "--json-steps",
            "--timeout",
            "150",
            "run",
            "--script",
            script.to_str().unwrap(),
        ])
        .output()
        .expect("spawn cli");

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter(|v| v.get("cmd").and_then(serde_json::Value::as_str) == Some("grab"))
        .collect()
}

#[test]
fn grab_reports_width_and_height_for_viewport_full_page_and_element() {
    if !chrome_available() {
        eprintln!("skipping: no usable Chrome on this host");
        return;
    }

    let dir = std::env::temp_dir().join("bac-grab-envelope-gate");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let page = dir.join("tall.html");
    std::fs::write(&page, TALL_PAGE).expect("write fixture");
    let shot = |n: &str| dir.join(n).to_string_lossy().into_owned();

    let steps = vec![
        format!(
            r#"{{"cmd":"goto","url":"file://{}"}}"#,
            page.to_string_lossy()
        ),
        format!(
            r#"{{"cmd":"grab","path":"{}","format":"png"}}"#,
            shot("v.png")
        ),
        format!(
            r#"{{"cmd":"grab","path":"{}","format":"png","full_page":true}}"#,
            shot("f.png")
        ),
        format!(
            r##"{{"cmd":"grab","path":"{}","format":"png","element":"#last"}}"##,
            shot("e.png")
        ),
    ];

    let grabs = run_steps("dims", &steps);
    assert_eq!(grabs.len(), 3, "expected three captures: {grabs:?}");

    for g in &grabs {
        let d = &g["data"];
        assert!(
            d["width"].as_u64().is_some_and(|w| w > 0),
            "every capture must report a width: {d}"
        );
        assert!(
            d["height"].as_u64().is_some_and(|h| h > 0),
            "every capture must report a height: {d}"
        );
    }

    let viewport_h = grabs[0]["data"]["height"]
        .as_u64()
        .expect("viewport height");
    let full_h = grabs[1]["data"]["height"].as_u64().expect("full height");
    let element_h = grabs[2]["data"]["height"].as_u64().expect("element height");

    assert_eq!(
        full_h,
        BLOCK_PX * BLOCKS,
        "--full-page must span the document, not the viewport"
    );
    assert!(
        full_h > viewport_h,
        "full page ({full_h}) must exceed viewport ({viewport_h})"
    );
    assert_eq!(
        element_h, BLOCK_PX,
        "--element must be clipped to the node box"
    );
}

/// The dimensions must describe the file on disk, not a guess from CDP.
#[test]
fn the_reported_size_matches_the_file_that_was_written() {
    if !chrome_available() {
        eprintln!("skipping: no usable Chrome on this host");
        return;
    }

    let dir = std::env::temp_dir().join("bac-grab-envelope-gate");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let page = dir.join("tall.html");
    std::fs::write(&page, TALL_PAGE).expect("write fixture");
    let shot = dir.join("check-match.png").to_string_lossy().into_owned();

    let grabs = run_steps(
        "match",
        &[
            format!(
                r#"{{"cmd":"goto","url":"file://{}"}}"#,
                page.to_string_lossy()
            ),
            format!(r#"{{"cmd":"grab","path":"{shot}","format":"png","full_page":true}}"#),
        ],
    );
    assert_eq!(grabs.len(), 1);
    let reported_w = grabs[0]["data"]["width"].as_u64().expect("width");
    let reported_h = grabs[0]["data"]["height"].as_u64().expect("height");

    // Cross-check with the independent local probe.
    let out = Command::new(bin())
        .args(["--json", "image", "info", "--path", &shot])
        .output()
        .expect("image info");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("info envelope");
    assert_eq!(v["data"]["width"].as_u64(), Some(reported_w));
    assert_eq!(v["data"]["height"].as_u64(), Some(reported_h));
}
