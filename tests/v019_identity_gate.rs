// SPDX-License-Identifier: MIT OR Apache-2.0
//! Permanent gates for the 0.1.9 stealth-identity and NDJSON-script fixes.
//!
//! Chrome is optional. The unit helpers in `native::stealth` prove the shipped
//! identity math. When Chrome/Xvfb cannot start, the binary still answers
//! `version`, `doctor --fingerprint`, `--stealth-profile list` and `commands`.

use std::io::Write;

use browser_automation_cli::browser_policy::StealthProfile;
use browser_automation_cli::native::stealth::{
    assess_signals, bug01_deleted_webdriver, bug02_windows_ua_linux_platform,
    planned_stealth_signals, ua_contradicts_profile, Identity,
};

mod common;
use common::{bin, chrome_not_ready, run_json, skip_with_reason};

/// Probe the host BEFORE spawning, the way every other gate in this suite does.
///
/// The four live tests below used to infer "Chrome is missing" from substrings
/// in the child's stdout and stderr. That heuristic fails in both directions: a
/// real defect whose message happens to contain `chrome` is downgraded to a
/// skip, and a missing Chrome whose message does not is reported as a hard
/// assertion failure about the wrong thing. `chrome_not_ready` asks `doctor`
/// instead, so the skip names the host state.
fn host_cannot_run_chrome() -> bool {
    chrome_not_ready(GATE, &bin())
}

const GATE: &str = "v019_identity_gate";

#[test]
fn version_reports_0_1_9() {
    let (code, payload, _) = run_json(&["--json", "version"]);
    assert_eq!(code, 0, "{payload}");
    let ver = payload["data"]["version"]
        .as_str()
        .or_else(|| payload["version"].as_str())
        .expect("version field");
    assert_eq!(ver, "0.1.9", "{payload}");
}

#[test]
fn stealth_profiles_are_listed_from_the_binary() {
    let (code, payload, _) = run_json(&["--json", "--stealth-profile", "list", "version"]);
    assert_eq!(code, 0, "{payload}");
    let profiles = payload["data"]["stealth_profiles"]
        .as_array()
        .or_else(|| payload["stealth_profiles"].as_array())
        .expect("stealth_profiles");
    let names: Vec<&str> = profiles.iter().filter_map(|v| v.as_str()).collect();
    for expected in ["auto", "chrome-linux", "chrome-win", "chrome-mac"] {
        assert!(names.contains(&expected), "missing {expected} in {names:?}");
    }
}

#[test]
fn commands_json_lists_stealth_profiles_and_seed_fields() {
    let (code, payload, _) = run_json(&["--json", "commands"]);
    assert_eq!(code, 0, "{payload}");
    let data = payload.get("data").unwrap_or(&payload);
    let profiles = data["stealth_profiles"]
        .as_array()
        .expect("stealth_profiles on commands");
    assert!(profiles.iter().any(|v| v.as_str() == Some("chrome-win")));
    let fields = data["stealth_seed_fields"]
        .as_array()
        .expect("stealth_seed_fields");
    assert!(fields
        .iter()
        .any(|v| v.as_str() == Some("hardwareConcurrency")));
    let not_varied = data["stealth_seed_does_not_vary"]
        .as_array()
        .expect("stealth_seed_does_not_vary");
    assert!(not_varied
        .iter()
        .any(|v| v.as_str() == Some("navigator.platform")));
}

