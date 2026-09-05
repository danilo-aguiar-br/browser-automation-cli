// SPDX-License-Identifier: MIT OR Apache-2.0
//! Permanent gate: the WebGL mask must answer the SAME string inside a worker.
//!
//! # Why this gate reads a worker and not a canvas
//!
//! The main thread already passed. Measured 2026-09-04 over 10 launches, the
//! canvas, the WebGL2 canvas, a main-thread `OffscreenCanvas`, a
//! `WEBGL_debug_renderer_info` read through `getExtension` and a pristine
//! `about:blank` iframe realm leaked 0 of 10. A gate that reads any of those
//! would have been green against the broken build, which is the same reason
//! the defect survived every casual check for a whole release.
//!
//! The leak lived in worker scope, and only there:
//!
//! | path | leaked, of 10 launches |
//! |---|---|
//! | `Worker` from a `blob:` URL | 10 |
//! | `Worker` with `{type:"module"}` | 10 |
//! | classic same-origin `Worker` | 8 |
//!
//! A `Worker` has its own global scope and no document, so
//! `Page.addScriptToEvaluateOnNewDocument` never reaches it. The crate's own
//! `Worker` wrapper bails out for `{type:"module"}`, for every scheme that is
//! not `http:`/`https:` — `blob:` is a scheme — and for cross-origin.
//!
//! # What regressing looks like
//!
//! | case | the symptom |
//! |---|---|
//! | worker scope loses the mask | the page reports a masked GPU while a worker reports the HOST GPU, which is a pair no real machine produces |
//! | `navigator.platform` stops travelling | the window says `Linux x86_64` and the worker says `MacIntel` in the same session |
//! | the `toString` mask regresses to an own property | `Function.prototype.toString.call(getParameter)` hands back the patch's own source |
//!
//! # Why the assertion is equality and not a denylist
//!
//! Listing the strings that must not appear only catches the rasterisers
//! somebody remembered. Requiring every scope to answer the SAME string
//! catches any divergence, including one produced by a GPU nobody has seen.
//!
//! # Skip policy
//!
//! No binary, or no usable Chrome, means SKIP LOUDLY — never a silent pass.
//! The fixture is served from an in-process loopback listener, so this gate
//! makes NO network call.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::thread;

mod common;
use common::{binary, chrome_mentioned_in_doctor_json, missing_binary};

const GATE: &str = "webgl_worker_scope_gate";

/// Repeats, because the defect was REPORTED as intermittent.
///
/// The measurement says it is not — the leak is a coverage hole and every
/// affected path leaked on every launch. The repeat is kept anyway: it costs
/// two extra launches and it is the only thing that would tell a future
/// reader whether a NEW failure is deterministic or flaky. One launch cannot.
const LAUNCHES: usize = 3;

fn cannot_run() -> bool {
    if missing_binary(GATE) {
        return true;
    }
    if !chrome_mentioned_in_doctor_json() {
        common::skip_with_remedy(
            GATE,
            "doctor reports no usable Chrome.",
            "install a system Chrome/Chromium.",
        );
        return true;
    }
    false
}

/// Read the renderer the way a detector does, and never the way a naive
/// fixture does.
///
/// `getParameter(37446)` returns `null` in a scope that has not asked for
/// `WEBGL_debug_renderer_info` first. A fixture that skips the extension
/// measures `null` on the leaking paths and reports a clean run.
const READ_JS: &str = "function rd(g){if(!g)return 'NOCTX';try{g.getExtension('WEBGL_debug_renderer_info');return String(g.getParameter(37446));}catch(e){return 'ERR:'+e}}";

const CLASSIC_WORKER: &str = "self.onmessage=function(){var o={};\
function rd(g){if(!g)return 'NOCTX';try{g.getExtension('WEBGL_debug_renderer_info');return String(g.getParameter(37446));}catch(e){return 'ERR:'+e}}\
try{o.classic_worker=rd(new OffscreenCanvas(1,1).getContext('webgl'));}catch(e){o.classic_worker='ERR:'+e}\
o.classic_worker_platform=String(navigator.platform);self.postMessage(o);};";

const MODULE_WORKER: &str = "self.onmessage=function(){var o={};\
function rd(g){if(!g)return 'NOCTX';try{g.getExtension('WEBGL_debug_renderer_info');return String(g.getParameter(37446));}catch(e){return 'ERR:'+e}}\
try{o.module_worker=rd(new OffscreenCanvas(1,1).getContext('webgl'));}catch(e){o.module_worker='ERR:'+e}\
o.module_worker_platform=String(navigator.platform);self.postMessage(o);};";

