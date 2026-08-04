// SPDX-License-Identifier: MIT OR Apache-2.0
//! Permanent gate for the wave-6 scrape cases the wave-6 gate left uncovered.
//!
//! `tests/scrape_wave6_gate.rs` proved the `<link rel="next">` chain, `--limit`,
//! `--max-depth`, near-duplicate reporting and the distance threshold. Three
//! behaviours were shipped as "closed" but never exercised end to end, and each
//! fails in a way no unit test can see:
//!
//! | case | what regressing looks like |
//! |---|---|
//! | `<a rel="next">` | only the `<head>` spelling is followed, so a series paginated with a plain anchor stops at page 1 |
//! | `rel=next` vs robots | pagination discovery becomes a side door that fetches a `Disallow`ed path the ordinary crawl would refuse |
//! | JSON Feed / non-feed | `--format feed` answers only for XML, so a JSON Feed looks empty and a non-feed body raises instead of reporting `found:false` |
//! | `batch-scrape --dedup-similar` | collapsing was only ever proven through `crawl`; the batch envelope uses a different key (`results`) and could silently skip the pass |
//!
//! Every case is paired with its negative control, because the dangerous
//! failure here is the silent one: a robots leak that still returns `ok`, or a
//! collapse that removes rows without a counter.
//!
//! # Skip policy
//!
//! No binary means SKIP LOUDLY — never a silent pass. The fixture server is an
//! in-process loopback TCP listener: this gate makes **no** network call.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::Command;
use std::thread;

fn binary() -> Option<PathBuf> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/browser-automation-cli");
    p.exists().then_some(p)
}

/// True when the host cannot run the gate. Prints why; never silently passes.
fn cannot_run() -> bool {
    if binary().is_none() {
        eprintln!(
            "SKIP scrape_wave7_e2e_gate: target/debug/browser-automation-cli absent. \
             This is NOT a pass; run `cargo build` first."
        );
        return true;
    }
    false
}

/// robots.txt that forbids exactly the third hop of the `rel=next` chain.
const ROBOTS: &str = "User-agent: *\nDisallow: /p3.html\n";

const JSON_FEED: &str = r#"{"version":"https://jsonfeed.org/version/1.1",
"title":"JSON Fixture","home_page_url":"https://example.com/","items":[
{"id":"1","url":"https://example.com/1","title":"JF first","content_text":"Hello",
"date_published":"2020-01-02T15:04:05Z","authors":[{"name":"Ada"}]},
{"id":"2","url":"https://example.com/2","title":"JF second"}]}"#;

/// Two near-identical bodies (one word apart) plus one unrelated body.
const DUP_A: &str = "<html><body><p>Rust is a systems programming language focused on \
memory safety speed and fearless concurrency without needing a garbage collector at \
runtime</p></body></html>";
const DUP_B: &str = "<html><body><p>Rust is a systems programming language focused on \
memory safety speed and fearless concurrency without needing any garbage collector at \
runtime</p></body></html>";
const DUP_C: &str = "<html><body><p>Baking sourdough demands patience an active starter \
careful hydration strong flour and a very hot steamy oven</p></body></html>";

/// Body for a fixture path, or `None` for 404.
fn fixture(path: &str) -> Option<(&'static str, &'static str)> {
    match path {
        "/robots.txt" => Some(("text/plain", ROBOTS)),
        "/feed.json" => Some(("application/feed+json", JSON_FEED)),
        "/notfeed.html" => Some((
            "text/html",
            "<html><body><p>plain html not a feed at all</p></body></html>",
        )),
        // a1 reaches a2 ONLY through <a rel="next">; the wave-6 gate covers the
        // <link rel="next"> spelling, so a hit on a2/a3 proves the anchor form.
        "/a1.html" => Some((
            "text/html",
            "<html><head><title>A1</title></head><body><h1>A1</h1>\
             <p>anchor chain page one distinct words</p>\
             <a rel=\"next\" href=\"/a2.html\">Next</a></body></html>",
        )),
        "/a2.html" => Some((
            "text/html",
            "<html><head><title>A2</title></head><body><h1>A2</h1>\
             <p>anchor chain page two separate words</p>\
             <a rel=\"next\" href=\"/a3.html\">Next</a></body></html>",
        )),
        "/a3.html" => Some((
            "text/html",
            "<html><head><title>A3</title></head><body><h1>A3</h1>\
             <p>anchor chain page three another words</p></body></html>",
        )),
        // p1 -> p2 -> p3 by <link rel=next>; p3 is robots-disallowed above.
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
            "<html><head><title>P3</title></head><body><h1>P3</h1>\
             <p>gamma three other</p></body></html>",
        )),
        "/d1.html" => Some(("text/html", DUP_A)),
        "/d2.html" => Some(("text/html", DUP_B)),
        "/d3.html" => Some(("text/html", DUP_C)),
        _ => None,
    }
}

