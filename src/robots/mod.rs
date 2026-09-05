// SPDX-License-Identifier: MIT OR Apache-2.0
//! robots.txt honor for one-shot invocations (no cross-process cache).
//!
//! Default policy is honor. Override requires BOTH `--ignore-robots` and
//! `--i-accept-robots-risk`. Uses `robotstxt::DefaultMatcher` (Google port).
//!
//! # Workload
//!
//! **I/O-bound** (network GET of `/robots.txt`). Reuses process-wide
//! [`shared_http_client`](crate::robots::shared_http_client) so keep-alive / TLS session cache survive batch scrapes.
//! Not CPU-bound → no rayon. Not a long-lived daemon → no connection pool beyond
//! reqwest's internal pool for this one-shot process.

mod fetch;
mod policy;
mod politeness;

#[cfg(test)]
mod tests;

pub use fetch::enforce_robots;
pub use policy::*;
pub use politeness::{
    parse_crawl_delay_secs, remember_crawl_delay, set_min_delay_override_ms, wait_origin,
};

use std::sync::OnceLock;
use std::time::Duration;

use crate::constants::HTTP_USER_AGENT;
use crate::error::{CliError, ErrorKind};
use crate::xdg::policy::{key, policy_usize};

/// The user-agent token `robots.txt` rules are matched against.
///
/// # Why this is not simply `HTTP_USER_AGENT`
///
/// Stealth sends a browser User-Agent on the wire while the product's honest
/// identity string stays what it is. `robots.txt` groups rules by user-agent,
/// so matching against a token the server never saw reads a different group
/// than the one that governs the request. A site with a permissive `*` group
/// and a restrictive `Chrome` group would be evaluated against the wrong one,
/// in either direction.
///
/// Resolution order: XDG `robots_user_agent`, then the stealth identity when
/// stealth is active, then the honest product string.
#[must_use]
pub fn robots_user_agent() -> String {
    if let Some(token) = crate::xdg::resolve_robots_user_agent() {
        return token;
    }
    if crate::browser_policy::stealth_enabled() {
        if let Some(ua) = crate::native::stealth::wire_user_agent() {
            return ua;
        }
    }
    HTTP_USER_AGENT.to_string()
}

/// Process-wide async HTTP client (rules: create `reqwest::Client` once, reuse).
///
/// Total + connect timeouts from XDG (`http_timeout_secs` /
/// `http_connect_timeout_secs`). System `HTTP_PROXY*` env is **never** honored;
/// egress routing comes from `--proxy` / XDG `proxy_url` alone.
///
/// # HTTP/2 is a fingerprint, not a performance knob
///
/// The `SETTINGS` frame and the opening `WINDOW_UPDATE` are sent in the clear
/// at connection open, before any request exists. A bot check reads them and
/// compares against the browser the User-Agent claims to be. Library defaults
/// are three orders of magnitude away from Chrome's, so leaving them alone
/// identifies the client without a single header being inspected.
///
/// Uses `get_or_init` (stable). `get_or_try_init` is still unstable (`once_cell_try`);
/// on first failure we surface the error without poisoning the lock.
///
/// # Errors
///
/// [`ErrorKind::Usage`] when `--proxy` / XDG `proxy_url` does not parse as a
/// `reqwest::Proxy`, carrying the `proxy_url_invalid` suggestion.
/// [`ErrorKind::Software`] when `ClientBuilder::build` fails — a TLS backend
/// that cannot be initialised, or a rejected combination of the HTTP/2 and
/// header settings applied above. The failure is surfaced without poisoning the
/// `OnceLock`, so a later call may still succeed.
pub fn shared_http_client() -> Result<&'static reqwest::Client, CliError> {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    if let Some(c) = CLIENT.get() {
        return Ok(c);
    }
    let total = crate::xdg::resolve_http_timeout_secs();
    let connect = crate::xdg::resolve_http_connect_timeout_secs();
    let mut builder = reqwest::Client::builder()
        .user_agent(HTTP_USER_AGENT)
        .timeout(Duration::from_secs(total))
        .connect_timeout(Duration::from_secs(connect))
        .redirect(reqwest::redirect::Policy::limited(policy_usize(
            key::HTTP_REDIRECT_MAX,
        )))
        .pool_max_idle_per_host(policy_usize(key::HTTP_POOL_MAX_IDLE_PER_HOST))
        // Explicit TCP_NODELAY: reduce small-response latency on robots/scrape
        // (rules_rust_latencia_reduzir — socket options on owned HTTP path).
        // CDP WebSocket is owned by chromiumoxide; not configurable here.
        .tcp_nodelay(true)
        // Without a jar, `--warmup` was heating a connection pool and nothing
        // else: its own rustdoc claimed a cookie side effect the client could
        // not have, so an edge that hands out a session cookie on the root
        // still saw the deep request arrive with no session at all.
        //
        // The jar lives and dies with this process. That is a real limit, and
        // the scrape envelope states it as `cookie_jar_persistent: false`
        // rather than leaving the caller to infer a guarantee from silence.
        .cookie_store(true);

    builder = apply_http2_profile(builder);
    builder = apply_stealth_headers(builder);
    builder = apply_egress_proxy(builder)?;

    let built = builder
        .build()
        .map_err(|e| CliError::new(ErrorKind::Software, format!("http client: {e}")))?;
    Ok(CLIENT.get_or_init(|| built))
}

