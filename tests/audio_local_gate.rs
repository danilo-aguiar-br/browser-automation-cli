// SPDX-License-Identifier: MIT OR Apache-2.0
//! Offline gates for local `audio` pipeline (skip when ffmpeg missing).

use std::process::Command;

mod common;

fn has_ffmpeg() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn make_tiny_wav(path: &std::path::Path) {
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=0.5",
            "-c:a",
            "pcm_s16le",
            path.to_str().expect("fixture path is valid UTF-8"),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("spawn ffmpeg fixture");
    assert!(status.success(), "ffmpeg wav fixture failed");
}

#[test]
fn audio_in_inventory() {
    let out = common::cmd()
        .args(["--json", "commands"])
        .output()
        .expect("commands");
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let cmds = v["data"]["commands"].as_array().expect("commands array");
    assert_eq!(cmds.len(), 71);
    assert!(cmds.iter().any(|c| c.as_str() == Some("audio")));
    assert!(cmds.iter().any(|c| c.as_str() == Some("video")));
    assert!(cmds.iter().any(|c| c.as_str() == Some("image")));
}

#[test]
fn schema_audio_includes_mvp_actions() {
    let out = common::cmd()
        .args(["--json", "schema", "audio"])
        .output()
        .expect("schema audio");
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let text = v.to_string();
    assert!(text.contains("convert"), "schema missing convert: {text}");
    assert!(text.contains("trim"), "schema missing trim: {text}");
    assert!(text.contains("download"), "schema missing download: {text}");
    assert!(text.contains("mp3"), "schema missing mp3: {text}");
}

