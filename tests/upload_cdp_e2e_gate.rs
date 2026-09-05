// SPDX-License-Identifier: MIT OR Apache-2.0
//! Permanent gate for CDP file upload against a real Chrome.
//!
//! `upload` was shipped and documented as a cookbook recipe but never executed
//! end to end, and it is the kind of command that cannot be proven any other
//! way: `DOM.setFileInputFiles` either reaches the renderer and the page's
//! `change` handler observes a real `File`, or it does not. A unit test can
//! only assert that the CDP message was serialised.
//!
//! | case | what regressing looks like |
//! |---|---|
//! | upload reaches the input | the step answers `ok` while `input.files` stays empty, so a form submits with no attachment |
//! | `--script -` reads stdin | the documented pipe form breaks and every cookbook `run` recipe has to fall back to a temp file |
//! | `--script` is a path | the cookbook's inline-JSON form is resurrected and every documented `run` recipe fails with `no-input` |
//!
//! The last two cases exist because the shipped cookbook recipe *was* wrong:
//! `--script` takes a file path or `-` for NDJSON on stdin, and inline JSON is
//! interpreted as a path. This gate pins both accepted forms and the rejection,
//! so the recipe cannot silently drift back.
//!
//! # Skip policy
//!
//! No binary, or no usable Chrome, means SKIP LOUDLY — never a silent pass.
//! The fixture page is served from an in-process loopback listener: this gate
//! makes **no** network call.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::thread;

mod common;
use common::{binary, chrome_mentioned_in_doctor_json, missing_binary};

const GATE: &str = "upload_cdp_e2e_gate";

/// True when the host cannot run the gate. Prints why; never silently passes.
fn cannot_run() -> bool {
    if missing_binary(GATE) {
        return true;
    }
    if !chrome_mentioned_in_doctor_json() {
        common::skip_with_remedy(
            "upload_cdp_e2e_gate",
            "doctor reports no usable Chrome.",
            "install a system Chrome/Chromium.",
        );
        return true;
    }
    false
}

// Ask the product itself whether a browser is usable, rather than guessing
// paths. A plain comment, not a doc comment: the probe these words described
// moved to `tests/common/mod.rs` in 0.1.9, and a `///` with no item under it
// documents whatever happens to follow it.

/// The page records name+size into `#out` when the input's `change` fires, so a
/// successful upload is observable from the page's own point of view.
const UPLOAD_PAGE: &str = "<html><head><title>Upload Fixture</title></head><body>\
<h1>Upload</h1><form><input id=\"f\" type=\"file\"></form><div id=\"out\">none</div>\
<script>document.getElementById('f').addEventListener('change',function(e){\
var f=e.target.files[0];\
document.getElementById('out').textContent=f?(f.name+'|'+f.size):'none';});</script>\
</body></html>";

/// Serve the upload page on an ephemeral loopback port until the process exits.
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
    let path = line.split_whitespace().nth(1).unwrap_or("/").to_string();
    let response = if path == "/upload.html" {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{UPLOAD_PAGE}",
            UPLOAD_PAGE.len()
        )
    } else {
        "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into()
    };
    let _ = stream.write_all(response.as_bytes());
}

/// A scratch dir that doubles as the isolated XDG config home.
///
/// The guard is returned, not the path: dropping it removes the directory, and
/// the upload fixtures written into it would go with it. The old body used a
/// pid-keyed name with no removal at all.
fn scratch(name: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("bac-upload-{name}-"))
        .tempdir()
        .expect("create scratch dir")
}

fn set(cfg: &PathBuf, key: &str, value: &str) {
    let out = common::isolated_cmd(&binary().expect("binary"))
        // `HOME` is what isolates config on macOS: `directories` resolves to
        // ~/Library/Application Support and never reads `XDG_CONFIG_HOME`.
        // Full measurement (2026-09-04) lives in `tests/scrape_wave6_gate.rs`.
        .env("HOME", cfg)
        .env("XDG_CONFIG_HOME", cfg)
        .args(["-q", "--json", "config", "set", key, value])
        .output()
        .expect("spawn config set");
    assert!(out.status.success(), "config set {key}={value}");
}

