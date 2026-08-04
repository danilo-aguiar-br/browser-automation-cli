// SPDX-License-Identifier: MIT OR Apache-2.0
//! Offline gates for `crawl --dry-run` and `crawl --output-mode llms-txt`.
//!
//! # Why both live in one file
//!
//! They are the two halves of the same idea: let an agent see what a crawl
//! WILL do before paying for it, and let it hand the result to a model without
//! post-processing. Neither adds a command — both are knobs on the crawl that
//! already walks the site, honours robots and budgets the page count.
//!
//! # Why `--dry-run` is tested with an unreachable host
//!
//! The claim is "resolves the plan and fetches nothing". Pointing it at a host
//! that cannot resolve makes that claim falsifiable: if any request were
//! attempted the command would fail, so a success envelope IS the evidence.
//! A test against a reachable host could pass while quietly hitting the network.

use std::path::PathBuf;
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_browser-automation-cli"))
}

/// A host that cannot resolve, so any real fetch turns into a failure.
const UNREACHABLE: &str = "https://this-host-does-not-resolve.invalid/start";

fn run(args: &[&str]) -> (bool, String) {
    let out = Command::new(bin()).args(args).output().expect("spawn cli");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[test]
fn dry_run_resolves_the_plan_without_touching_the_network() {
    let (ok, stdout) = run(&[
        "--json",
        "crawl",
        UNREACHABLE,
        "--limit",
        "50",
        "--max-depth",
        "3",
        "--dry-run",
    ]);
    assert!(ok, "a plan must not need the network: {stdout}");

    let v: serde_json::Value = serde_json::from_str(&stdout).expect("JSON envelope");
    assert_eq!(v["ok"], true);
    assert_eq!(v["data"]["dry_run"], true);
    assert_eq!(v["data"]["seed"], UNREACHABLE);
    assert_eq!(v["data"]["limit"], 50);
    assert_eq!(v["data"]["max_depth"], 3);
    assert!(
        v["data"].get("pages").is_none(),
        "a plan reports no pages, it only reports intent"
    );
}

#[test]
fn dry_run_echoes_the_effective_values_not_the_flags_as_typed() {
    // `--max-depth` is omitted, so the plan must show the resolved default
    // rather than nothing: the point of a plan is what WILL run.
    let (ok, stdout) = run(&["--json", "crawl", UNREACHABLE, "--dry-run"]);
    assert!(ok, "{stdout}");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("JSON envelope");
    assert!(
        v["data"]["max_depth"].as_u64().is_some(),
        "resolved max_depth must be present even when unflagged"
    );
    assert!(
        v["data"]["robots"].as_str().is_some(),
        "the robots policy in force must be visible before the crawl runs"
    );
}

#[test]
fn dry_run_is_advertised_in_the_crawl_schema() {
    let (ok, stdout) = run(&["--json", "schema", "crawl"]);
    assert!(ok);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("JSON envelope");
    let props = &v["data"]["schema"]["properties"];
    assert_eq!(props["dry_run"]["type"], "boolean");
    let modes = props["output_mode"]["description"]
        .as_str()
        .unwrap_or_default();
    assert!(
        modes.contains("llms-txt"),
        "output_mode must advertise llms-txt: {modes}"
    );
}

#[test]
fn the_human_plan_line_states_zero_requests() {
    let (ok, stdout) = run(&["crawl", UNREACHABLE, "--dry-run"]);
    assert!(ok, "{stdout}");
    assert!(
        stdout.contains("requests=0"),
        "the plan must say plainly that nothing was fetched: {stdout}"
    );
}
