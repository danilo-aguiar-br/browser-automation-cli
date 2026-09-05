// SPDX-License-Identifier: MIT OR Apache-2.0
//! Clap-vs-schema parity: every flag clap accepts must appear in `schema --cmd`.
//!
//! # Why this exists as a second gate
//!
//! `scripts/schema-drift-check.sh` compares `docs/schemas/*.json` against the
//! live binary and is correct on its own axis. It cannot see this failure,
//! because both sides of that comparison derive from the SAME hand-written
//! schema module: the generator asks the binary, the binary asks
//! `src/commands/meta/schema/*.rs`. When a flag is added to the clap enum and
//! wired in the dispatcher, nothing in that loop forces a schema edit, so the
//! file and the binary agree perfectly about a surface neither describes.
//!
//! An agent discovers what it may pass by reading `schema`. A flag absent there
//! does not exist for the agent, whatever the parser happens to accept.
//!
//! # Why this is a Rust test and not a script
//!
//! It was `scripts/clap_schema_parity.py` until 2026-08-18. The product is Rust
//! end to end and ships no interpreter, so the gate died on any host without
//! that interpreter — and it is reached from `scripts/ci-check.sh`, the product's own
//! closure criterion. The port also buys the correct binary path:
//! `CARGO_BIN_EXE_` is resolved by cargo at compile time, where the script had
//! to guard by hand against measuring a stale artifact.
//!
//! # What this gate first reported, and why it was wrong
//!
//! The first cut reported 29 undocumented flags across `console`, `heap`,
//! `mitm`, `net`, `perf` and `storage`. Every one was a false positive with a
//! single cause: the clap side unions `<cmd> --help` with `<cmd> <sub> --help`,
//! so it sees two levels, while the schema side read one. A command that
//! dispatches on an action word keeps each action's arguments under
//! `action.actions.<name>.properties`. See [`collect_flags`], which recurses.
//!
//! Recorded rather than deleted, because the failure mode matters more than the
//! finding: a gate is a measuring instrument, and an instrument that disagrees
//! with the artifact is not evidence about the artifact until the instrument
//! itself has been checked.
//!
//! # Why `--help` and not a parse of the Rust source
//!
//! The same reason the drift gate trusts the binary: `--help` is what clap
//! actually built, so it cannot disagree with the parser. A regex over
//! `src/cli/` would introduce a third opinion, and a third opinion is a third
//! thing that can be wrong on its own.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

mod common;

/// A near-empty universe means the walk failed, and every command would then
/// look clean. Measured 2026-08-28: 71 commands in the live inventory.
const MIN_COMMANDS: usize = 40;

/// Capture stdout plus stderr of a help invocation; clap may use either.
fn run(args: &[&str]) -> String {
    match common::cmd().args(args).output() {
        Ok(o) => format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        ),
        Err(_) => String::new(),
    }
}

/// Every long flag clap prints for this invocation.
///
/// Mirrors `^\s{2,}(?:-[a-zA-Z], )?(--[a-z0-9][a-z0-9-]*)`: an indented line
/// whose first token is a long flag, optionally preceded by its short alias.
fn flags_in(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in text.lines() {
        if !line.starts_with("  ") {
            continue;
        }
        let trimmed = line.trim_start();
        // `-q, --quiet` — step over the short alias and its separator.
        let candidate = if trimmed.len() > 3
            && trimmed.starts_with('-')
            && !trimmed.starts_with("--")
            && trimmed.as_bytes()[1].is_ascii_alphanumeric()
            && trimmed.as_bytes()[2] == b','
            && trimmed.as_bytes()[3] == b' '
        {
            &trimmed[4..]
        } else {
            trimmed
        };
        let Some(name) = long_flag_at_start(candidate) else {
            continue;
        };
        out.insert(name);
    }
    out
}

