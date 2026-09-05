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
//! 3. Every top-level `pub fn` published by a module that also holds
//!    process-wide state must have at least one production call site. `--headed`
//!    was declared, resolved into a process global, and read at zero call sites
//!    for three releases. The set of such modules is DISCOVERED from the source,
//!    not listed: a hand-kept list of three files was the reason
//!    `src/mitm_local/policy.rs` could publish a getter nobody read.
//!
//!    Arity was a declared limitation of that property until 2026-09-04: it read
//!    only `pub fn name()`, so a published `pub fn name(arg)` that nobody calls
//!    was invisible, and hiding a dead getter behind one parameter was enough to
//!    walk out of the gate. Inherent methods inside `impl` blocks stay out of
//!    scope on purpose — they are reached through a receiver whose type this
//!    line-based scan cannot resolve, and guessing at it would either report the
//!    whole crate or ratify it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

mod common;
use common::root;

/// Files whose literals are user-facing advice. Everything else in `src/` is
/// allowed to spell foreign argv — Chrome switches, `redis-server` options, the
/// `lighthouse` CLI — and scanning those would drown the signal.
const CATALOG_FILES: &[&str] = &["src/i18n/en.rs", "src/i18n/pt_br.rs"];

/// Where suggestion text is allowed to live at all.
const SUGGESTION_HOME: &str = "src/i18n/";

/// The types whose `static` declaration marks a module as publishing state to
/// the whole process rather than to its own caller.
///
/// A `static X: AtomicBool` or a `static X: OnceLock<T>` is resolved once and
/// then read from anywhere, which is precisely the shape that can be published
/// and never read. A plain `const` is not here: it is inlined at every use and
/// cannot drift away from its readers.
const PROCESS_STATE_TYPES: &[&str] = &[
    "AtomicBool",
    "AtomicUsize",
    "AtomicU32",
    "AtomicU64",
    "AtomicI64",
    "OnceLock",
    "LazyLock",
    "Lazy",
];

/// Path prefixes that may spell those types without owning readable state.
///
/// Written out so that `std::sync::OnceLock<T>` and `once_cell::sync::Lazy<T>`
/// are recognised as the same declaration as their bare form.
const STATE_TYPE_PREFIXES: &[&str] = &[
    "std::sync::atomic::",
    "std::sync::",
    "core::sync::atomic::",
    "once_cell::sync::",
    "once_cell::",
];

/// A near-empty published-name set means the walk failed.
///
/// Re-pinned 2026-09-01. The old floor of 8 was calibrated against a hand-kept
/// list of three files under `src/browser_policy/`, so it could not have
/// detected the structural miss it was supposed to guard: the discovery found
/// 17 modules publishing 56 zero-argument getters, and a floor of 8 would have
/// been satisfied by a walk that read one file and gave up on the rest.
///
/// The floor stays at 40 after the 2026-09-04 arity widening. Dropping the
/// zero-argument restriction can only ADD names, so a floor calibrated on the
/// narrower scan still fails loudly on a broken walk and never on a healthy one.
const MIN_POLICY_GETTERS: usize = 40;

/// Likewise for the module set itself. Measured 2026-09-01: 17 modules.
///
/// The getter floor alone does not prove the DISCOVERY worked — one very large
/// module could carry it. This pins the fan-out separately.
const MIN_POLICY_MODULES: usize = 14;

/// Where the XDG policy knobs are declared by the `policy_knobs!` macro.
///
/// The macro generates `POLICY_KEYS`, `policy_set` and `policy_list_entries`
/// from this table, so a row here is enough to make `config set <key>` return
/// `ok` and `config list-keys` advertise the key. Reaching the RUNTIME is a
/// separate act: some call site has to ask for `key::NAME`. Nothing tied the
/// two together until property 4 below.
const KNOBS_TABLE_FILE: &str = "src/xdg/policy/knobs/table.rs";

/// A near-empty knob table means the parse failed, and every knob would then
/// look phantom. Measured 2026-08-17: 107 rows after the 0.1.9 cleanup.
const MIN_KNOBS: usize = 90;

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

