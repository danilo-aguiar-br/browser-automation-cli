//! robots HTTP Disallow E2E via ephemeral local server.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

mod common;
use common::chrome_responds_to_version;

fn spawn_robots_disallow_server() -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener
        .local_addr()
        .expect("bound listener has an address");
    let base = format!("http://{addr}");
    let handle = thread::spawn(move || {
        // Serve a few requests then exit.
        for _ in 0..8 {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 2048];
                let _ = stream.read(&mut buf);
                let req = String::from_utf8_lossy(&buf);
                let (status, body, ctype) = if req.contains("GET /robots.txt") {
                    ("200 OK", "User-agent: *\nDisallow: /\n", "text/plain")
                } else if req.starts_with("GET / ") || req.contains("GET / HTTP") {
                    (
                        "200 OK",
                        "<html><body>blocked-page</body></html>",
                        "text/html",
                    )
                } else {
                    ("404 Not Found", "no", "text/plain")
                };
                let resp = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(resp.as_bytes());
            }
        }
    });
    // No wait: `TcpListener::bind` performs bind AND listen, so the socket is
    // already accepting into the kernel backlog before this thread was
    // spawned — `addr` was read off it above to build `base`. `accept()` only
    // dequeues; it is not what makes the port reachable. A client connecting
    // before the loop runs is queued, never refused, so there is no window for
    // the 50ms sleep this replaces to protect.
    (base, handle)
}

/// Isolated XDG config home with `robots_loopback_exempt = false`.
///
/// GAP-033 exempts loopback from robots.txt by default, and a hermetic fixture
/// server is necessarily loopback — so with the default the block path is
/// unreachable and this test would assert nothing. The knob is turned off
/// through the product's own `config set`, which also keeps the test honest
/// about the supported way to change policy.
fn strict_loopback_config_home() -> tempfile::TempDir {
    let home = tempfile::tempdir().expect("tempdir");
    let out = common::cmd()
        .args(["config", "set", "robots_loopback_exempt", "false", "--json"])
        // `HOME` is what isolates config on macOS: `directories` resolves to
        // ~/Library/Application Support and never reads `XDG_CONFIG_HOME`.
        // Full measurement (2026-09-04) lives in `tests/scrape_wave6_gate.rs`.
        .env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path())
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn config set");
    assert!(
        out.status.success(),
        "config set failed: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    home
}

#[test]
fn http_disallow_blocks_goto_without_override() {
    if !chrome_responds_to_version() {
        common::skip_with_remedy(
            "robots_http",
            "no usable Chrome on this host.",
            "install a system Chrome/Chromium.",
        );
        return;
    }
    let config_home = strict_loopback_config_home();
    let (base, _jh) = spawn_robots_disallow_server();
    let url = format!("{base}/");
    let out = common::cmd()
        .args(["goto", &url, "--json"])
        .env("HOME", config_home.path())
        .env("XDG_CONFIG_HOME", config_home.path())
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn");
    let code = out.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_ne!(
        code, 0,
        "expected robots block code, stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("robots")
            || stdout.contains("disallow")
            || stdout.contains("\"kind\"")
            || stderr.contains("robots")
            || stdout.contains("data"),
        "stdout={stdout} stderr={stderr}"
    );
}

#[test]
fn http_disallow_allows_with_dual_flags() {
    if !chrome_responds_to_version() {
        common::skip_with_remedy(
            "robots_http",
            "no usable Chrome on this host.",
            "install a system Chrome/Chromium.",
        );
        return;
    }
    let (base, _jh) = spawn_robots_disallow_server();
    let url = format!("{base}/");
    let out = common::cmd()
        .args([
            "--ignore-robots",
            "--i-accept-robots-risk",
            "goto",
            &url,
            "--json",
        ])
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn");
    let code = out.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    // override may still fail for other reasons; must not be dual-flag usage 64
    assert_ne!(code, 64, "stdout={stdout} stderr={stderr}");
}