/// `--flag[=<V>]`, `--flag <V>` and bare `--flag` all yield `--flag`.
fn long_flag_at_start(s: &str) -> Option<String> {
    let b = s.as_bytes();
    if b.len() < 3 || b[0] != b'-' || b[1] != b'-' {
        return None;
    }
    if !(b[2].is_ascii_lowercase() || b[2].is_ascii_digit()) {
        return None;
    }
    let mut end = 3;
    while end < b.len()
        && (b[end].is_ascii_lowercase() || b[end].is_ascii_digit() || b[end] == b'-')
    {
        end += 1;
    }
    Some(s[..end].to_string())
}

/// Names listed under the `Commands:` block, if the command has one.
///
/// Mirrors `^\s{2}([a-z][a-z0-9-]*)\s{2,}\S`.
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
        // A non-indented line ends the block (`Options:`, `Arguments:`, ...).
        if !line.is_empty() && !line.starts_with(' ') {
            break;
        }
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
        if name.is_empty() || name == "help" || !name.starts_with(|c: char| c.is_ascii_lowercase())
        {
            continue;
        }
        let after = &rest[name.len()..];
        if after.starts_with("  ") && !after.trim_start().is_empty() {
            out.insert(name);
        }
    }
    out
}

/// Add every long flag a properties object declares, at ANY nesting depth.
///
/// # Why the walk recurses
///
/// A command that dispatches on an action word keeps each action's arguments
/// under `action.actions.<name>.properties`, not at the top level. Reading only
/// the top level while the clap side already unions `<cmd> <sub> --help`
/// compares two different depths, and every nested flag is then reported as
/// undocumented — 29 findings of fictional debt, measured 2026-08-06.
///
/// # Why the key is a fallback for `argv`
///
/// Only some properties carry an explicit `argv`; most identify themselves by
/// key alone. A first cut that collected `argv` and nothing else reported 127
/// undocumented flags, including `qr --text`, which the schema documents as the
/// property `text`. The property key IS the documentation when `argv` is
/// absent, so it counts.
fn collect_flags(props: &serde_json::Map<String, Value>, out: &mut BTreeSet<String>) {
    for (key, prop) in props {
        let key_flag = format!("--{}", key.replace('_', "-"));
        let Some(obj) = prop.as_object() else {
            out.insert(key_flag);
            continue;
        };
        match obj.get("argv").and_then(Value::as_str) {
            Some(argv) if argv.starts_with("--") => {
                out.insert(argv.to_string());
            }
            _ => {
                out.insert(key_flag);
            }
        }
        if let Some(actions) = obj.get("actions").and_then(Value::as_object) {
            for spec in actions.values() {
                if let Some(nested) = spec.get("properties").and_then(Value::as_object) {
                    collect_flags(nested, out);
                }
            }
        }
    }
}

/// Long flags the published schema declares for `command`.
///
/// `None` means the command has NO schema at all, which is a different failure
/// from a schema that omits flags and is reported separately.
/// Flags the published schema declares for `command`, or `None` when the
/// product publishes no schema for it.
///
/// # Why every other failure panics instead of returning `None`
///
/// This function used to answer `None` for three unrelated events: the binary
/// failed to spawn, the binary exited non-zero for any reason, and the binary
/// answered that the command has no schema. The caller records `None` as
/// "no schema published at all" and the assertion tells a human to go write one.
///
/// On 2026-08-25 that misreport fired for real. Under load — a full pre-publish
/// gate compiling in parallel — one spawn of `schema --cmd extract` came back
/// unsuccessful, and the gate reported `FAIL extract (no schema published at
/// all)` and failed the release. Measured immediately afterwards, the schema
/// was there: exit 0 and 4118 bytes of JSON. Three consecutive re-runs passed.
///
/// So the exit code is read instead of thrown away. `2` is the product's usage
/// exit and is the only one that means "no such schema"; a spawn error, a
/// signal, or unparseable stdout on a successful exit are infrastructure
/// failures, and they now fail loudly and say what happened. A gate that cannot
/// tell "I could not look" from "it is not there" reports the second and sends
/// someone hunting for a file that exists.
fn schema_flags(command: &str) -> Option<BTreeSet<String>> {
    let out = common::cmd()
        .args(["--json", "schema", "--cmd", command])
        .output()
        .unwrap_or_else(|e| panic!("could not spawn the binary for `schema --cmd {command}`: {e}"));
    match out.status.code() {
        Some(0) => {}
        // The product's usage exit: this command genuinely publishes no schema.
        Some(2) => return None,
        other => panic!(
            "`schema --cmd {command}` exited {other:?}, which is neither success \
             nor the usage exit that means 'no schema'. This is a failure to \
             MEASURE, not a missing schema; do not add a schema on account of \
             it.\nstderr: {}",
            String::from_utf8_lossy(&out.stderr)
        ),
    }
    let payload: Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "`schema --cmd {command}` exited 0 but stdout is not JSON: {e}\n\
             stdout: {}",
            String::from_utf8_lossy(&out.stdout)
        )
    });
    let mut declared = BTreeSet::new();
    if let Some(props) = payload
        .get("data")
        .and_then(|d| d.get("properties"))
        .and_then(Value::as_object)
    {
        collect_flags(props, &mut declared);
    }
    Some(declared)
}

