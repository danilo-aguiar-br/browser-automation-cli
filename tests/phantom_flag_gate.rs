// SPDX-License-Identifier: MIT OR Apache-2.0
//! Fail when a user-facing suggestion names a flag the product does not declare.
//!
//! # Why this exists
//!
//! The block-detection envelope told the agent to try "a different egress via
//! `--proxy`" while no such flag existed anywhere in the product. An agent that
//! obeyed the remediation earned `exit 2` and a usage error, so the one field
//! meant to unblock it was the field that trapped it.
//!
//! A gate for exactly that class already existed — `scripts/agent-ops-check.sh`
//! checks that `agent-ops-*` suggestions cite only global flags. It missed this
//! one twice over. It reads `locales/*.ftl` and nothing else, so a `format!` in
//! Rust was invisible to it; and it filters to keys prefixed `agent-ops-`, so
//! every other suggestion key was out of scope even inside the file it does read.
//!
//! Worse, a unit test asserted the string contained `--engine browser` OR
//! `--proxy`. The phantom satisfied the assertion, so the suite ratified it.
//!
//! # Why this is a Rust test and not a script
//!
//! It was a 411-line Python script until 2026-08-10. The product rule is that
//! the tool is Rust end to end, and the script was not a detail: `ci-check.sh`
//! reaches it through `agent-ops-check.sh`, so the product's own closure
//! criterion failed on any host without `python3`. Porting it here also buys the
//! correct binary path — `CARGO_BIN_EXE_` is resolved by cargo at compile time,
//! where the script had to guess between `target/debug` and `target/release` and
//! could measure a stale binary from an earlier version.
//!
//! # What this checks
//!
//! Three properties, all anchored on the LIVE binary or the LIVE source rather
//! than on a list kept by hand:
//!
//! 1. Every `--flag` named in `locales/*.ftl` or in the Rust message catalogs
//!    must be declared somewhere in the product. The universe is built by
//!    walking `--help` recursively.
//! 2. No file outside `src/i18n/` may pass an inline literal containing `--` as
//!    the suggestion argument of `with_suggestion`. That removes the escape
//!    hatch that let a suggestion be built by `format!` in the first place.
//! 3. Every `pub fn name()` published by `src/browser_policy.rs` must have at
//!    least one production call site. `--headed` was declared, resolved into a
//!    process global, and read at zero call sites for three releases.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_browser-automation-cli");

/// Files whose literals are user-facing advice. Everything else in `src/` is
/// allowed to spell foreign argv — Chrome switches, `redis-server` options, the
/// `lighthouse` CLI — and scanning those would drown the signal.
const CATALOG_FILES: &[&str] = &["src/i18n/en.rs", "src/i18n/pt_br.rs"];

/// Where suggestion text is allowed to live at all.
const SUGGESTION_HOME: &str = "src/i18n/";

/// Where the resolved policy is published to the rest of the process.
const POLICY_FILE: &str = "src/browser_policy.rs";

/// A near-empty universe means the walk failed, and every citation would then
/// look phantom. Refuse rather than emit a wall of false alarms.
///
/// The floor is 200 rather than the 20 the Python original used. Twenty is low
/// enough to be satisfied by a walk that collects the global options and then
/// fails on every subcommand, which is precisely the silent under-collection
/// that would let a phantom flag through. Measured 2026-08-10: 242 declared
/// flags. A legitimate drop below 200 should fail loudly and be re-pinned by
/// hand, not absorbed.
const MIN_UNIVERSE: usize = 200;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Capture stdout plus stderr of a help invocation; clap may use either.
fn run_help(args: &[&str]) -> String {
    let out = Command::new(BIN).args(args).arg("--help").output();
    match out {
        Ok(o) => format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        ),
        Err(_) => String::new(),
    }
}

