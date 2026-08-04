// SPDX-License-Identifier: MIT OR Apache-2.0
//! Offline gates for local `video` pipeline (skip when ffmpeg/ffprobe missing).

use std::path::PathBuf;
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_browser-automation-cli"))
}

fn has_ffmpeg() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn make_tiny_mp4(path: &std::path::Path) {
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=0.4:size=160x120:rate=10",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=0.4",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
            path.to_str().unwrap(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("spawn ffmpeg fixture");
    assert!(status.success(), "ffmpeg fixture failed");
}

#[test]
fn video_in_inventory() {
    let out = Command::new(bin())
        .args(["--json", "commands"])
        .output()
        .expect("commands");
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let cmds = v["data"]["commands"].as_array().expect("commands array");
    assert_eq!(cmds.len(), 69);
    assert!(cmds.iter().any(|c| c.as_str() == Some("video")));
}

#[test]
fn schema_video_includes_wave_b_actions() {
    let out = Command::new(bin())
        .args(["--json", "schema", "video"])
        .output()
        .expect("schema video");
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let text = v.to_string();
    assert!(text.contains("trim"), "schema missing trim: {text}");
    assert!(
        text.contains("thumbnail"),
        "schema missing thumbnail: {text}"
    );
    assert!(
        text.contains("no_faststart"),
        "schema missing no_faststart: {text}"
    );
    assert!(
        !text.contains("\"faststart\""),
        "stale faststart flag still in schema"
    );
}

#[test]
fn ssrf_blocks_loopback_download() {
    let tmp = tempfile::tempdir().unwrap();
    let out_path = tmp.path().join("x.mp4");
    let out = Command::new(bin())
        .args([
            "--json",
            "video",
            "download",
            "http://127.0.0.1:9/x.mp4",
            "-o",
            out_path.to_str().unwrap(),
        ])
        .output()
        .expect("download");
    assert!(!out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], false);
    let msg = v["error"]["message"].as_str().unwrap_or("");
    assert!(
        msg.to_ascii_lowercase().contains("ssrf") || msg.contains("127.0.0.1"),
        "{msg}"
    );
}

#[test]
fn convert_default_webm_auto_reencodes_when_ffmpeg_present() {
    if !has_ffmpeg() {
        eprintln!("skip: ffmpeg not on PATH");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("in.mp4");
    let output = tmp.path().join("out.webm");
    make_tiny_mp4(&input);
    let out = Command::new(bin())
        .args([
            "--json",
            "video",
            "convert",
            "--path",
            input.to_str().unwrap(),
            "--format",
            "webm",
            "-o",
            output.to_str().unwrap(),
            "--select",
            "auto_reencoded,bytes_out,video_codec,path_out",
        ])
        .output()
        .expect("convert");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["data"]["auto_reencoded"], true);
    assert!(v["data"]["bytes_out"].as_u64().unwrap_or(0) > 1000);
    assert!(output.is_file());
}

#[test]
fn trim_and_thumbnail_when_ffmpeg_present() {
    if !has_ffmpeg() {
        eprintln!("skip: ffmpeg not on PATH");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("in.mp4");
    let clip = tmp.path().join("clip.mp4");
    let thumb = tmp.path().join("t.png");
    make_tiny_mp4(&input);
    let out = Command::new(bin())
        .args([
            "--json",
            "video",
            "trim",
            "--path",
            input.to_str().unwrap(),
            "--start",
            "0",
            "--duration",
            "0.2",
            "-o",
            clip.to_str().unwrap(),
        ])
        .output()
        .expect("trim");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let out2 = Command::new(bin())
        .args([
            "--json",
            "video",
            "thumbnail",
            "--path",
            input.to_str().unwrap(),
            "--at",
            "0.05",
            "-o",
            thumb.to_str().unwrap(),
        ])
        .output()
        .expect("thumbnail");
    assert!(
        out2.status.success(),
        "{}",
        String::from_utf8_lossy(&out2.stdout)
    );
    assert!(clip.is_file() && thumb.is_file());
}
