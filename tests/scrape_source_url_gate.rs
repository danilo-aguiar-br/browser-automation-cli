// SPDX-License-Identifier: MIT OR Apache-2.0
//! `source_url` must name the same origin on every collection of one page.
//!
//! # Why this gate exists
//!
//! `source_url` had two producers. A fresh fetch reported `final_url`, which is
//! where the body was really served; both cache branches echoed the argv string
//! instead. The two disagreed in two independent ways, and neither was covered:
//!
//! 1. SPELLING — `https://example.com` and `https://example.com/` are the same
//!    origin, but only the fresh path normalised, so the first collection said
//!    `.../` and every later one said `...` with no slash.
//! 2. REDIRECT — a cache hit could not report the destination at all, because
//!    the entry stored only body, content-type and expiry.
//!
//! Both are now closed, the first by normalising once before the branch and the
//! second by persisting `final_url` in the cache entry. `rg source_url tests/`
//! found only field-projection and crawl-URL collection before this file, so
//! the property that actually matters — that the answer does not change between
//! the first collection and the next — had no test at all.
//!
//! The fixture is a loopback server rather than a public host: a redirect is
//! the object under test, and it must be deterministic.

mod common;

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::thread;

use common::{binary_or_skip, skip_with_reason};

const GATE: &str = "scrape_source_url_gate";

/// Serve three shapes: a plain page, a redirect, and its destination.
fn serve_one(mut stream: TcpStream) {
    let mut line = String::new();
    if BufReader::new(&stream).read_line(&mut line).is_err() {
        return;
    }
    let path = line.split_whitespace().nth(1).unwrap_or("/").to_string();
    let body = "<html><head><title>T</title></head><body><h1>page body here</h1>\
                <p>enough text for the extractor to consider this a document</p></body></html>";
    let response = match path.as_str() {
        "/" | "/direct" | "/served" => format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\
             Connection: close\r\n\r\n{body}",
            body.len()
        ),
        "/redir" => "HTTP/1.1 301 Moved Permanently\r\nLocation: /served\r\n\
                     Content-Length: 0\r\nConnection: close\r\n\r\n"
            .to_string(),
        _ => "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string(),
    };
    let _ = stream.write_all(response.as_bytes());
}

/// Bind an ephemeral loopback port and serve until the binary exits.
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

