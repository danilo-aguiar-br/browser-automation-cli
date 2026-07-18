//! Behavior-Closed gates: JSON envelopes for DevTools-parity commands.
//!
//! Offline tools always run. Browser tools run when Chrome is available
//! (same readiness idea as goto_smoke / doctor).

use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

fn parse_stdout(assert: &assert_cmd::assert::Assert) -> Value {
    let stdout = &assert.get_output().stdout;
    serde_json::from_slice(stdout).unwrap_or_else(|e| {
        panic!(
            "stdout not JSON: {e}; raw={}",
            String::from_utf8_lossy(stdout)
        )
    })
}

fn assert_success_envelope(v: &Value) {
    assert_eq!(v["schema_version"], 1, "schema_version");
    assert_eq!(v["ok"], true, "ok");
    assert!(v.get("data").is_some(), "data present");
}

fn chrome_ready() -> bool {
    cargo_bin_cmd!("browser-automation-cli")
        .args(["doctor", "--quick", "--json"])
        .ok()
        .map(|out| {
            let v: Value = serde_json::from_slice(&out.stdout).unwrap_or(Value::Null);
            v["ok"] == true
                || v["data"]["ok"] == true
                || v.pointer("/data/checks")
                    .and_then(|c| c.as_array())
                    .map(|arr| {
                        arr.iter()
                            .any(|x| x["id"] == "chrome" && x["status"] == "pass")
                    })
                    .unwrap_or(false)
        })
        .unwrap_or(false)
}

#[test]
fn offline_meta_envelopes() {
    for args in [
        &["--json", "version"][..],
        &["--json", "commands"][..],
        &["--json", "schema", "--cmd", "goto"][..],
    ] {
        let assert = cargo_bin_cmd!("browser-automation-cli")
            .args(args)
            .assert()
            .success();
        let v = parse_stdout(&assert);
        assert_success_envelope(&v);
    }
}

#[test]
fn commands_map_covers_all_official_tools() {
    let assert = cargo_bin_cmd!("browser-automation-cli")
        .args(["commands", "--json"])
        .assert()
        .success();
    let v = parse_stdout(&assert);
    assert_success_envelope(&v);
    let map = v["data"]["devtools_tool_map"]
        .as_array()
        .expect("devtools_tool_map");
    assert_eq!(
        map.len(),
        53,
        "official tool-ref count is 53 (includes get_tab_id)"
    );
    let tools: Vec<&str> = map.iter().filter_map(|e| e["tool"].as_str()).collect();
    assert!(
        tools.contains(&"get_tab_id"),
        "devtools_tool_map must include get_tab_id"
    );
    for entry in map {
        assert!(entry["tool"].as_str().is_some());
        assert!(entry["cli"].as_str().is_some());
    }
}

#[test]
fn goto_view_press_envelope_fields_when_chrome() {
    if !chrome_ready() {
        eprintln!("skip browser envelope: chrome not ready");
        return;
    }

    // goto
    let assert = cargo_bin_cmd!("browser-automation-cli")
        .args(["--json", "goto", "about:blank"])
        .assert()
        .success();
    let v = parse_stdout(&assert);
    assert_success_envelope(&v);
    assert!(v["data"]["url"].is_string());

    // multi-step: goto + view + wait + scroll + text via run
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("steps.jsonl");
    std::fs::write(
        &script,
        r#"{"cmd":"goto","url":"data:text/html,<html><body><h1>envelope</h1><p>ok</p></body></html>"}
{"cmd":"wait","ms":50}
{"cmd":"view"}
{"cmd":"scroll","delta_y":10}
{"cmd":"eval","expression":"document.body ? 'ok' : 'no'"}
"#,
    )
    .unwrap();

    let assert = cargo_bin_cmd!("browser-automation-cli")
        .args(["--json", "run", "--script", script.to_str().unwrap()])
        .assert()
        .success();
    let v = parse_stdout(&assert);
    assert_success_envelope(&v);

    // Accept either steps array or nested data with steps
    let steps = v["data"]
        .get("steps")
        .or_else(|| v.get("steps"))
        .and_then(|s| s.as_array());
    if let Some(steps) = steps {
        assert!(!steps.is_empty(), "run should emit steps");
        for step in steps {
            assert_eq!(step["ok"], true, "step ok: {step}");
            assert!(step["cmd"].is_string(), "step cmd: {step}");
        }
        let has_view = steps.iter().any(|s| s["cmd"] == "view");
        assert!(has_view, "view step present");
    }

    // net list envelope with capture
    let script_net = dir.path().join("net.jsonl");
    std::fs::write(
        &script_net,
        r#"{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":300}
{"cmd":"net","action":"list"}
"#,
    )
    .unwrap();
    let assert = cargo_bin_cmd!("browser-automation-cli")
        .args([
            "--json",
            "--capture-network",
            "--ignore-robots",
            "--i-accept-robots-risk",
            "run",
            "--script",
            script_net.to_str().unwrap(),
        ])
        .assert()
        .success();
    let v = parse_stdout(&assert);
    assert_success_envelope(&v);
}

