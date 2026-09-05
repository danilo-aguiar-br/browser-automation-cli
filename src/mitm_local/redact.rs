// SPDX-License-Identifier: MIT OR Apache-2.0
//! Secret masking for captured traffic: headers, URLs, bodies and WS frames.
//!
//! # Why this module exists
//!
//! Redaction lived in one nine-line helper in `util.rs` that walked five header
//! names. Measured 2026-09-01: a request carrying `?api_key=…` in the query, an
//! `{"access_token": "…"}` body and a WebSocket frame holding a session token was
//! written to disk with all three in the clear, while the capture presented
//! itself as redacted. Masking `Authorization` and nothing else does not make a
//! capture safe to hand to an agent — it makes it LOOK safe, which is worse,
//! because the operator stops checking.
//!
//! Gathering all four surfaces here rather than extending the header helper is
//! the point: the defect was that redaction had no single place to be complete
//! in, so each new capture surface shipped without it and nobody noticed.
//!
//! # Why the key lists differ by surface
//!
//! `key` in a query string is almost always an API key. `key` in a JSON body is
//! as likely to be the left half of a `{"key": …, "value": …}` pair, and masking
//! it would destroy the shape of the payload — which is the only reason a
//! capture is worth reading. So the body list is the conservative one, and the
//! query list carries the generic names the URL surface makes unambiguous.
//!
//! The lists name keys, never patterns. Entropy heuristics were rejected: a
//! false positive here silently corrupts the artifact the operator is debugging
//! with, and they have no way to tell corruption from an empty field.

use serde_json::Value;

use crate::constants::MITM_REDACTED_PLACEHOLDER;

use super::types::BTreeMapString;

/// Header names whose value is masked. Canonical form (see [`canon`]).
const SECRET_HEADERS: &[&str] = &[
    "authorization",
    "cookie",
    "setcookie",
    "proxyauthorization",
    "xapikey",
];

/// Keys masked in a request/response body or a WebSocket frame.
///
/// Deliberately narrower than [`QUERY_EXTRA_KEYS`]: a body is structured data
/// the operator reads for shape, and over-masking it costs more than it buys.
const SECRET_BODY_KEYS: &[&str] = &[
    "password",
    "passwd",
    "token",
    "accesstoken",
    "refreshtoken",
    "idtoken",
    "apikey",
    "secret",
    "clientsecret",
    "privatekey",
    "sessionid",
    "authorization",
    "credentials",
];

/// Extra keys masked in a query string only.
///
/// `key`, `sig` and `auth` are too generic for a JSON body but unambiguous as
/// URL parameters, where they are how API keys and request signatures travel.
const QUERY_EXTRA_KEYS: &[&str] = &["key", "sig", "signature", "session", "auth"];