/// Capture stdout plus stderr of a help invocation; clap may use either.
fn run_help(args: &[&str]) -> String {
    let out = common::cmd().args(args).arg("--help").output();
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

/// Drop every `#[cfg(test)]` region from `text`, returning production lines only.
///
/// A gate that promises a PRODUCTION call site has to measure production. This
/// scanner used to read all of `src/`, so a `key::NAME` written inside
/// `mod tests` satisfied it — and that is exactly how `shutdown_poll_ms`
/// survived a property built to catch it. The gate ratified the phantom it was
/// meant to reject, and the debt could be paid by writing a test instead of
/// wiring the knob.
///
/// Brace counting starts at the first `{` after the attribute, so it handles
/// both `#[cfg(test)] mod tests { … }` and a single `#[cfg(test)] fn helper()`.
fn strip_test_regions(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        let is_test_attr = trimmed.starts_with("#[cfg(test)]")
            || (trimmed.starts_with("#[cfg(all(test") || trimmed.starts_with("#[cfg(any(test"));
        if !is_test_attr {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        // Consume the attributed item: count braces from the first `{` onward.
        let mut depth = 0usize;
        let mut opened = false;
        for body in lines.by_ref() {
            for ch in body.chars() {
                match ch {
                    '{' => {
                        depth += 1;
                        opened = true;
                    }
                    '}' => depth = depth.saturating_sub(1),
                    _ => {}
                }
            }
            // A `#[cfg(test)] use …;` has no braces at all: stop at its `;`.
            if !opened && body.trim_end().ends_with(';') {
                break;
            }
            if opened && depth == 0 {
                break;
            }
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
/// True when `line` DECLARES a `static` of one of `PROCESS_STATE_TYPES`.
///
/// The declaration is matched, never the mention: a doc comment that names
/// `AtomicBool` and a `let flag: AtomicBool` inside a function both fail here,
/// because only `static NAME: TYPE` publishes a value to the whole process. A
/// `const` is deliberately out: it is inlined at every use and cannot drift
/// away from its readers.
fn declares_process_state(line: &str) -> bool {
    let mut head = line.trim_start();
    for vis in ["pub(crate) ", "pub(super) ", "pub "] {
        if let Some(tail) = head.strip_prefix(vis) {
            head = tail.trim_start();
        }
    }
    let Some(rest) = head.strip_prefix("static ") else {
        return false;
    };
    let Some((_, ty)) = rest.split_once(':') else {
        return false;
    };
    let mut ty = ty.trim_start();
    // `std::sync::OnceLock<T>` is the same declaration as a bare `OnceLock<T>`.
    for prefix in STATE_TYPE_PREFIXES {
        if let Some(tail) = ty.strip_prefix(prefix) {
            ty = tail;
        }
    }
    PROCESS_STATE_TYPES.iter().any(|t| {
        ty.strip_prefix(t)
            .is_some_and(|tail| tail.is_empty() || tail.starts_with(['<', ' ', '=']))
    })
}

/// `pub fn` names published at the TOP LEVEL of `text`, at ANY arity.
///
/// Mirrors `^pub (?:const |async |unsafe )*fn ([a-z_][a-z0-9_]*)(?:<…>)?\(`,
/// flush left, so an inherent method inside an `impl` block is out of scope by
/// indentation alone — see the module header for why methods are excluded.
///
/// # Why the arity restriction was removed
///
/// This read `\(\)` until 2026-09-04, and the restriction was load-bearing in
/// the wrong direction: `auto_headed()` was caught because it happened to take
/// nothing, while the identical defect one parameter wider was invisible. The
/// property being tested is "published and read by nobody", and nothing in that
/// sentence mentions how many arguments the thing takes.
///
/// Setters are dropped: the resolver calls them by construction, so a setter
/// without an outside caller proves nothing about a dead flag.
fn top_level_pub_fns(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|line| {
            let mut rest = line.strip_prefix("pub ")?;
            for modifier in ["const ", "async ", "unsafe "] {
                if let Some(tail) = rest.strip_prefix(modifier) {
                    rest = tail;
                }
            }
            let rest = rest.strip_prefix("fn ")?;
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_')
                .collect();
            if name.is_empty() {
                return None;
            }
            // `name(` or `name<T>(`; anything else is not a function header.
            let tail = &rest[name.len()..];
            let opens =
                tail.starts_with('(') || tail.strip_prefix('<').is_some_and(|t| t.contains(">("));
            opens.then_some(name)
        })
        .filter(|n| !n.starts_with("set_") && n != "publish")
        .collect()
}

/// Every module under `src/` that holds process-wide state AND publishes a
/// top-level `pub fn`, paired with the names it publishes.
///
/// # Why this is discovered and not listed
///
/// The list this replaced held three files under `src/browser_policy/`, which
/// is the module that happened to be audited when the property was written.
/// Measured 2026-09-01: 17 modules match the structural criterion, and one of
/// the 14 the list never mentioned — `src/mitm_local/policy.rs` — published
/// `redact_secrets()` to zero readers. The list was not incomplete by accident;
/// a list is incomplete by construction the moment a module is added.
fn policy_modules() -> Vec<(String, BTreeSet<String>)> {
    let root = root();
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    let mut out = Vec::new();
    for path in files {
        // A dedicated `tests.rs` is test code in full; state declared there is
        // not process state the product publishes.
        if path.file_name().is_some_and(|n| n == "tests.rs") {
            continue;
        }
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        let text = strip_test_regions(&raw);
        if !text.lines().any(declares_process_state) {
            continue;
        }
        let getters = top_level_pub_fns(&text);
        if getters.is_empty() {
            continue;
        }
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .display()
            .to_string();
        out.push((rel, getters));
    }
    out
}

/// True when `line` READS `name`: calls it, path-qualifies it, or hands it over
/// as a value.
///
/// # Why the needle is the bare identifier and not `name(`
///
/// The old needle was `name(`, which was sound while property 3 only looked at
/// zero-argument getters. Widening to any arity made it wrong in both
/// directions at once. `crate::policy::allow(x)`, `super::allow(x)` and a `use`
/// plus a bare `allow(x)` all still end in `allow(`, so paths were never the
/// problem; but `iter.map(allow)` and `fut.then(allow)` hand the function over
/// WITHOUT calling it, and that is a reader. A needle demanding the parenthesis
/// would have reported every such function as dead — twenty false accusations
/// are indistinguishable from no gate at all.
///
/// Both boundaries are checked. `line.contains("redact_secrets")` also matches
/// `mitm_redact_secrets`, so a dead name could borrow a live one's call site.
/// A trailing `:` is excluded twice over: `name:` is a struct field or a type
/// ascription, and `name::x` is a module path — neither reaches the function.
fn references(line: &str, name: &str) -> bool {
    let bytes = line.as_bytes();
    line.match_indices(name).any(|(idx, _)| {
        let left_ok = idx == 0 || {
            let prev = bytes[idx - 1];
            !(prev.is_ascii_alphanumeric() || prev == b'_')
        };
        if !left_ok {
            return false;
        }
        !matches!(bytes.get(idx + name.len()),
            Some(c) if c.is_ascii_alphanumeric() || *c == b'_' || *c == b':')
    })
}

/// Drop every `use` item from `text`.
///
/// A re-export is not a reader: `pub use policy::allow;` republishes the name
/// without ever asking for its value, and a plain `use` only brings it into
/// scope — the CALL that follows is the reader, and it is counted on its own
/// line. Multi-line `use crate::x::{a, b};` is consumed through to its `;`,
/// because a bare `a,` on a continuation line is indistinguishable by shape from
/// a function handed to `map`.
fn strip_use_items(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        let head = line.trim_start();
        let is_use = ["use ", "pub use ", "pub(crate) use ", "pub(super) use "]
            .iter()
            .any(|p| head.starts_with(p));
        if !is_use {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if line.trim_end().ends_with(';') {
            continue;
        }
        for body in lines.by_ref() {
            if body.trim_end().ends_with(';') {
                break;
            }
        }
    }
    out
}

/// Property 3 as a PURE function over text already in memory.
///
/// # Why it is split out from the disk walk
///
/// A gate that has never failed is a gate nobody has tested, and the only
/// honest way to make this one fail on demand is to hand it a module that does
/// not exist on disk. `the_detector_*` tests below feed it synthetic sources;
/// `check_policy_getters_have_callers` feeds it `src/`. Same function, so a
/// detector that stops biting breaks the synthetic tests first, loudly, instead
/// of passing the real scan vacuously and silently.
fn find_unread_published_fns(
    modules: &[(String, BTreeSet<String>)],
    sources: &[(String, String)],
) -> Vec<String> {
    // A name can legitimately be declared by more than one module (`enabled`,
    // `active`), so ownership is a SET. A caller only proves the name is read
    // when it sits outside every module that declares that name.
    let mut owners: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (rel, published) in modules {
        for name in published {
            owners.entry(name.clone()).or_default().insert(rel.clone());
        }
    }

    let mut counts: BTreeMap<&str, usize> = owners.keys().map(|n| (n.as_str(), 0)).collect();
    for (rel, text) in sources {
        let readable = strip_use_items(text);
        for line in readable.lines() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            for (name, homes) in &owners {
                if homes.contains(rel) || !references(line, name) {
                    continue;
                }
                *counts.get_mut(name.as_str()).expect("name seeded above") += 1;
            }
        }
    }

    counts
        .into_iter()
        .filter(|(_, c)| *c == 0)
        .map(|(name, _)| {
            let home: Vec<&str> = owners[name].iter().map(String::as_str).collect();
            format!(
                "{}: `{name}` is published and read by ZERO production call sites \
                 — the value it carries changes nothing",
                home.join(", ")
            )
        })
        .collect()
}

/// Every production `.rs` file under `src/`, test regions already removed.
///
/// A dedicated `tests.rs` is test code in full, so it is dropped whole: a name
/// whose only reader is its own unit test still has no production reader, and
/// counting that reader is exactly how `chrome_header_order` survived.
fn production_sources() -> Vec<(String, String)> {
    let root = root();
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    files
        .into_iter()
        .filter(|path| path.file_name().is_none_or(|n| n != "tests.rs"))
        .filter_map(|path| {
            let rel = path
                .strip_prefix(&root)
                .unwrap_or(&path)
                .display()
                .to_string();
            let raw = std::fs::read_to_string(&path).ok()?;
            Some((rel, strip_test_regions(&raw)))
        })
        .collect()
}

fn check_policy_getters_have_callers() -> Vec<String> {
    let modules = policy_modules();
    if modules.len() < MIN_POLICY_MODULES {
        return vec![format!(
            "discovered only {} modules publishing process state (expected at least \
             {MIN_POLICY_MODULES}); the walk failed and the scan would pass vacuously",
            modules.len()
        )];
    }

    let published: BTreeSet<&String> = modules.iter().flat_map(|(_, names)| names).collect();
    if published.len() < MIN_POLICY_GETTERS {
        return vec![format!(
            "policy modules publish only {} names (expected at least {MIN_POLICY_GETTERS}); \
             the scan would pass vacuously",
            published.len()
        )];
    }

    find_unread_published_fns(&modules, &production_sources())
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

/// The module text both detector tests audit: one published function that takes
/// an argument, which the pre-2026-09-04 scan could not see at all.
const SYNTHETIC_MODULE: &str = "\
static FLAG: AtomicBool = AtomicBool::new(false);
pub fn com_argumentos(x: u8) -> u8 {
    x
}
";

fn synthetic_modules() -> Vec<(String, BTreeSet<String>)> {
    let published = top_level_pub_fns(SYNTHETIC_MODULE);
    assert!(
        published.contains("com_argumentos"),
        "extraction missed a one-argument `pub fn`, so the widening never happened: {published:?}"
    );
    vec![("src/synthetic/policy.rs".to_string(), published)]
}

/// The gate must BITE. Re-export and import are deliberately present: neither is
/// a reader, and a detector that counted them would pass this test while letting
/// the real defect through.
#[test]
fn the_detector_reports_a_published_fn_with_arguments_that_nobody_calls() {
    let sources = vec![
        (
            "src/synthetic/policy.rs".to_string(),
            SYNTHETIC_MODULE.to_string(),
        ),
        (
            "src/synthetic/reexport.rs".to_string(),
            "pub use crate::synthetic::policy::com_argumentos;\n\
             use crate::synthetic::policy::{\n    com_argumentos,\n};\n\
             // com_argumentos(1) inside a comment is not a call site\n"
                .to_string(),
        ),
    ];
    let problems = find_unread_published_fns(&synthetic_modules(), &sources);
    assert_eq!(
        problems.len(),
        1,
        "the detector must accuse a published one-argument fn whose only mentions \
         are a re-export, an import and a comment; got {problems:?}"
    );
    assert!(problems[0].contains("com_argumentos"), "{problems:?}");
}

/// The gate must NOT bite a function that is used. Both shapes of use are here
/// because widening past zero arguments made them diverge: one calls the
/// function, the other only hands it over.
#[test]
fn the_detector_stays_silent_when_a_production_call_site_exists() {
    for reader in [
        "fn caller() {\n    let _ = crate::synthetic::policy::com_argumentos(1);\n}\n",
        "fn caller() {\n    let _ = [1u8].iter().copied().map(com_argumentos);\n}\n",
    ] {
        let sources = vec![
            (
                "src/synthetic/policy.rs".to_string(),
                SYNTHETIC_MODULE.to_string(),
            ),
            ("src/synthetic/reader.rs".to_string(), reader.to_string()),
        ];
        let problems = find_unread_published_fns(&synthetic_modules(), &sources);
        assert!(
            problems.is_empty(),
            "a real reader was reported as absent, which is the false positive that \
             makes a gate get deleted:\n{reader}\n{problems:?}"
        );
    }
}

/// Every knob name declared by `policy_knobs!`, read out of the macro table.
///
/// A row looks like `NAME => xdg_key_name,` followed by its description line.
/// Only the constant name matters here, because that is the identifier call
/// sites spell as `key::NAME`.
fn declared_knobs() -> BTreeSet<String> {
    let Ok(text) = std::fs::read_to_string(root().join(KNOBS_TABLE_FILE)) else {
        return BTreeSet::new();
    };
    text.lines()
        .filter_map(|line| {
            let t = line.trim();
            if t.starts_with("//") {
                return None;
            }
            let name = t.split_once("=>")?.0.trim();
            let ok = !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
            ok.then(|| name.to_string())
        })
        .collect()
}

/// Property 4: fail when a published XDG knob is read by nobody.
///
/// # Why property 3 cannot catch it
///
/// Property 3 scans zero-argument getters of the modules that hold process
/// state, `src/xdg/policy/access.rs` among them. The knob
/// table is a different surface with a different call shape: knobs are reached
/// through `policy_u64(key::NAME)` rather than through a generated getter, so
/// they were invisible to that scan no matter how many of them were dead.
///
/// Measured 2026-08-17, before this property existed: the table declared 107
/// knobs and `src/` spelled only 89 of them. The eighteen survivors were whole
/// families — seven `heap_*`, five `mitm_*`, two `robots_*`, two `http_*`, two
/// `lightpanda_*`. Every one of them accepted `config set`, echoed the value
/// back through `config get`, and changed nothing at runtime, because the code
/// kept reading the compile-time constant the knob was supposed to override.
///
/// That is worse than an absent feature. An operator who raises
/// `mitm_proxy_seconds_max` and watches the capture still stop at the old
/// ceiling has no way to tell a misconfiguration from a lie.
fn check_knobs_have_call_sites() -> Vec<String> {
    let knobs = declared_knobs();
    if knobs.len() < MIN_KNOBS {
        return vec![format!(
            "{KNOBS_TABLE_FILE} parsed to only {} knobs (expected at least {MIN_KNOBS}); \
             the table scan failed and every knob would look phantom",
            knobs.len()
        )];
    }

    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut files = Vec::new();
    rust_files(&root().join("src"), &mut files);
    for path in files {
        let rel = path
            .strip_prefix(root())
            .unwrap_or(&path)
            .display()
            .to_string();
        // The table declares them; declaring is not reading.
        if rel == KNOBS_TABLE_FILE {
            continue;
        }
        // A dedicated `tests.rs` module is test code in full; reading a knob
        // there proves nothing about the runtime.
        if path.file_name().is_some_and(|n| n == "tests.rs") {
            continue;
        }
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        let text = strip_test_regions(&raw);
        for line in text.lines() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            for (idx, _) in line.match_indices("key::") {
                let name: String = line[idx + "key::".len()..]
                    .chars()
                    .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
                    .collect();
                if !name.is_empty() {
                    seen.insert(name);
                }
            }
        }
    }

    knobs
        .difference(&seen)
        .map(|k| {
            format!(
                "{KNOBS_TABLE_FILE}: `{k}` is published to `config set` and read by ZERO \
                 call sites — the operator can set it and the runtime will ignore them"
            )
        })
        .collect()
}

#[test]
fn every_published_xdg_knob_has_a_production_call_site() {
    let problems = check_knobs_have_call_sites();
    assert!(
        problems.is_empty(),
        "config that accepts a value and ignores it is worse than config that \
         refuses it:\n{}",
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
