//! Wave 0 parity gate: default-ON DevTools inventory cmds must be registered.
//!
//! Does not require Chrome. Uses `browser-automation-cli commands --json`.

use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;

#[test]
fn parity_default_on_commands_registered() {
    let assert = cargo_bin_cmd!("browser-automation-cli")
        .args(["commands", "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: Value = serde_json::from_str(stdout.trim()).expect("commands json");
    // Envelope or raw payload
    let data = v.get("data").cloned().unwrap_or(v);
    let cmds = data
        .get("commands")
        .and_then(|c| c.as_array())
        .expect("commands array");
    let set: std::collections::BTreeSet<&str> = cmds.iter().filter_map(|c| c.as_str()).collect();

    let required = data
        .get("parity_default_on")
        .and_then(|c| c.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>())
        .unwrap_or_else(|| {
            vec![
                "goto",
                "view",
                "press",
                "write",
                "keys",
                "type",
                "wait",
                "hover",
                "drag",
                "fill-form",
                "upload",
                "back",
                "forward",
                "reload",
                "eval",
                "grab",
                "console",
                "net",
                "page",
                "dialog",
                "run",
                "exec",
                "doctor",
                "commands",
                "schema",
                "version",
            ]
        });

    for req in required {
        assert!(
            set.contains(req),
            "missing parity default-ON command: {req}; have={set:?}"
        );
    }
}

#[test]
fn wave1_surface_in_help() {
    let assert = cargo_bin_cmd!("browser-automation-cli")
        .arg("--help")
        .assert()
        .success();
    let help = String::from_utf8_lossy(&assert.get_output().stdout);
    for needle in [
        "hover",
        "drag",
        "fill-form",
        "upload",
        "back",
        "forward",
        "reload",
    ] {
        assert!(help.contains(needle), "help missing {needle}: {help}");
    }
}