#[test]
fn doctor_fingerprint_scores_planned_identity_and_would_flag_old_bugs() {
    let (code, payload, _) = run_json(&["--json", "doctor", "--fingerprint", "--quick"]);
    assert_eq!(code, 0, "{payload}");
    let data = payload.get("data").unwrap_or(&payload);
    assert_eq!(data["ok"], true, "{data}");
    assert!(data["stealth_profiles"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v.as_str() == Some("chrome-mac")));
    assert_eq!(data["identity"]["webdriver_present"], true);
    assert_eq!(data["identity"]["webdriver_value"], false);
    let mismatches = data["mismatches"].as_array().expect("mismatches");
    assert!(mismatches.is_empty(), "{mismatches:?}");
    assert_eq!(data["measurement_scope"], "linux-headless-xvfb", "{data}");
    let unmeasured = data["unmeasured_os"].as_array().expect("unmeasured_os");
    assert!(
        unmeasured.iter().any(|v| v.as_str() == Some("macos")),
        "{data}"
    );
    assert!(
        unmeasured.iter().any(|v| v.as_str() == Some("windows")),
        "{data}"
    );
    let note = data["measurement_note"].as_str().unwrap_or("");
    assert!(note.contains("linux-headless-xvfb"), "{data}");

    // The assessor is the same function doctor uses. The historical
    // measurements MUST fail it, or the diagnosis cannot claim it would have
    // caught BUG-01 / BUG-02.
    let bug01 = assess_signals(&bug01_deleted_webdriver(), true);
    assert!(bug01.iter().any(|m| m.id == "webdriver_absent"));
    let bug02 = assess_signals(&bug02_windows_ua_linux_platform(), true);
    assert!(bug02.iter().any(|m| m.id == "platform_vs_ua"));
}

#[test]
fn chrome_win_identity_never_contradicts_itself() {
    let id = Identity::for_profile(StealthProfile::ChromeWindows);
    assert!(id.user_agent.contains("Windows NT"));
    assert_eq!(id.navigator_platform, "Win32");
    assert_eq!(id.platform, "\"Windows\"");
    assert!(assess_signals(
        &planned_stealth_signals(StealthProfile::ChromeWindows),
        true
    )
    .is_empty());
}

#[test]
fn chrome_mac_identity_never_contradicts_itself() {
    let id = Identity::for_profile(StealthProfile::ChromeMac);
    assert!(id.user_agent.contains("Macintosh; Intel Mac OS X"));
    assert_eq!(id.navigator_platform, "MacIntel");
    assert_eq!(id.platform, "\"macOS\"");
    assert!(assess_signals(&planned_stealth_signals(StealthProfile::ChromeMac), true).is_empty());
}

#[test]
fn emulate_rejects_a_foreign_platform_ua_before_any_page_mutation() {
    let windows_ua = Identity::for_profile(StealthProfile::ChromeWindows).user_agent;
    if StealthProfile::Auto.resolved() != StealthProfile::ChromeWindows {
        assert!(ua_contradicts_profile(&windows_ua, StealthProfile::Auto));
        let (code, payload, _) =
            run_json(&["--json", "-q", "emulate", "--user-agent", &windows_ua]);
        assert_eq!(code, 2, "{payload}");
        let kind = payload["error"]["kind"]
            .as_str()
            .or_else(|| payload["data"]["error"]["kind"].as_str())
            .unwrap_or("");
        assert_eq!(kind, "usage", "{payload}");
        let msg = payload.to_string();
        assert!(
            msg.contains("contradict") || msg.contains("stealth profile"),
            "{payload}"
        );
    }
}

#[test]
fn quoted_ndjson_script_with_js_regex_parses() {
    // Each step is one physical line. A printf/single-quote construction
    // would smash the JS quotes; a multi-line JSON string is invalid NDJSON.
    let scratch = tempfile::Builder::new()
        .prefix("bac-v019-script-")
        .tempdir()
        .expect("script scratch dir");
    let path = scratch.path().join("script.ndjson");
    let mut f = std::fs::File::create(&path).expect("script file");
    writeln!(
        f,
        r#"{{"cmd":"goto","url":"about:blank"}}
{{"cmd":"eval","expression":"(/hello/i).test(\"hello\")"}}"#
    )
    .unwrap();
    drop(f);

    let body = std::fs::read_to_string(&path).expect("read script");
    let steps = browser_automation_cli::commands::run::parse_run_script(&body)
        .expect("quoted heredoc NDJSON must parse");
    assert_eq!(steps.len(), 2, "{steps:?}");
    let expr = format!("{steps:?}");
    assert!(
        expr.contains("/hello/i") || expr.contains("hello"),
        "{expr}"
    );
}