/// Advertise Chrome's HTTP/2 `SETTINGS` instead of the library defaults.
///
/// Values come from XDG with named constants as the floor, so no magic number
/// lives in this function.
fn apply_http2_profile(builder: reqwest::ClientBuilder) -> reqwest::ClientBuilder {
    if !crate::xdg::resolve_http2_enabled() {
        // Opt-out is honoured rather than silently overridden, but it is a
        // detection regression and the envelope says so via `http2_profile`.
        return builder.http1_only();
    }
    builder
        .http2_initial_stream_window_size(crate::xdg::resolve_http2_initial_stream_window_size())
        .http2_initial_connection_window_size(
            crate::xdg::resolve_http2_initial_connection_window_size(),
        )
        .http2_max_header_list_size(crate::xdg::resolve_http2_max_header_list_size())
        .http2_max_frame_size(crate::xdg::resolve_http2_max_frame_size())
        .http2_adaptive_window(crate::xdg::resolve_http2_adaptive_window())
}

/// Send Chrome's request headers when stealth is on.
///
/// The User-Agent is set through the same map rather than through
/// `ClientBuilder::user_agent` so that one identity governs every header: a
/// `user-agent` from one source and `sec-ch-ua` from another is a mismatch a
/// bot check reads directly.
///
/// Order is NOT delivered here — see [`crate::native::stealth::wire_headers`]
/// for why, and `header_order_controlled` in the scrape envelope for how the
/// caller is told.
fn apply_stealth_headers(builder: reqwest::ClientBuilder) -> reqwest::ClientBuilder {
    let Some(pairs) = crate::native::stealth::wire_headers() else {
        return builder;
    };
    let mut map = reqwest::header::HeaderMap::new();
    for (name, value) in pairs {
        // A header that will not parse is skipped rather than fatal: losing one
        // of twelve degrades the disguise, while failing the build would take
        // down every scrape on this host.
        if let (Ok(n), Ok(v)) = (
            reqwest::header::HeaderName::from_bytes(name.as_bytes()),
            reqwest::header::HeaderValue::from_str(&value),
        ) {
            map.insert(n, v);
        } else {
            tracing::warn!(
                target: "browser_automation_cli::stealth",
                header = %name,
                "stealth header rejected by the HTTP client; disguise is incomplete"
            );
        }
    }
    builder.default_headers(map)
}

/// Route egress through `--proxy` / XDG `proxy_url`, or through nothing at all.
///
/// # Why the order of these two calls is load-bearing
///
/// Measured against the reqwest documentation rather than assumed:
/// `ClientBuilder::no_proxy` **clears** the proxy list and disables system
/// proxy discovery, while `ClientBuilder::proxy` **appends** to that list and
/// also disables system discovery on its own. Calling `no_proxy()` after
/// `proxy()` would therefore erase the operator's proxy and route the request
/// directly — the exact silent-egress failure this function exists to prevent.
/// So the two are mutually exclusive branches, never a sequence.
fn apply_egress_proxy(builder: reqwest::ClientBuilder) -> Result<reqwest::ClientBuilder, CliError> {
    let Some(url) = crate::browser_policy::proxy() else {
        // Product law: do not inherit HTTP_PROXY / HTTPS_PROXY / ALL_PROXY env.
        return Ok(builder.no_proxy());
    };

    let mut proxy = reqwest::Proxy::all(url).map_err(|e| {
        CliError::with_suggestion(
            ErrorKind::Usage,
            format!("invalid proxy url '{url}': {e}"),
            crate::i18n::suggestion_key("proxy_url_invalid", None),
        )
    })?;

    // Chrome's bypass-list syntax and reqwest's NoProxy syntax are both
    // comma-separated host patterns, so the operator writes the list once —
    // and loopback is added to it whether or not they did.
    //
    // # Why this client needs the loopback exemption too
    //
    // Bypassing loopback in Chrome's OWN argv is not enough, and the first
    // attempt at this fix proved it. Measured with `--proxy http://127.0.0.1:1`
    // AFTER the Chrome-side bypass was in place: the launch still failed with
    // "Timed out after 20000ms waiting for Chrome CDP endpoint", because the
    // failing request was OURS. CDP discovery — `/json/version`, `/json/list`,
    // the WebSocket upgrade — goes out through this shared client, and this
    // client was routing 127.0.0.1 through the operator's egress proxy.
    //
    // So the control channel had two ways to be swallowed and only one was
    // closed. This closes the other.
    let bypass = crate::native::cdp::chrome::merge_proxy_bypass(
        crate::browser_policy::proxy_bypass(),
        crate::xdg::resolve_cdp_proxy_bypass_loopback(),
    );
    if let Some(list) = bypass {
        if let Some(no_proxy) = reqwest::NoProxy::from_string(&list) {
            proxy = proxy.no_proxy(Some(no_proxy));
        }
    }

    if let Some((user, pass)) = crate::xdg::resolve_proxy_credentials() {
        // Credentials come from XDG only. In argv they would be readable by any
        // process on the host through the process table.
        proxy = proxy.basic_auth(&user, &pass);
    }

    Ok(builder.proxy(proxy))
}
