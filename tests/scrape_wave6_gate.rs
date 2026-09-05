//! Permanent gate for the wave-6 scrape backlog: feed, `rel=next`, near-dup.
//!
//! # What has to break for this to fail
//!
//! Three behaviours were shipped together and each has a distinct way of
//! silently regressing:
//!
//! | case | what regressing looks like |
//! |---|---|
//! | `--format feed` | a feed URL answers `ok` with no `feed` payload, so the agent sees an empty page instead of entries |
//! | `rel=next` | a paginated series stops at page 1 because `<link rel="next">` lives in `<head>` and carries no anchor |
//! | near-dup collapse | rows disappear with **no** `similar_collapsed` counter, so an agent cannot tell a collapse from a fetch failure |
//!
//! Each case is paired with its **negative control**, because the failure that
//! matters most here is the silent one. A test that only asserted "entries came
//! back" would stay green if `rel=next` followed links unconditionally and blew
//! past `--max-depth`; a test that only asserted "rows were collapsed" would
//! stay green if collapsing ran even when disabled. So this file asserts both
//! the ON behaviour and that the OFF behaviour is genuinely untouched, plus
//! that `--limit` and `--max-depth` still bind a `rel=next` chain.
//!
//! # Skip policy
//!
//! No binary means SKIP LOUDLY — never a silent pass. The fixture server is a
//! plain loopback TCP listener in-process: this gate makes **no** network call.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::thread;

mod common;
use common::{binary, missing_binary};

const GATE: &str = "scrape_wave6_gate";

/// True when the host cannot run the gate. Prints why; never silently passes.
fn cannot_run() -> bool {
    if missing_binary(GATE) {
        return true;
    }
    false
}

const RSS: &str = "<?xml version=\"1.0\"?><rss version=\"2.0\"><channel>\
<title>Fixture Feed</title><link>https://example.com/</link><description>d</description>\
<item><title>First post</title><link>https://example.com/1</link>\
<pubDate>Mon, 02 Jan 2006 15:04:05 GMT</pubDate><description>Hello</description></item>\
<item><title>Second post</title><link>https://example.com/2</link></item>\
</channel></rss>";

const DUP_A: &str = "<html><head><title>A</title></head><body><p>Rust is a systems \
programming language focused on memory safety speed and fearless concurrency \
without needing a garbage collector at runtime</p></body></html>";

const DUP_B: &str = "<html><head><title>B</title></head><body><p>Rust is a systems \
programming language focused on memory safety speed and fearless concurrency \
without needing any garbage collector at runtime</p></body></html>";

const DUP_C: &str = "<html><head><title>C</title></head><body><p>Baking sourdough \
demands patience an active starter careful hydration strong flour and a very hot \
steamy oven</p></body></html>";

/// Body for a fixture path, or `None` for 404.
fn fixture(path: &str) -> Option<(&'static str, &'static str)> {
    match path {
        "/feed.xml" => Some(("application/rss+xml", RSS)),
        // p1 reaches p2 ONLY through <link rel="next"> — no anchor at all, so a
        // hit on p2 proves rel=next discovery rather than ordinary link following.
        "/p1.html" => Some((
            "text/html",
            "<html><head><title>P1</title><link rel=\"next\" href=\"/p2.html\"></head>\
             <body><h1>P1</h1><p>alpha one distinct</p></body></html>",
        )),
        "/p2.html" => Some((
            "text/html",
            "<html><head><title>P2</title><link rel=\"next\" href=\"/p3.html\"></head>\
             <body><h1>P2</h1><p>beta two different</p></body></html>",
        )),
        "/p3.html" => Some((
            "text/html",
            "<html><head><title>P3</title></head><body><h1>P3</h1><p>gamma three other</p></body></html>",
        )),
        "/hub.html" => Some((
            "text/html",
            "<html><head><title>Hub</title></head><body>\
             <a href=\"/a.html\">A</a><a href=\"/b.html\">B</a><a href=\"/c.html\">C</a>\
             <p>hub navigation index listing entries elsewhere</p></body></html>",
        )),
        "/a.html" => Some(("text/html", DUP_A)),
        "/b.html" => Some(("text/html", DUP_B)),
        "/c.html" => Some(("text/html", DUP_C)),
        _ => None,
    }
}