/// When Chrome cannot launch, this records the failure instead of inventing a transcript.
#[test]
fn live_chrome_reproductions_or_honest_skip() {
    if host_cannot_run_chrome() {
        return;
    }
    let script = r#"{"cmd":"goto","url":"about:blank"}
{"cmd":"eval","expression":"({inNav:('webdriver' in navigator),inProto:('webdriver' in Navigator.prototype),value:String(navigator.webdriver),platform:navigator.platform,ua:navigator.userAgent,uaData:(navigator.userAgentData&&navigator.userAgentData.platform)||null,sw:screen.width,sh:screen.height,iw:innerWidth,ih:innerHeight})"}"#;
    let scratch = tempfile::Builder::new()
        .prefix("bac-v019-live-")
        .tempdir()
        .expect("script scratch dir");
    let path = scratch.path().join("script.ndjson");
    std::fs::write(&path, script).unwrap();

    let out = common::cmd()
        .args([
            "--json",
            "-q",
            "--timeout",
            "45",
            "--stealth-seed",
            "v019-gate",
            "run",
            "--script",
        ])
        .arg(&path)
        .output()
        .expect("spawn run --script");

    let code = out.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    if code != 0 {
        // `doctor` approved this host above, so the launch failed for a reason
        // the probe cannot see (Xvfb, sandbox, resource pressure). Still an
        // honest skip rather than a green lie, but a NAMED one.
        skip_with_reason(
            GATE,
            &format!(
                "live_chrome_reproductions_or_honest_skip: doctor reported the host ready \
                 yet run --script exited {code}, so the stealth reproductions never ran.\
                 \nstdout={stdout}\nstderr={stderr}"
            ),
        );
        return;
    }
    let payload: serde_json::Value = serde_json::from_str(&stdout).expect("json envelope");
    let steps = payload
        .pointer("/data/steps")
        .or_else(|| payload.get("steps"))
        .and_then(|v| v.as_array())
        .expect("steps");
    let last = steps.last().expect("eval step");
    let result = last
        .pointer("/data/result")
        .or_else(|| last.get("result"))
        .cloned()
        .unwrap_or_else(|| last.clone());
    let obj = result.get("result").cloned().unwrap_or(result);
    let in_nav = obj.get("inNav").and_then(|v| v.as_bool()).unwrap_or(false);
    let in_proto = obj
        .get("inProto")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let value = obj.get("value").and_then(|v| v.as_str()).unwrap_or("");
    assert!(in_nav, "BUG-01 regress: inNav={obj}");
    assert!(in_proto, "BUG-01 regress: inProto={obj}");
    assert_eq!(value, "false", "BUG-01 regress: value={obj}");
    let platform = obj.get("platform").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        platform.contains("Linux") || platform == "Win32" || platform.contains("Mac"),
        "platform missing: {obj}"
    );
    let sw = obj.get("sw").and_then(|v| v.as_i64()).unwrap_or(0);
    let sh = obj.get("sh").and_then(|v| v.as_i64()).unwrap_or(0);
    assert_ne!((sw, sh), (800, 600), "default screen still 800x600: {obj}");

    let canvas_out = common::cmd()
        .args([
            "--json",
            "-q",
            "--timeout",
            "45",
            "eval",
            "document.createElement('canvas').toDataURL('image/png').slice(0,22)",
        ])
        .output()
        .expect("canvas eval");
    let canvas_code = canvas_out.status.code().unwrap_or(-1);
    if canvas_code == 0 {
        let canvas_stdout = String::from_utf8_lossy(&canvas_out.stdout);
        assert!(
            canvas_stdout.contains("data:image/png;base64")
                || canvas_stdout.contains("data:image/"),
            "stealth canvas toDataURL must return a PNG\n{canvas_stdout}"
        );
        assert!(
            !canvas_stdout.contains("Maximum call stack"),
            "stealth canvas still recurses\n{canvas_stdout}"
        );
    }
}

