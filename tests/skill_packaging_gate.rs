// SPDX-License-Identifier: MIT OR Apache-2.0
//! The published Agent Skills under `skills/` had no owner inside `cargo test`.
//!
//! # Why this gate exists
//!
//! MEASURED 2026-09-04: `rg -l -F 'skills/' tests/ src/` returned nothing, so no
//! Rust test opened that directory at all. The seven documentation gates in
//! `tests/` scan `docs/` and the repository root and never descend into
//! `skills/`, and the only coverage that directory had lived in
//! `scripts/doc-coverage-check.sh`, which does not run under `cargo test`.
//!
//! The consequence was measurable the same day:
//! `skills/browser-automation-cli-pt/SKILL.md` carried 5172 whitespace-separated
//! words against a 5000 ceiling, and the defect survived precisely because the
//! only verifier that could have seen it is not on the daily path. A packaging
//! rule whose sole enforcement is a script nobody runs is a rule with no
//! compiler.
//!
//! # What it measures
//!
//! One test per property, so a red run names the rule rather than a total:
//!
//! 1. each `SKILL.md` stays under the word ceiling the packaging format imposes;
//! 2. the frontmatter `description` exists, fits its character ceiling, and
//!    carries no colon in its VALUE, which would reopen the YAML scalar;
//! 3. no Markdown under `skills/` opens a fenced code block;
//! 4. no Markdown cites a `GAP-<digit>` identifier or a `v<digit>.<digit>`
//!    version label, both of which rot the moment the repository moves on;
//! 5. the two language directories carry the SAME relative paths;
//! 6. each matching pair carries the same number of headings, so a translation
//!    cannot silently drop a section;
//! 7. no file names a competing product;
//! 8. no file invents a product environment variable, which this product bans;
//! 9. each skill's corpus publishes the six envelope fields an operator needs to
//!    read a browser-mode answer.
//!
//! # Why it never spawns the binary
//!
//! Every property above is a property of bytes on disk. Reading them with the
//! standard library keeps this gate offline, fast, and green on a host with
//! neither Chrome nor a network — which is what lets it run on the daily path,
//! the exact property the shell script it replaces lacked.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

mod common;

/// Directory-name prefix that marks a published skill of this product.
const SKILL_DIR_PREFIX: &str = "browser-automation-cli-";

/// How many language directories must be found, so a discovery rule that stops
/// matching the tree fails loudly instead of auditing nothing.
const EXPECTED_SKILL_DIRS: usize = 2;

/// Whitespace-separated words a single `SKILL.md` may carry.
const MAX_SKILL_WORDS: usize = 5000;

/// Characters the frontmatter `description` value may carry.
const MAX_DESCRIPTION_CHARS: usize = 1024;

/// Products this skill must never name, in any case.
const BANNED_PRODUCTS: &[&str] = &[
    "firecrawl",
    "vercel",
    "agent-browser",
    "mcp chrome devtools",
];

/// Prefix of a product environment variable, which this product does not have.
const PRODUCT_ENV_PREFIX: &str = "BROWSER_AUTOMATION_";

/// Envelope fields every skill corpus must teach, because an operator who
/// cannot read them cannot tell which browser mode actually ran.
const ENVELOPE_FIELDS: &[&str] = &[
    "browser_mode_requested",
    "browser_mode_effective",
    "browser_mode_source",
    "display_backend",
    "runtime_enable_used",
    "serp_endpoint",
];

fn skills_root() -> PathBuf {
    common::root().join("skills")
}

/// Path as written relative to the repository root, for a message an operator
/// can paste into an editor.
fn relative(path: &Path) -> String {
    path.strip_prefix(common::root())
        .unwrap_or(path)
        .display()
        .to_string()
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("skill packaging gate cannot read {}: {e}", relative(path)))
}

