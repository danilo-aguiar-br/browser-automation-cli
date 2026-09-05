// SPDX-License-Identifier: MIT OR Apache-2.0
//! Every hand-written config resolver must prove what it does with a stored zero.
//!
//! # The gap this closes
//!
//! The crate resolves XDG config through two families with different
//! disciplines. The `policy_knobs!` table validates ONCE, in `policy_u64`, for
//! every key it declares: a stored zero is dropped and the named default takes
//! over. The hand-written family in `src/xdg/resolve*.rs` repeats that guard by
//! hand, function by function.
//!
//! On 2026-08-25 a bug surfaced from exactly that shape: `max_attempts = 0`
//! reached a loop that skipped every attempt and panicked with exit 101. The fix
//! landed at the one site where it showed, and the class stayed open — dozens of
//! copies of the same guard, none of them checked by anything.
//!
//! Migrating the family into `policy_knobs!` was considered and rejected: it
//! would be a REGRESSION, because `policy_u64` validates exactly `> 0` while
//! several of these functions validate a range (`1..=100` for JPEG quality, an
//! upper bound for the Lightpanda session timeout). Flattening them into a
//! single `> 0` check would quietly widen every one of those.
//!
//! So the invariant is asserted instead of centralized.
//!
//! # The invariant
//!
//! A resolver that CALLS `load_config()` owns the guard, and must show one of:
//!
//! - a `.filter(` in its body, rejecting the values it will not accept; or
//! - a doc comment that names `` `0` ``, stating that zero is a legal input.
//!
//! A resolver that does NOT call `load_config()` is delegating to a layer that
//! already validated — `policy_u64`, or another resolver — and is exempt by
//! construction. That is the whole discriminator: whoever reads the config owes
//! the proof.
//!
//! # The second invariant: narrowing
//!
//! Accepting a value is half the job; the other half is carrying it without
//! losing it. A resolver that narrows the CONFIG value with an `as` cast must
//! say in its doc comment why that cast is lossless, and the marker this gate
//! looks for is the word `cast`.
//!
//! This half was added on 2026-08-25, after an audit found
//! `resolve_manifest_max_variants` passing the guard check — it has a
//! `.filter(` — while narrowing the operator's value with a bare `as usize`
//! that nothing explained. The cast turned out to be lossless, but the gate
//! had approved it without knowing that, which is the same thing as not
//! checking. A cast of a crate CONSTANT inside `unwrap_or(..)` is exempt: it
//! is compile-time known and narrows no operator input.
//!
//! # What this cannot catch
//!
//! It reads text, not semantics. It cannot tell a correct range from a wrong
//! one, and a `.filter(|&n| n > 0)` on a knob that needed `n >= 2` passes here.
//! It cannot tell a true explanation of a cast from a false one either — it
//! only forces one to be written. What it does catch is the omission — the
//! resolver added next year with no guard, no statement and no reason — which
//! is the shape the `max_attempts` bug had.

use std::path::Path;

/// Files holding the hand-written resolver family.
const RESOLVER_FILES: &[&str] = &[
    "src/xdg/resolve.rs",
    "src/xdg/resolve_media.rs",
    "src/xdg/resolve_scrape.rs",
];

/// Return types this gate treats as numeric, and therefore zero-sensitive.
const NUMERIC_RETURNS: &[&str] = &["u8", "u16", "u32", "u64", "usize", "i32", "i64", "f64"];

/// One resolver, sliced out of the source text.
struct Resolver {
    name: String,
    doc: String,
    body: String,
}

/// Slice every `pub fn resolve_*() -> <numeric>` out of one file.
///
/// Deliberately a line scanner rather than a parser: the shape it looks for is
/// four lines of `rustfmt` output that this file family never deviates from, and
/// a parser would be a dependency plus a second thing to keep correct. The
/// closing brace is found by column, which holds because every one of these is a
/// free function at module level.
fn resolvers_in(text: &str) -> Vec<Resolver> {
    let mut out = Vec::new();
    let mut doc = String::new();
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("///") {
            doc.push_str(trimmed);
            doc.push('\n');
            continue;
        }
        // Attributes sit between the doc comment and the signature; keep the doc.
        if trimmed.starts_with("#[") {
            continue;
        }
        let is_resolver = line.starts_with("pub fn resolve_")
            && line.contains("() -> ")
            && NUMERIC_RETURNS
                .iter()
                .any(|t| line.contains(&format!("() -> {t} {{")));
        if !is_resolver {
            doc.clear();
            continue;
        }
        let name = line
            .trim_start_matches("pub fn ")
            .split('(')
            .next()
            .unwrap_or_default()
            .to_string();
        let mut body = String::new();
        for inner in lines.by_ref() {
            if inner == "}" {
                break;
            }
            body.push_str(inner);
            body.push('\n');
        }
        out.push(Resolver {
            name,
            doc: std::mem::take(&mut doc),
            body,
        });
    }
    out
}

