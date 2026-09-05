// SPDX-License-Identifier: MIT OR Apache-2.0
//! What the disguise actually covers, stated in the envelope.
//!
//! # Why a product declares its own gaps
//!
//! Anti-detection is a stack of independent layers, and this product ships
//! some of them and not others. An agent that assumes full coverage will read
//! a block as a transient failure and retry, which escalates the block. An
//! agent told which layers are live can pick a different route on the first
//! failure instead of the fifth.
//!
//! The alternative — saying nothing — is worse than saying "no". Silence reads
//! as a guarantee, and a guarantee the transport cannot keep is the defect
//! this module exists to prevent.

use serde_json::{json, Map, Value};

use super::types::ScrapeEngine;

/// Add the disguise-coverage fields to a scrape envelope.
///
/// Every field is emitted unconditionally, including the `false` ones. A field
/// that appears only when true cannot be distinguished by a consumer from a
/// field that was never implemented, and `--filter tls_impersonation=false`
/// would silently match nothing.
///
/// # Why the engine is a parameter
///
/// The disclosure describes a transport, and this product has two. Writing one
/// report for `reqwest` and emitting it for both understated the browser
/// engine: a real Chrome owns its own JA3 and emits headers in its own order,
/// so `tls_impersonation: false` was factually wrong there. Understating is
/// the same class of defect as overstating — both make the caller route on a
/// property the transport does not have.
pub(super) fn insert_disguise_disclosure(map: &mut Map<String, Value>, engine: ScrapeEngine) {
    map.insert(
        "stealth".into(),
        json!(crate::browser_policy::stealth_enabled()),
    );
    map.insert(
        "fingerprint_stable_across_processes".into(),
        json!(crate::xdg::resolve_stealth_seed().is_some()),
    );
    // Cookies live in the client for the length of this process and are gone at
    // DIE. Stated rather than omitted: a caller that assumes a persistent jar
    // will read a fresh challenge as a transient failure and retry into a
    // harder block. `storage export` is the supported way to carry a session.
    map.insert("cookie_jar_persistent".into(), json!(false));
    // A profile that claims Windows on a Linux host is a mismatch the
    // transport betrays anyway: TLS and HTTP/2 carry the real stack, and a
    // bot check that compares them against the claimed platform sees the
    // contradiction the User-Agent tried to hide. Reported rather than
    // blocked, because the caller may want the foreign identity for a reason
    // this product cannot see — but reported, because a caller who picked
    // `chrome-win` by accident deserves to learn it here rather than from a
    // block it cannot explain.
    map.insert(
        "profile_contradicts_host".into(),
        json!(crate::native::stealth::profile_contradicts_host()),
    );

    match engine {
        ScrapeEngine::Http => {
            map.insert("http_version".into(), json!(negotiated_http_version()));
            map.insert("http2_profile".into(), json!(http2_profile()));
            // Layer 1. Impersonating a JA4 fingerprint needs BoringSSL, which
            // is C, and this product is pure Rust by policy. Not a gap to be
            // closed later: a stated boundary.
            map.insert("tls_impersonation".into(), json!(false));
            // WHICH stack produced that fingerprint, not just whether it was
            // disguised. `tls_impersonation: false` says the JA3 was not forged
            // and leaves open what it actually is, so a caller correlating a
            // block against a fingerprint database had to guess the library.
            map.insert("transport_fingerprint".into(), json!("rustls"));
            // Layer 4, partial. The VALUES of Chrome's headers are sent; the
            // ORDER is not, because `reqwest` writes headers by iterating a
            // `HeaderMap` whose iteration order comes from its hash state
            // rather than from insertion.
            map.insert("header_order_controlled".into(), json!(false));
        }
        ScrapeEngine::Browser => {
            // Chrome negotiates its own ALPN and its own SETTINGS frame. There
            // is no XDG knob in that path, so reporting this crate's HTTP/2
            // profile here would describe a client that did not make the
            // request.
            map.insert("http_version".into(), json!("h2"));
            map.insert("http2_profile".into(), json!("chrome-native"));
            // Not impersonation in the `wreq` sense — it is the real thing.
            // The JA3/JA4 on the wire is Chrome's because the peer IS Chrome.
            map.insert("tls_impersonation".into(), json!(true));
            // The twin of the `http` line, emitted here for the same reason the
            // module doc gives for emitting the `false` fields: a key present in
            // half the envelopes cannot be filtered on. Chrome's TLS is
            // BoringSSL, so this names the real stack rather than repeating the
            // engine name the envelope already carries.
            map.insert("transport_fingerprint".into(), json!("chrome-boringssl"));
            map.insert("header_order_controlled".into(), json!(true));
        }
    }
}

