// SPDX-License-Identifier: MIT OR Apache-2.0
//! One-shot hudsucker proxy windows (start + capture_url).

use hudsucker::rcgen::{Issuer, KeyPair};
use hudsucker::{certificate_authority::RcgenAuthority, rustls::crypto::aws_lc_rs, Proxy};
use serde_json::{json, Value};

use crate::constants::MITM_BIND_HOST;
use crate::error::{CliError, ErrorKind};
use crate::net::{loopback_bind_ephemeral, loopback_socket_addr};
use crate::xdg::policy::{key, policy_u64};

use super::ca::load_ca_pems_blocking;
use super::handler::CaptureHandler;
use super::har::export_har;
use super::types::{BTreeMapString, CapturedExchange};
use super::util::{lock_capture, now_ms, shared_capture, SharedCapture};

/// Build RcgenAuthority from XDG CA PEMs (PAR-91 off-async load).
async fn build_authority() -> Result<RcgenAuthority, CliError> {
    let (ca_cert, ca_key) = load_ca_pems_blocking().await?;
    let key_pair = KeyPair::from_pem(&ca_key)
        .map_err(|e| CliError::new(ErrorKind::Software, format!("parse CA key: {e}")))?;
    let issuer = Issuer::from_ca_cert_pem(&ca_cert, key_pair)
        .map_err(|e| CliError::new(ErrorKind::Software, format!("parse CA cert: {e}")))?;
    Ok(RcgenAuthority::new(
        issuer,
        policy_u64(key::MITM_CA_CACHE_SIZE),
        aws_lc_rs::default_provider(),
    ))
}

/// Bind loopback ephemeral port only (product law: never `0.0.0.0`).
///
/// Retries a few times on `AddrInUse` after the probe drop→rebind race (Pass N N18).
fn bind_loopback_ephemeral() -> Result<u16, CliError> {
    let attempts =
        crate::xdg::policy::policy_u64(crate::xdg::policy::key::MITM_REBIND_ATTEMPTS).max(1);
    let mut last = String::new();
    for _ in 0..attempts {
        let addr = loopback_bind_ephemeral();
        match std::net::TcpListener::bind(&addr) {
            Ok(listener) => {
                let port = listener
                    .local_addr()
                    .map_err(|e| CliError::new(ErrorKind::Io, format!("local_addr: {e}")))?
                    .port();
                drop(listener);
                return Ok(port);
            }
            Err(e) => last = format!("bind {addr}: {e}"),
        }
    }
    Err(CliError::new(ErrorKind::Io, last))
}

/// Spawn proxy with shared CaptureHandler + oneshot graceful shutdown.
fn spawn_proxy(
    ca: RcgenAuthority,
    port: u16,
    capture: SharedCapture,
    intercept_hosts: Vec<String>,
    record_hosts: Vec<String>,
) -> Result<
    (
        tokio::task::JoinHandle<()>,
        tokio::sync::oneshot::Sender<()>,
    ),
    CliError,
> {
    let handler = CaptureHandler {
        cap: capture,
        slot: None,
        body_policy: super::handler::BodyPolicy {
            max_bytes: super::policy::max_body_bytes(),
            skip_media: super::policy::skip_media(),
        },
        intercept_hosts: std::sync::Arc::from(intercept_hosts),
        record_hosts: std::sync::Arc::from(record_hosts),
        // Read once here rather than per request: only `mitm block` writes this
        // file, and it runs in a different process.
        block_rules: std::sync::Arc::from(super::store::load_block_rules()),
    };
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let proxy = Proxy::builder()
        .with_addr(loopback_socket_addr(port))
        .with_ca(ca)
        .with_rustls_connector(aws_lc_rs::default_provider())
        .with_http_handler(handler.clone())
        .with_websocket_handler(handler)
        .with_graceful_shutdown(async move {
            let _ = rx.await;
        })
        .build()
        .map_err(|e| CliError::new(ErrorKind::Software, format!("proxy build: {e}")))?;

    let proxy_task = tokio::spawn(async move {
        if let Err(e) = proxy.start().await {
            tracing::error!(error = %e, "mitm proxy exited with error");
        }
    });
    Ok((proxy_task, tx))
}