/// Whether this resolver owes a proof and fails to show one.
///
/// Extracted from the gate below so the rule can be exercised in both
/// directions. A predicate that is only ever asserted on conforming input
/// passes just as well when it always answers `false`, which is the failure
/// mode this crate has already shipped once, in the `sg_local` scanner whose
/// single test asserted `count >= 1` and never that anything was let through.
fn offends(r: &Resolver) -> bool {
    // Delegating resolvers are exempt: the layer they call already validated,
    // and duplicating the guard here would be the very repetition this gate
    // exists to bound.
    if !r.body.contains("load_config()") {
        return false;
    }
    let guards = r.body.contains(".filter(");
    let declares_zero_legal = r.doc.contains("`0`");
    if !guards && !declares_zero_legal {
        return true;
    }
    narrows_config_value(r) && !r.doc.contains("cast")
}

/// Whether this resolver narrows the OPERATOR's value with an `as` cast.
///
/// The discriminator is the mapping closure, not the word `as`. Both shapes in
/// the family today — `|n| n as usize` and `.map(|n| n.min(..) as u32)` — apply
/// the cast to the value that came out of the config, while
/// `unwrap_or(DEFAULT_X as usize)` applies it to a crate constant whose value
/// the compiler already knows. Splitting the line at ` as ` and asking what
/// stands to the LEFT separates the two without a parser: a closure head is
/// there in the first case and absent in the second.
///
/// `.filter(|&n| n <= i32::MAX as u32)` is left alone by the same rule — the
/// binding is `|&n|`, and what is cast is the type constant on the right of the
/// comparison, not `n`.
///
/// EVERY occurrence of ` as ` on the line is examined, not the first. The first
/// version of this predicate split the line and looked only at the head, which
/// missed `.map_or(DEFAULT as usize, |n| n as usize)` — one line carrying both
/// an exempt cast and an offending one, with the exempt one in front. The
/// negative test below caught it before this gate ever ran on the crate.
fn narrows_config_value(r: &Resolver) -> bool {
    r.body.lines().any(|l| {
        l.match_indices(" as ").any(|(i, _)| {
            let left = l.get(..i).unwrap_or_default();
            left.contains("|n|") || left.contains(".map(")
        })
    })
}

#[test]
fn every_config_reading_resolver_states_what_it_does_with_zero() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut checked = 0usize;
    let mut offenders: Vec<String> = Vec::new();

    for rel in RESOLVER_FILES {
        let text =
            std::fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"));
        for r in resolvers_in(&text) {
            if !r.body.contains("load_config()") {
                continue;
            }
            checked += 1;
            if offends(&r) {
                offenders.push(format!("{rel}::{}", r.name));
            }
        }
    }

    assert!(
        checked >= 44,
        "the scanner found only {checked} config-reading resolvers, which means it \
         stopped matching the source shape rather than that the family shrank; \
         fix the scanner before trusting a green result here"
    );
    assert!(
        offenders.is_empty(),
        "these resolvers read the config without saying what a stored `0` does. \
         Add `.filter(..)` to reject it, or say `0` in the doc comment to accept \
         it on purpose: {offenders:#?}"
    );
}

#[test]
fn the_scanner_actually_recognizes_the_shape_it_looks_for() {
    // The guard above is only as good as its parser. A scanner that silently
    // stopped matching would report zero offenders and pass, which is the same
    // failure mode the `checked >= 40` floor defends against — asserted here
    // directly, on a fixture whose answer is known.
    let fixture = "\
/// Guarded knob.
pub fn resolve_guarded() -> u64 {
    load_config()
        .ok()
        .and_then(|c| c.thing)
        .filter(|&n| n > 0)
        .unwrap_or(1)
}

/// Knob where `0` is legal.
pub fn resolve_zero_ok() -> u64 {
    load_config().ok().and_then(|c| c.thing).unwrap_or(1)
}

/// Delegating knob.
pub fn resolve_delegated() -> u64 {
    resolve_guarded()
}

/// Not numeric.
pub fn resolve_flag() -> bool {
    load_config().ok().and_then(|c| c.flag).unwrap_or(true)
}
";
    let found = resolvers_in(fixture);
    let names: Vec<&str> = found.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["resolve_guarded", "resolve_zero_ok", "resolve_delegated"],
        "the scanner must pick up numeric resolvers and skip the boolean one"
    );
    assert!(
        found[0].body.contains(".filter("),
        "the guarded body must carry its filter"
    );
    assert!(
        found[1].doc.contains("`0`"),
        "the doc of the zero-legal knob must survive into the slice"
    );
    assert!(
        !found[2].body.contains("load_config()"),
        "the delegating body must be recognizable as delegating"
    );
}

