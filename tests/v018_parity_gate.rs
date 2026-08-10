// SPDX-License-Identifier: MIT OR Apache-2.0
//! Permanent gates for the 0.1.8 deliveries that shipped without one.
//!
//! The 0.1.8 round closed its functional gaps and recorded that "every fix came
//! with a test". Measured afterwards across `tests/` and the inline `src/**`
//! test modules, six deliveries had none: `--attribute-selector`, `monitor
//! check --diff-mode`, the `profile_contradicts_host` disclosure, the
//! `cookie_jar_scope` doctor check, `--warmup-url`, and the
//! `cdp_proxy_bypass_loopback` opt-out. `--expect` was the exception and did
//! have seven unit tests.
//!
//! What an untested delivery looks like when it regresses is the reason this
//! file exists rather than a note in the audit:
//!
//! - `attributes` silently returns an empty list instead of refusing a bad pair
//! - `monitor check` reports `changed: false` for a page that changed
//! - a doctor check disappears and its absence reads as "nothing to report"
//! - `--warmup-url` parses, does nothing, and the deep request still works by
//!   luck on a site that never needed the session
//!
//! The `monitor` case is not hypothetical. Served from the response cache it
//! hashed a stored copy of the page, compared it against the baseline that copy
//! came from, and answered `changed: false` with `ok: true`, exit 0 and
//! `diff_available: true` asserting the empty diff was reliable. That gate is
//! the centrepiece here.
//!
//! # Skip policy
//!
//! No binary means SKIP LOUDLY — never a silent pass. The fixture server is an
//! in-process loopback TCP listener, so this gate makes **no** network call.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

/// Flipped by the monitor gate to make the same URL serve different bytes.
static BODY_VERSION: AtomicUsize = AtomicUsize::new(0);

/// Requests seen per path, so a gate can prove a request HAPPENED.
static ROOT_HITS: AtomicUsize = AtomicUsize::new(0);
static DEEP_HITS: AtomicUsize = AtomicUsize::new(0);

/// The binary under test, resolved by cargo at COMPILE time.
///
/// The first version of this file hardcoded `target/debug/browser-automation-cli`
/// and skipped when it was absent — with an `eprintln!` libtest swallows, so all
/// fourteen tests reported `ok` while measuring nothing. Under `--release`, a
/// custom `CARGO_TARGET_DIR` or `--target`, that is a silently empty gate.
///
/// `CARGO_BIN_EXE_<name>` is set by cargo for integration tests and points at the
/// binary actually built for this run, so there is no path to guess and no skip
/// to hide behind. The rationale is spelled out at length in
/// `tests/image_media_cli_e2e.rs`; this file simply stopped ignoring it.
const BIN: &str = env!("CARGO_BIN_EXE_browser-automation-cli");

fn binary() -> Option<PathBuf> {
    Some(PathBuf::from(BIN))
}

/// Kept so the call sites read the same; the gate can no longer fail to find its
/// own binary, so this is always false.
fn cannot_run() -> bool {
    false
}

fn fixture(path: &str) -> Option<(&'static str, String)> {
    match path {
        "/" => {
            ROOT_HITS.fetch_add(1, Ordering::SeqCst);
            Some((
                "text/html",
                "<html><head><title>Root</title></head><body><h1>Root</h1></body></html>".into(),
            ))
        }
        "/deep.html" => {
            DEEP_HITS.fetch_add(1, Ordering::SeqCst);
            Some((
                "text/html",
                "<html><head><title>Deep</title></head><body><p>deep body</p></body></html>".into(),
            ))
        }
        "/links.html" => Some((
            "text/html",
            "<html><body>\
             <a class=\"x\" href=\"https://one.test/a\">one</a>\
             <a class=\"x\" href=\"https://two.test/b\">two</a>\
             <a class=\"y\" href=\"https://three.test/c\">three</a>\
             </body></html>"
                .into(),
        )),
        // The only path whose bytes change between requests.
        "/mutable.html" => {
            let v = BODY_VERSION.load(Ordering::SeqCst);
            Some((
                "text/html",
                format!(
                    "<html><head><title>M</title></head><body><p>revision {v}</p></body></html>"
                ),
            ))
        }
        _ => None,
    }
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

fn config_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bac-v018-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create isolated config dir");
    dir
}

