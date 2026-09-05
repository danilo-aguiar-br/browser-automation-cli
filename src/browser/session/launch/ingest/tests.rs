// SPDX-License-Identifier: MIT OR Apache-2.0
//! Two gates that judge THIS directory from the outside, plus the eviction policy.
//!
//! # Why they are a file and not a block
//!
//! Nothing here exercises a CDP event. The first half pins the ring policy in
//! [`super::rings`] as a pure function; the second half is a class gate that
//! reads the SOURCE of every `assert_net` module with `include_str!` and proves
//! no reader addresses a capture key no producer writes — the root cause of the
//! 0.1.9 audit rather than one of its symptoms.
//!
//! A gate that parses sibling files is not an appendix to the decoder it sits
//! next to. Kept inline it doubled the length of `ingest.rs` and buried the one
//! function that actually turns an event into buffer content.

use super::*;

// These cover the eviction policy itself, which shipped without a test the
// first time the console and network rings were capped. A ceiling nothing
// exercises is a ceiling the next refactor can quietly delete, and the
// symptom of its absence — a buffer that grows until the process dies — is
// invisible until the machine it runs on is the one paying.

#[test]
fn cap_ring_keeps_the_newest_and_counts_what_it_dropped() {
    let mut buf: Vec<u8> = (1..=10).collect();
    let mut dropped = 0u64;
    OneShotSession::cap_ring(&mut buf, &mut dropped, 4);
    assert_eq!(buf, vec![7, 8, 9, 10], "eviction must be from the FRONT");
    assert_eq!(dropped, 6);
}

#[test]
fn cap_ring_leaves_a_buffer_under_the_cap_untouched() {
    let mut buf: Vec<u8> = vec![1, 2, 3];
    let mut dropped = 0u64;
    OneShotSession::cap_ring(&mut buf, &mut dropped, 4);
    assert_eq!(buf, vec![1, 2, 3]);
    assert_eq!(dropped, 0, "nothing was evicted, so nothing may be counted");
}

#[test]
fn cap_ring_at_the_exact_cap_evicts_nothing() {
    // The off-by-one that would silently drop a row every single push.
    let mut buf: Vec<u8> = vec![1, 2, 3, 4];
    let mut dropped = 0u64;
    OneShotSession::cap_ring(&mut buf, &mut dropped, 4);
    assert_eq!(buf, vec![1, 2, 3, 4]);
    assert_eq!(dropped, 0);
}

/// Keys every `console_log` record carries, as written a few lines above.
const CONSOLE_RECORD_KEYS: &[&str] = &["type", "text", "args"];

/// Keys every `network_log` record carries when it is created.
const NETWORK_RECORD_KEYS: &[&str] = &["requestId", "method", "url", "resourceType"];

/// Keys `enrich_with_response` adds to an EXISTING network record.
///
/// Separate from [`NETWORK_RECORD_KEYS`] because they arrive on a different
/// CDP event: the request event cannot carry a response status, and reading
/// one off the request log is exactly how `mitm capture-url` came to emit
/// `status: null` with `ok: true`.
const NETWORK_RESPONSE_KEYS: &[&str] = &["status", "mimeType"];

/// Keys the readers address on an ENVELOPE they built, not on a record.
///
/// Declared so the gate can tell "this key needs a producer" apart from
/// "this key is a field of an answer this module composed itself".
const READER_ENVELOPE_KEYS: &[&str] = &["requests", "messages"];

/// Every module `assert_net/mod.rs` declares, paired with its source.
///
/// # Why membership is checked instead of listed
///
/// The first shape of this gate carried a list of TWO files written by
/// hand. This directory has SIX modules, and `assert_page.rs` — which
/// reads `console_log` directly — was not one of the two.
///
/// Measured 2026-08-30, one wave after this gate was built AND proven to
/// bite: `assert_page.rs` read a `message` key with a fallback, and
/// nothing has ever written `message` onto a console record. The live
/// instance of the very class this gate exists to catch was sitting in the
/// one file the gate did not open.
///
/// A list written by hand inherits the blind spot of whoever wrote it,
/// because it is written while looking at the cases already known. Adding
/// the third file would only postpone the fourth. Membership is now proven
/// against `mod.rs` — the compiler's own record of what this directory
/// contains — by
/// [`the_reader_set_covers_every_module_this_directory_declares`].
const ASSERT_NET_MODULES: &[(&str, &str)] = &[
    (
        "assert_page",
        include_str!("../../assert_net/assert_page.rs"),
    ),
    ("buffers", include_str!("../../assert_net/buffers.rs")),
    ("console", include_str!("../../assert_net/console.rs")),
    ("cookie", include_str!("../../assert_net/cookie.rs")),
    ("dialog", include_str!("../../assert_net/dialog.rs")),
    ("net", include_str!("../../assert_net/net.rs")),
];