/// Serve fixtures on an ephemeral loopback port until the process exits.
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
fn config_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bac-wave7-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create isolated config dir");
    dir
}

/// Run the CLI with an isolated config, returning parsed stdout JSON.
fn run(cfg: &PathBuf, args: &[&str]) -> serde_json::Value {
    let out = Command::new(binary().expect("binary"))
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

fn set(cfg: &PathBuf, key: &str, value: &str) {
    let v = run(cfg, &["-q", "--json", "config", "set", key, value]);
    assert_eq!(v["ok"], serde_json::json!(true), "config set {key}={value}");
}

/// Loopback fixtures need an SSRF exemption; `robots_loopback_exempt` is left to
/// the caller because one test deliberately keeps robots **enforced**.
fn prepare(name: &str, robots_exempt: bool) -> PathBuf {
    let cfg = config_dir(name);
    set(&cfg, "http_ssrf_mode", "allow_loopback");
    set(
        &cfg,
        "robots_loopback_exempt",
        if robots_exempt { "true" } else { "false" },
    );
    cfg
}

/// Last path segment of every crawled row, including error rows.
fn crawled_paths(v: &serde_json::Value) -> Vec<String> {
    v["data"]["pages"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|p| p["source_url"].as_str())
                .filter_map(|u| u.rsplit('/').next())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn anchor_rel_next_follows_the_chain() {
    if cannot_run() {
        return;
    }
    let cfg = prepare("anchornext", true);
    set(&cfg, "scrape_follow_rel_next", "true");
    let port = start_fixture_server();
    let url = format!("http://127.0.0.1:{port}/a1.html");
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
            "a1.html".to_string(),
            "a2.html".to_string(),
            "a3.html".to_string()
        ],
        "<a rel=next> is a real pagination spelling and must be followed too: {}",
        v["data"]
    );
}

#[test]
fn rel_next_cannot_outrank_robots_disallow() {
    if cannot_run() {
        return;
    }
    // robots stays ENFORCED on loopback: this is the whole point of the case.
    let cfg = prepare("relnextrobots", false);
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
    let pages = v["data"]["pages"].as_array().expect("pages array");
    let p3 = pages.iter().find(|p| {
        p["source_url"]
            .as_str()
            .unwrap_or_default()
            .ends_with("p3.html")
    });

    // p3 may be absent (never enqueued) or present as an explicit block, but it
    // must NEVER carry fetched content: that would be a robots leak through the
    // pagination side door.
    if let Some(row) = p3 {
        assert!(
            row.get("error").is_some(),
            "a Disallow-ed rel=next target must be reported blocked, never fetched: {row}"
        );
        assert!(
            row.get("text").is_none(),
            "robots-blocked row must carry no scraped body: {row}"
        );
    }
    // The allowed part of the chain must still have been crawled, otherwise this
    // test would pass simply because the crawl did nothing at all.
    let mut allowed = crawled_paths(&v);
    allowed.retain(|p| p == "p1.html" || p == "p2.html");
    allowed.sort();
    assert_eq!(
        allowed,
        vec!["p1.html".to_string(), "p2.html".to_string()],
        "robots must block only the Disallow-ed hop, not the whole chain: {}",
        v["data"]
    );
}

