//! Permanent gate: the `search` FAILURE envelope names its endpoint.
//!
//! # What has to break for this to fail
//!
//! `serp_endpoint` used to be computed AFTER the empty-result `return Err`, so
//! it existed only on the success envelope. An operator who pointed
//! `search_base_url` at an endpoint the product does not understand got exit
//! 65 with `error.kind: "data"` — the exact same machine-readable shape a
//! genuinely empty web produces. The two diagnoses are opposite (fix your
//! configuration versus widen your query) and the envelope could not tell them
//! apart, because the classification was written at the point a payload was
//! EMITTED rather than at the point the endpoint was DECIDED.
//!
//! # The negative control is the point
//!
//! A build that stapled `serp_endpoint` onto every error would also satisfy a
//! gate that only looked for the key. So this file asserts the VALUE is
//! derived: the failure must echo the configured `search_base_url` verbatim,
//! and the success branch on the same fixture host must report `unknown` too.
//! A constant cannot satisfy both.
//!
//! # Skip policy
//!
//! No binary means SKIP LOUDLY. The fixture SERP is a plain loopback TCP
//! listener in-process: this gate makes **no** network call.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::thread;

mod common;
use common::{binary, missing_binary};

const GATE: &str = "search_endpoint_provenance_gate";

/// A SERP that answers with nothing but its own navigation.
///
/// Every link is same-host, so the organic filter drops all of them and the
/// result set is empty — which is the branch this gate exists to inspect.
const CHROME_ONLY: &str = "<html><body><a href=\"/settings\">Settings</a>\
<a href=\"/about\">About</a><a href=\"/help\">Help</a></body></html>";

/// The same SERP with one genuine off-host destination among the chrome.
const WITH_ORGANIC: &str = "<html><body><a href=\"/settings\">Settings</a>\
<a href=\"https://docs.rs/chromiumoxide\">chromiumoxide</a></body></html>";

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
    let target = line.split_whitespace().nth(1).unwrap_or("/").to_string();
    let body = if target.starts_with("/robots.txt") {
        let txt = "User-agent: *\nAllow: /\n";
        let _ = stream.write_all(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\
                 Connection: close\r\n\r\n{txt}",
                txt.len()
            )
            .as_bytes(),
        );
        return;
    } else if target.starts_with("/organic") {
        WITH_ORGANIC
    } else if target.starts_with("/chrome") {
        CHROME_ONLY
    } else {
        let _ = stream
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        return;
    };
    let _ = stream.write_all(
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\
             Connection: close\r\n\r\n{body}",
            body.len()
        )
        .as_bytes(),
    );
}

/// Run the CLI under an isolated config; returns (exit code, parsed stdout).
fn run(cfg: &Path, args: &[&str]) -> (Option<i32>, serde_json::Value) {
    let bin = binary().expect("binary");
    let out = common::isolated_cmd(&bin)
        // `HOME` is what isolates config on macOS: `directories` resolves to
        // ~/Library/Application Support and never reads `XDG_CONFIG_HOME`.
        .env("HOME", cfg)
        .env("XDG_CONFIG_HOME", cfg)
        .args(args)
        .output()
        .expect("spawn browser-automation-cli");
    let value = serde_json::from_slice(&out.stdout).unwrap_or(serde_json::Value::Null);
    (out.status.code(), value)
}

/// An isolated config pointed at one fixture path, with the loopback opt-ins.
///
/// The defaults refuse loopback for SSRF and enforce robots; both are XDG keys,
/// so the gate adapts to the product's policy instead of the product relaxing
/// for the gate.
fn prepare(name: &str, port: u16, path: &str) -> (tempfile::TempDir, String) {
    let cfg = tempfile::Builder::new()
        .prefix(&format!("bac-serp-{name}-"))
        .tempdir()
        .expect("create isolated config dir");
    let base = format!("http://127.0.0.1:{port}{path}?");
    for (k, v) in [
        ("http_ssrf_mode", "allow_loopback"),
        ("robots_loopback_exempt", "true"),
        ("search_base_url", base.as_str()),
    ] {
        let (_, out) = run(cfg.path(), &["-q", "--json", "config", "set", k, v]);
        assert_eq!(out["ok"], serde_json::json!(true), "config set {k}={v}");
    }
    (cfg, base)
}