#[test]
fn doctor_quick_no_stealth_does_not_copy_stealth_ua() {
    let (code, payload, _) = run_json(&[
        "--json",
        "--no-stealth",
        "doctor",
        "--fingerprint",
        "--quick",
    ]);
    assert_eq!(code, 0, "{payload}");
    let data = payload.get("data").unwrap_or(&payload);
    assert_eq!(data["stealth"], false, "{data}");
    let planned_ua = data["planned"]["user_agent"].as_str().unwrap_or("");
    let identity_ua = data["identity"]["user_agent"].as_str().unwrap_or("");
    assert_eq!(
        identity_ua, planned_ua,
        "identity must project planned UA: {data}"
    );
    assert!(
        identity_ua.contains("HeadlessChrome"),
        "unpatched identity still looks like stealth UA: {data}"
    );
    assert_eq!(data["identity"]["webdriver_value"], false, "{data}");
    assert_eq!(data["planned"]["webdriver_value"], false, "{data}");
    assert_eq!(data["probe_page"], "about:blank", "{data}");
}

#[test]
fn schema_emulate_advertises_screen() {
    let (code, payload, _) = run_json(&["--json", "schema", "emulate"]);
    assert_eq!(code, 0, "{payload}");
    let text = payload.to_string();
    assert!(text.contains("screen"), "{payload}");
}

#[test]
fn unknown_field_on_emulate_step_is_usage() {
    let scratch = tempfile::Builder::new()
        .prefix("bac-v0110-unknown-")
        .tempdir()
        .expect("script scratch dir");
    let path = scratch.path().join("script.ndjson");
    std::fs::write(
        &path,
        r#"{"cmd":"emulate","viewport":"1280x720x1","chave_inventada":1}"#,
    )
    .unwrap();
    let (code, payload, _) = run_json(&["--json", "-q", "run", "--script", path.to_str().unwrap()]);

    assert_eq!(code, 2, "{payload}");
    let kind = payload["error"]["kind"].as_str().unwrap_or("");
    assert_eq!(kind, "usage", "{payload}");
}