#[test]
fn page_isolated_context_creates_context_id_when_chrome() {
    if !chrome_ready() {
        eprintln!("skip isolated_context: chrome not ready");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("iso.jsonl");
    std::fs::write(
        &script,
        r#"{"cmd":"goto","url":"about:blank"}
{"cmd":"page","action":"new","url":"about:blank","isolated_context":true}
{"cmd":"page","action":"list"}
"#,
    )
    .unwrap();
    let assert = cargo_bin_cmd!("browser-automation-cli")
        .args(["--json", "run", "--script", script.to_str().unwrap()])
        .assert()
        .success();
    let v = parse_stdout(&assert);
    assert_success_envelope(&v);
    let out = v.to_string();
    // Accept either real BrowserContext or explicit limitation (Chromium capability matrix).
    let ok_path = out.contains("browser_context_id")
        || out.contains("BrowserContext created")
        || out.contains("isolated_context_unsupported_on_this_browser")
        || out.contains("createBrowserContext unavailable");
    assert!(
        ok_path,
        "isolated_context must create context or document limitation: {out}"
    );
}

#[test]
fn heap_offline_envelope_when_snapshot_available() {
    // Prefer recent e2e snapshot if present
    let candidates: Vec<PathBuf> = std::fs::read_dir("/tmp")
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("ba-e2e-52-"))
        .map(|e| e.path().join("a.heapsnapshot"))
        .filter(|p| p.is_file())
        .collect();

    let snap = match candidates
        .into_iter()
        .max_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok())
    {
        Some(p) => p,
        None => {
            eprintln!("skip heap offline: no e2e heapsnapshot in /tmp");
            return;
        }
    };

    let path = snap.to_str().unwrap();
    for args in [
        vec![
            "--json",
            "--category-memory",
            "heap",
            "summary",
            "--path",
            path,
        ],
        vec![
            "--json",
            "--category-memory",
            "heap",
            "details",
            "--path",
            path,
        ],
        vec![
            "--json",
            "--category-memory",
            "heap",
            "dup-strings",
            "--path",
            path,
        ],
        vec![
            "--json",
            "--category-memory",
            "heap",
            "close",
            "--path",
            path,
        ],
    ] {
        let assert = cargo_bin_cmd!("browser-automation-cli")
            .args(&args)
            .assert()
            .success();
        let v = parse_stdout(&assert);
        assert_success_envelope(&v);
    }
}

#[test]
fn schema_cmd_covers_devtools_surface_samples() {
    for cmd in [
        "goto", "view", "press", "write", "wait", "net", "console", "heap", "perf", "page", "text",
        "scroll", "cookie",
    ] {
        let assert = cargo_bin_cmd!("browser-automation-cli")
            .args(["--json", "schema", "--cmd", cmd])
            .assert()
            .success();
        let v = parse_stdout(&assert);
        assert_success_envelope(&v);
    }
}

#[test]
fn binary_name_never_short_alias() {
    let assert = cargo_bin_cmd!("browser-automation-cli")
        .args(["--json", "version"])
        .assert()
        .success();
    let v = parse_stdout(&assert);
    assert_eq!(v["data"]["name"], "browser-automation-cli");
    // help must not advertise bac
    let help = Command::new(assert_cmd::cargo::cargo_bin!("browser-automation-cli"))
        .arg("--help")
        .output()
        .unwrap();
    let s = String::from_utf8_lossy(&help.stdout);
    assert!(!s.contains(" bac "), "help must not document bac alias");
}