/// The page reads the renderer from the main thread and from three workers,
/// then publishes one JSON blob the gate can compare field by field.
fn fixture_page() -> String {
    format!(
        "<!doctype html><meta charset=\"utf-8\"><title>webgl worker scope</title>\
<pre id=\"out\">PENDING</pre><script>\
var R={{}};{READ_JS}\
R.window_renderer=rd(document.createElement('canvas').getContext('webgl'));\
R.window_platform=String(navigator.platform);\
try{{R.tostring=Function.prototype.toString.call(WebGLRenderingContext.prototype.getParameter);}}catch(e){{R.tostring='THROW'}}\
var B=[\"self.onmessage=function(){{var o={{}};\",\
\"function rd(g){{if(!g)return 'NOCTX';try{{g.getExtension('WEBGL_debug_renderer_info');return String(g.getParameter(37446));}}catch(e){{return 'ERR:'+e}}}}\",\
\"try{{o.blob_worker=rd(new OffscreenCanvas(1,1).getContext('webgl'));}}catch(e){{o.blob_worker='ERR:'+e}}\",\
\"o.blob_worker_platform=String(navigator.platform);self.postMessage(o);}};\"].join('\\n');\
function spawn(label,make){{return new Promise(function(res){{\
var done=false;function fin(x){{if(done)return;done=true;for(var k in x)R[k]=x[k];res();}}\
var w;try{{w=make();}}catch(e){{var a={{}};a[label]='SPAWN_ERR:'+e;return fin(a);}}\
w.onmessage=function(ev){{fin(ev.data);try{{w.terminate();}}catch(_){{}}}};\
w.onerror=function(e){{var b={{}};b[label]='WORKER_ERR:'+(e&&(e.message||e.type));fin(b);}};\
try{{w.postMessage(0);}}catch(e){{var c={{}};c[label]='POST_ERR:'+e;fin(c);}}\
setTimeout(function(){{var d={{}};d[label]='TIMEOUT';fin(d);}},8000);}});}}\
Promise.all([\
spawn('blob_worker',function(){{return new Worker(URL.createObjectURL(new Blob([B],{{type:'text/javascript'}})));}}),\
spawn('classic_worker',function(){{return new Worker('w.js');}}),\
spawn('module_worker',function(){{return new Worker('wm.mjs',{{type:'module'}});}})\
]).then(function(){{document.getElementById('out').textContent=JSON.stringify(R);}});\
</script>"
    )
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
    let (ctype, body) = match path.as_str() {
        "/webgl.html" => ("text/html", fixture_page()),
        "/w.js" => ("text/javascript", CLASSIC_WORKER.to_string()),
        "/wm.mjs" => ("text/javascript", MODULE_WORKER.to_string()),
        _ => {
            let _ = stream.write_all(
                b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
            return;
        }
    };
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
}

fn scratch() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("bac-webgl-worker-")
        .tempdir()
        .expect("create scratch dir")
}

fn set(cfg: &Path, key: &str, value: &str) {
    let out = common::isolated_cmd(&binary().expect("binary"))
        // `HOME` is what isolates config on macOS: `directories` resolves to
        // ~/Library/Application Support and never reads `XDG_CONFIG_HOME`.
        .env("HOME", cfg)
        .env("XDG_CONFIG_HOME", cfg)
        .args(["-q", "--json", "config", "set", key, value])
        .output()
        .expect("spawn config set");
    assert!(out.status.success(), "config set {key}={value}");
}

/// One launch, returning the page's own JSON report.
fn launch(dir: &Path, script: &Path) -> serde_json::Value {
    let out = common::isolated_cmd(&binary().expect("binary"))
        .env("HOME", dir)
        .env("XDG_CONFIG_HOME", dir)
        .args([
            "-q",
            "--json",
            "--timeout",
            "120",
            // An explicit foreign profile, so the mask has a value to carry
            // that cannot be confused with the host's own answer.
            "--stealth-profile",
            "chrome-linux",
            "run",
            "--script",
            &script.to_string_lossy(),
        ])
        .output()
        .expect("spawn run --script");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout was not JSON ({e}).\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    });
    assert_eq!(v["ok"], serde_json::json!(true), "fixture run failed: {v}");
    let raw = v["data"]["steps"]
        .as_array()
        .expect("steps array")
        .iter()
        .find_map(|s| s.pointer("/data/value").and_then(serde_json::Value::as_str))
        .unwrap_or_default()
        .to_string();
    serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("page report was not JSON ({e}): {raw} / {}", v["data"]))
}

#[test]
fn every_worker_scope_reports_the_same_masked_renderer() {
    if cannot_run() {
        return;
    }
    let scratch_dir = scratch();
    let dir = scratch_dir.path().to_path_buf();
    set(&dir, "http_ssrf_mode", "allow_loopback");
    set(&dir, "robots_loopback_exempt", "true");

    let port = start_fixture_server();
    let script = dir.join("steps.jsonl");
    std::fs::write(
        &script,
        format!(
            "{{\"cmd\":\"goto\",\"url\":\"http://127.0.0.1:{port}/webgl.html\",\"navigation_timeout_ms\":45000}}\n\
             {{\"cmd\":\"wait\",\"text\":\"blob_worker\",\"wait_timeout_ms\":20000}}\n\
             {{\"cmd\":\"eval\",\"expression\":\"document.getElementById('out').textContent\",\"typed\":true}}\n"
        ),
    )
    .expect("write script");

    for run in 0..LAUNCHES {
        let r = launch(&dir, &script);
        let window = r["window_renderer"].as_str().unwrap_or("MISSING");
        assert!(
            window.starts_with("ANGLE ("),
            "run {run}: the window itself lost the mask, so the gate cannot \
             judge the workers: {r}"
        );
        // The whole point: a scope that answers differently is the leak.
        for path in ["blob_worker", "classic_worker", "module_worker"] {
            assert_eq!(
                r[path].as_str().unwrap_or("MISSING"),
                window,
                "run {run}: `{path}` disagrees with the window. A worker has \
                 its own global scope; the mask has to be carried into it. \
                 Full report: {r}"
            );
        }
        // Vendor, renderer, cores and platform are cross-checked together, so
        // a worker answering the host platform is the same class of tell.
        let platform = r["window_platform"].as_str().unwrap_or("MISSING");
        for path in [
            "blob_worker_platform",
            "classic_worker_platform",
            "module_worker_platform",
        ] {
            assert_eq!(
                r[path].as_str().unwrap_or("MISSING"),
                platform,
                "run {run}: `{path}` contradicts the window's platform: {r}"
            );
        }
        // An own `toString` property does NOT close this: the strong check
        // reaches the prototype method and never consults the own property.
        let ts = r["tostring"].as_str().unwrap_or("MISSING");
        assert!(
            ts.contains("[native code]"),
            "run {run}: Function.prototype.toString.call handed back the \
             patch's own source: {ts}"
        );
    }
}
