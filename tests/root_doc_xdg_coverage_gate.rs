// SPDX-License-Identifier: MIT OR Apache-2.0
//! The root `llms-full` documents publish the XDG surface twice — once as a
//! NUMERAL and once as an ENUMERATION — and nothing was checking that the two
//! halves of the same sentence agree with each other or with the binary.
//!
//! # Why this gate exists
//!
//! MEASURED 2026-09-04: `llms-full.txt` said "Complete XDG surface, all **217**
//! keys grouped by family" and then listed 205 keys under that sentence.
//! `llms-full.pt-BR.txt` carried the identical defect in Portuguese. The
//! numeral had been updated when the surface grew; the list beneath it had not.
//!
//! Every existing gate was green while that was true, and each for a defensible
//! reason:
//!
//! - `tests/doc_binary_numeral_gate.rs` compares the NUMERAL against
//!   `config list-keys` and never looks at the prose under it, so the freshly
//!   updated `217` satisfied it.
//! - `scripts/doc-coverage-check.sh` checks key coverage in
//!   `docs/CONFIGURATION.md`, which is a different file from the root
//!   `llms-full` pair.
//! - `tests/schema_input_drift_gate.rs` compares `docs/schemas/*.json` against
//!   `schema --cmd`, and the root documents are not schemas.
//!
//! So a sentence that promised N and delivered N-12 had no owner. This gate is
//! that owner: it re-measures the live surface from the RUNNING BINARY and
//! demands that both root documents enumerate every key of it, and that the
//! numeral each document publishes equals what it actually lists.
//!
//! # What exactly it measures
//!
//! For each of `llms-full.txt` and `llms-full.pt-BR.txt`:
//!
//! 1. the anchor sentence's numeral equals the live key count;
//! 2. every live key appears in the enumeration below that sentence — a
//!    failure names each missing key, not just a total;
//! 3. the enumeration lists no key the binary does not have, which is the same
//!    defect running the other way;
//! 4. each family line's own `(N)` equals how many keys that line lists, so a
//!    per-family counter cannot rot the way the global one did.
//!
//! # Why it never reads a frozen list
//!
//! A hand-written expectation in this file would rot exactly like the sentence
//! it polices, and the first person to see a red run would "fix" it by editing
//! the constant. The authority is `config list-keys --json` through
//! `common::cmd()`, which runs the binary against an isolated home so the
//! operator's own configuration cannot change the answer.

use std::collections::BTreeSet;
use std::path::PathBuf;

mod common;

/// Sentences that open the enumeration, in the two published languages.
const ANCHORS: &[&str] = &["keys grouped by family", "chaves agrupadas por família"];

/// Nouns the anchor's numeral attaches to, in the two published languages.
const KEY_NOUNS: &[&str] = &[" keys", " chaves"];

/// The two root documents that publish the whole surface to a reader with no
/// shell. Named rather than globbed: a NEW root document that also enumerates
/// the surface should be added here deliberately.
const ROOT_DOCUMENTS: &[&str] = &["llms-full.txt", "llms-full.pt-BR.txt"];

/// A near-empty surface would make both documents look complete.
const MIN_LIVE_KEYS: usize = 100;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// One "all N keys grouped by family" block, as the document publishes it.
#[derive(Debug)]
struct Enumeration {
    /// One-based line of the anchor sentence, so a failure names a place to go.
    line: usize,
    /// The total the anchor sentence claims.
    declared_total: usize,
    /// Every key listed in the family lines beneath the anchor.
    keys: BTreeSet<String>,
    /// Family lines whose own `(N)` disagrees with what they list.
    family_mismatches: Vec<String>,
}

/// Backticks and bold markers are decoration around the same text.
fn undecorate(line: &str) -> String {
    line.chars().filter(|c| *c != '`' && *c != '*').collect()
}

/// Digits immediately preceding ` keys` / ` chaves` on the anchor line.
fn declared_total(line: &str) -> Option<usize> {
    for noun in KEY_NOUNS {
        let Some(at) = line.find(noun) else { continue };
        let digits: String = line[..at]
            .chars()
            .rev()
            .take_while(char::is_ascii_digit)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        if let Ok(n) = digits.parse::<usize>() {
            return Some(n);
        }
    }
    None
}

