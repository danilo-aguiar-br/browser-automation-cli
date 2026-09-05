//! Behavior-Closed gates: JSON envelopes for DevTools-parity commands.
//!
//! Offline tools always run. Browser tools run when Chrome is available
//! (same readiness idea as goto_smoke / doctor).

use serde_json::Value;

mod common;
use common::chrome_ready_via_doctor_checks;

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

#[test]
fn offline_meta_envelopes() {
    for args in [
        &["--json", "version"][..],
        &["--json", "commands"][..],
        &["--json", "schema", "--cmd", "goto"][..],
    ] {
        let assert = common::assert_bin().args(args).assert().success();
        let v = parse_stdout(&assert);
        assert_success_envelope(&v);
    }
}

#[test]
fn commands_map_covers_all_official_tools() {
    let assert = common::assert_bin()
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
    if !chrome_ready_via_doctor_checks() {
        common::skip_with_remedy(
            "devtools_envelope_behavior::browser_envelope",
            "doctor reports no usable Chrome.",
            "install a system Chrome/Chromium.",
        );
        return;
    }
    // This body navigates to `https://example.com` and then asserts that the
    // capture buffer holds a request, so it depends on the network as much as on
    // Chrome. Guarding only Chrome would turn a disconnected host into a failure
    // report about the product.
    if common::public_network_unreachable("devtools_envelope_behavior::browser_envelope") {
        return;
    }

    // goto
    let assert = common::assert_bin()
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

    let assert = common::assert_bin()
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
    let assert = common::assert_bin()
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

    // The envelope's SHAPE was the only thing asserted here, and shape is what a
    // buffer that captured nothing also has: the defect this suite failed to
    // catch answered `ok: true` with `count: 0` on every page. `net list` is the
    // one step in this file that exists to prove traffic was recorded, so it has
    // to require a record. `example.com` is one request, which is the floor, and
    // the floor is what makes this assertion true or false rather than decorative.
    let steps = v["data"]
        .get("steps")
        .or_else(|| v.get("steps"))
        .and_then(|s| s.as_array())
        .expect("run must emit steps");
    let net = steps
        .iter()
        .find(|s| s["cmd"] == "net")
        .expect("the net step must appear in the transcript");
    let count = net
        .pointer("/data/count")
        .or_else(|| net.get("count"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_else(|| panic!("net step carries no count: {net}"));
    assert!(
        count >= 1,
        "net list captured no request for a page that issues at least one; \
         an empty capture buffer is indistinguishable from a page with no \
         traffic, which is exactly the failure this gate exists to reject: {net}"
    );
}

#[test]
fn page_isolated_context_creates_context_id_when_chrome() {
    if !chrome_ready_via_doctor_checks() {
        common::skip_with_remedy(
            "devtools_envelope_behavior::isolated_context",
            "doctor reports no usable Chrome.",
            "install a system Chrome/Chromium.",
        );
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
    let assert = common::assert_bin()
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

/// The V8 heapsnapshot the offline `heap` verbs are exercised against.
///
/// # Why this test writes its own input
///
/// It used to scan `/tmp` for `ba-e2e-52-*/a.heapsnapshot` left behind by SOME
/// EARLIER RUN and feed the most recent one to the CLI. That was non-hermetic
/// in both directions: with the leftovers present it asserted against an input
/// of uncontrolled provenance, and without them it declined — so it never once
/// proved the offline heap verbs work on a clean machine.
///
/// The graph is the shape the parser's own unit fixture uses (see
/// `src/native/heap_snapshot/tests.rs`): root(0) → A(1) → B(2), with root also
/// retaining C(3). Node fields are `type, name, id, self_size, edge_count`, and
/// `to_node` is a FLAT index — node_index * 5, not the node ordinal.
const HEAP_FIXTURE: &str = r#"{
    "snapshot": {
        "meta": {
            "node_fields": ["type","name","id","self_size","edge_count"],
            "node_types": [["hidden","object","string","synthetic"]],
            "edge_fields": ["type","name_or_index","to_node"],
            "edge_types": [["context","element","property","internal","hidden","shortcut","weak"]]
        },
        "node_count": 4,
        "edge_count": 3
    },
    "nodes": [
        3, 0, 10, 0, 2,
        1, 1, 11, 100, 1,
        1, 2, 12, 50, 0,
        1, 3, 13, 25, 0
    ],
    "edges": [
        2, 4, 5,
        2, 5, 15,
        2, 6, 10
    ],
    "strings": ["(GC roots)", "A", "B", "C", "toA", "toC", "toB"]
}"#;

#[test]
fn heap_offline_envelopes_on_a_written_snapshot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let snap = dir.path().join("a.heapsnapshot");
    std::fs::write(&snap, HEAP_FIXTURE).expect("write heapsnapshot fixture");

    let path = snap.to_str().expect("utf-8 tempdir path");
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
        let assert = common::assert_bin().args(&args).assert().success();
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
        let assert = common::assert_bin()
            .args(["--json", "schema", "--cmd", cmd])
            .assert()
            .success();
        let v = parse_stdout(&assert);
        assert_success_envelope(&v);
    }
}

#[test]
fn binary_name_never_short_alias() {
    let assert = common::assert_bin()
        .args(["--json", "version"])
        .assert()
        .success();
    let v = parse_stdout(&assert);
    assert_eq!(v["data"]["name"], "browser-automation-cli");
    // help must not advertise bac
    let help = common::cmd().arg("--help").output().unwrap();
    let s = String::from_utf8_lossy(&help.stdout);
    assert!(!s.contains(" bac "), "help must not document bac alias");
}