/// True when `s` at `i` starts a long flag of at least `min_tail` trailing chars.
fn flag_at(bytes: &[u8], i: usize, min_tail: usize) -> Option<usize> {
    if i + 2 >= bytes.len() || bytes[i] != b'-' || bytes[i + 1] != b'-' {
        return None;
    }
    if !bytes[i + 2].is_ascii_lowercase() {
        return None;
    }
    let mut end = i + 3;
    while end < bytes.len()
        && (bytes[end].is_ascii_lowercase() || bytes[end].is_ascii_digit() || bytes[end] == b'-')
    {
        end += 1;
    }
    // `--ab` has a one-char tail after the leading letter; the citation rule asks
    // for three or more so that `--` in prose and `---` rules are not flags.
    if end - (i + 3) >= min_tail {
        Some(end)
    } else {
        None
    }
}

/// Every long flag clap prints for this invocation, from an `Options:` block.
///
/// Mirrors `^\s+(?:-\w,\s+)?(--[a-z][a-z0-9-]*)`: an indented line whose first
/// token is a long flag, optionally preceded by its short alias.
fn flags_in(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.len() == line.len() {
            continue; // clap indents option lines; a flush-left line is a header
        }
        let candidate = if trimmed.starts_with('-')
            && !trimmed.starts_with("--")
            && trimmed.len() > 3
            && trimmed.as_bytes()[2] == b','
        {
            trimmed[3..].trim_start()
        } else {
            trimmed
        };
        if let Some(end) = flag_at(candidate.as_bytes(), 0, 0) {
            out.insert(candidate[..end].to_string());
        }
    }
    out
}

/// Names listed under the `Commands:` block, if there is one.
fn subcommands_in(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut in_block = false;
    for line in text.lines() {
        if line.starts_with("Commands:") {
            in_block = true;
            continue;
        }
        if !in_block {
            continue;
        }
        if !line.is_empty() && !line.starts_with(' ') {
            break;
        }
        // `^\s{2}([a-z][a-z0-9-]*)\s{2,}\S`
        let Some(rest) = line.strip_prefix("  ") else {
            continue;
        };
        if rest.starts_with(' ') {
            continue;
        }
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
            .collect();
        if name.is_empty() || name == "help" {
            continue;
        }
        let after = &rest[name.len()..];
        if after.starts_with("  ") && after.trim_start().chars().next().is_some() {
            out.insert(name);
        }
    }
    out
}

/// Every long flag the product declares, at any depth.
///
/// Walked rather than listed. A hand-kept list ages silently and then blames the
/// artifact for the instrument's drift.
fn flag_universe() -> BTreeSet<String> {
    let top = run_help(&[]);
    let mut universe = flags_in(&top);
    for cmd in subcommands_in(&top) {
        let cmd_help = run_help(&[&cmd]);
        universe.extend(flags_in(&cmd_help));
        for sub in subcommands_in(&cmd_help) {
            universe.extend(flags_in(&run_help(&[&cmd, &sub])));
        }
    }
    universe
}

/// Flags a body of prose names, requiring at least three trailing characters.
fn cited_flags(line: &str) -> BTreeSet<String> {
    let bytes = line.as_bytes();
    let mut out = BTreeSet::new();
    let mut i = 0;
    while i < bytes.len() {
        if let Some(end) = flag_at(bytes, i, 2) {
            out.insert(line[i..end].to_string());
            i = end;
        } else {
            i += 1;
        }
    }
    out
}

fn rust_files(dir: &Path, acc: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut items: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    items.sort();
    for path in items {
        if path.is_dir() {
            rust_files(&path, acc);
        } else if path.extension().is_some_and(|e| e == "rs") {
            acc.push(path);
        }
    }
}