/// The negative half: the rule must actually reject something.
///
/// # Why this test has to exist
///
/// `every_config_reading_resolver_states_what_it_does_with_zero` is green today
/// and would stay green against an `offends` that always answers `false`. The
/// only thing separating a working gate from a decorative one is an input it is
/// known to refuse. The fixture below is the exact shape the `max_attempts` bug
/// had on 2026-08-25: a config read, no filter, and a doc comment that never
/// mentions what a stored zero does.
#[test]
fn a_config_reading_resolver_with_no_guard_and_no_statement_is_caught() {
    let fixture = "\
/// Floor delay between same-origin GETs (ms).
pub fn resolve_unguarded() -> u64 {
    load_config().ok().and_then(|c| c.thing).unwrap_or(1)
}

/// Knob that rejects zero.
pub fn resolve_guarded() -> u64 {
    load_config()
        .ok()
        .and_then(|c| c.thing)
        .filter(|&n| n > 0)
        .unwrap_or(1)
}

/// Knob where `0` is legal and says so.
pub fn resolve_zero_ok() -> u64 {
    load_config().ok().and_then(|c| c.thing).unwrap_or(1)
}

/// Knob that delegates to a validated layer.
pub fn resolve_delegated() -> u64 {
    resolve_guarded()
}
";
    let caught: Vec<String> = resolvers_in(fixture)
        .iter()
        .filter(|r| offends(r))
        .map(|r| r.name.clone())
        .collect();
    assert_eq!(
        caught,
        vec!["resolve_unguarded".to_string()],
        "exactly the unguarded, undocumented resolver must be caught — a filter, \
         a stated `0`, or delegation each clear the bar"
    );
}

/// The narrowing half, in both directions.
///
/// # Why this test has to exist
///
/// `narrows_config_value` answering `false` for everything would leave the
/// whole second invariant decorative and every existing resolver green, which
/// is precisely how the first version of this gate let
/// `resolve_manifest_max_variants` through. So the fixture below carries one
/// resolver of each shape the family actually contains, and the assertion is
/// on the exact set, never on a count.
#[test]
fn a_resolver_that_narrows_the_operators_value_without_explaining_it_is_caught() {
    let fixture = "\
/// Narrows the stored value and never says why.
pub fn resolve_bare_cast() -> usize {
    load_config()
        .ok()
        .and_then(|c| c.thing)
        .filter(|&n| n > 0)
        .map_or(DEFAULT_THING as usize, |n| n as usize)
}

/// Narrows the stored value and explains the cast.
///
/// The stored knob is a `u32`, so the cast to `usize` is lossless.
pub fn resolve_explained_cast() -> usize {
    load_config()
        .ok()
        .and_then(|c| c.thing)
        .filter(|&n| n > 0)
        .map_or(DEFAULT_THING as usize, |n| n as usize)
}

/// Casts only the crate constant, never the operator's value.
pub fn resolve_constant_cast() -> usize {
    load_config()
        .ok()
        .and_then(|c| c.thing)
        .filter(|&n| n > 0)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(DEFAULT_THING as usize)
}

/// Compares against a type constant inside the guard.
pub fn resolve_typed_bound() -> u32 {
    load_config()
        .ok()
        .and_then(|c| c.thing)
        .filter(|&n| n <= i32::MAX as u32)
        .unwrap_or(1)
}
";
    let found = resolvers_in(fixture);
    let narrowing: Vec<&str> = found
        .iter()
        .filter(|r| narrows_config_value(r))
        .map(|r| r.name.as_str())
        .collect();
    assert_eq!(
        narrowing,
        vec!["resolve_bare_cast", "resolve_explained_cast"],
        "only a cast applied to the value out of the config counts — a cast of \
         a crate constant in `unwrap_or` and a type constant inside `.filter` \
         narrow nothing the operator controls"
    );

    let caught: Vec<&str> = found
        .iter()
        .filter(|r| offends(r))
        .map(|r| r.name.as_str())
        .collect();
    assert_eq!(
        caught,
        vec!["resolve_bare_cast"],
        "of the two that narrow, exactly the one with no `cast` in its doc is \
         rejected"
    );
}