/// Fold a name to letters and digits, lowercased.
///
/// Makes `api_key`, `API-KEY` and `apiKey` one name, so the lists hold one entry
/// per SECRET rather than one per spelling. A list that has to enumerate
/// spellings ages badly and then misses the one spelling that mattered.
fn canon(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// True when this header name carries a credential.
fn is_secret_header(name: &str) -> bool {
    let c = canon(name);
    SECRET_HEADERS.contains(&c.as_str())
}

/// True when this body/frame key carries a credential.
fn is_secret_body_key(name: &str) -> bool {
    let c = canon(name);
    SECRET_BODY_KEYS.contains(&c.as_str())
}

/// True when this query parameter carries a credential.
fn is_secret_query_key(name: &str) -> bool {
    let c = canon(name);
    SECRET_BODY_KEYS.contains(&c.as_str()) || QUERY_EXTRA_KEYS.contains(&c.as_str())
}

/// Mask sensitive header values in place.
pub(super) fn redact_headers(h: &mut BTreeMapString) {
    for (k, v) in h.iter_mut() {
        if is_secret_header(k) {
            *v = MITM_REDACTED_PLACEHOLDER.into();
        }
    }
}

/// Rewrite `k=v&k=v` pairs, masking values whose key `secret` accepts.
///
/// `None` means nothing matched, which lets every caller leave the original
/// bytes untouched instead of round-tripping a string that did not change.
fn redact_pairs(text: &str, secret: fn(&str) -> bool) -> Option<String> {
    if !text.contains('=') {
        return None;
    }
    let out = text
        .split('&')
        .map(|pair| match pair.split_once('=') {
            Some((k, _)) if secret(k) => format!("{k}={MITM_REDACTED_PLACEHOLDER}"),
            _ => pair.to_string(),
        })
        .collect::<Vec<_>>()
        .join("&");
    (out != text).then_some(out)
}

/// Mask credentials carried in the query string of an absolute URL.
///
/// The URL is split by hand rather than parsed and re-serialised: a parser
/// normalises percent-encoding, default ports and trailing slashes, and an
/// operator comparing the capture against their own logs would see differences
/// that redaction did not cause. Only the bytes between `?` and `#` move.
pub(super) fn redact_url(url: &mut String) {
    let Some(qpos) = url.find('?') else {
        return;
    };
    let rest = &url[qpos + 1..];
    let (query, fragment) = match rest.find('#') {
        Some(h) => (&rest[..h], &rest[h..]),
        None => (rest, ""),
    };
    if let Some(clean) = redact_pairs(query, is_secret_query_key) {
        *url = format!("{}?{clean}{fragment}", &url[..qpos]);
    }
}

/// Walk a JSON value, masking the values of secret keys. True when it changed.
///
/// Returning "changed" rather than cloning to compare keeps a large body from
/// being duplicated in memory just to find out that it held no secret — which is
/// the common case, and the case where a capture is biggest.
fn redact_json(v: &mut Value) -> bool {
    match v {
        Value::Object(map) => {
            let mut changed = false;
            for (k, val) in map.iter_mut() {
                if is_secret_body_key(k) && !val.is_null() {
                    *val = Value::String(MITM_REDACTED_PLACEHOLDER.into());
                    changed = true;
                } else {
                    let inner = redact_json(val);
                    changed = changed || inner;
                }
            }
            changed
        }
        Value::Array(items) => items.iter_mut().fold(false, |acc, item| {
            let inner = redact_json(item);
            acc || inner
        }),
        _ => false,
    }
}

/// Mask credentials inside a captured body, in place.
///
/// JSON first, then form-urlencoded, then nothing. A body that parses as
/// neither is left byte for byte: guessing at free text is how a redactor starts
/// mangling the payloads it was meant to make readable.
///
/// # The limit that used to be here, closed 2026-09-04
///
/// Bodies were truncated to `policy::max_body_bytes()` BEFORE reaching this
/// function, so a JSON payload past that ceiling arrived as a fragment
/// `serde_json` refused, and a secret inside it survived into the capture.
///
/// The fix was ORDER, not heuristics. `body::redact_then_clip` now calls this
/// on the INTACT document and clips afterwards. That is cheap because the bytes
/// were already fully resident: the readers bound the READ at
/// `BUFFER_CEILING_BYTES`, two orders of magnitude above the retain budget, so
/// nothing new is held in memory.
///
/// The text-scanning fallback stays REFUSED, and for the same reason as before:
/// a heuristic scanner over free text trades a real risk of mangling captures
/// for a speculative gain. This function still parses or does nothing — it just
/// finally receives the input it was always written for.
pub(super) fn redact_body(body: &mut Option<String>) {
    let Some(text) = body.as_mut() else {
        return;
    };
    if let Ok(mut parsed) = serde_json::from_str::<Value>(text) {
        if redact_json(&mut parsed) {
            if let Ok(s) = serde_json::to_string(&parsed) {
                *text = s;
            }
        }
        return;
    }
    if let Some(clean) = redact_pairs(text, is_secret_body_key) {
        *text = clean;
    }
}

/// Mask credentials inside a WebSocket frame preview.
///
/// Frames are overwhelmingly JSON, and an auth frame is usually the FIRST one on
/// the socket, so leaving them unmasked put a live session token at the top of
/// every capture of an authenticated socket.
pub(super) fn redact_ws_preview(preview: &mut String) {
    let mut wrapped = Some(std::mem::take(preview));
    redact_body(&mut wrapped);
    *preview = wrapped.unwrap_or_default();
}

#[cfg(test)]
pub(super) use redact_headers as redact_headers_for_test;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_masking_is_case_and_separator_insensitive() {
        let mut h = BTreeMapString::new();
        h.insert("X-API-Key".into(), "live".into());
        h.insert("SET-COOKIE".into(), "sid=1".into());
        h.insert("Accept".into(), "application/json".into());
        redact_headers(&mut h);
        assert_eq!(h["X-API-Key"], MITM_REDACTED_PLACEHOLDER);
        assert_eq!(h["SET-COOKIE"], MITM_REDACTED_PLACEHOLDER);
        assert_eq!(h["Accept"], "application/json", "innocent header survives");
    }

    #[test]
    fn query_secrets_are_masked_and_the_rest_of_the_url_is_untouched() {
        let mut u = "https://api.test/v1/items?q=shoes&api_key=abc123&page=2#frag".to_string();
        redact_url(&mut u);
        assert_eq!(
            u,
            format!(
                "https://api.test/v1/items?q=shoes&api_key={MITM_REDACTED_PLACEHOLDER}&page=2#frag"
            )
        );
    }

    #[test]
    fn a_url_without_a_query_is_returned_byte_for_byte() {
        let mut u = "https://api.test/v1/items".to_string();
        let before = u.clone();
        redact_url(&mut u);
        assert_eq!(u, before);
    }

    #[test]
    fn nested_json_secrets_are_masked_at_any_depth() {
        let mut b = Some(
            r#"{"user":{"name":"ana","credentials":{"password":"hunter2"}},"items":[{"token":"t"}]}"#
                .to_string(),
        );
        redact_body(&mut b);
        let got = b.expect("body present");
        assert!(!got.contains("hunter2"), "nested password survived: {got}");
        assert!(
            !got.contains("\"t\""),
            "token inside an array survived: {got}"
        );
        assert!(got.contains("ana"), "non-secret field was destroyed: {got}");
    }

    #[test]
    fn a_body_with_no_secret_keeps_its_exact_bytes() {
        // Not just "still parses": reserialising would reflow the whitespace an
        // operator diffs against their own logs.
        let raw = r#"{ "key": "name", "value": 1 }"#;
        let mut b = Some(raw.to_string());
        redact_body(&mut b);
        assert_eq!(b.as_deref(), Some(raw), "untouched body was rewritten");
    }

    #[test]
    fn form_encoded_bodies_are_masked_too() {
        let mut b = Some("grant_type=password&password=hunter2&scope=read".to_string());
        redact_body(&mut b);
        let got = b.expect("body present");
        assert!(!got.contains("hunter2"));
        assert!(got.contains("scope=read"));
    }

    #[test]
    fn free_text_bodies_are_left_alone() {
        let mut b = Some("plain server error, no structure here".to_string());
        let before = b.clone();
        redact_body(&mut b);
        assert_eq!(b, before);
    }

    #[test]
    fn websocket_previews_go_through_the_same_masking() {
        let mut p = r#"{"op":"auth","access_token":"live-token"}"#.to_string();
        redact_ws_preview(&mut p);
        assert!(!p.contains("live-token"), "ws auth frame survived: {p}");
        assert!(p.contains("auth"), "frame shape was destroyed: {p}");
    }

    #[test]
    fn body_key_list_stays_narrower_than_the_query_list() {
        // The `{key, value}` pair is why. If this ever flips, a JSON body of
        // that shape starts coming back with its labels masked.
        assert!(is_secret_query_key("key"));
        assert!(!is_secret_body_key("key"));
    }
}