/// Every enumeration block in `doc`.
///
/// Pure on purpose: the failure-proof test drives this same parser and the same
/// comparison with synthetic text, so the detector that guards the repository
/// is the detector that was seen biting.
fn enumerations(doc: &str) -> Vec<Enumeration> {
    let lines: Vec<&str> = doc.lines().collect();
    let mut out = Vec::new();
    for (idx, raw) in lines.iter().enumerate() {
        let anchor = undecorate(raw);
        if !ANCHORS.iter().any(|a| anchor.contains(a)) {
            continue;
        }
        let Some(total) = declared_total(&anchor) else {
            continue;
        };
        let mut keys = BTreeSet::new();
        let mut family_mismatches = Vec::new();
        // The block is the run of indented sub-bullets directly beneath the
        // anchor; the next top-level bullet ends it.
        for (offset, family_raw) in lines[idx + 1..].iter().enumerate() {
            let Some(body) = family_raw.strip_prefix("  - ") else {
                break;
            };
            let body = undecorate(body);
            let Some((label, listed)) = body.split_once(": ") else {
                break;
            };
            let listed: Vec<String> = listed
                .split(',')
                .map(|k| k.trim().to_string())
                .filter(|k| !k.is_empty())
                .collect();
            let claimed = label
                .split_once('(')
                .and_then(|(_, rest)| rest.split_once(')'))
                .and_then(|(n, _)| n.trim().parse::<usize>().ok());
            if let Some(claim) = claimed {
                if claim != listed.len() {
                    family_mismatches.push(format!(
                        "line {}: `{}` claims {claim} keys and lists {}",
                        idx + offset + 2,
                        label.trim(),
                        listed.len()
                    ));
                }
            }
            keys.extend(listed);
        }
        out.push(Enumeration {
            line: idx + 1,
            declared_total: total,
            keys,
            family_mismatches,
        });
    }
    out
}

/// Everything wrong with `doc`, as lines an operator can act on.
///
/// Empty means the document agrees with the live surface. This is the function
/// the failure-proof test exercises against a document that is missing a key.
fn audit(label: &str, doc: &str, live: &BTreeSet<String>) -> Vec<String> {
    let blocks = enumerations(doc);
    if blocks.is_empty() {
        return vec![format!(
            "{label}: no `all N keys grouped by family` enumeration found; either \
             the sentence was reworded and this gate must follow it, or the whole \
             surface listing was dropped"
        )];
    }
    let mut problems = Vec::new();
    for block in &blocks {
        if block.declared_total != live.len() {
            problems.push(format!(
                "{label}:{} declares {} keys, the binary has {}",
                block.line,
                block.declared_total,
                live.len()
            ));
        }
        if block.keys.len() != block.declared_total {
            problems.push(format!(
                "{label}:{} declares {} keys and enumerates {}",
                block.line,
                block.declared_total,
                block.keys.len()
            ));
        }
        for missing in live.difference(&block.keys) {
            problems.push(format!(
                "{label}:{} enumerates no `{missing}`, which the binary has",
                block.line
            ));
        }
        for unknown in block.keys.difference(live) {
            problems.push(format!(
                "{label}:{} enumerates `{unknown}`, which the binary does not have",
                block.line
            ));
        }
        for mismatch in &block.family_mismatches {
            problems.push(format!("{label}: {mismatch}"));
        }
    }
    problems
}

/// The configuration keys the product actually has, asked of the product.
///
/// `.data.keys` is the array `config list-keys` publishes, the same authority
/// `tests/doc_binary_numeral_gate.rs` uses for the numeral.
fn live_keys() -> BTreeSet<String> {
    let out = common::cmd()
        .args(["--json", "config", "list-keys"])
        .output()
        .expect("`config list-keys` must run");
    let envelope: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "`config list-keys --json` must emit an envelope: {e}; stderr={}",
            String::from_utf8_lossy(&out.stderr)
        )
    });
    envelope["data"]["keys"]
        .as_array()
        .expect("the envelope must carry `data.keys` as an array")
        .iter()
        .map(|k| {
            k.get("key")
                .or(Some(k))
                .and_then(serde_json::Value::as_str)
                .expect("each entry must name a key")
                .to_string()
        })
        .collect()
}

