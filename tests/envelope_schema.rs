//! Gate: success/error JSON envelopes expose stable agent fields.
//! Behavior-Closed DoD requires more than `--help` presence.

use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;

fn parse_stdout(assert: &assert_cmd::assert::Assert) -> Value {
    let stdout = &assert.get_output().stdout;
    serde_json::from_slice(stdout).expect("stdout must be JSON")
}

#[test]
fn version_envelope_has_schema_ok_data() {
    let assert = cargo_bin_cmd!("browser-automation-cli")
        .args(["--json", "version"])
        .assert()
        .success();
    let v = parse_stdout(&assert);
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["ok"], true);
    assert!(v.get("data").is_some());
    assert_eq!(v["data"]["name"], "browser-automation-cli");
}

#[test]
fn commands_json_includes_devtools_tool_map() {
    let assert = cargo_bin_cmd!("browser-automation-cli")
        .args(["commands", "--json"])
        .assert()
        .success();
    let v = parse_stdout(&assert);
    assert_eq!(v["ok"], true);
    let data = &v["data"];
    assert!(data["commands"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c.as_str() == Some("text")));
    assert!(data["commands"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c.as_str() == Some("scroll")));
    assert!(data["commands"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c.as_str() == Some("cookie")));
    let map = data["devtools_tool_map"].as_array().expect("tool map");
    assert!(
        map.len() >= 50,
        "expected full DevTools map, got {}",
        map.len()
    );
    assert_eq!(data["binary"], "browser-automation-cli");
}

#[test]
fn global_flags_present_in_root_help() {
    let assert = cargo_bin_cmd!("browser-automation-cli")
        .args(["--help"])
        .assert()
        .success();
    let help = String::from_utf8_lossy(&assert.get_output().stdout);
    for needle in [
        "--quiet",
        "--verbose",
        "--debug",
        "--step-timeout",
        "--headed",
        "--json",
        "--correlation-id",
    ] {
        assert!(help.contains(needle), "root help missing {needle}");
    }
}

#[test]
fn correlation_id_echoed_on_success_envelope() {
    let assert = cargo_bin_cmd!("browser-automation-cli")
        .args(["--json", "--correlation-id", "agent-corr-1", "version"])
        .assert()
        .success();
    let v = parse_stdout(&assert);
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["ok"], true);
    assert_eq!(v["correlation_id"], "agent-corr-1");
}

#[test]
fn correlation_id_omitted_when_unset() {
    let assert = cargo_bin_cmd!("browser-automation-cli")
        .args(["--json", "version"])
        .assert()
        .success();
    let v = parse_stdout(&assert);
    assert!(
        v.get("correlation_id").is_none(),
        "must omit correlation_id when unset: {v}"
    );
}

#[test]
fn text_scroll_cookie_registered_in_help() {
    for (cmd, needles) in [
        (&["text", "--help"][..], &["target"][..]),
        (&["scroll", "--help"][..], &["delta-x", "delta-y"][..]),
        (&["cookie", "--help"][..], &["list", "set", "clear"][..]),
        (&["page", "select", "--help"][..], &["page-id"][..]),
    ] {
        let assert = cargo_bin_cmd!("browser-automation-cli")
            .args(cmd)
            .assert()
            .success();
        let help = String::from_utf8_lossy(&assert.get_output().stdout);
        for n in needles {
            assert!(
                help.contains(n),
                "{} help missing `{n}`:\n{help}",
                cmd.join(" ")
            );
        }
    }
}

#[test]
fn usage_error_envelope_shape() {
    let assert = cargo_bin_cmd!("browser-automation-cli")
        .args(["--json", "schema", "--cmd", "___no_such_command___"])
        .assert()
        .failure();
    let v = parse_stdout(&assert);
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["ok"], false);
    assert!(v["error"]["kind"].is_string());
    assert!(v["error"]["message"].is_string());
    assert!(v["error"]["exit_code"].is_number());
}