/// The gate's reader set covers every module the directory declares.
///
/// This is what makes the coverage DERIVED rather than remembered. A new
/// module added to `assert_net/mod.rs` fails here until it is entered
/// above, so a future file that reads the capture buffers cannot be exempt
/// from the class gate in silence — which is exactly how `assert_page.rs`
/// stayed exempt while holding a live instance of the class.
#[test]
fn the_reader_set_covers_every_module_this_directory_declares() {
    let mod_rs = include_str!("../../assert_net/mod.rs");
    let declared: Vec<&str> = mod_rs
        .lines()
        .filter_map(|l| l.trim().strip_prefix("mod ")?.strip_suffix(';'))
        .collect();
    assert!(
        !declared.is_empty(),
        "no `mod` declaration parsed out of assert_net/mod.rs; the parse is \
         broken, and a broken parse here would silently pass this gate"
    );
    for m in &declared {
        assert!(
            ASSERT_NET_MODULES.iter().any(|(n, _)| n == m),
            "`{m}` is declared in assert_net/mod.rs and missing from the \
             gate's reader set, so anything it reads is exempt from the \
             class gate without a word: {declared:?}"
        );
    }
}

/// Every string literal passed to `.get(...)` in `src`.
fn keys_read(src: &str) -> Vec<String> {
    const NEEDLE: &str = r#".get(""#;
    let mut out = Vec::new();
    let mut rest = src;
    while let Some(i) = rest.find(NEEDLE) {
        rest = &rest[i + NEEDLE.len()..];
        if let Some(j) = rest.find('"') {
            out.push(rest[..j].to_string());
            rest = &rest[j..];
        }
    }
    out
}

/// A capture key may be READ only where something WRITES it.
///
/// # Why this gate exists
///
/// This is the ROOT CAUSE of the 0.1.9 audit rather than one of its
/// symptoms. `net --resource-types` filtered on `resource_type`, `type` and
/// `resourceType`; the producer wrote none of the three, so the flag
/// answered `ok: true` with zero rows on every page for as long as it
/// shipped. `console --service-worker-id` and the mitm `status` read were
/// the same defect in two further places.
///
/// Reading several spellings LOOKS like tolerance and is really evidence
/// that nobody checked which name the producer uses. No reviewer sees a
/// bug, no existing test fails, and the envelope reports success — only a
/// gate that crosses the two sides can catch it.
///
/// Measured 2026-08-30, AFTER that audit closed with ten green gates:
/// `console_list` still read a `level` key with a fallback to `type`, and
/// nothing has ever written `level`. The refusal EIGHT LINES BELOW it
/// rejects `service_worker_id` and states that ingest writes "`type`,
/// `text` and `args` and nothing else" — a sentence that already condemned
/// `level` in the same function. The fix for one sibling was applied and
/// the other survived, which is why this has to be mechanical instead of
/// remembered.
#[test]
fn no_reader_addresses_a_capture_key_that_no_producer_writes() {
    let produced: Vec<&str> = CONSOLE_RECORD_KEYS
        .iter()
        .chain(NETWORK_RECORD_KEYS)
        .chain(NETWORK_RESPONSE_KEYS)
        .chain(READER_ENVELOPE_KEYS)
        .copied()
        .collect();
    let mut orphans: Vec<String> = Vec::new();
    for (name, src) in ASSERT_NET_MODULES {
        // Only the modules that actually hold a capture buffer are judged
        // against the capture key set. `cookie.rs` and `dialog.rs` read
        // keys of their own domains, and measuring them here would produce
        // noise that trains a reader to ignore this gate.
        if !src.contains("console_log") && !src.contains("network_log") {
            continue;
        }
        for key in keys_read(src) {
            if !produced.contains(&key.as_str()) {
                orphans.push(format!("{name} reads .get({key:?})"));
            }
        }
    }
    assert!(
        orphans.is_empty(),
        "these reads address a capture key no producer writes, so they \
         select nothing and answer ok:true instead of failing: {orphans:#?}"
    );
}