/// Property 1: every cited flag must be a flag the product declares.
fn check_citations(universe: &BTreeSet<String>) -> Vec<String> {
    let root = root();
    let mut problems = Vec::new();
    let mut sources = vec![root.join("locales/en.ftl"), root.join("locales/pt-BR.ftl")];
    sources.extend(CATALOG_FILES.iter().map(|f| root.join(f)));

    for path in sources {
        let Ok(text) = std::fs::read_to_string(&path) else {
            problems.push(format!("missing source file: {}", path.display()));
            continue;
        };
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .display()
            .to_string();
        for (idx, line) in text.lines().enumerate() {
            let stripped = line.trim_start();
            // Rust doc comments describe the code, not the agent.
            if stripped.starts_with("///") || stripped.starts_with("//!") {
                continue;
            }
            for flag in cited_flags(line) {
                if !universe.contains(&flag) {
                    problems.push(format!(
                        "{rel}:{} cites {flag}, which the product does not declare",
                        idx + 1
                    ));
                }
            }
        }
    }
    problems
}

/// Line range of the `with_suggestion(...)` call opening at `start`.
///
/// Balanced on parentheses rather than assumed to be one line. A version that
/// only read the line carrying `with_suggestion` went quiet the moment rustfmt
/// moved the literal onto a continuation line, and a gate that only detects the
/// instance already fixed is not a gate.
fn call_span(lines: &[&str], start: usize) -> std::ops::Range<usize> {
    let mut depth: i32 = 0;
    for (offset, line) in lines.iter().enumerate().skip(start) {
        depth += line.matches('(').count() as i32 - line.matches(')').count() as i32;
        if depth <= 0 && offset > start {
            return start..offset + 1;
        }
        if depth <= 0 && offset == start && line.contains('(') {
            return start..start + 1;
        }
    }
    start..(start + 12).min(lines.len())
}

/// The final argument of `with_suggestion(kind, message, suggestion)`.
///
/// Only the LAST argument is under this rule. The middle one is `error.message`,
/// which the product keeps in English on purpose: it is the machine half of the
/// envelope and agents match on it. Testing the whole call reported three false
/// positives on messages that legitimately name `--timeout`, `--engine` and
/// `--ignore-robots`.
fn last_argument(call_text: &str) -> String {
    let Some(open) = call_text.find('(') else {
        return String::new();
    };
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    let mut args: Vec<String> = Vec::new();
    let mut current = String::new();

    for ch in call_text[open + 1..].chars() {
        if in_string {
            current.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
            current.push(ch);
            continue;
        }
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            _ => {}
        }
        if ch == ',' && depth == 0 {
            args.push(std::mem::take(&mut current));
            continue;
        }
        current.push(ch);
    }
    args.push(current);
    args.into_iter()
        .rev()
        .find(|a| !a.trim().is_empty())
        .unwrap_or_default()
}

/// True when the text holds a string literal that names a long flag.
///
/// Mirrors `"[^"]*--[a-z][a-z0-9-]{2,}[^"]*"`.
fn has_flag_literal(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'"' {
            i += 1;
            continue;
        }
        let start = i + 1;
        let mut end = start;
        while end < bytes.len() && bytes[end] != b'"' {
            end += 1;
        }
        if end >= bytes.len() {
            return false;
        }
        let inner = &text[start..end];
        let inner_bytes = inner.as_bytes();
        for j in 0..inner_bytes.len() {
            if flag_at(inner_bytes, j, 2).is_some() {
                return true;
            }
        }
        i = end + 1;
    }
    false
}

/// Property 2: suggestion prose may not be assembled outside the catalog.
fn check_no_inline_suggestions() -> Vec<String> {
    let root = root();
    let mut problems = Vec::new();
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);

    for path in files {
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .display()
            .to_string();
        if rel.starts_with(SUGGESTION_HOME) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        if !text.contains("with_suggestion") {
            continue;
        }
        let lines: Vec<&str> = text.lines().collect();
        for idx in 0..lines.len() {
            if !lines[idx].contains("with_suggestion") {
                continue;
            }
            let stripped = lines[idx].trim_start();
            if stripped.starts_with("///") || stripped.starts_with("//!") {
                continue;
            }
            let span = call_span(&lines, idx);
            let call_text: String = lines[span]
                .iter()
                .filter(|b| !b.trim_start().starts_with("//"))
                .copied()
                .collect::<Vec<_>>()
                .join("\n");
            let Some(at) = call_text.find("with_suggestion") else {
                continue;
            };
            if has_flag_literal(&last_argument(&call_text[at..])) {
                problems.push(format!(
                    "{rel}:{} builds suggestion prose naming a flag outside {SUGGESTION_HOME}",
                    idx + 1
                ));
            }
        }
    }
    problems
}