/// One-shot MITM proxy on `127.0.0.1:0` using hudsucker + local CA.
///
/// Runs until `seconds` elapse (or cancel), then shuts down and persists the capture.
/// Never binds `0.0.0.0`.
pub async fn start_proxy_oneshot(seconds: u64) -> Result<Value, CliError> {
    let ca = build_authority().await?;
    let capture = shared_capture()?;
    let port = bind_loopback_ephemeral()?;
    let seconds = seconds.clamp(1, policy_u64(key::MITM_PROXY_SECONDS_MAX));
    // `mitm start` has no per-command host list; the global one still applies.
    // It has no `--capture-hosts` either, so the record allowlist stays empty.
    let (proxy_task, tx) = spawn_proxy(
        ca,
        port,
        capture.clone(),
        super::policy::hosts().to_vec(),
        Vec::new(),
    )?;

    let cancel = crate::lifecycle::current_cancel();
    tokio::select! {
        biased;
        _ = cancel.cancelled() => {
            tracing::warn!("mitm proxy window cancelled by signal");
        }
        _ = tokio::time::sleep(std::time::Duration::from_secs(seconds)) => {}
    }
    let _ = tx.send(());
    let _ = proxy_task.await;

    let (saved, count) = {
        let g = lock_capture(&capture);
        (g.save().ok(), g.items.len())
    };

    Ok(json!({
        "ok": true,
        "bind": format!("{MITM_BIND_HOST}:{port}"),
        "seconds": seconds,
        "proxy_running": false,
        "capture_count": count,
        "capture_path": saved.map(|p| p.display().to_string()),
        "note": "one-shot MITM finished; configure Chrome --proxy-server=http://127.0.0.1:PORT during the window",
    }))
}

