// SPDX-License-Identifier: MIT OR Apache-2.0
//! End-to-end gates for `video manifest` (GAP-VID-WAVE-C surface).
//!
//! # Why this file exists
//!
//! `tests/video_manifest_parse.rs` proves the parser is correct. That is not
//! the same claim as "an agent can reach it". For several releases the parser
//! was correct, tested, and unreachable: `video` exposed no action that routed
//! a playlist body into it, while the changelog listed manifest support as
//! delivered. Coverage of a library function is not coverage of a product
//! surface, and this file is the difference between the two.
//!
//! Everything here spawns the real binary and reads stdout, so a dispatch arm
//! that is deleted, renamed, or never wired fails loudly instead of silently
//! leaving a tested function with no caller.

#![cfg(feature = "media-manifest")]

use std::path::PathBuf;
use std::process::{Command, Stdio};

use std::io::Write;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_browser-automation-cli"))
}

const MASTER: &str = "#EXTM3U\n\
#EXT-X-STREAM-INF:BANDWIDTH=1280000,RESOLUTION=720x480,CODECS=\"avc1.4d401f\"\n\
low/index.m3u8\n\
#EXT-X-STREAM-INF:BANDWIDTH=2560000,RESOLUTION=1280x720\n\
mid/index.m3u8\n";

const MEDIA: &str = "#EXTM3U\n\
#EXT-X-TARGETDURATION:10\n\
#EXT-X-MEDIA-SEQUENCE:3\n\
#EXTINF:9.9,\n\
seg1.ts\n\
#EXTINF:9.9,\n\
seg2.ts\n\
#EXT-X-ENDLIST\n";

/// Write a fixture into the test's own temp dir and return its path.
fn fixture(name: &str, body: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("bac-video-manifest-cli");
    std::fs::create_dir_all(&dir).expect("fixture dir");
    let path = dir.join(name);
    std::fs::write(&path, body).expect("write fixture");
    path
}

fn run_json(args: &[&str], stdin_body: Option<&str>) -> serde_json::Value {
    let mut cmd = Command::new(bin());
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::null());
    if stdin_body.is_some() {
        cmd.stdin(Stdio::piped());
    }
    let mut child = cmd.spawn().expect("spawn browser-automation-cli");
    if let Some(body) = stdin_body {
        child
            .stdin
            .as_mut()
            .expect("stdin pipe")
            .write_all(body.as_bytes())
            .expect("feed stdin");
    }
    let out = child.wait_with_output().expect("collect output");
    serde_json::from_slice(&out.stdout).expect("stdout must be a JSON envelope")
}

#[test]
fn manifest_is_an_advertised_video_action() {
    let v = run_json(&["--json", "schema", "video"], None);
    let actions = v
        .pointer("/data/schema/properties/action/enum")
        .and_then(serde_json::Value::as_array)
        .expect("video schema exposes an action enum");
    assert!(
        actions.iter().any(|a| a == "manifest"),
        "schema must advertise manifest: {actions:?}"
    );
}

#[test]
fn a_master_playlist_reports_its_variant_ladder() {
    let path = fixture("master.m3u8", MASTER);
    let v = run_json(
        &[
            "--json",
            "video",
            "manifest",
            "--path",
            path.to_str().unwrap(),
        ],
        None,
    );
    assert_eq!(v["ok"], true, "envelope: {v}");
    assert_eq!(v["data"]["kind"], "hls_master");
    assert_eq!(v["data"]["variant_count"], 2);
    assert_eq!(v["data"]["action"], "manifest");
}

#[test]
fn a_media_playlist_summarises_instead_of_dumping_segments() {
    let path = fixture("media.m3u8", MEDIA);
    let v = run_json(
        &[
            "--json",
            "video",
            "manifest",
            "--path",
            path.to_str().unwrap(),
        ],
        None,
    );
    assert_eq!(v["data"]["kind"], "hls_media");
    assert_eq!(v["data"]["segment_count"], 2);
    assert!(
        v["data"].get("segments").is_none(),
        "a media playlist must not dump every segment URI into the envelope"
    );
}

#[test]
fn stdin_and_base_url_absolutise_relative_uris() {
    let v = run_json(
        &[
            "--json",
            "video",
            "manifest",
            "--stdin",
            "--base-url",
            "https://cdn.example.com/v/master.m3u8",
        ],
        Some(MASTER),
    );
    assert_eq!(v["ok"], true, "envelope: {v}");
    assert_eq!(
        v["data"]["variants"][0]["uri"],
        "https://cdn.example.com/v/low/index.m3u8"
    );
}

#[test]
fn select_projects_the_manifest_envelope() {
    let path = fixture("master.m3u8", MASTER);
    let v = run_json(
        &[
            "--json",
            "video",
            "manifest",
            "--path",
            path.to_str().unwrap(),
            "--select",
            "kind,variant_count",
        ],
        None,
    );
    let obj = v["data"].as_object().expect("object envelope");
    assert!(obj.contains_key("kind") && obj.contains_key("variant_count"));
    assert!(
        !obj.contains_key("variants"),
        "projection must drop unselected fields: {obj:?}"
    );
}

#[test]
fn a_body_that_is_neither_dialect_fails_closed() {
    let path = fixture("not-a-manifest.txt", "just some prose\n");
    let v = run_json(
        &[
            "--json",
            "video",
            "manifest",
            "--path",
            path.to_str().unwrap(),
        ],
        None,
    );
    assert_eq!(v["ok"], false, "unrecognised bodies must not succeed: {v}");
}

#[test]
fn path_and_stdin_together_are_a_usage_error() {
    let path = fixture("master.m3u8", MASTER);
    let v = run_json(
        &[
            "--json",
            "video",
            "manifest",
            "--path",
            path.to_str().unwrap(),
            "--stdin",
        ],
        Some(MASTER),
    );
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["kind"], "usage");
}