/// Top-level verbs, straight from the binary's own inventory.
fn command_names() -> Vec<String> {
    let out = common::cmd()
        .args(["--json", "commands"])
        .output()
        .expect("commands --json must run");
    assert!(out.status.success(), "commands --json exited non-zero");
    let payload: Value = serde_json::from_slice(&out.stdout).expect("commands --json must be JSON");
    let items = payload
        .get("data")
        .and_then(|d| d.get("commands"))
        .and_then(Value::as_array)
        .expect("commands --json missing data.commands");
    items
        .iter()
        .filter_map(|item| match item {
            Value::String(s) => Some(s.clone()),
            Value::Object(o) => o.get("name").and_then(Value::as_str).map(str::to_string),
            _ => None,
        })
        .collect()
}

#[test]
fn every_clap_flag_appears_in_schema() {
    let mut globals = flags_in(&run(&["--help"]));
    globals.insert("--help".to_string());
    globals.insert("--version".to_string());

    let names = command_names();
    assert!(
        names.len() >= MIN_COMMANDS,
        "inventory collapsed to {} commands; the walk failed rather than the product shrinking",
        names.len()
    );

    let mut missing: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut no_schema: Vec<String> = Vec::new();
    let mut checked = 0usize;

    for name in &names {
        let top_help = run(&[name, "--help"]);
        let mut exposed = flags_in(&top_help);
        for sub in subcommands_in(&top_help) {
            exposed.extend(flags_in(&run(&[name, &sub, "--help"])));
        }
        for g in &globals {
            exposed.remove(g);
        }
        if exposed.is_empty() {
            continue;
        }

        let Some(declared) = schema_flags(name) else {
            no_schema.push(name.clone());
            continue;
        };

        checked += 1;
        let gap: BTreeSet<String> = exposed.difference(&declared).cloned().collect();
        if !gap.is_empty() {
            missing.insert(name.clone(), gap);
        }
    }

    let mut report = String::new();
    for (name, flags) in &missing {
        for flag in flags {
            report.push_str(&format!(
                "FAIL  {name}  {flag}  (accepted by clap, absent from schema)\n"
            ));
        }
    }
    for name in &no_schema {
        report.push_str(&format!("FAIL  {name}  (no schema published at all)\n"));
    }

    let total: usize = missing.values().map(BTreeSet::len).sum::<usize>() + no_schema.len();
    println!("----");
    println!("commands_with_flags={checked}  undocumented_flags={total}");

    assert!(
        total == 0,
        "{report}== clap-schema-parity FAILED ==\n\
         Add the flag to src/commands/meta/schema/, then regenerate:\n  \
         bash scripts/generate_command_schemas.sh"
    );
    println!("== clap-schema-parity PASS ==");
}