#[test]
fn both_root_documents_enumerate_the_whole_live_xdg_surface() {
    let live = live_keys();
    assert!(
        live.len() >= MIN_LIVE_KEYS,
        "`config list-keys` published only {} keys; a near-empty surface would \
         make every document look complete",
        live.len()
    );

    let mut problems = Vec::new();
    for name in ROOT_DOCUMENTS {
        let path = root().join(name);
        let doc = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        problems.extend(audit(name, &doc, &live));
    }

    assert!(
        problems.is_empty(),
        "the root documents disagree with the live XDG surface. \
         `config list-keys --json` is the authority; update the prose AND the \
         list beneath it, do not relax this gate:\n{}",
        problems.join("\n")
    );
}

/// The gate must fail when the defect is present, or it proves nothing.
///
/// Driven on synthetic text rather than on a temporarily corrupted repository
/// file, so the proof leaves no edit behind and survives a reader who never
/// runs it.
#[test]
fn the_audit_fires_when_the_enumeration_is_short_of_the_surface() {
    let live: BTreeSet<String> = ["audio_default_format", "lang", "timeout"]
        .into_iter()
        .map(str::to_string)
        .collect();

    // The exact shape of the 2026-09-04 incident, shrunk to three keys: the
    // numeral says 3, the list beneath it carries 2, and `lang` is the key that
    // fell out.
    let stale = "- Complete XDG surface, all **3** keys grouped by family. The source of truth is `config list-keys --json`\n\
                 \x20 - `audio_*` (1): audio_default_format\n\
                 \x20 - standalone (2): timeout\n\
                 - Restore a default: `config unset <KEY>`\n";
    let problems = audit("synthetic.txt", stale, &live);
    assert!(
        problems.iter().any(|p| p.contains("enumerates no `lang`")),
        "the missing key must be named, got {problems:?}"
    );
    assert!(
        problems.iter().any(|p| p.contains("enumerates 2")),
        "the numeral/content gap must be reported, got {problems:?}"
    );
    assert!(
        problems
            .iter()
            .any(|p| p.contains("claims 2 keys and lists 1")),
        "the per-family counter must be checked, got {problems:?}"
    );

    // A key the binary does not have is the same defect running backwards.
    let extra = "- Complete XDG surface, all **3** keys grouped by family\n\
                 \x20 - standalone (3): audio_default_format, lang, timeout, ghost_key\n";
    assert!(
        audit("synthetic.txt", extra, &live)
            .iter()
            .any(|p| p.contains("enumerates `ghost_key`")),
        "a key absent from the binary must be reported"
    );

    // The complete document must be silent, or the gate cries wolf.
    let good = "- Complete XDG surface, all **3** keys grouped by family\n\
                \x20 - `audio_*` (1): audio_default_format\n\
                \x20 - standalone (2): lang, timeout\n\
                - Restore a default: `config unset <KEY>`\n";
    assert!(
        audit("synthetic.txt", good, &live).is_empty(),
        "a document that matches the surface must pass"
    );

    // A document that stopped enumerating anything is a failure, never a pass.
    assert!(
        !audit("synthetic.txt", "- nothing to see here\n", &live).is_empty(),
        "a vanished enumeration must fail rather than silently check nothing"
    );
}

/// The expected surface must come from the binary, not from a constant here.
///
/// Guards the failure mode where someone "fixes" a red run by freezing the key
/// list in this file, which would restore the exact defect the gate removes —
/// one more hand-written list with no compiler.
#[test]
fn the_authority_is_the_binary_and_it_answers() {
    let live = live_keys();
    assert!(
        live.contains("lang"),
        "`config list-keys` must publish real key names, got {} entries",
        live.len()
    );
    assert!(
        live.len() >= MIN_LIVE_KEYS,
        "the live surface collapsed to {} keys",
        live.len()
    );
}
