// SPDX-License-Identifier: MIT OR Apache-2.0
//! Where this process's traffic goes: proxy, bypass list, and the warm-up visit.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

static WARMUP: AtomicBool = AtomicBool::new(false);
static PROXY: OnceLock<Option<String>> = OnceLock::new();
static PROXY_BYPASS: OnceLock<Option<String>> = OnceLock::new();
static WARMUP_URL: OnceLock<Option<String>> = OnceLock::new();

/// Publish the `--warmup` request. Called once from CLI dispatch.
pub fn set_warmup(enabled: bool) {
    WARMUP.store(enabled, Ordering::Relaxed);
}

/// Whether to visit the origin root before the target URL.
#[must_use]
pub fn warmup_enabled() -> bool {
    WARMUP.load(Ordering::Relaxed)
}

/// Publish the `--warmup-url` override. Called once from CLI dispatch.
pub fn set_warmup_url(url: Option<String>) {
    let _ = WARMUP_URL.set(url);
}

/// The URL to warm instead of the target's origin root.
///
/// The default warm-up visits the root because that is where a browser lands.
/// Some edges hand out the session somewhere else — a login page, a locale
/// splash, a region redirector — and warming the root there buys a cookie the
/// target does not accept. This lets the caller name the real entry point
/// instead of giving up on the warm-up entirely.
#[must_use]
pub fn warmup_url() -> Option<&'static str> {
    WARMUP_URL.get().and_then(|v| v.as_deref())
}

/// Publish the resolved egress proxy. Called once from CLI dispatch.
///
/// Repeat calls are ignored, which keeps the value immutable for the rest of
/// the process rather than letting a later caller redirect egress silently.
pub fn set_proxy(url: Option<String>, bypass: Option<String>) {
    let _ = PROXY.set(url);
    let _ = PROXY_BYPASS.set(bypass);
}

/// The egress proxy URL for both Chrome and the HTTP engine.
#[must_use]
pub fn proxy() -> Option<&'static str> {
    PROXY.get().and_then(|v| v.as_deref())
}

/// Hosts that bypass the proxy, in Chrome's bypass-list syntax.
#[must_use]
pub fn proxy_bypass() -> Option<&'static str> {
    PROXY_BYPASS.get().and_then(|v| v.as_deref())
}
