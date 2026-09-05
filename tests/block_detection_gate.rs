//! Permanent gate for the block detector reaching the ENVELOPE (gaps.md G1).
//!
//! # What has to break for this to fail
//!
//! G1 is the highest-return gap in the audit: a CAPTCHA used to arrive as
//! `ok: true` with `exit 0`, so an agent quoted the wall as if it were the page.
//! Phase 1 of the roadmap asks for three things *together*:
//!
//! 1. `exit 6`
//! 2. `error.kind: "blocked"`
//! 3. `data.block_detection`
//!
//! The first two shipped; the third did not, and nothing noticed because the
//! only test on that path called `BlockDetection::to_json()` directly. That
//! function stayed green while no envelope in the product ever carried its
//! output — function coverage is not path coverage, and this file asserts the
//! path.
//!
//! # The negative control is the point
//!
//! A gate that only fetched a CAPTCHA page would also pass on a build that
//! flagged *every* response as blocked, which would be a far worse product than
//! the one this gap describes. So an ordinary page is fetched through the same
//! server, the same command and the same flags, and must come back clean.
//!
//! # Skip policy
//!
//! No binary means SKIP LOUDLY. The fixture server is a plain loopback TCP
//! listener in-process: this gate makes **no** network call.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::thread;

mod common;
use common::{binary, missing_binary};

const GATE: &str = "block_detection_gate";

fn cannot_run() -> bool {
    if missing_binary(GATE) {
        return true;
    }
    false
}

/// A challenge page: HTTP 200, valid HTML, and not a single transport signal.
///
/// That combination is the whole difficulty of G1. `status_code` is 200,
/// `http_error` is false and the body parses, so every transport-shaped check
/// in the envelope reports success while the content is a wall.
const CHALLENGE: &str = "<html><head><title>Just a moment...</title></head><body>\
<div class=\"cf-challenge-running\">Checking your browser before accessing</div>\
<form id=\"captcha-form\"></form></body></html>";

/// An ordinary page with enough prose to survive main-content extraction.
const CLEAN: &str = "<html><head><title>Clean Fixture</title></head><body>\
<h1>Clean Fixture</h1><p>This page carries ordinary prose about local fixture \
servers, loopback ports and deterministic gates, with no challenge markup and \
no vendor headers anywhere in the response.</p></body></html>";

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
    let response = match path.as_str() {
        // Vendor header AND body signature: attribution must name the vendor,
        // not fall back to "generic".
        "/challenge.html" => format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\ncf-ray: 8f0a1b2c3d4e5f60-GRU\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{CHALLENGE}",
            CHALLENGE.len()
        ),
        "/clean.html" => format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\
             Connection: close\r\n\r\n{CLEAN}",
            CLEAN.len()
        ),
        "/robots.txt" => {
            let body = "User-agent: *\nAllow: /\n";
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\
                 Connection: close\r\n\r\n{body}",
                body.len()
            )
        }
        _ => "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
    };
    let _ = stream.write_all(response.as_bytes());
}

/// An isolated XDG config dir so the gate never reads the developer's config.
///
/// The guard is returned, not the path: dropping it removes the directory, and a
/// caller holding only the path would hand the CLI a config home that no longer
/// exists. The old body used a pid-keyed name with no removal at all.
fn config_dir(name: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("bac-block-{name}-"))
        .tempdir()
        .expect("create isolated config dir")
}

/// Run the CLI under an isolated config; returns parsed stdout.
fn run(cfg: &Path, args: &[&str]) -> serde_json::Value {
    let bin = binary().expect("binary");
    let out = common::isolated_cmd(&bin)
        // `HOME` is what isolates config on macOS: `directories` resolves to
        // ~/Library/Application Support and never reads `XDG_CONFIG_HOME`.
        // Full measurement (2026-09-04) lives in `tests/scrape_wave6_gate.rs`.
        .env("HOME", cfg)
        .env("XDG_CONFIG_HOME", cfg)
        .args(args)
        .output()
        .expect("spawn");
    serde_json::from_slice(&out.stdout).unwrap_or(serde_json::Value::Null)
}

/// Loopback fixtures need SSRF and robots exemptions; both are XDG knobs.
///
/// The defaults refuse loopback on purpose, so this is the gate adapting to the
/// product's own policy rather than the product relaxing for the gate.
fn prepare(name: &str) -> tempfile::TempDir {
    let cfg = config_dir(name);
    for (k, v) in [
        ("http_ssrf_mode", "allow_loopback"),
        ("robots_loopback_exempt", "true"),
    ] {
        let out = run(cfg.path(), &["-q", "--json", "config", "set", k, v]);
        assert_eq!(out["ok"], serde_json::json!(true), "config set {k}={v}");
    }
    cfg
}

/// Scrape one fixture path on `engine`; returns (exit code, parsed stdout).
fn scrape_on(name: &str, port: u16, path: &str, engine: &str) -> (Option<i32>, serde_json::Value) {
    let bin = binary().expect("binary");
    let url = format!("http://127.0.0.1:{port}{path}");
    let cfg = prepare(name);
    let out = common::isolated_cmd(&bin)
        .env("HOME", cfg.path())
        .env("XDG_CONFIG_HOME", cfg.path())
        .args([
            "-q",
            "--timeout",
            "120",
            "--json",
            "scrape",
            &url,
            "--format",
            "text",
            "--engine",
            engine,
        ])
        .output()
        .expect("spawn");
    let value = serde_json::from_slice(&out.stdout).unwrap_or(serde_json::Value::Null);
    (out.status.code(), value)
}

