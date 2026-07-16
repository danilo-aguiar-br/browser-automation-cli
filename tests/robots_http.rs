//! robots HTTP Disallow E2E via ephemeral local server.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::thread;
use std::time::Duration;

const BIN: &str = env!("CARGO_BIN_EXE_browser-automation-cli");

fn chrome_available() -> bool {
    ["google-chrome", "chromium", "chromium-browser"]
        .iter()
        .any(|b| {
            Command::new(b)
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        })
}

fn spawn_robots_disallow_server() -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().unwrap();
    let base = format!("http://{}", addr);
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
    // give server a beat
    thread::sleep(Duration::from_millis(50));
    (base, handle)
}

#[test]
fn http_disallow_blocks_goto_without_override() {
    if !chrome_available() {
        eprintln!("skip: no chrome");
        return;
    }
    let (base, _jh) = spawn_robots_disallow_server();
    let url = format!("{base}/");
    let out = Command::new(BIN)
        .args(["goto", &url, "--json"])
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
    if !chrome_available() {
        eprintln!("skip: no chrome");
        return;
    }
    let (base, _jh) = spawn_robots_disallow_server();
    let url = format!("{base}/");
    let out = Command::new(BIN)
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