/// The published skill directories, sorted.
///
/// Fail-closed: a directory that cannot be opened panics rather than vanishing
/// from the audit, which is the failure mode `tests/doc_measured_claims_gate.rs`
/// records from its own first shape.
fn skill_dirs() -> Vec<PathBuf> {
    let base = skills_root();
    let entries = std::fs::read_dir(&base)
        .unwrap_or_else(|e| panic!("skill packaging gate cannot open {}: {e}", relative(&base)));
    let mut out = Vec::new();
    for entry in entries {
        let entry = entry.unwrap_or_else(|e| {
            panic!(
                "skill packaging gate cannot read an entry under {}: {e}",
                relative(&base)
            )
        });
        let path = entry.path();
        if path.is_dir()
            && entry
                .file_name()
                .to_string_lossy()
                .starts_with(SKILL_DIR_PREFIX)
        {
            out.push(path);
        }
    }
    out.sort();
    assert_eq!(
        out.len(),
        EXPECTED_SKILL_DIRS,
        "skill discovery found {} directories under {} matching `{SKILL_DIR_PREFIX}*`; \
         this gate audits nothing when that rule stops matching the tree",
        out.len(),
        relative(&base)
    );
    out
}

/// Every regular file under `dir`, sorted, walked fail-closed.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let entries = std::fs::read_dir(&d)
            .unwrap_or_else(|e| panic!("skill packaging gate cannot walk {}: {e}", relative(&d)));
        for entry in entries {
            let entry = entry.unwrap_or_else(|e| {
                panic!(
                    "skill packaging gate cannot read an entry under {}: {e}",
                    relative(&d)
                )
            });
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// Every file under `skills/`, across both languages.
fn all_skill_files() -> Vec<PathBuf> {
    skill_dirs().iter().flat_map(|d| files_under(d)).collect()
}

/// Every Markdown file under `skills/`.
fn all_skill_markdown() -> Vec<PathBuf> {
    all_skill_files()
        .into_iter()
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("md"))
        .collect()
}

/// The value of the frontmatter `description`, if the document declares one.
///
/// Returns everything after the FIRST colon, which is what makes rule 2
/// checkable at all: a second colon in the value is the defect, so it must
/// survive into the returned string instead of being split away.
fn description_value(doc: &str) -> Option<String> {
    let body = doc.strip_prefix("---\n")?;
    let end = body.find("\n---")?;
    body[..end]
        .lines()
        .find_map(|line| line.strip_prefix("description:"))
        .map(|value| value.trim().to_string())
}

/// One-based line numbers where a fenced code block opens.
fn fence_lines(text: &str) -> Vec<usize> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| line.starts_with("```"))
        .map(|(idx, _)| idx + 1)
        .collect()
}

