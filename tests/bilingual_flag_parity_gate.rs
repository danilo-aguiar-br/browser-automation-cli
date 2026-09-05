//! A translated pair that enumerates flags must enumerate the SAME flags.
//!
//! # Why this gate exists
//!
//! MEASURED 2026-09-04: the global flag `--mitm-ws` was listed in `README.md`
//! and absent from `README.pt-BR.md`. `scripts/audit_bilingual_docs.sh` printed
//! `Summary: ok=18 fail=0` before the correction and `Summary: ok=18 fail=0`
//! after it, because that script compares CLI INVOCATIONS between the two
//! halves and never looks at a flag enumerated in a prose bullet.
//!
//! The general shape, which is the reason this file exists rather than one more
//! entry in that script: a gate that verifies the PRESENCE of an item never
//! verifies an ASSERTION about items. The Portuguese reader was handed a list
//! that claimed to be the MITM globals and was missing one of them, and every
//! verifier in the repository agreed the pair was fine.
//!
//! # Why CHANGELOG and gaps.md are excluded
//!
//! Same reason as `tests/doc_binary_numeral_gate.rs`: both are dated records. A
//! changelog entry names the flags that shipped in one release, and the two
//! languages legitimately word those entries differently over time. Editing
//! history to satisfy a gate is not a fix.

use std::collections::BTreeSet;
use std::path::PathBuf;

/// Repository root, from this test binary's manifest directory.
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Directory names never walked, because none of them ships to a reader.
const SKIPPED_DIRS: &[&str] = &[".git", "target", "node_modules", ".atomwrite"];

/// File names excluded from the scan, each because it is a dated record.
const DATED_RECORDS: &[&str] = &["CHANGELOG.md", "CHANGELOG.pt-BR.md", "gaps.md"];

/// The infix that marks the Portuguese half of a pair.
const PT_INFIX: &str = ".pt-BR.";

/// Flags allowed to appear on one side only, each with the reason.
///
/// EMPTY on purpose, MEASURED 2026-09-04: every asymmetry in the tree that day
/// was a translation gap, not an intentional one, so there was nothing
/// legitimate to list. The constant exists so that a future intentional case
/// has a sanctioned channel with a justification attached to it.
///
/// An entry is `(document stem, flag, why it is legitimately one-sided)`. An
/// entry without a real reason in its comment is forbidden: an allowlist that
/// accepts anything is how a gate dies while still reporting green.
const UNILATERAL: &[(&str, &str, &str)] = &[];

/// Every English document that has a Portuguese sibling, paired with it.
///
/// Fail-closed walker, for the reason `tests/doc_measured_claims_gate.rs`
/// records: its first shape used `.ok()`, and a directory it could not open
/// vanished from the total in silence. Duplicated rather than shared, because
/// an integration test is its own crate and `tests/common/mod.rs` is not this
/// wave's to edit.
fn bilingual_pairs() -> Vec<(PathBuf, PathBuf)> {
    let mut out = Vec::new();
    let mut stack = vec![root()];
    while let Some(d) = stack.pop() {
        let entries =
            std::fs::read_dir(&d).unwrap_or_else(|e| panic!("cannot walk {}: {e}", d.display()));
        for e in entries {
            let e = e.unwrap_or_else(|err| panic!("cannot read entry in {}: {err}", d.display()));
            let p = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if p.is_dir() {
                if !SKIPPED_DIRS.contains(&name.as_str()) {
                    stack.push(p);
                }
            } else if !DATED_RECORDS.contains(&name.as_str()) && !name.contains(PT_INFIX) {
                let Some(ext) = p.extension().and_then(|x| x.to_str()) else {
                    continue;
                };
                if ext != "md" && ext != "txt" {
                    continue;
                }
                let stem = name.trim_end_matches(ext).trim_end_matches('.').to_string();
                let pt = p.with_file_name(format!("{stem}{PT_INFIX}{ext}"));
                if pt.is_file() {
                    out.push((p, pt));
                }
            }
        }
    }
    out.sort();
    out
}

/// Every long-flag token in `text`, as a set.
///
/// A long flag is `--` followed by a lowercase letter, then lowercase letters,
/// digits and hyphens. The leading `--` must not itself be preceded by `-` or
/// by an alphanumeric character, which is what keeps a Markdown `---` rule and
/// a hyphenated word out of the set.
///
/// Pure on purpose: `the_detector_fires_on_a_one_sided_flag` drives this same
/// function with synthetic text, so the detector that guards the repository is
/// the detector that was proven to bite.
fn long_flags(text: &str) -> BTreeSet<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut found = BTreeSet::new();
    let mut i = 0usize;
    while i + 2 < chars.len() {
        if chars[i] != '-' || chars[i + 1] != '-' {
            i += 1;
            continue;
        }
        let preceded_by_word = i > 0 && (chars[i - 1] == '-' || chars[i - 1].is_alphanumeric());
        let mut j = i + 2;
        if preceded_by_word || !chars[j].is_ascii_lowercase() {
            // Advance past this run of hyphens so `---` is consumed once.
            while i < chars.len() && chars[i] == '-' {
                i += 1;
            }
            continue;
        }
        while j < chars.len()
            && (chars[j].is_ascii_lowercase() || chars[j].is_ascii_digit() || chars[j] == '-')
        {
            j += 1;
        }
        let token: String = chars[i..j].iter().collect();
        // A trailing hyphen belongs to the prose, not to the flag.
        found.insert(token.trim_end_matches('-').to_string());
        i = j;
    }
    found
}