/// Every key declared above is one this file actually writes.
///
/// The other half of the gate. Without it the declared set could drift into
/// a wishlist: a producer that stopped writing a key would leave the
/// consumer gate passing against a promise nobody keeps.
#[test]
fn every_declared_producer_key_is_actually_written_here() {
    let src = include_str!("../ingest.rs");
    for key in CONSOLE_RECORD_KEYS
        .iter()
        .chain(NETWORK_RECORD_KEYS)
        .chain(NETWORK_RESPONSE_KEYS)
    {
        let written = src.contains(&format!("{key:?}:")) || src.contains(&format!("{key:?}.to_"));
        assert!(
            written,
            "`{key}` is declared as produced and this file never writes it"
        );
    }
}

/// The screencast ring keeps the NEWEST frames and counts what it evicted.
///
/// # Why this test exists
///
/// The frame buffer was capped years before the console and network rings,
/// with `if len < CAP { push }` — which keeps the OLDEST frames and drops
/// everything after the ceiling, reporting nothing. That shape passed every
/// gate this repository has, because no test and no envelope field ever
/// asked what happened at the ceiling.
///
/// Both assertions below FAIL against the old shape: the first because it
/// would hold `[1, 2, 3]`, the second because there was no counter at all.
#[test]
fn the_screencast_ring_keeps_the_newest_frames_and_counts_the_rest() {
    let mut frames: Vec<String> = Vec::new();
    let mut dropped = 0u64;
    for i in 1..=10 {
        frames.push(format!("frame-{i}"));
        OneShotSession::cap_ring(&mut frames, &mut dropped, 3);
    }
    assert_eq!(
        frames,
        vec![
            "frame-8".to_string(),
            "frame-9".to_string(),
            "frame-10".to_string()
        ],
        "a screencast that outruns the ring must show its END, not its start"
    );
    assert_eq!(dropped, 7, "every evicted frame must be countable");
}

/// A recording under the real ceiling loses nothing and says so.
///
/// Pins the constant against the ring so a future edit that lowers
/// `SCREENCAST_FRAME_BUFFER_CAP` cannot start truncating ordinary
/// recordings without this failing first.
#[test]
fn a_short_recording_reports_zero_dropped_under_the_real_cap() {
    let cap = crate::constants::SCREENCAST_FRAME_BUFFER_CAP;
    assert!(cap >= 100, "a ceiling this low would truncate normal use");
    let mut frames: Vec<String> = (0..cap).map(|i| format!("f{i}")).collect();
    let mut dropped = 0u64;
    OneShotSession::cap_ring(&mut frames, &mut dropped, cap);
    assert_eq!(frames.len(), cap);
    assert_eq!(
        dropped, 0,
        "nothing was evicted, so nothing may be reported"
    );
}

#[test]
fn cap_zero_means_unbounded_not_discard_everything() {
    // The XDG convention for "no ceiling". Reading it as a literal cap
    // would empty the buffer on every push and report success.
    let mut buf: Vec<u8> = (1..=10).collect();
    let mut dropped = 0u64;
    OneShotSession::cap_ring(&mut buf, &mut dropped, 0);
    assert_eq!(buf.len(), 10);
    assert_eq!(dropped, 0);
}

#[test]
fn dropped_accumulates_across_calls() {
    // `perf stop` reports one total for the whole recording, so the counter
    // must survive every eviction rather than describe only the last.
    let mut buf: Vec<u8> = Vec::new();
    let mut dropped = 0u64;
    for i in 0..10u8 {
        buf.push(i);
        OneShotSession::cap_ring(&mut buf, &mut dropped, 3);
    }
    assert_eq!(buf, vec![7, 8, 9]);
    assert_eq!(dropped, 7);
}

#[test]
fn cap_ring_is_shared_by_the_string_ring_the_trace_uses() {
    // Same policy, different element type: proving it here is what keeps
    // the trace ring from growing its own copy of the eviction rule.
    let mut buf: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
    let mut dropped = 0u64;
    OneShotSession::cap_ring(&mut buf, &mut dropped, 2);
    assert_eq!(buf, vec!["b".to_string(), "c".to_string()]);
    assert_eq!(dropped, 1);
}