#[test]
fn ssrf_blocks_loopback_download() {
    let tmp = tempfile::tempdir().unwrap();
    let out_path = tmp.path().join("x.mp3");
    let out = common::cmd()
        .args([
            "--json",
            "audio",
            "download",
            "http://127.0.0.1:9/x.mp3",
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
fn convert_wav_to_mp3_when_ffmpeg_present() {
    if !has_ffmpeg() {
        common::skip_with_remedy(
            "audio_local_gate",
            "ffmpeg is not on PATH.",
            "install ffmpeg to exercise the transcode path.",
        );
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let wav = tmp.path().join("in.wav");
    let mp3 = tmp.path().join("out.mp3");
    make_tiny_wav(&wav);
    let out = common::cmd()
        .args([
            "--json",
            "audio",
            "convert",
            "--path",
            wav.to_str().unwrap(),
            "--format",
            "mp3",
            "-o",
            mp3.to_str().unwrap(),
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
    assert!(mp3.is_file());
    assert_eq!(v["data"]["action"], "convert");
    assert_eq!(v["data"]["container_out"], "mp3");
    // No raw media on stdout
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(!s.contains("RIFF"));
}

#[test]
fn config_audio_keys_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg_home = tmp.path().join("config");
    std::fs::create_dir_all(&cfg_home).unwrap();
    let out = common::cmd()
        // `HOME` is what isolates config on macOS: `directories` resolves to
        // ~/Library/Application Support and never reads `XDG_CONFIG_HOME`.
        // Full measurement (2026-09-04) lives in `tests/scrape_wave6_gate.rs`.
        .env("HOME", &cfg_home)
        .env("XDG_CONFIG_HOME", &cfg_home)
        .args(["--json", "config", "set", "audio_default_bitrate", "128k"])
        .output()
        .expect("config set");
    assert!(
        out.status.success(),
        "set stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = common::cmd()
        .env("HOME", &cfg_home)
        .env("XDG_CONFIG_HOME", &cfg_home)
        .args(["--json", "config", "get", "audio_default_bitrate"])
        .output()
        .expect("config get");
    assert!(
        out.status.success(),
        "get stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["data"]["value"], "128k");
    // Family round-trip (remaining 3 keys; bitrate already set)
    for (key, val) in [
        ("audio_default_format", "flac"),
        ("audio_max_input_bytes", "5000000"),
        ("audio_download_max_bytes", "3000000"),
    ] {
        let out = common::cmd()
            .env("HOME", &cfg_home)
            .env("XDG_CONFIG_HOME", &cfg_home)
            .args(["--json", "config", "set", key, val])
            .output()
            .expect("set family");
        assert!(out.status.success(), "set {key}");
        let out = common::cmd()
            .env("HOME", &cfg_home)
            .env("XDG_CONFIG_HOME", &cfg_home)
            .args(["--json", "config", "get", key])
            .output()
            .expect("get family");
        assert!(out.status.success(), "get {key}");
        let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(v["ok"], true);
        let got = v["data"]["value"].to_string();
        assert!(got.contains(val), "{key}: {got}");
    }
    let out = common::cmd()
        .env("HOME", &cfg_home)
        .env("XDG_CONFIG_HOME", &cfg_home)
        .args(["--json", "config", "get", "audio_max_input_bytes"])
        .output()
        .expect("get max");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let max = v["data"]["value"].as_u64().unwrap_or(0);
    assert!(max > 0, "max_input must not be 0: {v}");
    // ASK the binary where its config lives; do not reconstruct the path.
    //
    // This used to be `cfg_home.join("browser-automation-cli/config.toml")`,
    // which encodes two assumptions that are both false on macOS — measured
    // 2026-09-04. First, `directories` there resolves to
    // `~/Library/Application Support/...` and never reads `XDG_CONFIG_HOME`, so
    // the `.env` override above steers nothing and the sandbox redirects
    // through `HOME` instead. Second, the directory is named
    // `cli.browser-automation.browser-automation-cli`, not the bare crate name.
    // The test therefore failed with `NotFound` on every macOS run, blaming the
    // audio keys for a path it had invented.
    //
    // `config path` publishes `config_file` precisely so a caller never has to
    // know the host's layout, and a test is a caller.
    let out = common::cmd()
        .env("HOME", &cfg_home)
        .env("XDG_CONFIG_HOME", &cfg_home)
        .args(["--json", "config", "path"])
        .output()
        .expect("config path");
    let paths: serde_json::Value = serde_json::from_slice(&out.stdout).expect("config path JSON");
    let cfg_file = paths["data"]["config_file"]
        .as_str()
        .expect("config path must publish config_file");
    let toml = std::fs::read_to_string(cfg_file).expect("config.toml");
    assert!(
        toml.contains("audio_default_bitrate"),
        "TOML missing audio key: {toml}"
    );
    assert!(toml.contains("audio_default_format"));
    assert!(toml.contains("audio_max_input_bytes"));
    assert!(toml.contains("audio_download_max_bytes"));
}

#[test]
fn trim_stream_copy_omits_null_reencode_reason() {
    if !has_ffmpeg() {
        common::skip_with_remedy(
            "audio_local_gate",
            "ffmpeg is not on PATH.",
            "install ffmpeg to exercise the transcode path.",
        );
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let wav = tmp.path().join("in.wav");
    let mp3 = tmp.path().join("a.mp3");
    let cut = tmp.path().join("cut.mp3");
    make_tiny_wav(&wav);
    let out = common::cmd()
        .args([
            "--json",
            "audio",
            "convert",
            "--path",
            wav.to_str().unwrap(),
            "--format",
            "mp3",
            "-o",
            mp3.to_str().unwrap(),
        ])
        .output()
        .expect("wav->mp3");
    assert!(out.status.success());
    let out = common::cmd()
        .args([
            "--json",
            "audio",
            "trim",
            "--path",
            mp3.to_str().unwrap(),
            "--start",
            "0",
            "--duration",
            "0.2",
            "-o",
            cut.to_str().unwrap(),
        ])
        .output()
        .expect("trim");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(
        !s.contains(r#""reencode_reason":null"#),
        "CLEAN STDOUT forbids reencode_reason null: {s}"
    );
}

#[test]
fn convert_ogg_from_8k_wav_when_ffmpeg() {
    if !has_ffmpeg() {
        common::skip_with_remedy(
            "audio_local_gate",
            "ffmpeg is not on PATH.",
            "install ffmpeg to exercise the transcode path.",
        );
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let wav = tmp.path().join("in8k.wav");
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=0.3:sample_rate=8000",
            "-c:a",
            "pcm_s16le",
            wav.to_str().unwrap(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("spawn 8k fixture");
    assert!(status.success(), "8k wav fixture failed");
    let ogg = tmp.path().join("out.ogg");
    let out = common::cmd()
        .args([
            "--json",
            "audio",
            "convert",
            "--path",
            wav.to_str().unwrap(),
            "--format",
            "ogg",
            "-o",
            ogg.to_str().unwrap(),
        ])
        .output()
        .expect("convert ogg");
    assert!(
        out.status.success(),
        "8k->ogg failed stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["data"]["container_out"], "ogg");
    assert!(ogg.is_file());
}

#[test]
fn convert_lossy_includes_suggestion_when_reencode() {
    if !has_ffmpeg() {
        common::skip_with_remedy(
            "audio_local_gate",
            "ffmpeg is not on PATH.",
            "install ffmpeg to exercise the transcode path.",
        );
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let wav = tmp.path().join("in.wav");
    let mp3 = tmp.path().join("a.mp3");
    let m4a = tmp.path().join("a.m4a");
    make_tiny_wav(&wav);
    let out = common::cmd()
        .args([
            "--json",
            "audio",
            "convert",
            "--path",
            wav.to_str().unwrap(),
            "--format",
            "mp3",
            "-o",
            mp3.to_str().unwrap(),
        ])
        .output()
        .expect("wav->mp3");
    assert!(out.status.success());
    let out = common::cmd()
        .args([
            "--json",
            "audio",
            "convert",
            "--path",
            mp3.to_str().unwrap(),
            "--format",
            "m4a",
            "-o",
            m4a.to_str().unwrap(),
        ])
        .output()
        .expect("mp3->m4a");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["data"]["lossy_transcode"], true);
    let sug = v["data"]["suggestion"].as_str().unwrap_or("");
    assert!(!sug.is_empty(), "missing suggestion on lossy convert");
}

#[test]
fn info_select_projection() {
    if !has_ffmpeg() {
        common::skip_with_remedy(
            "audio_local_gate",
            "ffmpeg is not on PATH.",
            "install ffmpeg to exercise the transcode path.",
        );
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let wav = tmp.path().join("in.wav");
    make_tiny_wav(&wav);
    let out = common::cmd()
        .args([
            "--json",
            "audio",
            "info",
            "--path",
            wav.to_str().unwrap(),
            "--select",
            "format,path,bytes",
        ])
        .output()
        .expect("info");
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], true);
    let d = &v["data"];
    assert!(d.get("container").is_some() || d.get("path").is_some());
    assert!(d.get("action").is_some());
}