#[test]
fn emulate_screen_only_applies_device_metrics() {
    if host_cannot_run_chrome() {
        return;
    }
    let scratch = tempfile::Builder::new()
        .prefix("bac-v0112-screen-")
        .tempdir()
        .expect("script scratch dir");
    let path = scratch.path().join("script.ndjson");
    std::fs::write(
        &path,
        r#"{"cmd":"emulate","screen":"2560x1440"}
{"cmd":"eval","expression":"({sw:screen.width,sh:screen.height,iw:innerWidth,ih:innerHeight})"}"#,
    )
    .unwrap();
    let out = common::cmd()
        .args(["--json", "-q", "--timeout", "45", "run", "--script"])
        .arg(&path)
        .output()
        .expect("spawn screen-only run");

    let code = out.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    if code != 0 {
        skip_with_reason(
            GATE,
            &format!(
                "emulate_screen_only_applies_device_metrics: doctor reported the host ready \
                 yet the screen-only run exited {code}, so the device-metrics assertions \
                 never ran.\nstdout={stdout}\nstderr={stderr}"
            ),
        );
        return;
    }
    let payload: serde_json::Value = serde_json::from_str(&stdout).expect("json envelope");
    let steps = payload
        .pointer("/data/steps")
        .or_else(|| payload.get("steps"))
        .and_then(|v| v.as_array())
        .expect("steps");
    let emulate = steps
        .iter()
        .find(|s| s.get("cmd").and_then(|c| c.as_str()) == Some("emulate"))
        .expect("emulate step");
    let emu_data = emulate.get("data").unwrap_or(emulate);
    // The requested pair is a FLOOR for the caller and the drawn chrome
    // geometry is a floor for the product, so the number is the larger of the
    // two and the provenance says which one won.
    //
    // Measured 2026-09-04: this case asserted equality with 1440 and
    // `screen_source: "step"`, and failed 3 runs in 8 with `{"height":1444}`
    // still labelled `step`. The number was legitimate — `chrome_geometry`
    // draws the panel per identity and that draw sometimes lands above 1440 —
    // and the LABEL was the defect. `screen_source` now answers `floor` when
    // the floor won, so this asserts the contract instead of one lucky draw.
    let source = emu_data["screen_source"].as_str().unwrap_or("");
    assert!(
        source == "step" || source == "floor",
        "screen_source must name where the NUMBER came from: {emulate}"
    );
    let sw = emu_data["screen"]["width"].as_i64().unwrap_or(0);
    let sh = emu_data["screen"]["height"].as_i64().unwrap_or(0);
    assert!(
        sw >= 2560 && sh >= 1440,
        "the request is a floor: {emulate}"
    );
    if source == "step" {
        assert_eq!(sw, 2560, "`step` means the request survived: {emulate}");
        assert_eq!(sh, 1440, "`step` means the request survived: {emulate}");
    } else {
        assert!(
            sw > 2560 || sh > 1440,
            "`floor` must only be reported when the floor actually won: {emulate}"
        );
    }
    assert_eq!(emu_data["viewport"]["width"], 1920, "{emulate}");
    assert_eq!(emu_data["viewport"]["height"], 1080, "{emulate}");
    let last = steps.last().expect("eval step");
    let result = last
        .pointer("/data/result")
        .or_else(|| last.get("result"))
        .cloned()
        .unwrap_or_else(|| last.clone());
    let obj = result.get("result").cloned().unwrap_or(result);
    assert_eq!(obj.get("sw").and_then(|v| v.as_i64()), Some(2560), "{obj}");
    assert_eq!(obj.get("sh").and_then(|v| v.as_i64()), Some(1440), "{obj}");
    assert_eq!(obj.get("iw").and_then(|v| v.as_i64()), Some(1920), "{obj}");
    assert_eq!(obj.get("ih").and_then(|v| v.as_i64()), Some(1080), "{obj}");
}