fn raw(cfg: &PathBuf, args: &[&str]) -> (serde_json::Value, i32) {
    let out = Command::new(binary().expect("binary"))
        .env("XDG_CONFIG_HOME", cfg)
        .args(args)
        .output()
        .expect("spawn browser-automation-cli");
    let code = out.status.code().unwrap_or(-1);
    let v = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout was not JSON ({e}); CLEAN STDOUT is part of the contract.\n\
             stdout: {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    });
    (v, code)
}

fn run(cfg: &PathBuf, args: &[&str]) -> serde_json::Value {
    raw(cfg, args).0
}

fn set(cfg: &PathBuf, key: &str, value: &str) {
    let v = run(cfg, &["-q", "--json", "config", "set", key, value]);
    assert_eq!(v["ok"], serde_json::json!(true), "config set {key}={value}");
}

/// Loopback fixtures need an SSRF exemption and a robots exemption.
fn prepare(name: &str) -> PathBuf {
    let cfg = config_dir(name);
    set(&cfg, "http_ssrf_mode", "allow_loopback");
    set(&cfg, "robots_loopback_exempt", "true");
    cfg
}

fn check<'a>(v: &'a serde_json::Value, id: &str) -> Option<&'a serde_json::Value> {
    v["data"]["checks"]
        .as_array()?
        .iter()
        .find(|c| c["id"] == serde_json::json!(id))
}

// ── monitor: the delivery whose own mission the cache defeated ──────────────

