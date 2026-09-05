//! Permanent gate for content-coding round trips (gaps.md G15).
//!
//! # What has to break for this to fail
//!
//! The stealth identity advertises `accept-encoding: gzip, deflate, br, zstd`
//! because Chrome does. For most of the product's life the HTTP client was
//! built without the matching decoders, so servers honoured the offer and the
//! CLI handed the caller a gzip-framed byte string as if it were page text —
//! with `ok: true`, `status_code: 200` and exit 0.
//!
//! Measured on five public hosts before the fix: every `data.text` began with
//! the `1f 8b` gzip magic. Nothing in the envelope disagreed, because every
//! transport-shaped field was genuinely fine. The bytes were the only witness.
//!
//! # Why this is the same defect as the PDF one
//!
//! G12 was the first instance of the class: content decoded with the wrong
//! reader, delivered as success. G15 is the second. What both had in common is
//! that no test ever pointed the product at a server whose response needed
//! decoding — the local fixtures all served `identity`. This file removes that
//! blind spot, so a third instance has somewhere to fail.
//!
//! # The structural assertion is the durable half
//!
//! Checking that the text is readable catches today's bug. Checking that no
//! `content-encoding` survives in the envelope catches the general one:
//! `tower-http` strips that header precisely when it has decompressed, so a
//! surviving value is proof the body was passed through raw. That assertion
//! keeps working for a coding nobody has thought of yet.
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

const GATE: &str = "compression_gate";

/// Prose long enough to survive main-content extraction, and distinctive
/// enough that a partial decode cannot pass by accident.
const PAGE: &str = "<html><head><title>Compression Fixture</title></head><body>\
<h1>Compression Fixture</h1><p>This page exists to prove that a response \
carrying a content coding arrives at the caller as readable text rather than \
as framed bytes. The sentinel phrase is decompressed-payload-marker and it \
must appear verbatim in the scrape envelope.</p></body></html>";

/// The phrase a raw gzip frame cannot contain.
const SENTINEL: &str = "decompressed-payload-marker";

fn cannot_run() -> bool {
    if missing_binary(GATE) {
        return true;
    }
    false
}

/// Deflate-format bytes for `PAGE`, produced by the same crate the client uses.
///
/// Building the fixture with a compressor rather than a checked-in blob keeps
/// the test honest about what a real server sends, and makes the expected
/// plaintext impossible to get wrong.
fn gzip_page() -> Vec<u8> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(PAGE.as_bytes()).expect("gzip fixture body");
    enc.finish().expect("finish gzip fixture")
}

fn deflate_page() -> Vec<u8> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(PAGE.as_bytes())
        .expect("deflate fixture body");
    enc.finish().expect("finish deflate fixture")
}

fn brotli_page() -> Vec<u8> {
    let mut out = Vec::new();
    let mut input = PAGE.as_bytes();
    brotli::BrotliCompress(&mut input, &mut out, &Default::default()).expect("brotli fixture body");
    out
}

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

/// Write a response whose body is already encoded, declaring the coding.
fn encoded_response(stream: &mut TcpStream, coding: &str, body: &[u8]) {
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Encoding: {coding}\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
}

fn serve_one(mut stream: TcpStream) {
    let mut line = String::new();
    if BufReader::new(&stream).read_line(&mut line).is_err() {
        return;
    }
    let path = line.split_whitespace().nth(1).unwrap_or("/").to_string();
    match path.as_str() {
        "/gzip.html" => encoded_response(&mut stream, "gzip", &gzip_page()),
        "/deflate.html" => encoded_response(&mut stream, "deflate", &deflate_page()),
        "/brotli.html" => encoded_response(&mut stream, "br", &brotli_page()),
        // The negative control: same server, same command, no coding at all.
        "/identity.html" => {
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\
                 Connection: close\r\n\r\n",
                PAGE.len()
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(PAGE.as_bytes());
        }
        "/robots.txt" => {
            let body = "User-agent: *\nAllow: /\n";
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\
                 Connection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(resp.as_bytes());
        }
        _ => {
            let _ = stream.write_all(
                b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
        }
    }
}

/// An isolated XDG config dir so the gate never reads the developer's config.
///
/// The guard is returned, not the path: dropping it removes the directory, and a
/// caller holding only the path would hand the CLI a config home that no longer
/// exists. The old body used a pid-keyed name with no removal at all, so every
/// run left one directory per case behind for good.
fn config_dir(name: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("bac-compress-{name}-"))
        .tempdir()
        .expect("create isolated config dir")
}

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
/// The defaults refuse loopback on purpose, so this is the gate adapting to
/// the product's policy rather than the product relaxing for the gate.
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

/// Scrape one fixture path on the HTTP engine; returns (exit code, stdout).
fn scrape(name: &str, port: u16, path: &str) -> (Option<i32>, serde_json::Value) {
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
            "http",
        ])
        .output()
        .expect("spawn");
    let value = serde_json::from_slice(&out.stdout).unwrap_or(serde_json::Value::Null);
    (out.status.code(), value)
}

/// Every coding the identity advertises must round-trip to readable text.
///
/// Driven from one list rather than four copies: the failure message names the
/// coding, and adding a fifth coding to the header without adding a decoder
/// fails here instead of in production.
#[test]
fn every_advertised_coding_arrives_decoded() {
    if cannot_run() {
        return;
    }
    let port = start_fixture_server();

    for (name, path, coding) in [
        ("gzip", "/gzip.html", "gzip"),
        ("deflate", "/deflate.html", "deflate"),
        ("brotli", "/brotli.html", "br"),
    ] {
        let (code, value) = scrape(name, port, path);
        assert_eq!(code, Some(0), "{coding}: expected exit 0, got {code:?}");
        assert_eq!(value["ok"], serde_json::json!(true), "{coding}: {value}");

        let text = value["data"]["text"].as_str().unwrap_or_default();
        assert!(
            text.contains(SENTINEL),
            "{coding}: body did not decode; got {:?}",
            text.chars().take(64).collect::<String>()
        );
        // The exact byte pattern the audit found in production output.
        assert!(
            !text.starts_with('\u{1f}'),
            "{coding}: envelope carries a raw compressed frame"
        );
    }
}

/// The negative control: an uncompressed page must behave identically.
///
/// Without it, a build that mangled every response the same way could still
/// satisfy the cases above by accident.
#[test]
fn an_uncompressed_page_is_unaffected() {
    if cannot_run() {
        return;
    }
    let port = start_fixture_server();
    let (code, value) = scrape("identity", port, "/identity.html");
    assert_eq!(code, Some(0), "{value}");
    assert!(value["data"]["text"]
        .as_str()
        .unwrap_or_default()
        .contains(SENTINEL));
}

/// The general assertion: a surviving `content-encoding` proves no decode.
///
/// `tower-http` removes the header exactly when it has decompressed the body,
/// so this holds for codings that do not exist yet. It is the half of this
/// gate that will still be doing work after the current bug is forgotten.
#[test]
fn no_content_encoding_survives_into_the_envelope() {
    if cannot_run() {
        return;
    }
    let port = start_fixture_server();
    let (_, value) = scrape("hdr", port, "/gzip.html");
    let serialized = value.to_string().to_ascii_lowercase();
    assert!(
        !serialized.contains("\"content-encoding\""),
        "a declared content coding reached the caller undecoded: {serialized}"
    );
}