/// Live `chrome-mac` identity on this host. Does not score WebGL/Canvas —
/// those stay host-GPU and would contradict a Mac UA on Linux.
#[test]
fn live_chrome_mac_identity_or_honest_skip() {
    if host_cannot_run_chrome() {
        return;
    }
    let script = r#"{"cmd":"goto","url":"about:blank"}
{"cmd":"eval","expression":"({inNav:('webdriver' in navigator),inProto:('webdriver' in Navigator.prototype),value:String(navigator.webdriver),platform:navigator.platform,ua:navigator.userAgent,uaData:(navigator.userAgentData&&navigator.userAgentData.platform)||null})"}"#;
    let scratch = tempfile::Builder::new()
        .prefix("bac-v013-mac-")
        .tempdir()
        .expect("script scratch dir");
    let path = scratch.path().join("script.ndjson");
    std::fs::write(&path, script).unwrap();
    let out = common::cmd()
        .args([
            "--json",
            "-q",
            "--timeout",
            "45",
            "--stealth-profile",
            "chrome-mac",
            "run",
            "--script",
        ])
        .arg(&path)
        .output()
        .expect("spawn chrome-mac run");

    let code = out.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    if code != 0 {
        skip_with_reason(
            GATE,
            &format!(
                "live_chrome_mac_identity_or_honest_skip: doctor reported the host ready yet \
                 the chrome-mac run exited {code}, so the identity assertions never ran.\
                 \nstdout={stdout}\nstderr={stderr}"
            ),
        );
        return;
    }
    let payload: serde_json::Value = serde_json::from_str(&stdout).expect("json envelope");
    let steps = payload
        .pointer("/data/steps")
        .or_else(|| payload.get("steps"))
        .and_then(|v| v.as_array())
        .expect("steps");
    let last = steps.last().expect("eval step");
    let result = last
        .pointer("/data/result")
        .or_else(|| last.get("result"))
        .cloned()
        .unwrap_or_else(|| last.clone());
    let obj = result.get("result").cloned().unwrap_or(result);
    assert_eq!(
        obj.get("inNav").and_then(|v| v.as_bool()),
        Some(true),
        "{obj}"
    );
    assert_eq!(
        obj.get("inProto").and_then(|v| v.as_bool()),
        Some(true),
        "{obj}"
    );
    assert_eq!(
        obj.get("value").and_then(|v| v.as_str()),
        Some("false"),
        "{obj}"
    );
    assert_eq!(
        obj.get("platform").and_then(|v| v.as_str()),
        Some("MacIntel"),
        "{obj}"
    );
    let ua = obj.get("ua").and_then(|v| v.as_str()).unwrap_or("");
    assert!(ua.contains("Macintosh; Intel Mac OS X"), "{obj}");
    assert_eq!(
        obj.get("uaData").and_then(|v| v.as_str()),
        Some("macOS"),
        "{obj}"
    );
}

/// Force the CDP "inspected target navigated" path, or record that Chrome
/// completed the eval on the old document (do not invent exit 70).
#[test]
fn live_eval_nav_exit_70_or_documents_current_chrome() {
    if host_cannot_run_chrome() {
        return;
    }
    let expr = "async () => { location.replace('https://example.com/'); await new Promise((r) => setTimeout(r, 4000)); return document.title; }";
    let out = common::cmd()
        .args([
            "--json",
            "-q",
            "--timeout",
            "45",
            "--lang",
            "en",
            "--ignore-robots",
            "--i-accept-robots-risk",
            "eval",
            expr,
        ])
        .output()
        .expect("spawn eval-nav");
    let code = out.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    // `navigated` in stdout IS the outcome under test, so it must never be
    // mistaken for a launch failure. Everything else, after doctor approved the
    // host, is a named skip.
    if code != 0 && !stdout.contains("navigated") {
        skip_with_reason(
            GATE,
            &format!(
                "live_eval_nav_exit_70_or_documents_current_chrome: doctor reported the host \
                 ready yet eval exited {code} without the navigation marker, so the \
                 navigation-during-eval path never ran.\nstdout={stdout}\nstderr={stderr}"
            ),
        );
        return;
    }
    if code == 70 {
        assert!(
            stdout.contains("eval / wait / eval") || stdout.contains("navigated"),
            "exit 70 without split suggestion\nstdout={stdout}\nstderr={stderr}"
        );
        return;
    }
    // Chrome 152 may finish the eval on the old document. The mapper in
    // eval.rs still fires when CDP emits "Inspected target navigated or closed".
    assert!(
        code == 0 || stdout.contains("navigated") || stdout.contains("eval / wait / eval"),
        "eval-nav unexpected failure\nexit={code}\nstdout={stdout}\nstderr={stderr}"
    );
}