/// The HTTP version the shared client is able to negotiate.
///
/// Reported rather than inferred from the response because the interesting
/// property is what was OFFERED in ALPN. Chrome always lists `h2` there, so a
/// client that can only offer `http/1.1` is identifiable during the TLS
/// handshake, before any response exists to inspect.
fn negotiated_http_version() -> &'static str {
    negotiated_http_version_with(crate::xdg::resolve_http2_enabled())
}

/// The pure core of [`negotiated_http_version`], taking the knob as an argument.
///
/// # Why the facade is not enough
///
/// The facade reads XDG, and the policy snapshot behind it is a `OnceLock`:
/// the first test in a binary to touch it freezes the value for every test
/// after it. An assertion written against the facade therefore measures
/// whichever neighbour ran first, which is why this axis had ZERO coverage
/// while six tests in this file reported green — every one of them asserted
/// literals that are identical in both branches.
fn negotiated_http_version_with(http2_enabled: bool) -> &'static str {
    if http2_enabled {
        "h2"
    } else {
        "http/1.1"
    }
}

/// Which HTTP/2 `SETTINGS` profile the client advertises.
///
/// `chrome` means the frame carries Chrome's values. `partial` is honest about
/// the rest: `reqwest` exposes no way to set `HEADER_TABLE_SIZE` or
/// `MAX_CONCURRENT_STREAMS`, nor to order the four pseudo-headers, so the
/// frame resembles Chrome without matching it byte for byte.
fn http2_profile() -> &'static str {
    http2_profile_with(
        crate::xdg::resolve_http2_enabled(),
        crate::xdg::resolve_http2_initial_stream_window_size(),
        crate::xdg::resolve_http2_initial_connection_window_size(),
    )
}