/// Property 3: fail when a published policy value is read by nobody.
///
/// # Why the other two properties cannot catch it
///
/// Property 1 proves a cited flag EXISTS among the declared flags. Property 2
/// proves advice is built inside the catalog. Both are properties of the
/// CATALOG. This one is a property of the CALL GRAPH, and they are independent:
/// `--proxy` passed property 1 while still having no effect in the engine whose
/// own error message recommends it.
///
/// A caller is a mention in `src/` outside the declaring module, outside a
/// `tests` module, and outside a comment. Test-only callers are excluded on
/// purpose: a getter whose sole reader is its own unit test is exactly the shape
/// `chrome_header_order` had.
fn check_policy_getters_have_callers() -> Vec<String> {
    let root = root();
    let policy_path = root.join(POLICY_FILE);
    let Ok(policy_text) = std::fs::read_to_string(&policy_path) else {
        return vec![format!(
            "{POLICY_FILE} not found; cannot audit policy getters"
        )];
    };

    // `^pub fn ([a-z_][a-z0-9_]*)\(\)` at line start, taking only zero-arg fns.
    let getters: BTreeSet<String> = policy_text
        .lines()
        .filter_map(|line| line.strip_prefix("pub fn "))
        .filter_map(|rest| {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_')
                .collect();
            rest[name.len()..].starts_with("()").then_some(name)
        })
        // Setters are called by the resolver in the same file by construction.
        .filter(|n| !n.starts_with("set_") && n != "publish" && !n.is_empty())
        .collect();

    if getters.is_empty() {
        return vec![format!(
            "{POLICY_FILE} declares no policy getters; the scan would pass vacuously"
        )];
    }

    let mut counts: std::collections::BTreeMap<String, usize> =
        getters.iter().map(|g| (g.clone(), 0)).collect();

    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    for path in files {
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .display()
            .to_string();
        if rel == POLICY_FILE || rel.ends_with("tests.rs") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let mut in_tests = false;
        let mut brace_depth = 0i32;
        for line in text.lines() {
            let stripped = line.trim_start();
            if stripped.starts_with("//") {
                continue;
            }
            if line.contains("mod tests") && !line.contains("#[cfg(test)]") {
                in_tests = true;
                brace_depth = 0;
            }
            if in_tests {
                brace_depth += line.matches('{').count() as i32 - line.matches('}').count() as i32;
                if brace_depth <= 0 && line.contains('}') {
                    in_tests = false;
                }
                continue;
            }
            for getter in &getters {
                if line.contains(&format!("{getter}()")) {
                    *counts.get_mut(getter).expect("getter seeded above") += 1;
                }
            }
        }
    }

    counts
        .into_iter()
        .filter(|(_, c)| *c == 0)
        .map(|(g, _)| {
            format!(
                "{POLICY_FILE}: `{g}()` is published and read by ZERO production \
                 call sites — the flag it carries changes nothing"
            )
        })
        .collect()
}

#[test]
fn every_cited_flag_exists_among_the_declared_flags() {
    let universe = flag_universe();
    assert!(
        universe.len() >= MIN_UNIVERSE,
        "flag universe has only {} entries, so the help walk failed; every citation \
         would look phantom and the report would be noise",
        universe.len()
    );
    let problems = check_citations(&universe);
    assert!(
        problems.is_empty(),
        "suggestion prose cites flags the product does not declare:\n{}",
        problems.join("\n")
    );
}