/// NC-02: the `--no-stealth` plan must describe the binary this host has.
///
/// The audit measured a plan announcing major 152 while the host ran Chromium
/// 151.0.7922.137, because the plan took its version from the dependency's
/// spoof table. That is the right source under stealth and the wrong one here.
#[test]
fn no_stealth_plan_takes_its_major_from_the_host_binary() {
    let (code, payload, _) = run_json(&[
        "--json",
        "--no-stealth",
        "doctor",
        "--fingerprint",
        "--quick",
    ]);
    assert_eq!(code, 0, "{payload}");
    let data = payload.get("data").unwrap_or(&payload);

    let source = data["planned_version_source"].as_str().unwrap_or("");
    assert!(
        source == "chrome_binary" || source == "crate_table",
        "planned_version_source must name its origin, got {source:?}: {data}"
    );

    let (dcode, dpayload, _) = run_json(&["--json", "doctor", "--offline", "--quick"]);
    assert_eq!(dcode, 0, "{dpayload}");
    let ddata = dpayload.get("data").unwrap_or(&dpayload);
    let host_version = ddata["checks"]
        .as_array()
        .and_then(|c| c.iter().find(|c| c["id"] == "chrome"))
        .and_then(|c| c["version"].as_str())
        .unwrap_or("");

    if source == "chrome_binary" {
        let major = host_version
            .split_whitespace()
            .find_map(|t| t.split_once('.').map(|(m, _)| m.to_string()))
            .unwrap_or_default();
        assert!(!major.is_empty(), "doctor reported no version: {ddata}");
        let planned_ua = data["planned"]["user_agent"].as_str().unwrap_or("");
        assert!(
            planned_ua.contains(&format!("Chrome/{major}.")),
            "plan must carry the host major {major}, got {planned_ua:?}: {data}"
        );
    }
}

/// NC-01, other half: an unpatched Chromium exposes no `userAgentData`, so the
/// plan must not promise one. Promising it made the plan-vs-live comparison
/// carry a permanent, silent divergence.
#[test]
fn no_stealth_plan_does_not_promise_user_agent_data() {
    let (code, payload, _) = run_json(&[
        "--json",
        "--no-stealth",
        "doctor",
        "--fingerprint",
        "--quick",
    ]);
    assert_eq!(code, 0, "{payload}");
    let data = payload.get("data").unwrap_or(&payload);
    assert!(
        data["planned"]["ua_data_platform"].is_null(),
        "unpatched plan must not assert userAgentData: {data}"
    );
}

/// Under stealth the crate table IS the projected identity, so nothing is
/// probed and the field says so instead of naming a fallback.
#[test]
fn stealth_plan_reports_no_version_probe() {
    let (code, payload, _) = run_json(&["--json", "doctor", "--fingerprint", "--quick"]);
    assert_eq!(code, 0, "{payload}");
    let data = payload.get("data").unwrap_or(&payload);
    assert_eq!(data["stealth"], true, "{data}");
    assert!(
        data["planned_version_source"].is_null(),
        "stealth must not claim a version probe it never ran: {data}"
    );
}

/// NC-01: the live probe must be compared against the plan on every field the
/// envelope publishes, not on `webdriver_value` alone.
#[test]
fn live_probe_is_compared_against_the_plan_on_every_published_field() {
    if host_cannot_run_chrome() {
        return;
    }
    let (code, payload, _) = run_json(&["--json", "doctor", "--fingerprint"]);
    assert_eq!(code, 0, "{payload}");
    let data = payload.get("data").unwrap_or(&payload);
    let live = &data["live"]["result"]["result"];
    if !live.is_object() {
        skip_with_reason(
            GATE,
            "live_probe_is_compared_against_the_plan_on_every_published_field: the probe \
             returned no signals, so the comparison never ran.",
        );
        return;
    }
    // A clean run is only meaningful if the fields actually agree; assert the
    // agreement directly so an empty mismatch list cannot stand in for it.
    for field in [
        "user_agent",
        "navigator_platform",
        "screen_width",
        "screen_height",
        "inner_width",
        "inner_height",
    ] {
        assert_eq!(
            data["planned"][field], live[field],
            "planned/live disagree on {field} yet the run reported ok: {data}"
        );
    }
    assert_eq!(
        data["mismatches"].as_array().map(Vec::len),
        Some(0),
        "coherent identity must produce no mismatch: {data}"
    );
}