/// Which side of a pair a flag is missing from.
#[derive(Debug, PartialEq, Eq)]
enum MissingFrom {
    /// Present in Portuguese, absent from English.
    English,
    /// Present in English, absent from Portuguese.
    Portuguese,
}

/// The flags one half of a pair enumerates and the other does not.
fn parity_gaps(en: &str, pt: &str) -> Vec<(String, MissingFrom)> {
    let (a, b) = (long_flags(en), long_flags(pt));
    let mut gaps: Vec<(String, MissingFrom)> = a
        .difference(&b)
        .map(|f| (f.clone(), MissingFrom::Portuguese))
        .chain(b.difference(&a).map(|f| (f.clone(), MissingFrom::English)))
        .collect();
    gaps.sort_by(|x, y| x.0.cmp(&y.0));
    gaps
}

#[test]
fn every_bilingual_pair_enumerates_the_same_long_flags() {
    let pairs = bilingual_pairs();
    // A scan that found no pair is not a pass: it means the discovery rule
    // stopped matching the tree's naming, and the gate is guarding nothing.
    assert!(
        pairs.len() >= 10,
        "found only {} bilingual document pairs; the `.pt-BR.` naming this gate \
         discovers by must have changed",
        pairs.len()
    );

    let mut report: Vec<String> = Vec::new();
    for (en_path, pt_path) in pairs {
        let en = std::fs::read_to_string(&en_path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", en_path.display()));
        let pt = std::fs::read_to_string(&pt_path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", pt_path.display()));
        let rel = en_path
            .strip_prefix(root())
            .unwrap_or(&en_path)
            .to_path_buf();
        let stem = rel.display().to_string();
        for (flag, side) in parity_gaps(&en, &pt) {
            if UNILATERAL
                .iter()
                .any(|(doc, allowed, _)| *doc == stem && *allowed == flag)
            {
                continue;
            }
            let missing = match side {
                MissingFrom::Portuguese => pt_path.display().to_string(),
                MissingFrom::English => en_path.display().to_string(),
            };
            report.push(format!("{stem}: `{flag}` is missing from {missing}"));
        }
    }
    assert!(
        report.is_empty(),
        "translated pairs enumerate different flags, so one language's reader \
         is handed an incomplete list while every existing verifier stays \
         green. Add the flag to the half that lacks it, or justify it in \
         UNILATERAL:\n{}",
        report.join("\n")
    );
}

/// The detector must fail when the violation is present, or it proves nothing.
///
/// Reproduces the 2026-09-04 incident in memory: `--mitm-ws` listed among the
/// MITM globals on one side and absent from the other.
#[test]
fn the_detector_fires_on_a_one_sided_flag() {
    let en = "- Optional MITM: global `--mitm`, `--mitm-ca-dir`, `--mitm-ws`, `--mitm-hosts`";
    let pt = "- MITM opcional: global `--mitm`, `--mitm-ca-dir`, `--mitm-hosts`";
    let gaps = parity_gaps(en, pt);
    assert_eq!(
        gaps,
        vec![("--mitm-ws".to_string(), MissingFrom::Portuguese)],
        "the missing global must be reported against the half that lacks it"
    );

    // And the mirror case, so a flag added only to the translation is caught
    // too: a Portuguese-only flag is just as much a broken pair.
    let gaps = parity_gaps(pt, en);
    assert_eq!(gaps, vec![("--mitm-ws".to_string(), MissingFrom::English)]);

    // An identical pair must be silent, or the gate reports noise and gets
    // ignored.
    assert!(parity_gaps(en, en).is_empty());
}

/// What is and is not a long flag, so the set compared is the set intended.
#[test]
fn the_tokenizer_reads_flags_and_not_prose() {
    let flags = long_flags(
        "---\ntitle: x\n---\n\nUse `--mitm-max-body-bytes`, `--proxy`, and **--no-stealth**.\n\
         Well-known hyphen-words and em—dashes are not flags. Neither is `-q` nor `x--y`.",
    );
    let expected: BTreeSet<String> = ["--mitm-max-body-bytes", "--no-stealth", "--proxy"]
        .into_iter()
        .map(str::to_string)
        .collect();
    assert_eq!(flags, expected, "unexpected token set: {flags:?}");
}