#[test]
fn no_suggestion_prose_is_assembled_outside_the_catalog() {
    let problems = check_no_inline_suggestions();
    assert!(
        problems.is_empty(),
        "suggestion prose built outside {SUGGESTION_HOME} is untranslatable by \
         construction:\n{}",
        problems.join("\n")
    );
}

#[test]
fn every_published_policy_value_has_a_production_call_site() {
    let problems = check_policy_getters_have_callers();
    assert!(
        problems.is_empty(),
        "a published policy value nobody reads is a flag that changes nothing:\n{}",
        problems.join("\n")
    );
}

/// Where the global flag struct is declared.
const GLOBALS_FILE: &str = "src/cli/global.rs";

/// Report every `--flag` declared as a global that no production code reads.
///
/// # Why the existing scans did not catch this family
///
/// `every_published_policy_value_has_a_production_call_site` audits
/// `browser_policy` getters, so it only sees flags that were routed through
/// that module. `--mitm-max-body-bytes`, `--mitm-no-media-bodies` and
/// `--mitm-redact-secrets` never were: they sat in the struct, appeared in
/// `--help`, and no line outside `src/cli/` ever named them. The help text
/// promised a body ceiling, a media filter and a redaction switch, and the
/// capture applied none of the three.
///
/// A field is "read" when its name appears outside `src/cli/`. That is
/// deliberately loose — one honest mention is enough to prove the wiring
/// exists — because the failure this catches is total absence, not partial use.
fn check_global_flags_have_consumers() -> Vec<String> {
    let root = root();
    let Ok(globals_text) = std::fs::read_to_string(root.join(GLOBALS_FILE)) else {
        return vec![format!(
            "{GLOBALS_FILE} not found; cannot audit global flags"
        )];
    };

    // `pub name: Type,` inside the globals struct. Sub-structs that group flags
    // (`agent_ops`) are skipped: their own fields are audited by the module
    // that owns them, and the group name is read by the parser itself.
    let fields: BTreeSet<String> = globals_text
        .lines()
        .map(str::trim)
        .filter_map(|line| line.strip_prefix("pub "))
        .filter_map(|rest| rest.split_once(": ").map(|(n, _)| n))
        .filter(|n: &&str| !n.is_empty() && n.chars().all(|c| c.is_ascii_lowercase() || c == '_'))
        .map(str::to_string)
        .filter(|n| n != "agent_ops")
        .collect();

    if fields.len() < 20 {
        return vec![format!(
            "{GLOBALS_FILE}: only {} global fields parsed; the scan would pass vacuously",
            fields.len()
        )];
    }

    let mut counts: std::collections::BTreeMap<String, usize> =
        fields.iter().map(|f| (f.clone(), 0)).collect();

    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    for path in files {
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .display()
            .to_string();
        // `src/cli/` declares the flags; a mention there proves nothing.
        if rel.starts_with("src/cli/") || rel.ends_with("tests.rs") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for line in text.lines() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            for field in &fields {
                if line.contains(field.as_str()) {
                    *counts.get_mut(field).expect("field seeded above") += 1;
                }
            }
        }
    }

    counts
        .into_iter()
        .filter(|(_, c)| *c == 0)
        .map(|(f, _)| {
            format!(
                "{GLOBALS_FILE}: `--{}` is declared, documented in --help, and read by \
                 ZERO production call sites outside src/cli/",
                f.replace('_', "-")
            )
        })
        .collect()
}

#[test]
fn every_global_flag_has_a_production_consumer() {
    let problems = check_global_flags_have_consumers();
    assert!(
        problems.is_empty(),
        "a flag that parses and changes nothing is worse than a missing flag: \
         the caller reads --help, believes the promise, and gets the old behaviour \
         with exit 0:\n{}",
        problems.join("\n")
    );
}