#[test]
fn json_feed_is_parsed_and_projected() {
    if cannot_run() {
        return;
    }
    let cfg = prepare("jsonfeed", true);
    let port = start_fixture_server();
    let url = format!("http://127.0.0.1:{port}/feed.json");
    let v = run(&cfg, &["-q", "--json", "scrape", &url, "--format", "feed"]);
    assert_eq!(v["data"]["feed_found"], serde_json::json!(true));
    let feed = &v["data"]["feed"];
    assert_eq!(
        feed["feed_type"],
        serde_json::json!("json"),
        "JSON Feed must be recognised as its own dialect: {feed}"
    );
    assert_eq!(feed["title"], serde_json::json!("JSON Fixture"));
    assert_eq!(feed["entry_count"], serde_json::json!(2));
    assert_eq!(feed["entries"][0]["title"], serde_json::json!("JF first"));
    assert_eq!(
        feed["entries"][0]["url"],
        serde_json::json!("https://example.com/1")
    );
    assert_eq!(feed["entries"][0]["authors"], serde_json::json!(["Ada"]));
    assert!(
        feed["entries"][0]["published"].is_string(),
        "date_published must project onto `published`: {feed}"
    );
    assert!(
        feed["entries"][1].get("authors").is_none(),
        "absent fields are omitted, never a dead null: {feed}"
    );
}

#[test]
fn non_feed_body_reports_not_found_instead_of_failing() {
    if cannot_run() {
        return;
    }
    let cfg = prepare("notfeed", true);
    let port = start_fixture_server();
    let url = format!("http://127.0.0.1:{port}/notfeed.html");
    let v = run(&cfg, &["-q", "--json", "scrape", &url, "--format", "feed"]);
    assert_eq!(
        v["ok"],
        serde_json::json!(true),
        "an HTML body under --format feed is not an error: {v}"
    );
    assert_eq!(v["data"]["feed_found"], serde_json::json!(false));
    assert_eq!(v["data"]["feed"]["found"], serde_json::json!(false));
    assert!(
        v["data"]["feed"].get("entries").is_none(),
        "a body that is not a feed must not invent an empty entry list: {}",
        v["data"]
    );
}

#[test]
fn batch_scrape_collapses_near_duplicates_and_reports_them() {
    if cannot_run() {
        return;
    }
    let cfg = prepare("batchdedup", true);
    set(&cfg, "scrape_dedup_similar_distance", "8");
    let port = start_fixture_server();
    let urls = config_dir("batchdedup").join("urls.txt");
    std::fs::write(
        &urls,
        format!(
            "http://127.0.0.1:{port}/d1.html\n\
             http://127.0.0.1:{port}/d2.html\n\
             http://127.0.0.1:{port}/d3.html\n"
        ),
    )
    .expect("write urls file");
    let urls = urls.to_string_lossy().to_string();

    let v = run(
        &cfg,
        &[
            "-q",
            "--json",
            "batch-scrape",
            "--urls-file",
            &urls,
            "--format",
            "text",
            "--dedup-similar",
        ],
    );
    assert_eq!(
        v["data"]["count"],
        serde_json::json!(2),
        "d1/d2 differ by one word and must collapse; d3 is unrelated: {}",
        v["data"]
    );
    assert_eq!(
        v["data"]["similar_collapsed"],
        serde_json::json!(1),
        "a silent collapse is indistinguishable from a failed fetch: {}",
        v["data"]
    );
    assert_eq!(v["data"]["similar_distance"], serde_json::json!(8));
    let absorbed = v["data"]["results"]
        .as_array()
        .expect("results array")
        .iter()
        .any(|r| r["similar_duplicates"] == serde_json::json!(1));
    assert!(
        absorbed,
        "the surviving row must name what it absorbed: {}",
        v["data"]
    );

    // Negative control: without the flag nothing collapses and no counter appears.
    let off = run(
        &cfg,
        &[
            "-q",
            "--json",
            "batch-scrape",
            "--urls-file",
            &urls,
            "--format",
            "text",
        ],
    );
    assert_eq!(
        off["data"]["count"],
        serde_json::json!(3),
        "collapsing must stay opt-in: {}",
        off["data"]
    );
    assert!(
        off["data"].get("similar_collapsed").is_none(),
        "an inactive pass must not add counters to the envelope: {}",
        off["data"]
    );
}