fn scrape(name: &str, port: u16, path: &str) -> (Option<i32>, serde_json::Value) {
    scrape_on(name, port, path, "http")
}

/// True when Chrome is not discoverable; the browser-engine case needs one.
fn chrome_missing() -> bool {
    let Some(bin) = binary() else { return true };
    let probe = common::isolated_cmd(&bin)
        .args(["-q", "--json", "doctor", "--offline", "--quick"])
        .output();
    let found = probe
        .ok()
        .and_then(|o| serde_json::from_slice::<serde_json::Value>(&o.stdout).ok())
        .and_then(|v| {
            v.pointer("/data/checks")?
                .as_array()?
                .iter()
                .find(|c| c.get("id").and_then(|i| i.as_str()) == Some("chrome"))
                .and_then(|c| c.get("status").and_then(|s| s.as_str()))
                .map(|s| s == "pass")
        })
        .unwrap_or(false);
    !found
}

/// A bot check must fail loudly AND carry a machine-readable attribution.
#[test]
fn challenge_page_fails_with_exit_6_and_carries_block_detection() {
    if cannot_run() {
        return;
    }
    let port = start_fixture_server();
    let (code, env) = scrape("challenge", port, "/challenge.html");

    assert_eq!(
        code,
        Some(6),
        "a bot check must exit 6, not succeed. envelope={env}"
    );
    assert_eq!(
        env.pointer("/ok").and_then(serde_json::Value::as_bool),
        Some(false),
        "a bot check must not report ok. envelope={env}"
    );
    assert_eq!(
        env.pointer("/error/kind").and_then(|v| v.as_str()),
        Some("blocked"),
        "error.kind must be `blocked`. envelope={env}"
    );

    // The part that was missing. Without it the vendor and the signal exist only
    // inside the English prose of `message`, so an agent has to regex an error
    // string to branch -- the parse-the-prose failure this product rejects
    // everywhere else.
    let detection = env
        .pointer("/data/block_detection")
        .unwrap_or_else(|| panic!("data.block_detection absent from the envelope: {env}"));
    assert_eq!(
        detection.get("waf").and_then(|v| v.as_str()),
        Some("cloudflare"),
        "the vendor header must drive attribution: {detection}"
    );
    assert!(
        detection.get("signal").and_then(|v| v.as_str()).is_some(),
        "the matched signal must be named: {detection}"
    );
    assert!(
        detection.get("phase").and_then(|v| v.as_str()).is_some(),
        "the phase (header/body) must be named: {detection}"
    );
    assert!(
        env.pointer("/error/suggestion")
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.to_lowercase().contains("retry the same")),
        "the suggestion must not advise a plain retry, which escalates a block: {env}"
    );
}

/// NEGATIVE CONTROL: an ordinary page through the same path stays clean.
///
/// Without this case, a build that flagged every response as blocked would pass
/// the test above while destroying the product.
#[test]
fn ordinary_page_is_not_flagged_as_blocked() {
    if cannot_run() {
        return;
    }
    let port = start_fixture_server();
    let (code, env) = scrape("clean", port, "/clean.html");

    assert_eq!(
        code,
        Some(0),
        "an ordinary page must succeed; the detector must not fire on prose. \
         envelope={env}"
    );
    assert_eq!(
        env.pointer("/ok").and_then(serde_json::Value::as_bool),
        Some(true),
        "an ordinary page must report ok. envelope={env}"
    );
    assert!(
        env.pointer("/data/block_detection").is_none(),
        "a clean fetch must carry no block_detection payload. envelope={env}"
    );
}

/// ENGINE PARITY: the browser engine must report a block too.
///
/// This is the case that mattered most and was missing. Detection shipped on
/// `--engine http` alone, so the same CAPTCHA exited 6 through one engine and
/// came back `ok: true` through the other -- and the product's own advice when
/// a WAF is in front is to switch to `--engine browser`, which moved the agent
/// from the engine that reports blocks to the engine that stayed silent.
///
/// Attribution is deliberately NOT asserted to equal the http engine's: this
/// path gets a rendered DOM from CDP, with no response headers to attribute
/// with, so a generic body challenge stays `generic` here. The exit code, the
/// error kind and the payload SHAPE are what must match.
#[test]
fn browser_engine_also_reports_the_block() {
    if cannot_run() {
        return;
    }
    if chrome_missing() {
        common::skip_with_reason("browser_engine_also_reports_the_block", "no usable Chrome.");
        return;
    }
    let port = start_fixture_server();
    let (code, env) = scrape_on("browser", port, "/challenge.html", "browser");

    assert_eq!(
        code,
        Some(6),
        "the browser engine must exit 6 on a bot check, exactly like the http \
         engine. A block that is loud on one engine and silent on the other is \
         worse than one that is silent on both, because the product tells agents \
         to switch to this engine precisely when a WAF is in front. envelope={env}"
    );
    assert_eq!(
        env.pointer("/error/kind").and_then(|v| v.as_str()),
        Some("blocked"),
        "error.kind must be `blocked` on the browser engine too. envelope={env}"
    );
    let detection = env
        .pointer("/data/block_detection")
        .unwrap_or_else(|| panic!("data.block_detection absent on browser engine: {env}"));
    for field in ["waf", "signal", "phase"] {
        assert!(
            detection.get(field).and_then(|v| v.as_str()).is_some(),
            "block_detection.{field} missing on the browser engine: {detection}"
        );
    }
}