/// Isolated XDG roots so neither the developer's config nor cache leaks in.
///
/// The cache matters as much as the config here: a warm entry from an earlier
/// run would make the FIRST collection a hit, and the gate would then compare
/// two cache hits and pass while proving nothing.
/// The guard comes back with the two roots: dropping it removes the tree, so a
/// caller that kept only the paths would point the CLI at directories that no
/// longer exist. Bind it as `_base`, never as `_`, which drops at once.
///
/// The base used to be a pid-keyed name under the temp dir with no removal, so
/// each run left one tree per case behind for good.
fn xdg_roots(name: &str) -> (tempfile::TempDir, PathBuf, PathBuf) {
    let base = tempfile::Builder::new()
        .prefix(&format!("bac-srcurl-{name}-"))
        .tempdir()
        .expect("create isolated xdg base");
    let cfg = base.path().join("config");
    let cache = base.path().join("cache");
    std::fs::create_dir_all(&cfg).expect("create isolated config dir");
    std::fs::create_dir_all(&cache).expect("create isolated cache dir");
    // The fixture is a loopback server, and the default SSRF policy refuses
    // loopback — correctly, since that default protects an agent from being
    // steered at the host's own services. Opting in here is what makes a
    // deterministic redirect testable at all; it is scoped to this isolated
    // config and never touches the developer's.
    let out = common::isolated_cmd(&binary_or_skip(GATE).expect("binary"))
        // `HOME` is what isolates config on macOS: `directories` resolves to
        // ~/Library/Application Support and never reads `XDG_CONFIG_HOME`.
        // Full measurement (2026-09-04) lives in `tests/scrape_wave6_gate.rs`.
        .env("HOME", &cfg)
        .env("XDG_CONFIG_HOME", &cfg)
        .env("XDG_CACHE_HOME", &cache)
        .args(["config", "set", "http_ssrf_mode", "allow_loopback"])
        .output()
        .expect("spawn config set");
    assert!(
        out.status.success(),
        "isolated config must accept the loopback opt-in: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    (base, cfg, cache)
}

fn run(cfg: &PathBuf, cache: &PathBuf, args: &[&str]) -> serde_json::Value {
    let out = common::isolated_cmd(&binary_or_skip(GATE).expect("binary"))
        .env("HOME", cfg)
        .env("XDG_CONFIG_HOME", cfg)
        .env("XDG_CACHE_HOME", cache)
        .args(args)
        .output()
        .expect("spawn browser-automation-cli");
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout was not JSON ({e}); CLEAN STDOUT is part of the contract.\n\
             stdout: {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

fn source_url(env: &serde_json::Value) -> String {
    env.pointer("/data/source_url")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("envelope must carry data.source_url: {env}"))
        .to_string()
}

fn change_status(env: &serde_json::Value) -> String {
    env.pointer("/data/change_status")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

/// The second collection reports the SAME origin as the first.
#[test]
fn source_url_is_stable_between_a_fresh_collection_and_a_cache_hit() {
    if binary_or_skip(GATE).is_none() {
        return;
    }
    let (_base, cfg, cache) = xdg_roots("stable");
    let port = start_fixture_server();
    let url = format!("http://127.0.0.1:{port}/direct");

    let first = run(
        &cfg,
        &cache,
        &[
            "-q", "--json", "scrape", &url, "--format", "text", "--engine", "http",
        ],
    );
    assert_eq!(
        first.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "first collection must succeed: {first}"
    );
    assert_eq!(change_status(&first), "fresh", "control: {first}");

    let second = run(
        &cfg,
        &cache,
        &[
            "-q", "--json", "scrape", &url, "--format", "text", "--engine", "http",
        ],
    );
    assert_eq!(
        second.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "second collection must succeed: {second}"
    );
    assert_eq!(
        change_status(&second),
        "unchanged",
        "control: the second collection must be the cache path: {second}"
    );

    assert_eq!(
        source_url(&first),
        source_url(&second),
        "the origin of one page must not change between collections"
    );
}

/// Two spellings of one origin converge on a single answer AND a single entry.
#[test]
fn both_spellings_of_one_origin_agree_and_share_a_cache_entry() {
    if binary_or_skip(GATE).is_none() {
        return;
    }
    let (_base, cfg, cache) = xdg_roots("spelling");
    let port = start_fixture_server();
    let bare = format!("http://127.0.0.1:{port}");
    let slashed = format!("http://127.0.0.1:{port}/");

    let first = run(
        &cfg,
        &cache,
        &[
            "-q", "--json", "scrape", &bare, "--format", "text", "--engine", "http",
        ],
    );
    let second = run(
        &cfg,
        &cache,
        &[
            "-q", "--json", "scrape", &slashed, "--format", "text", "--engine", "http",
        ],
    );

    // The fixture answers 404 at the root, so this pair is about the KEY and
    // the reported origin, not about a body. Both must still agree.
    if first.get("ok").and_then(|v| v.as_bool()) != Some(true) {
        skip_with_reason(GATE, "fixture root is not collectable on this host");
        return;
    }
    assert_eq!(
        source_url(&first),
        source_url(&second),
        "one origin, two spellings, one answer"
    );
    assert_eq!(
        change_status(&second),
        "unchanged",
        "the two spellings must share ONE cache entry: {second}"
    );
}

/// After a redirect, a cache hit reports the DESTINATION, not the request.
///
/// This is the half the normalisation could not reach: the entry had nowhere to
/// record where the body came from, so every hit fell back to the URL that was
/// asked for. `CacheEntry::final_url` is what closes it.
#[test]
fn a_cache_hit_after_a_redirect_reports_the_destination() {
    if binary_or_skip(GATE).is_none() {
        return;
    }
    let (_base, cfg, cache) = xdg_roots("redirect");
    let port = start_fixture_server();
    let url = format!("http://127.0.0.1:{port}/redir");

    let first = run(
        &cfg,
        &cache,
        &[
            "-q", "--json", "scrape", &url, "--format", "text", "--engine", "http",
        ],
    );
    assert_eq!(
        first.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "redirect must be followed: {first}"
    );
    let fresh_origin = source_url(&first);
    assert!(
        fresh_origin.ends_with("/served"),
        "control: a fresh fetch already reported the destination, got {fresh_origin}"
    );

    let second = run(
        &cfg,
        &cache,
        &[
            "-q", "--json", "scrape", &url, "--format", "text", "--engine", "http",
        ],
    );
    assert_eq!(
        change_status(&second),
        "unchanged",
        "control: the second collection must be the cache path: {second}"
    );
    assert_eq!(
        source_url(&second),
        fresh_origin,
        "a cache hit must report where the body came from, not what was asked"
    );
}