/// Serve fixtures on an ephemeral loopback port until the process exits.
///
/// Returns the bound port. The thread is detached on purpose: the gate is
/// short-lived and the listener dies with the test binary.
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
    let response = match fixture(&path) {
        Some((ctype, body)) => format!(
            "HTTP/1.1 200 OK\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ),
        None => "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
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
        .prefix(&format!("bac-wave6-{name}-"))
        .tempdir()
        .expect("create isolated config dir")
}

/// Run the CLI with an isolated config, returning parsed stdout JSON.
fn run(cfg: &Path, args: &[&str]) -> serde_json::Value {
    let out = common::isolated_cmd(&binary().expect("binary"))
        // BOTH, and `HOME` is the one that actually isolates on macOS.
        //
        // `directories` resolves to `~/Library/Application Support/...` there
        // and never reads `XDG_CONFIG_HOME`, so this override alone steered
        // nothing: every test in this binary shared the one config derived from
        // the process-wide sandbox `HOME`, and cargo runs them in PARALLEL.
        //
        // Measured 2026-09-04: 7 of 8 failed with `config set` returning
        // `ok:false` as tests raced the same file; `--test-threads=1` cut it to
        // 1, and that survivor failed because `scrape_follow_rel_next` was left
        // ON by a sibling test — leakage, not a product defect. On Linux the
        // XDG variable works and none of this was ever visible.
        .env("HOME", cfg)
        .env("XDG_CONFIG_HOME", cfg)
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

fn set(cfg: &Path, key: &str, value: &str) {
    let v = run(cfg, &["-q", "--json", "config", "set", key, value]);
    assert_eq!(v["ok"], serde_json::json!(true), "config set {key}={value}");
}

/// Loopback fixtures need SSRF and robots exemptions; both are XDG knobs.
fn prepare(name: &str) -> tempfile::TempDir {
    let cfg = config_dir(name);
    set(cfg.path(), "http_ssrf_mode", "allow_loopback");
    set(cfg.path(), "robots_loopback_exempt", "true");
    cfg
}

#[test]
fn feed_format_returns_structured_entries() {
    if cannot_run() {
        return;
    }
    let cfg_dir = prepare("feed");
    let cfg = cfg_dir.path().to_path_buf();
    let port = start_fixture_server();
    let url = format!("http://127.0.0.1:{port}/feed.xml");
    let v = run(&cfg, &["-q", "--json", "scrape", &url, "--format", "feed"]);
    assert_eq!(v["ok"], serde_json::json!(true), "envelope: {v}");
    let feed = &v["data"]["feed"];
    assert_eq!(v["data"]["feed_found"], serde_json::json!(true));
    assert_eq!(feed["feed_type"], serde_json::json!("rss2"));
    assert_eq!(feed["title"], serde_json::json!("Fixture Feed"));
    assert_eq!(feed["entry_count"], serde_json::json!(2));
    assert_eq!(feed["entries"][0]["title"], serde_json::json!("First post"));
    assert_eq!(
        feed["entries"][0]["url"],
        serde_json::json!("https://example.com/1")
    );
    assert!(
        feed["entries"][0]["published"].is_string(),
        "published date must survive parsing"
    );
    // CLEAN STDOUT: an absent field is omitted, never a dead null.
    assert!(
        feed["entries"][1].get("published").is_none(),
        "missing pubDate must be omitted, not null: {}",
        feed["entries"][1]
    );
}

#[test]
fn feed_max_entries_cap_is_honoured_and_flagged() {
    if cannot_run() {
        return;
    }
    let cfg_dir = prepare("feedcap");
    let cfg = cfg_dir.path().to_path_buf();
    set(&cfg, "scrape_feed_max_entries", "1");
    let port = start_fixture_server();
    let url = format!("http://127.0.0.1:{port}/feed.xml");
    let v = run(&cfg, &["-q", "--json", "scrape", &url, "--format", "feed"]);
    let feed = &v["data"]["feed"];
    assert_eq!(feed["entry_count"], serde_json::json!(1));
    assert_eq!(feed["entry_total"], serde_json::json!(2));
    assert_eq!(
        feed["truncated"],
        serde_json::json!(true),
        "a capped feed must say so, or the agent believes it saw everything"
    );
}

/// Collect crawled `source_url` values from a crawl envelope.
fn crawled_paths(v: &serde_json::Value) -> Vec<String> {
    v["data"]["pages"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|p| p["source_url"].as_str())
                .filter_map(|u| u.rsplit('/').next().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn rel_next_is_not_followed_by_default() {
    if cannot_run() {
        return;
    }
    let cfg_dir = prepare("relnextoff");
    let cfg = cfg_dir.path().to_path_buf();
    let port = start_fixture_server();
    let url = format!("http://127.0.0.1:{port}/p1.html");
    let v = run(
        &cfg,
        &[
            "-q",
            "--json",
            "crawl",
            &url,
            "--limit",
            "10",
            "--max-depth",
            "3",
            "--format",
            "text",
        ],
    );
    let mut paths = crawled_paths(&v);
    paths.sort();
    assert_eq!(
        paths,
        vec!["p1.html".to_string()],
        "default must stay OFF: following rel=next unasked changes every existing crawl"
    );
}

#[test]
fn rel_next_follows_the_chain_when_enabled() {
    if cannot_run() {
        return;
    }
    let cfg_dir = prepare("relnexton");
    let cfg = cfg_dir.path().to_path_buf();
    set(&cfg, "scrape_follow_rel_next", "true");
    let port = start_fixture_server();
    let url = format!("http://127.0.0.1:{port}/p1.html");
    let v = run(
        &cfg,
        &[
            "-q",
            "--json",
            "crawl",
            &url,
            "--limit",
            "10",
            "--max-depth",
            "3",
            "--format",
            "text",
        ],
    );
    let mut paths = crawled_paths(&v);
    paths.sort();
    assert_eq!(
        paths,
        vec![
            "p1.html".to_string(),
            "p2.html".to_string(),
            "p3.html".to_string()
        ],
        "head-only <link rel=next> must reach the whole series"
    );
    assert_eq!(v["data"]["follow_rel_next"], serde_json::json!(true));
}

#[test]
fn rel_next_still_obeys_limit_and_max_depth() {
    if cannot_run() {
        return;
    }
    let cfg_dir = prepare("relnextbounds");
    let cfg = cfg_dir.path().to_path_buf();
    set(&cfg, "scrape_follow_rel_next", "true");
    let port = start_fixture_server();
    let url = format!("http://127.0.0.1:{port}/p1.html");

    // --limit must cap the series even though more pages are reachable.
    let v = run(
        &cfg,
        &[
            "-q",
            "--json",
            "crawl",
            &url,
            "--limit",
            "2",
            "--max-depth",
            "5",
            "--format",
            "text",
        ],
    );
    assert_eq!(
        v["data"]["count"],
        serde_json::json!(2),
        "rel=next must never outrank --limit"
    );

    // --max-depth 1 admits p2 (depth 1) but must stop before p3 (depth 2).
    let v = run(
        &cfg,
        &[
            "-q",
            "--json",
            "crawl",
            &url,
            "--limit",
            "10",
            "--max-depth",
            "1",
            "--format",
            "text",
        ],
    );
    let mut paths = crawled_paths(&v);
    paths.sort();
    assert_eq!(
        paths,
        vec!["p1.html".to_string(), "p2.html".to_string()],
        "rel=next must never outrank --max-depth"
    );
}

#[test]
fn near_duplicate_collapse_is_off_by_default() {
    if cannot_run() {
        return;
    }
    let cfg_dir = prepare("dedupoff");
    let cfg = cfg_dir.path().to_path_buf();
    let port = start_fixture_server();
    let url = format!("http://127.0.0.1:{port}/hub.html");
    let v = run(
        &cfg,
        &[
            "-q",
            "--json",
            "crawl",
            &url,
            "--limit",
            "10",
            "--max-depth",
            "1",
            "--format",
            "text",
        ],
    );
    assert_eq!(
        v["data"]["count"],
        serde_json::json!(4),
        "hub + 3 leaves must all survive when collapsing is off"
    );
    assert!(
        v["data"].get("similar_collapsed").is_none(),
        "an inactive pass must not add counters to the envelope"
    );
}

#[test]
fn near_duplicate_collapse_reports_what_it_removed() {
    if cannot_run() {
        return;
    }
    let cfg_dir = prepare("dedupon");
    let cfg = cfg_dir.path().to_path_buf();
    set(&cfg, "scrape_dedup_similar", "true");
    set(&cfg, "scrape_dedup_similar_distance", "8");
    let port = start_fixture_server();
    let url = format!("http://127.0.0.1:{port}/hub.html");
    let v = run(
        &cfg,
        &[
            "-q",
            "--json",
            "crawl",
            &url,
            "--limit",
            "10",
            "--max-depth",
            "1",
            "--format",
            "text",
        ],
    );
    // a.html and b.html differ by one word; c.html and the hub are distinct.
    assert_eq!(v["data"]["count"], serde_json::json!(3));
    assert_eq!(
        v["data"]["similar_collapsed"],
        serde_json::json!(1),
        "a silent collapse is indistinguishable from a lost fetch: {}",
        v["data"]
    );
    assert_eq!(v["data"]["similar_distance"], serde_json::json!(8));
    let survivor_absorbed = v["data"]["pages"]
        .as_array()
        .expect("pages array")
        .iter()
        .any(|p| p["similar_duplicates"] == serde_json::json!(1));
    assert!(
        survivor_absorbed,
        "the surviving row must name what it absorbed: {}",
        v["data"]
    );
}

#[test]
fn near_duplicate_distance_zero_keeps_merely_similar_pages() {
    if cannot_run() {
        return;
    }
    let cfg_dir = prepare("dedupstrict");
    let cfg = cfg_dir.path().to_path_buf();
    set(&cfg, "scrape_dedup_similar", "true");
    set(&cfg, "scrape_dedup_similar_distance", "0");
    let port = start_fixture_server();
    let url = format!("http://127.0.0.1:{port}/hub.html");
    let v = run(
        &cfg,
        &[
            "-q",
            "--json",
            "crawl",
            &url,
            "--limit",
            "10",
            "--max-depth",
            "1",
            "--format",
            "text",
        ],
    );
    assert_eq!(
        v["data"]["count"],
        serde_json::json!(4),
        "distance 0 demands identical fingerprints, so a one-word edit must survive"
    );
    assert_eq!(v["data"]["similar_collapsed"], serde_json::json!(0));
}