fn search(cfg: &Path) -> (Option<i32>, serde_json::Value) {
    run(
        cfg,
        &[
            "-q",
            "--timeout",
            "120",
            "--json",
            "search",
            "chromiumoxide cdp",
            "--limit",
            "5",
        ],
    )
}

/// The failure envelope must carry the SAME field the success envelope does.
#[test]
fn an_empty_serp_failure_publishes_its_endpoint_provenance() {
    if missing_binary(GATE) {
        return;
    }
    let port = start_fixture_server();
    let (cfg, base) = prepare("empty", port, "/chrome");
    let (code, env) = search(cfg.path());

    assert_eq!(env["ok"], serde_json::json!(false), "envelope: {env}");
    assert_eq!(env["error"]["kind"], serde_json::json!("data"), "{env}");
    assert_eq!(code, Some(65), "sysexits data failure: {env}");
    assert_eq!(
        env["data"]["serp_endpoint"],
        serde_json::json!("unknown"),
        "the failure branch must classify the endpoint it just used; without \
         this field an unknown endpoint and an empty web are the same envelope: {env}"
    );
    assert_eq!(
        env["data"]["search_base_url"],
        serde_json::json!(base),
        "the echoed base proves the value is DERIVED from configuration and \
         not a constant stapled onto every error: {env}"
    );
}

/// Negative control: the same fixture host on the SUCCESS branch reports the
/// same `unknown`, and a real organic hit still comes back.
#[test]
fn a_successful_search_on_the_same_foreign_endpoint_agrees() {
    if missing_binary(GATE) {
        return;
    }
    let port = start_fixture_server();
    let (cfg, _base) = prepare("organic", port, "/organic");
    let (code, env) = search(cfg.path());

    assert_eq!(env["ok"], serde_json::json!(true), "envelope: {env}");
    assert_eq!(code, Some(0), "{env}");
    assert_eq!(
        env["data"]["serp_endpoint"],
        serde_json::json!("unknown"),
        "one endpoint cannot be `known` on success and `unknown` on failure: {env}"
    );
    assert_eq!(env["data"]["count"], serde_json::json!(1), "{env}");
    assert_eq!(
        env["data"]["results"][0]["url"],
        serde_json::json!("https://docs.rs/chromiumoxide"),
        "the same-host filter must keep the off-host destination: {env}"
    );
}

/// The dimension gate rejects before fetching, and must still name the endpoint.
///
/// Presence of `serp_endpoint` is a property of the FUNCTION, not of one lucky
/// branch: an agent told to read `data.serp_endpoint` on failure would find it
/// missing here, which is the same trap of a document promising more than the
/// binary delivers.
#[test]
fn the_dimension_refusal_also_names_the_endpoint() {
    if missing_binary(GATE) {
        return;
    }
    let port = start_fixture_server();
    let (cfg, base) = prepare("dims", port, "/organic");
    let (code, env) = run(
        cfg.path(),
        &[
            "-q",
            "--timeout",
            "120",
            "--json",
            "search",
            "chromiumoxide cdp",
            "--country",
            "br",
            "--search-lang",
            "pt",
        ],
    );

    assert_eq!(env["ok"], serde_json::json!(false), "envelope: {env}");
    assert_eq!(env["error"]["kind"], serde_json::json!("usage"), "{env}");
    assert_eq!(code, Some(2), "{env}");
    assert_eq!(
        env["data"]["serp_endpoint"],
        serde_json::json!("unknown"),
        "every exit taken after `base` was resolved must carry the same pair: {env}"
    );
    assert_eq!(
        env["data"]["search_base_url"],
        serde_json::json!(base),
        "{env}"
    );
}