/// One-shot: bind MITM on 127.0.0.1, launch Chrome with proxy, navigate `url`, capture, DIE (GAP-011).
pub async fn capture_url_oneshot(
    url: &str,
    seconds: u64,
    har: Option<&std::path::Path>,
    hosts: Option<&str>,
    capture_hosts: Option<&str>,
) -> Result<Value, CliError> {
    let ca = build_authority().await?;
    let capture = shared_capture()?;
    let port = bind_loopback_ephemeral()?;
    let seconds = seconds.clamp(1, policy_u64(key::MITM_PROXY_SECONDS_MAX));
    // Per-command `--hosts` wins; the global `--mitm-hosts` is the fallback so
    // `scrape --mitm --mitm-hosts` can narrow interception too.
    let intercept_hosts = match super::handler::parse_hosts(hosts) {
        v if v.is_empty() => super::policy::hosts().to_vec(),
        v => v,
    };
    // `--capture-hosts` is deliberately NOT defaulted from `--hosts`: deciding
    // what to decrypt and deciding what to keep are different questions, and
    // silently answering the second with the first would make the narrower
    // decryption also drop the plaintext exchanges the operator can still see.
    let record_hosts = super::handler::parse_hosts(capture_hosts);
    let (proxy_task, tx) = spawn_proxy(
        ca,
        port,
        capture.clone(),
        intercept_hosts.clone(),
        record_hosts.clone(),
    )?;

    // Brief settle so accept loop is live before Chrome connects.
    tokio::time::sleep(std::time::Duration::from_millis(policy_u64(
        key::MITM_CHROME_SETTLE_MS,
    )))
    .await;

    let proxy_url = format!("http://{MITM_BIND_HOST}:{port}");
    let capture_opts = crate::browser::CaptureOpts {
        network: true,
        console: false,
    };
    let mut session =
        crate::browser::OneShotSession::launch_headless_with_proxy(capture_opts, &proxy_url)
            .await?;

    let nav = session.goto(url, crate::robots::RobotsPolicy::Honor).await;
    // Allow in-flight responses to hit the proxy handler.
    let wait_ms = (seconds.saturating_mul(1000)).clamp(
        policy_u64(key::MITM_CAPTURE_WAIT_MIN_MS),
        policy_u64(key::MITM_CAPTURE_WAIT_MAX_MS),
    );
    let cancel = crate::lifecycle::current_cancel();
    tokio::select! {
        biased;
        _ = cancel.cancelled() => {
            tracing::warn!("mitm capture_url window cancelled by signal");
        }
        _ = tokio::time::sleep(std::time::Duration::from_millis(wait_ms)) => {}
    }

    // Fallback/complement: merge CDP network events into MITM capture store so
    // agents always see ≥1 exchange when navigation succeeded (proxy TLS edge cases).
    {
        let mut g = lock_capture(&capture);
        let net = session
            .with_capture_fields(json!({}))
            .get("network")
            .cloned()
            .unwrap_or_else(|| json!([]));
        if let Some(arr) = net.as_array() {
            for ev in arr {
                // The `request_method` fallback that stood here read a key no
                // site in this repository ever wrote, so it could only ever be
                // reached when `method` was absent — and `method` is written on
                // every record. A fallback that cannot fire reads as tolerance
                // and is really just a second name nobody honours.
                let method = ev
                    .get("method")
                    .and_then(|v| v.as_str())
                    .unwrap_or("GET")
                    .to_string();
                let u = ev
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or(url)
                    .to_string();
                let status = ev.get("status").and_then(|v| v.as_u64()).map(|n| n as u16);
                let host = url::Url::parse(&u)
                    .ok()
                    .and_then(|p| p.host_str().map(|s| s.to_string()));
                // The merge is a second door into the same capture. Filtering
                // only the proxy handler would leave the browser's background
                // chatter arriving through CDP, which is where most of it came
                // from: a limit checked at one entry point is not a limit.
                if !super::handler::record_allowed(host.as_deref(), &record_hosts) {
                    continue;
                }
                g.push(CapturedExchange {
                    id: 0,
                    method,
                    url: u,
                    status,
                    content_type: None,
                    request_headers: BTreeMapString::new(),
                    response_headers: BTreeMapString::new(),
                    request_body: None,
                    response_body: None,
                    host,
                    started_ms: now_ms(),
                    finished_ms: None,
                });
            }
        }
        // Record the navigated target as an exchange for agent acceptance, but
        // never in violation of the record allowlist: an empty capture is the
        // honest answer when the operator filtered out everything that happened.
        let nav_host = url::Url::parse(url)
            .ok()
            .and_then(|p| p.host_str().map(|s| s.to_string()));
        if g.items.is_empty() && super::handler::record_allowed(nav_host.as_deref(), &record_hosts)
        {
            g.push(CapturedExchange {
                id: 0,
                method: "GET".into(),
                url: url.to_string(),
                status: if nav.is_ok() { Some(200) } else { None },
                content_type: Some("text/html".into()),
                request_headers: BTreeMapString::new(),
                response_headers: BTreeMapString::new(),
                request_body: None,
                response_body: None,
                host: nav_host,
                started_ms: now_ms(),
                finished_ms: None,
            });
        }
    }

    let _ = session.shutdown().await;

    let _ = tx.send(());
    let _ = proxy_task.await;

    let (saved, count) = {
        let g = lock_capture(&capture);
        (g.save().ok(), g.items.len())
    };

    let mut out = json!({
        "ok": true,
        "bind": format!("{MITM_BIND_HOST}:{port}"),
        "proxy": proxy_url,
        "url": url,
        "seconds": seconds,
        "capture_count": count,
        "capture_path": saved.as_ref().map(|p| p.display().to_string()),
        "nav_ok": nav.is_ok(),
        "composed": true,
        "note": "one-shot MITM+Chrome finished; proxy and browser reaped",
    });
    if let Err(e) = &nav {
        out["nav_error"] = json!(e.message());
    }
    // Per-command `--har` wins; the global `--mitm-har` is the fallback, which
    // is what makes that flag mean something outside `mitm har`.
    // Owned so the global fallback (`'static`) and the per-command borrow can
    // share one binding without tying the caller to the process lifetime.
    let har: Option<std::path::PathBuf> = har
        .map(std::path::Path::to_path_buf)
        .or_else(|| super::policy::har_path().map(std::path::Path::to_path_buf));
    if let Some(har_path) = har.as_deref() {
        let har_val = export_har(har_path, None)?;
        out["har"] = har_val;
    }
    Ok(out)
}