#[test]
fn upload_delivers_the_file_to_a_real_chrome() {
    if cannot_run() {
        return;
    }
    let scratch_dir = scratch("e2e");
    let dir = scratch_dir.path().to_path_buf();
    set(&dir, "http_ssrf_mode", "allow_loopback");
    set(&dir, "robots_loopback_exempt", "true");

    // A payload whose byte length is asserted below, so a truncated or empty
    // transfer cannot pass as success.
    let payload = dir.join("payload.txt");
    let body = "hello-upload-payload-1234567890";
    std::fs::write(&payload, body).expect("write payload");

    let port = start_fixture_server();
    let script = dir.join("steps.jsonl");
    std::fs::write(
        &script,
        format!(
            "{{\"cmd\":\"goto\",\"url\":\"http://127.0.0.1:{port}/upload.html\"}}\n\
             {{\"cmd\":\"upload\",\"target\":\"input[type=file]\",\"path\":\"{}\"}}\n\
             {{\"cmd\":\"eval\",\"expression\":\"document.getElementById('out').textContent\"}}\n",
            payload.display()
        ),
    )
    .expect("write script");

    let out = common::isolated_cmd(&binary().expect("binary"))
        .env("HOME", &dir)
        .env("XDG_CONFIG_HOME", &dir)
        .args([
            "-q",
            "--json",
            "--timeout",
            "90",
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
    assert_eq!(
        v["ok"],
        serde_json::json!(true),
        "upload script must succeed end to end: {v}"
    );

    // The page itself reports what it received: name|size. This is the only
    // assertion that distinguishes a real transfer from an `ok` no-op.
    let seen = v["data"]["steps"]
        .as_array()
        .expect("steps array")
        .iter()
        .find_map(|s| {
            s.pointer("/data/result")
                .and_then(serde_json::Value::as_str)
        })
        .unwrap_or_default()
        .to_string();
    assert_eq!(
        seen,
        format!("payload.txt|{}", body.len()),
        "the page's change handler must observe the real file: {}",
        v["data"]
    );
}

#[test]
fn upload_works_through_script_on_stdin() {
    if cannot_run() {
        return;
    }
    let scratch_dir = scratch("stdin");
    let dir = scratch_dir.path().to_path_buf();
    set(&dir, "http_ssrf_mode", "allow_loopback");
    set(&dir, "robots_loopback_exempt", "true");

    let payload = dir.join("payload.txt");
    let body = "hello-upload-payload-1234567890";
    std::fs::write(&payload, body).expect("write payload");

    let port = start_fixture_server();
    let script = format!(
        "{{\"cmd\":\"goto\",\"url\":\"http://127.0.0.1:{port}/upload.html\"}}\n\
         {{\"cmd\":\"upload\",\"target\":\"input[type=file]\",\"path\":\"{}\"}}\n\
         {{\"cmd\":\"eval\",\"expression\":\"document.getElementById('out').textContent\"}}\n",
        payload.display()
    );

    // `--script -` is the form the cookbook publishes, because it needs no temp
    // file. Process substitution is NOT an alternative: the path jail rejects
    // /proc/<pid>/fd/<n>, so stdin is the only file-free route.
    let mut child = common::isolated_cmd(&binary().expect("binary"))
        .env("HOME", &dir)
        .env("XDG_CONFIG_HOME", &dir)
        .args(["-q", "--json", "--timeout", "90", "run", "--script", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn run --script -");
    child
        .stdin
        .take()
        .expect("stdin pipe")
        .write_all(script.as_bytes())
        .expect("write script to stdin");
    let out = child.wait_with_output().expect("collect run output");

    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout was not JSON ({e}).\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    });
    assert_eq!(
        v["ok"],
        serde_json::json!(true),
        "`--script -` must accept NDJSON on stdin: {v}"
    );
    let seen = v["data"]["steps"]
        .as_array()
        .expect("steps array")
        .iter()
        .find_map(|s| {
            s.pointer("/data/result")
                .and_then(serde_json::Value::as_str)
        })
        .unwrap_or_default()
        .to_string();
    assert_eq!(
        seen,
        format!("payload.txt|{}", body.len()),
        "the stdin form must deliver the same real upload as the path form: {}",
        v["data"]
    );
}

#[test]
fn run_script_takes_a_path_not_inline_json() {
    if binary().is_none() {
        common::skip_with_remedy(
            "upload_cdp_e2e_gate",
            "target/debug/browser-automation-cli absent.",
            "run `cargo build` first.",
        );
        return;
    }
    // No Chrome needed: the argument is rejected before any browser launch.
    let scratch_dir = scratch("inline");
    let dir = scratch_dir.path().to_path_buf();
    let inline = r#"[{"cmd":"goto","url":"https://example.com"}]"#;
    let out = common::isolated_cmd(&binary().expect("binary"))
        .env("HOME", &dir)
        .env("XDG_CONFIG_HOME", &dir)
        .args(["-q", "--json", "run", "--script", inline])
        .output()
        .expect("spawn run --script");

    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("error envelope must still be JSON on stdout");
    assert_eq!(
        v["ok"],
        serde_json::json!(false),
        "inline JSON is not a script path and must be refused: {v}"
    );
    assert_eq!(
        v["error"]["kind"],
        serde_json::json!("no-input"),
        "the refusal must be a no-input error, so docs recipes fail loudly: {v}"
    );
}