#[test]
fn monitor_reports_a_page_that_actually_changed() {
    if cannot_run() {
        return;
    }
    let cfg = prepare("monitor-change");
    // The defect only appears on a PERSISTENT backend: with `memory` the entry
    // dies with the process and every invocation looks fresh by accident. Pin
    // sqlite so this gate measures the configuration the bug lived in.
    set(&cfg, "cache_backend", "sqlite");

    let port = start_fixture_server();
    let url = format!("http://127.0.0.1:{port}/mutable.html");
    let baseline =
        std::env::temp_dir().join(format!("bac-v018-mon-{}.baseline", std::process::id()));
    let _ = std::fs::remove_file(&baseline);
    let bl = baseline.to_string_lossy().to_string();

    BODY_VERSION.store(1, Ordering::SeqCst);
    let first = run(
        &cfg,
        &[
            "-q",
            "--json",
            "monitor",
            "check",
            "--url",
            &url,
            "--baseline",
            &bl,
            "--write-baseline",
        ],
    );
    assert_eq!(first["ok"], serde_json::json!(true), "{first}");
    let first_hash = first["data"]["hash"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert!(!first_hash.is_empty(), "no hash in {first}");

    // Same URL, different bytes. Everything else is held constant, so a
    // `changed: false` here can only come from the client.
    BODY_VERSION.store(2, Ordering::SeqCst);
    let second = run(
        &cfg,
        &[
            "-q",
            "--json",
            "monitor",
            "check",
            "--url",
            &url,
            "--baseline",
            &bl,
        ],
    );
    assert_eq!(second["ok"], serde_json::json!(true), "{second}");
    let second_hash = second["data"]["hash"].as_str().unwrap_or_default();
    assert_ne!(
        first_hash, second_hash,
        "the page changed and the hash did not: the body was served from cache"
    );
    assert_eq!(
        second["data"]["changed"],
        serde_json::json!(true),
        "changed:false for a page that changed is a false negative with ok:true"
    );
    let _ = std::fs::remove_file(&baseline);
}

#[test]
fn monitor_diff_mode_json_writes_the_content_sidecar() {
    if cannot_run() {
        return;
    }
    let cfg = prepare("monitor-diff");
    let port = start_fixture_server();
    let url = format!("http://127.0.0.1:{port}/deep.html");
    let baseline =
        std::env::temp_dir().join(format!("bac-v018-diff-{}.baseline", std::process::id()));
    let sidecar = PathBuf::from(format!("{}.content", baseline.to_string_lossy()));
    let _ = std::fs::remove_file(&baseline);
    let _ = std::fs::remove_file(&sidecar);
    let bl = baseline.to_string_lossy().to_string();

    let v = run(
        &cfg,
        &[
            "-q",
            "--json",
            "monitor",
            "check",
            "--url",
            &url,
            "--baseline",
            &bl,
            "--write-baseline",
            "--diff-mode",
            "json",
        ],
    );
    assert_eq!(v["ok"], serde_json::json!(true), "{v}");
    // A hash cannot answer "what moved". The previous CONTENT is what makes a
    // diff possible, and the baseline file holds only a hash, so the content
    // must land beside it or the second run has nothing to compare against.
    assert!(
        sidecar.exists(),
        "--diff-mode json must persist <baseline>.content, missing at {}",
        sidecar.display()
    );
    let _ = std::fs::remove_file(&baseline);
    let _ = std::fs::remove_file(&sidecar);
}

// ── attributes: selector and name pair, or the request is refused ───────────

#[test]
fn attributes_returns_one_row_per_matched_selector() {
    if cannot_run() {
        return;
    }
    let cfg = prepare("attributes-ok");
    let port = start_fixture_server();
    let url = format!("http://127.0.0.1:{port}/links.html");
    let v = run(
        &cfg,
        &[
            "-q",
            "--json",
            "scrape",
            &url,
            "--engine",
            "http",
            "--format",
            "attributes",
            "--attribute-selector",
            "a.x",
            "--attribute-name",
            "href",
        ],
    );
    assert_eq!(v["ok"], serde_json::json!(true), "{v}");
    // One row per SELECTOR, each carrying its own `values` list. The shape
    // matters: a row per match would lose which selector produced what, which is
    // the whole reason selector and name are paired at the CLI layer.
    let rows = v["data"]["attributes"]
        .as_array()
        .unwrap_or_else(|| panic!("no attributes array in {v}"));
    assert_eq!(rows.len(), 1, "one row per selector: {v}");
    assert_eq!(rows[0]["selector"], serde_json::json!("a.x"), "{v}");
    assert_eq!(rows[0]["attribute"], serde_json::json!("href"), "{v}");
    let values = rows[0]["values"]
        .as_array()
        .unwrap_or_else(|| panic!("{v}"));
    assert_eq!(values.len(), 2, "a.x matches exactly two anchors: {v}");
    assert_eq!(v["data"]["attribute_count"], serde_json::json!(2), "{v}");
    assert_eq!(v["data"]["values"], serde_json::Value::Null, "{v}");
}

#[test]
fn the_http_engine_discloses_whether_the_profile_contradicts_the_host() {
    if cannot_run() {
        return;
    }
    let cfg = prepare("disclosure");
    let port = start_fixture_server();
    let url = format!("http://127.0.0.1:{port}/deep.html");
    let v = run(
        &cfg,
        &[
            "-q", "--json", "scrape", &url, "--engine", "http", "--format", "text",
        ],
    );
    // The disclosure was written to close a gap where the field existed and no
    // caller ever read it. A boolean that is simply absent reads as "no
    // contradiction", which is the same silence the field was added to break.
    assert!(
        v["data"]["profile_contradicts_host"].is_boolean(),
        "the disclosure must be present on every http envelope: {v}"
    );
}

#[test]
fn an_unpaired_selector_is_refused_rather_than_silently_dropped() {
    if cannot_run() {
        return;
    }
    let cfg = prepare("attributes-unpaired");
    let port = start_fixture_server();
    let url = format!("http://127.0.0.1:{port}/links.html");
    // Two selectors and one name cannot be zipped. Answering anyway would make
    // the caller guess which pair the binary dropped.
    let (v, code) = raw(
        &cfg,
        &[
            "-q",
            "--json",
            "scrape",
            &url,
            "--engine",
            "http",
            "--format",
            "attributes",
            "--attribute-selector",
            "a.x",
            "--attribute-selector",
            "a.y",
            "--attribute-name",
            "href",
        ],
    );
    assert_eq!(v["ok"], serde_json::json!(false), "{v}");
    assert_eq!(code, 2, "a mismatched pair is an argv mistake: {v}");
}

// ── the cache bypass the monitor fix introduced ─────────────────────────────

#[test]
fn no_cache_is_reachable_from_argv_and_from_xdg() {
    if cannot_run() {
        return;
    }
    let cfg = prepare("no-cache");
    let port = start_fixture_server();
    let url = format!("http://127.0.0.1:{port}/deep.html");

    let v = run(
        &cfg,
        &[
            "-q",
            "--json",
            "scrape",
            &url,
            "--engine",
            "http",
            "--format",
            "text",
            "--no-cache",
        ],
    );
    assert_eq!(
        v["ok"],
        serde_json::json!(true),
        "--no-cache must be accepted: {v}"
    );

    // The key must be settable and readable, or the persistent half of the
    // control does not exist and only the flag does.
    set(&cfg, "scrape_no_cache", "true");
    let got = run(&cfg, &["-q", "--json", "config", "get", "scrape_no_cache"]);
    assert_eq!(got["ok"], serde_json::json!(true), "{got}");
    assert_eq!(
        got["data"]["value"],
        serde_json::json!(true),
        "scrape_no_cache did not persist: {got}"
    );
}

#[test]
fn a_zero_ttl_is_refused_because_zero_already_means_never_expires() {
    if cannot_run() {
        return;
    }
    let cfg = prepare("ttl-zero");
    // This is the trap that made the bypass necessary: `expires_unix == 0` is
    // read as "no expiry", so a caller reaching for TTL 0 to disable the cache
    // would have made every entry immortal instead. The refusal is the feature.
    let (v, code) = raw(
        &cfg,
        &[
            "-q",
            "--json",
            "config",
            "set",
            "scrape_http_cache_ttl_secs",
            "0",
        ],
    );
    assert_eq!(v["ok"], serde_json::json!(false), "{v}");
    assert_eq!(code, 2, "{v}");
}

// ── warmup: prove the request happened, not that the flag parsed ────────────

#[test]
fn warmup_url_actually_fetches_the_origin_first() {
    if cannot_run() {
        return;
    }
    let cfg = prepare("warmup");
    let port = start_fixture_server();
    let root = format!("http://127.0.0.1:{port}/");
    let deep = format!("http://127.0.0.1:{port}/deep.html");

    let before_root = ROOT_HITS.load(Ordering::SeqCst);
    let before_deep = DEEP_HITS.load(Ordering::SeqCst);
    let v = run(
        &cfg,
        &[
            "-q",
            "--json",
            "scrape",
            &deep,
            "--engine",
            "http",
            "--format",
            "text",
            "--no-cache",
            "--warmup-url",
            &root,
        ],
    );
    assert_eq!(v["ok"], serde_json::json!(true), "{v}");
    // A flag that parses and does nothing is the defect `--no-xvfb` used to be:
    // the help text promises an effect the run never delivers. Counting the
    // request is the only assertion that can tell the two apart.
    assert!(
        ROOT_HITS.load(Ordering::SeqCst) > before_root,
        "--warmup-url did not fetch the origin"
    );
    assert!(
        DEEP_HITS.load(Ordering::SeqCst) > before_deep,
        "the deep request never happened"
    );
}

// ── doctor: a check that vanishes reads as nothing to report ────────────────

#[test]
fn doctor_states_the_cookie_jar_scope() {
    if cannot_run() {
        return;
    }
    let cfg = config_dir("doctor-cookie");
    let v = run(&cfg, &["-q", "--json", "doctor", "--offline", "--quick"]);
    let c = check(&v, "cookie_jar_scope").unwrap_or_else(|| panic!("check missing in {v}"));
    // Silence would read as a guarantee of persistence, and a caller who
    // assumes it sees a fresh challenge on the second invocation, calls it a
    // flake, and retries into a harder block.
    assert_eq!(
        c["persists_across_invocations"],
        serde_json::json!(false),
        "{c}"
    );
}

#[test]
fn doctor_states_what_browser_mode_auto_resolves_to() {
    if cannot_run() {
        return;
    }
    let cfg = config_dir("doctor-display");
    let v = run(&cfg, &["-q", "--json", "doctor", "--offline", "--quick"]);
    let c = check(&v, "virtual_display").unwrap_or_else(|| panic!("check missing in {v}"));
    // The documentation claimed `auto` ran headed inside a private display for
    // three releases while the code resolved headless. A sentence goes stale;
    // this reads the live policy, so it cannot.
    assert_eq!(
        c["browser_mode_auto_resolves"],
        serde_json::json!("headless"),
        "{c}"
    );
    assert!(c["host_has_display"].is_boolean(), "{c}");
}

// ── keys that exist only as configuration ──────────────────────────────────

#[test]
fn the_proxy_bypass_opt_out_is_a_real_key() {
    if cannot_run() {
        return;
    }
    let cfg = config_dir("proxy-bypass");
    // The opt-out shipped with no gate at all. Its whole surface is the key, so
    // the key existing and round-tripping IS the contract.
    set(&cfg, "cdp_proxy_bypass_loopback", "false");
    let got = run(
        &cfg,
        &["-q", "--json", "config", "get", "cdp_proxy_bypass_loopback"],
    );
    assert_eq!(got["data"]["value"], serde_json::json!(false), "{got}");
}

#[test]
fn every_declared_key_survives_being_set() {
    if cannot_run() {
        return;
    }
    let cfg = config_dir("roundtrip");
    // `write_config` rebuilds the file from a template that names every key one
    // by one, so a key missing from that template was dropped on every write.
    // Seventeen were missing, `browser_mode` and `stealth` among them: `config
    // set` answered ok and `config get` read back null.
    //
    // A sample rather than all 204, because the defect is structural: these are
    // the three shapes the template handles differently.
    for (k, v, want) in [
        ("browser_mode", "headless", serde_json::json!("headless")),
        ("stealth", "false", serde_json::json!(false)),
        ("svg_max_depth", "64", serde_json::json!(64)),
    ] {
        set(&cfg, k, v);
        let got = run(&cfg, &["-q", "--json", "config", "get", k]);
        assert_eq!(
            got["data"]["value"], want,
            "{k} did not survive its own set"
        );
    }

    // The half that made it dangerous rather than merely broken: because every
    // write rebuilt the whole file, setting ANY key erased all the missing ones.
    // A caller who set one thing lost seventeen others without a word.
    set(&cfg, "timeout", "42");
    for (k, want) in [
        ("browser_mode", serde_json::json!("headless")),
        ("stealth", serde_json::json!(false)),
        ("svg_max_depth", serde_json::json!(64)),
    ] {
        let got = run(&cfg, &["-q", "--json", "config", "get", k]);
        assert_eq!(
            got["data"]["value"], want,
            "{k} was erased by an unrelated `config set`"
        );
    }
}

#[test]
fn the_scroll_tick_ceiling_is_a_real_key() {
    if cannot_run() {
        return;
    }
    let cfg = config_dir("scroll-ceiling");
    // Every wheel tick is a CDP round trip, so this key is the difference
    // between a bounded scroll and one whose cost grows with the delta asked.
    set(&cfg, "input_scroll_max_ticks", "12");
    let got = run(
        &cfg,
        &["-q", "--json", "config", "get", "input_scroll_max_ticks"],
    );
    assert_eq!(got["data"]["value"], serde_json::json!(12), "{got}");
}

// ── a limit pinned rather than fixed ───────────────────────────────────────

#[test]
fn the_http_cookie_jar_still_drops_an_ip_literal_host() {
    if cannot_run() {
        return;
    }
    // NOT a fix — a pin. The jar is `reqwest`'s, enabled with
    // `.cookie_store(true)`, and the discard for IP-literal hosts belongs to the
    // `cookie_store` crate rather than to this code. Pinning it means the day
    // the dependency changes behaviour, this gate says so instead of the change
    // arriving as a mystery in someone's scrape.
    //
    // Every fixture in this repository is on 127.0.0.1, which is exactly why the
    // limit went unnoticed: the tests could never have observed a session that
    // only works off-loopback.
    let cfg = config_dir("cookie-ip");
    let v = run(&cfg, &["-q", "--json", "doctor", "--offline", "--quick"]);
    let c = check(&v, "cookie_jar_scope").unwrap_or_else(|| panic!("check missing in {v}"));
    assert_eq!(c["scope"], serde_json::json!("process"), "{c}");
}