/// Every `GAP-<digit>` identifier and `v<digit>.<digit>` version label in `text`.
fn perishable_labels(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    for i in 0..chars.len().saturating_sub(3) {
        if chars[i] == 'v'
            && chars[i + 1].is_ascii_digit()
            && chars[i + 2] == '.'
            && chars[i + 3].is_ascii_digit()
        {
            out.push(chars[i..=i + 3].iter().collect());
        }
    }
    for (idx, _) in text.match_indices("GAP-") {
        let tail = &text[idx + "GAP-".len()..];
        if tail.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            out.push(format!("GAP-{}", tail.chars().next().unwrap_or('?')));
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Competing products named anywhere in `text`, matched case-insensitively.
fn banned_products(text: &str) -> Vec<&'static str> {
    let lower = text.to_lowercase();
    BANNED_PRODUCTS
        .iter()
        .copied()
        .filter(|term| lower.contains(term))
        .collect()
}

/// Product environment variables invented in `text`.
fn product_env_vars(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (idx, _) in text.match_indices(PRODUCT_ENV_PREFIX) {
        let tail: String = text[idx + PRODUCT_ENV_PREFIX.len()..]
            .chars()
            .take_while(|c| c.is_ascii_uppercase() || *c == '_')
            .collect();
        if tail.starts_with(|c: char| c.is_ascii_uppercase()) {
            out.push(format!("{PRODUCT_ENV_PREFIX}{tail}"));
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Lines that open a heading of level one through three.
fn heading_count(text: &str) -> usize {
    text.lines()
        .filter(|line| {
            let hashes = line.chars().take_while(|c| *c == '#').count();
            (1..=3).contains(&hashes) && line[hashes..].starts_with(' ')
        })
        .count()
}

/// Relative paths a skill directory carries, keyed from that directory.
fn relative_paths(dir: &Path) -> BTreeSet<PathBuf> {
    files_under(dir)
        .into_iter()
        .map(|p| {
            p.strip_prefix(dir)
                .unwrap_or_else(|e| panic!("{} is not under {}: {e}", p.display(), dir.display()))
                .to_path_buf()
        })
        .collect()
}

#[test]
fn every_skill_md_stays_under_the_word_ceiling() {
    let mut over = Vec::new();
    for dir in skill_dirs() {
        let path = dir.join("SKILL.md");
        let words = read(&path).split_whitespace().count();
        if words > MAX_SKILL_WORDS {
            over.push(format!(
                "{}: {words} whitespace-separated words, ceiling is {MAX_SKILL_WORDS}, \
                 so it is {} over",
                relative(&path),
                words - MAX_SKILL_WORDS
            ));
        }
    }
    assert!(
        over.is_empty(),
        "a SKILL.md is over the word ceiling. Cut prose or move it into \
         `references/`; do NOT raise the ceiling:\n{}",
        over.join("\n")
    );
}

#[test]
fn every_skill_declares_a_colon_free_description_within_the_ceiling() {
    let mut problems = Vec::new();
    for dir in skill_dirs() {
        let path = dir.join("SKILL.md");
        let name = relative(&path);
        let Some(value) = description_value(&read(&path)) else {
            problems.push(format!(
                "{name}: the frontmatter declares no `description`, which is the \
                 field the loader matches a task against"
            ));
            continue;
        };
        let chars = value.chars().count();
        if chars > MAX_DESCRIPTION_CHARS {
            problems.push(format!(
                "{name}: `description` is {chars} characters, ceiling is \
                 {MAX_DESCRIPTION_CHARS}"
            ));
        }
        if value.contains(':') {
            problems.push(format!(
                "{name}: `description` carries a colon in its VALUE, which reopens \
                 the YAML scalar and truncates the description at that point"
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "a skill frontmatter `description` is unusable as published:\n{}",
        problems.join("\n")
    );
}

#[test]
fn no_skill_markdown_carries_a_fenced_code_block() {
    let mut problems = Vec::new();
    for path in all_skill_markdown() {
        for line in fence_lines(&read(&path)) {
            problems.push(format!(
                "{}:{line}: a fenced code block opens here; skill prose carries \
                 invocations inline, never in a fence",
                relative(&path)
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "fenced code blocks under `skills/`:\n{}",
        problems.join("\n")
    );
}

#[test]
fn no_skill_markdown_cites_a_gap_id_or_a_version_label() {
    let mut problems = Vec::new();
    for path in all_skill_markdown() {
        for label in perishable_labels(&read(&path)) {
            problems.push(format!(
                "{}: cites `{label}`, an identifier that rots the moment the \
                 repository moves past it; state the behaviour instead",
                relative(&path)
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "perishable identifiers under `skills/`:\n{}",
        problems.join("\n")
    );
}

#[test]
fn both_skill_directories_carry_the_same_relative_paths() {
    let dirs = skill_dirs();
    let (first, second) = (&dirs[0], &dirs[1]);
    let (a, b) = (relative_paths(first), relative_paths(second));
    let mut problems = Vec::new();
    for missing in a.difference(&b) {
        problems.push(format!(
            "{} has `{}` and {} does not",
            relative(first),
            missing.display(),
            relative(second)
        ));
    }
    for missing in b.difference(&a) {
        problems.push(format!(
            "{} has `{}` and {} does not",
            relative(second),
            missing.display(),
            relative(first)
        ));
    }
    assert!(
        problems.is_empty(),
        "the two language directories do not carry the same files, so one \
         language's reader is handed a smaller skill:\n{}",
        problems.join("\n")
    );
}

#[test]
fn matching_files_carry_the_same_heading_count() {
    let dirs = skill_dirs();
    let (first, second) = (&dirs[0], &dirs[1]);
    // Only paths present on BOTH sides: the missing-file case belongs to
    // `both_skill_directories_carry_the_same_relative_paths`, and reporting it
    // twice would make a single defect look like two.
    let shared: BTreeSet<PathBuf> = relative_paths(first)
        .intersection(&relative_paths(second))
        .cloned()
        .collect();
    let mut problems = Vec::new();
    for rel in shared {
        let (x, y) = (first.join(&rel), second.join(&rel));
        let (hx, hy) = (heading_count(&read(&x)), heading_count(&read(&y)));
        if hx != hy {
            problems.push(format!(
                "`{}`: {} carries {hx} headings and {} carries {hy}",
                rel.display(),
                relative(&x),
                relative(&y)
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "translated files carry different heading counts, so a section exists in \
         one language and not the other:\n{}",
        problems.join("\n")
    );
}

#[test]
fn no_skill_file_cites_a_competing_product() {
    let mut problems = Vec::new();
    for path in all_skill_files() {
        for term in banned_products(&read(&path)) {
            problems.push(format!(
                "{}: names `{term}`, a product this skill does not operate",
                relative(&path)
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "competing products named under `skills/`:\n{}",
        problems.join("\n")
    );
}

#[test]
fn no_skill_file_cites_a_product_environment_variable() {
    let mut problems = Vec::new();
    for path in all_skill_files() {
        for var in product_env_vars(&read(&path)) {
            problems.push(format!(
                "{}: cites `{var}`; this product has no environment variables, so \
                 an agent that reads this will configure nothing",
                relative(&path)
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "invented product environment variables under `skills/`:\n{}",
        problems.join("\n")
    );
}

#[test]
fn every_skill_corpus_publishes_the_six_envelope_fields() {
    let mut problems = Vec::new();
    for dir in skill_dirs() {
        // Corpus rather than SKILL.md alone: a field may legitimately be taught
        // in `references/`, and the reader loads the whole directory.
        let mut corpus = String::new();
        for path in files_under(&dir) {
            corpus.push_str(&read(&path));
            corpus.push('\n');
        }
        let missing: Vec<&str> = ENVELOPE_FIELDS
            .iter()
            .copied()
            .filter(|field| !corpus.contains(field))
            .collect();
        if !missing.is_empty() {
            problems.push(format!(
                "{}: the corpus never mentions {}",
                relative(&dir),
                missing.join(", ")
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "a skill corpus omits envelope fields, so its reader cannot tell which \
         browser mode actually ran:\n{}",
        problems.join("\n")
    );
}

/// Each detector must fire on the defect it polices, or the gate proves nothing.
///
/// Driven on synthetic text so the proof leaves no edit behind and survives a
/// reader who never mutates a skill file by hand.
#[test]
fn every_detector_fires_on_its_own_defect() {
    assert_eq!(
        description_value("---\nname: x\ndescription:  a b \n---\n# t\n").as_deref(),
        Some("a b"),
        "the description value must be read from the frontmatter"
    );
    assert!(
        description_value("---\nname: x\ndescription: use this: always\n---\n")
            .is_some_and(|v| v.contains(':')),
        "a colon inside the value must survive into the returned string"
    );
    assert_eq!(description_value("# no frontmatter\n"), None);

    assert_eq!(fence_lines("a\n```rust\nx\n```\n"), vec![2, 4]);
    assert!(fence_lines("prose with ``inline`` ticks\n").is_empty());

    assert_eq!(
        perishable_labels("see GAP-7 and v0.1 here"),
        ["GAP-7", "v0.1"]
    );
    assert!(perishable_labels("version four, gap seven").is_empty());

    assert_eq!(banned_products("Use FireCrawl instead"), ["firecrawl"]);
    assert!(banned_products("use the local browser").is_empty());

    assert_eq!(
        product_env_vars("export BROWSER_AUTOMATION_TIMEOUT=1"),
        ["BROWSER_AUTOMATION_TIMEOUT"]
    );
    assert!(product_env_vars("BROWSER_AUTOMATION_ is not a variable").is_empty());

    assert_eq!(heading_count("# a\n## b\n### c\nnot # a heading\n"), 3);
    assert_eq!(
        heading_count("#### too deep\n#nospace\n"),
        0,
        "only levels one through three, and only with a space after the hashes"
    );
}

/// The corpus the gate audits must be the one on disk, not an empty walk.
///
/// Guards the failure mode where a rename under `skills/` silently reduces every
/// test above to a loop over nothing, which reports green while auditing zero
/// bytes.
#[test]
fn the_walk_finds_the_published_skill_corpus() {
    let files = all_skill_files();
    assert!(
        files.len() >= 6,
        "the walk found only {} files under {}; every assertion in this gate is \
         vacuous when that number collapses",
        files.len(),
        relative(&skills_root())
    );
    let by_dir: BTreeMap<String, usize> =
        skill_dirs().iter().fold(BTreeMap::new(), |mut acc, d| {
            acc.insert(relative(d), files_under(d).len());
            acc
        });
    for (dir, count) in &by_dir {
        assert!(
            *count >= 3,
            "{dir} carries only {count} files; a skill that lost its references \
             is a defect, not a smaller audit"
        );
    }
    assert!(
        all_skill_markdown().len() >= 6,
        "the Markdown filter matched too few files to be auditing the skills"
    );
}