/// The pure core of [`http2_profile`], taking the three knobs as arguments.
///
/// Same reason as [`negotiated_http_version_with`]: the three states this
/// function distinguishes — `disabled`, `chrome-partial` and `custom` — are
/// unreachable from a test that has to go through the frozen XDG snapshot.
fn http2_profile_with(
    http2_enabled: bool,
    stream_window: u32,
    connection_window: u32,
) -> &'static str {
    if !http2_enabled {
        return "disabled";
    }
    let stock = stream_window == crate::constants::HTTP2_INITIAL_STREAM_WINDOW_SIZE
        && connection_window == crate::constants::HTTP2_INITIAL_CONNECTION_WINDOW_SIZE;
    if stock {
        "chrome-partial"
    } else {
        "custom"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The HTTP version reported follows the knob, in BOTH directions.
    ///
    /// Asserting only the `h2` side would pass against a function hardcoded to
    /// return `h2`, which is the shape the six older tests in this file have.
    #[test]
    fn the_reported_http_version_follows_the_knob() {
        assert_eq!(negotiated_http_version_with(true), "h2");
        assert_eq!(negotiated_http_version_with(false), "http/1.1");
    }

    /// All THREE profile states are reachable and distinct.
    #[test]
    fn the_http2_profile_separates_disabled_stock_and_custom() {
        let stock_stream = crate::constants::HTTP2_INITIAL_STREAM_WINDOW_SIZE;
        let stock_conn = crate::constants::HTTP2_INITIAL_CONNECTION_WINDOW_SIZE;

        // Disabled wins over the windows: a client that offers no h2 has no
        // SETTINGS frame to describe, so reporting a profile there would name a
        // frame that is never sent.
        assert_eq!(
            http2_profile_with(false, stock_stream, stock_conn),
            "disabled"
        );
        assert_eq!(
            http2_profile_with(true, stock_stream, stock_conn),
            "chrome-partial"
        );
        // EITHER window off the stock value is enough: the frame no longer
        // matches Chrome's, and `chrome-partial` would overstate it.
        assert_eq!(
            http2_profile_with(true, stock_stream + 1, stock_conn),
            "custom"
        );
        assert_eq!(
            http2_profile_with(true, stock_stream, stock_conn + 1),
            "custom"
        );
    }

    #[test]
    fn every_field_is_present_even_when_false() {
        for engine in [ScrapeEngine::Http, ScrapeEngine::Browser] {
            let mut map = Map::new();
            insert_disguise_disclosure(&mut map, engine);
            for key in [
                "stealth",
                "fingerprint_stable_across_processes",
                "cookie_jar_persistent",
                "profile_contradicts_host",
                "http_version",
                "http2_profile",
                "tls_impersonation",
                "header_order_controlled",
            ] {
                assert!(
                    map.contains_key(key),
                    "{engine:?}: missing disclosure field: {key}"
                );
            }
        }
    }

    #[test]
    fn transport_limits_are_declared_false_not_omitted() {
        // The regression this guards: emitting the field only when true makes
        // a stated limit indistinguishable from an unimplemented one.
        let mut map = Map::new();
        insert_disguise_disclosure(&mut map, ScrapeEngine::Http);
        assert_eq!(map.get("tls_impersonation"), Some(&json!(false)));
        assert_eq!(map.get("header_order_controlled"), Some(&json!(false)));
    }

    #[test]
    fn the_browser_engine_never_understates_a_real_chrome() {
        // The regression this guards: one report written for `reqwest` and
        // emitted for both engines, so a scrape carried by a real Chrome
        // claimed it did not impersonate TLS and did not control header order.
        let mut map = Map::new();
        insert_disguise_disclosure(&mut map, ScrapeEngine::Browser);
        assert_eq!(map.get("tls_impersonation"), Some(&json!(true)));
        assert_eq!(map.get("header_order_controlled"), Some(&json!(true)));
        assert_eq!(map.get("http2_profile"), Some(&json!("chrome-native")));
    }

    #[test]
    fn the_two_engines_never_report_the_same_transport() {
        let (mut http, mut browser) = (Map::new(), Map::new());
        insert_disguise_disclosure(&mut http, ScrapeEngine::Http);
        insert_disguise_disclosure(&mut browser, ScrapeEngine::Browser);
        assert_ne!(
            http.get("tls_impersonation"),
            browser.get("tls_impersonation"),
            "a shared answer for both engines is the defect this split fixed"
        );
    }

    #[test]
    fn the_platform_mismatch_is_stated_on_both_engines() {
        // The regression this guards: `profile_contradicts_host` existed with
        // no caller anywhere in the crate, so a mismatch the transport would
        // betray was computed and then dropped on the floor.
        for engine in [ScrapeEngine::Http, ScrapeEngine::Browser] {
            let mut map = Map::new();
            insert_disguise_disclosure(&mut map, engine);
            assert!(
                map.get("profile_contradicts_host")
                    .is_some_and(Value::is_boolean),
                "{engine:?}: the platform mismatch must be a boolean, never absent"
            );
        }
    }

    #[test]
    fn the_cookie_jar_limit_is_stated_on_both_engines() {
        for engine in [ScrapeEngine::Http, ScrapeEngine::Browser] {
            let mut map = Map::new();
            insert_disguise_disclosure(&mut map, engine);
            assert_eq!(map.get("cookie_jar_persistent"), Some(&json!(false)));
        }
    }
}
