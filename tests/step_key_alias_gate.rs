// SPDX-License-Identifier: MIT OR Apache-2.0
//! Fail when a `run --script` step key alias is accepted but never published.
//!
//! # Why this exists
//!
//! `run --script` accepted eight camelCase spellings — `pageIdx`, `resourceTypes`,
//! `includePreservedRequests` and their siblings — and `docs/schemas/*.json`
//! declared none of them. The schema is PROJECTED from the clap parser, so an
//! alias reached with `step.get("pageIdx")` is invisible to it by construction:
//! there was no edit anyone forgot, the two surfaces simply could not see each
//! other.
//!
//! That is the inverse of the defect this release was opened to fix. There, a
//! consumer read keys no producer wrote and answered `ok: true` with zero rows.
//! Here, the code accepted keys no published contract described. Both are the
//! same root cause seen from opposite ends — a producer and a consumer agreeing
//! on a name with no place where the agreement is written down — and closing
//! only one direction leaves the class alive.
//!
//! The remedy was `STEP_KEY_ALIASES`, read by the step handler AND by
//! `schema --cmd`. This gate is what keeps it the only door.

use std::collections::BTreeMap;

use browser_automation_cli::commands::run::STEP_KEY_ALIASES;

mod common;
use common::root;

/// Where step handlers may read a step key from JSON at all.
const STEP_HANDLER_DIR: &str = "src/commands/run/execute";

/// Every object in the document that looks like a derived property.
///
/// Properties are nested: a command's own args sit at the top, and an action
/// word carries a second level under `actions.<name>.properties`. Recursing is
/// what lets one predicate cover both without encoding the shape here.
fn walk_properties(v: &serde_json::Value) -> Box<dyn Iterator<Item = &serde_json::Value> + '_> {
    match v {
        serde_json::Value::Object(map) => {
            Box::new(std::iter::once(v).chain(map.values().flat_map(walk_properties)))
        }
        serde_json::Value::Array(items) => Box::new(items.iter().flat_map(walk_properties)),
        _ => Box::new(std::iter::empty()),
    }
}

/// Every alias the table declares must appear in that command's schema file.
///
/// This is the property whose absence WAS the gap: the aliases existed, worked,
/// and no artifact an agent reads mentioned them.
#[test]
fn every_declared_alias_is_published_in_its_command_schema() {
    let root = root();
    for (cmd, step_key, alias) in STEP_KEY_ALIASES {
        let path = root.join("docs/schemas").join(format!("{cmd}.schema.json"));
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let doc: serde_json::Value =
            serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));

        // A substring search over the whole document would pass on prose: the
        // string `serviceWorkerId` appears in `eval.schema.json` describing the
        // unrelated `--service-worker-id` FLAG. The assertion has to land on the
        // property that carries this step key, in its `step_key_aliases` array,
        // or it certifies a coincidence.
        let published = walk_properties(&doc).any(|prop| {
            prop.get("step_key").and_then(|v| v.as_str()) == Some(*step_key)
                && prop
                    .get("step_key_aliases")
                    .and_then(|v| v.as_array())
                    .is_some_and(|list| list.iter().any(|a| a.as_str() == Some(*alias)))
        });
        assert!(
            published,
            "alias `{alias}` is accepted by the code for {cmd}.{step_key} and no property in \
             {} declares it under `step_key_aliases`. Regenerate with \
             scripts/generate_command_schemas.sh; if it still does not appear, the property is \
             not reachable from the clap parser and the alias must be removed rather than \
             documented.",
            path.display()
        );
    }
}

/// No step handler may reach a camelCase key with an inline literal.
///
/// This is the escape hatch that produced the gap: an alias spelled inline is
/// invisible to the clap parser the schema is projected from, so it is accepted
/// and never published.
///
/// # Why this carries a debt list instead of demanding zero
///
/// The audit that opened this named `capture.rs` and its eight aliases.
/// Measured 2026-08-31 with the predicate below: `capture.rs` was a SAMPLE, not
/// the population — SEVEN other handlers hold 25 more inline aliases, none of
/// them declared in any schema either. That is the same discovery the comment in
/// `capture.rs` records one level down, where naming `resourceTypes` alone hid
/// seven identical siblings, repeating one level up.
///
/// Converting all 25 in the same change as the eight would rewrite seven files
/// the audit did not scope, so the debt is recorded with its measured size
/// instead of being silently tolerated. What this gate enforces TODAY is that
/// the class cannot GROW: a new file with an inline alias fails, and so does an
/// existing file that gains one. Closing the debt means moving a file's aliases
/// into `STEP_KEY_ALIASES` and dropping its entry below to zero.
///
/// # A gate that was blind to the shape it audits
///
/// The first version of the loop below read only the FIRST `.get("` on each
/// line. The shape it exists to catch puts both spellings on one expression —
/// `step.get("page_idx").or_else(|| step.get("pageIdx"))` — so on a single
/// line it would inspect `page_idx`, find no uppercase, and count ZERO.
///
/// It scored the 25 correctly anyway, which is the part worth recording:
/// rustfmt had broken every one of those expressions across lines because they
/// were too wide, so each alias happened to sit alone. A gate that is right
/// because of formatting is a gate that goes wrong when a shorter key lets the
/// same expression fit. That is the same defect one level up from the one this
/// file documents: something measured once, held as an invariant, and true only
/// by accident of the conditions at the time of measurement.
#[test]
fn inline_camelcase_step_keys_never_grow_beyond_the_measured_debt() {
    /// Handlers still reaching camelCase keys inline, with the count measured
    /// on 2026-08-31. Zero means the file is clean and must stay clean.
    ///
    /// Measured again 2026-08-31 after the `nav_steps` migration: `pointer.rs`
    /// and `forms.rs` dropped to ZERO, because every spelling they read now
    /// comes from `STEP_FIELDS` / `STEP_KEY_SYNONYMS` through
    /// `nav_steps::fields`. Their entries stay at 0 rather than being deleted,
    /// so a regression in a file that was cleaned is reported as a regression
    /// and not as a new file nobody had looked at.
    const DEBT: &[(&str, usize)] = &[
        ("perf_steps.rs", 7),
        ("capture_steps.rs", 7),
        ("page_steps/page.rs", 5),
        ("nav_steps/pointer.rs", 0),
        ("nav_steps/forms.rs", 0),
        // `step_beforeunload_action` is shared by `goto`, `reload` and
        // `print-pdf` and takes no `cmd`, so it cannot resolve through the
        // per-command table without a signature change in a handler outside
        // this pass. One read, recorded.
        ("helpers.rs", 1),
        ("assert_steps.rs", 1),
    ];

    let dir = root().join(STEP_HANDLER_DIR);
    let mut counted: BTreeMap<String, usize> = BTreeMap::new();
    let mut files_scanned = 0usize;
    let mut stack = vec![dir.clone()];

    while let Some(cur) = stack.pop() {
        let entries =
            std::fs::read_dir(&cur).unwrap_or_else(|e| panic!("read_dir {}: {e}", cur.display()));
        for entry in entries {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            // `OsStr` has no `PartialEq<&str>`, so this goes through `to_str`.
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            files_scanned += 1;
            // The debt list spells paths with `/`. On Windows the separator is
            // different, so every entry would miss and the gate would fail on a
            // supported host for a reason unrelated to the code it audits.
            let rel = path
                .strip_prefix(&dir)
                .expect("under handler dir")
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/");
            let text = std::fs::read_to_string(&path).expect("read step handler");
            for line in text.lines() {
                // Prose in a comment naming an alias is how the rule explains
                // itself beside the code; only the READ form counts.
                let code = line.split("//").next().unwrap_or(line);
                // EVERY read on the line, not just the first. The shape this
                // gate audits is `get("page_idx").or_else(|| get("pageIdx"))`,
                // which puts the snake key and its alias on ONE line: reading
                // only the first would see `page_idx`, find no uppercase, and
                // count zero for the very pattern the gate exists to catch.
                // The 25 below land on separate lines only because rustfmt
                // broke those expressions, which is an accident of width and
                // not a property anyone can rely on.
                for rest in code.split(".get(\"").skip(1) {
                    let Some(key) = rest.split('"').next() else {
                        continue;
                    };
                    let mut chars = key.chars();
                    let starts_lower = chars.next().is_some_and(|c| c.is_ascii_lowercase());
                    if starts_lower && key.chars().any(|c| c.is_ascii_uppercase()) {
                        *counted.entry(rel.clone()).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    assert!(
        files_scanned > 0,
        "scanned zero files under {} — the gate would pass vacuously",
        dir.display()
    );

    let allowed: BTreeMap<&str, usize> = DEBT.iter().copied().collect();
    let mut problems: Vec<String> = Vec::new();
    for (file, found) in &counted {
        let budget = allowed.get(file.as_str()).copied().unwrap_or(0);
        if *found > budget {
            problems.push(format!(
                "{file}: {found} inline camelCase key(s), budget {budget}"
            ));
        }
    }
    // A budget nobody spends is a budget that outlived its reason. Reporting it
    // is what keeps the list from becoming folklore, the way the expired
    // entries in `filesize-check.sh` did.
    for (file, budget) in DEBT {
        if counted.get(*file).copied().unwrap_or(0) < *budget {
            problems.push(format!(
                "{file}: budget {budget} is now too generous — lower it to the measured count"
            ));
        }
    }

    assert!(
        problems.is_empty(),
        "inline camelCase step keys are accepted without `schema --cmd` publishing them.\n\
         Move each to STEP_KEY_ALIASES in src/commands/run/inventory.rs and read it through a \
         table lookup, as src/commands/run/execute/page_steps/capture.rs does:\n{}",
        problems.join("\n")
    );
}

// ---------------------------------------------------------------------------
// Behaviour of the validator, not just the shape of the tables.
//
// Everything above audits source and schema files. What follows runs the
// product, because the defect this release closes was never visible in a file:
// a step was ACCEPTED, a field was DROPPED, and the envelope said `ok: true`.
// Only an invocation can show that.
//
// None of these launch Chrome. `validate_steps` runs `reject_unknown_step_fields`
// before BORN, so every assertion here is answered from argv alone.
// ---------------------------------------------------------------------------

use browser_automation_cli::commands::run::RUN_DISPATCHED_CMDS;

/// A second step that always fails preflight, so nothing is ever dispatched.
///
/// # Why the sentinel and not just a one-step script
///
/// A step whose fields are all ACCEPTED goes on to run, and a `goto` that runs
/// launches Chrome. Measured 2026-08-31: the synonym test took 60 s and needed
/// a browser to answer a question about argv. `validate_steps` walks every step
/// before BORN and stops at the first error, so a trailing unknown command
/// guarantees the walk covers the step under test and then aborts the script
/// with no launch at all.
const PREFLIGHT_SENTINEL: &str = r#"{"cmd":"cmd_inexistente_sentinela_gate"}"#;

/// Run a one-step script and return (exit code, stdout + stderr).
fn run_step(step: &str) -> (i32, String) {
    run_lines(&format!("{step}\n"))
}

/// Validate `step` without ever dispatching it; the run always exits 2.
///
/// The caller asserts on the MESSAGE: `unknown field` means the validator
/// refused a spelling, and its absence means the spelling was accepted and the
/// script died on the sentinel instead.
fn run_step_preflight_only(step: &str) -> String {
    let (code, out) = run_lines(&format!("{step}\n{PREFLIGHT_SENTINEL}\n"));
    assert_eq!(code, 2, "sentinel must abort preflight: {out}");
    out
}

fn run_lines(body: &str) -> (i32, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let script = dir.path().join("step.jsonl");
    std::fs::write(&script, body).expect("write script");
    let out = common::cmd()
        .args(["--json", "run", "--script"])
        .arg(&script)
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn run");
    (
        out.status.code().unwrap_or(-1),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

/// The step that motivated this whole pass.
///
/// `press` reads as "press a key" and dispatches a mouse click. Measured
/// 2026-08-31 before the fix: a `press` carrying `key` returned exit 0 with
/// `ok: true`, echoed the target back, and fired ZERO keyboard events. Nothing
/// in the envelope disagreed, so the caller had no way to learn that the field
/// had been discarded.
///
/// A generic "unknown field" would be true and useless: the caller's model of
/// the command is wrong, not their spelling. The assertion is therefore on the
/// message NAMING the substitute.
#[test]
fn press_with_a_key_field_is_refused_and_names_keys() {
    for cmd in ["press", "click"] {
        let (code, out) = run_step(&format!(
            r##"{{"cmd":"{cmd}","target":"#q","key":"Enter"}}"##
        ));
        assert_eq!(code, 2, "{cmd} with `key` must be a usage error: {out}");
        assert!(
            out.contains("keys"),
            "{cmd} rejecting `key` must name the `keys` command as the substitute: {out}"
        );
    }
}

/// `click` is an alias of `press`, down to the field list.
///
/// The dispatcher has matched `"press" | "click"` for a while and the inventory
/// never said so, which is how `click` came to be a working command that
/// `commands` did not advertise. Both spellings answering the SAME rejection is
/// what proves they reach one handler rather than two that happen to agree.
#[test]
fn click_and_press_resolve_to_the_same_handler() {
    let (press_code, press_out) = run_step(r#"{"cmd":"press","chave_inventada_gate":1}"#);
    let (click_code, click_out) = run_step(r#"{"cmd":"click","chave_inventada_gate":1}"#);
    assert_eq!(
        press_code, click_code,
        "press {press_out} / click {click_out}"
    );
    // The allowlist is printed in the suggestion, so an identical list is
    // evidence the two spellings share one row and not two copies of it.
    let allowed = |s: &str| {
        s.split("Allowed fields for ")
            .nth(1)
            .map(|t| t.split('"').next().unwrap_or_default().to_string())
            .unwrap_or_default()
    };
    let press_fields = allowed(&press_out);
    assert!(!press_fields.is_empty(), "no field list in {press_out}");
    assert_eq!(
        press_fields.trim_start_matches("press:"),
        allowed(&click_out).trim_start_matches("click:"),
        "click must accept exactly what press accepts"
    );
}

/// The fail-open arm is gone: no dispatched command tolerates an invented key.
///
/// This is the property, not a sample. Before, four commands rejected and
/// twenty-four accepted anything — and the four that rejected are exactly the
/// ones an audit would have spot-checked. Iterating the whole inventory is the
/// only version of this test that could have failed.
#[test]
fn an_invented_field_is_refused_on_every_dispatched_cmd() {
    let mut tolerated = Vec::new();
    for cmd in RUN_DISPATCHED_CMDS {
        let (code, out) = run_step(&format!(r#"{{"cmd":"{cmd}","chave_inventada_gate":1}}"#));
        if code != 2 || !out.contains("unknown field") {
            tolerated.push(format!("{cmd} -> exit {code}: {}", out.replace('\n', " ")));
        }
    }
    assert!(
        tolerated.is_empty(),
        "these cmds accepted an invented field instead of failing closed:\n{}",
        tolerated.join("\n")
    );
}

/// Every alias the table declares is accepted by the validator that now rejects.
///
/// Closing the fail-open arm is the change most likely to break a spelling that
/// worked, so the two halves are asserted together: the aliases published in
/// `docs/schemas` must survive the rejection that the same table now drives.
#[test]
fn every_declared_alias_survives_the_new_rejection() {
    let mut refused = Vec::new();
    for (cmd, _step_key, alias) in STEP_KEY_ALIASES {
        let out = run_step_preflight_only(&format!(r#"{{"cmd":"{cmd}","{alias}":1}}"#));
        // `console` and `net` need a capture flag this invocation does not
        // pass, so the step legitimately fails for THAT reason. What must never
        // appear is the field rejection.
        if out.contains("unknown field") {
            refused.push(format!("{cmd}.{alias}: {}", out.replace('\n', " ")));
        }
    }
    assert!(
        refused.is_empty(),
        "aliases published in docs/schemas are now rejected by the validator:\n{}",
        refused.join("\n")
    );
}

/// A spelling the handlers read must not be refused by the validator.
///
/// The synonym lists moved out of the handlers, and the risk of that move is
/// asymmetric: a spelling dropped from the table stops being READ (silent, the
/// old bug) and stops being ALLOWED (loud, this test). Sampling the spellings
/// that had no published schema is what covers the half a schema check cannot.
#[test]
fn undocumented_synonyms_the_handlers_read_are_still_accepted() {
    // Each pair is a spelling reached by a handler and published by NO schema.
    // They are the reason the synonyms could not simply be deleted.
    let cases = [
        (r##"{"cmd":"press","ref":"@e1"}"##, "press.ref"),
        (r##"{"cmd":"hover","selector":"#a"}"##, "hover.selector"),
        (
            r##"{"cmd":"select-option","trigger":"#t","option":"x"}"##,
            "select-option.trigger",
        ),
        (
            r##"{"cmd":"upload","uid":"#f","path":"/tmp/x"}"##,
            "upload.uid",
        ),
        (r#"{"cmd":"eval","js":"1+1"}"#, "eval.js"),
        (r#"{"cmd":"eval","function":"1+1"}"#, "eval.function"),
        (r##"{"cmd":"wait","sel":"#a"}"##, "wait.sel"),
        (r##"{"cmd":"scroll","dy":10}"##, "scroll.dy"),
        (
            r##"{"cmd":"goto","url":"about:blank","navigationTimeoutMs":9000}"##,
            "goto.navigationTimeoutMs",
        ),
        (
            r##"{"cmd":"keys","key":"Enter","target":"#q"}"##,
            "keys.target",
        ),
    ];
    let mut refused = Vec::new();
    for (step, label) in cases {
        let out = run_step_preflight_only(step);
        if out.contains("unknown field") {
            refused.push(format!("{label}: {}", out.replace('\n', " ")));
        }
    }
    assert!(
        refused.is_empty(),
        "spellings the handlers read are refused by the validator:\n{}",
        refused.join("\n")
    );
}

/// The fail-open that survived the top-level fix, one level down.
///
/// `reject_unknown_step_fields` inspected the step's OWN keys and never
/// descended. Measured 2026-08-31 with the top level already closed:
/// `{"cmd":"fill-form","fields":[{"target":"#a","value":"x","chave_inventada":1}]}`
/// returned exit 0 with `ok: true`, filled `#a`, and discarded the invented key
/// in silence. Same defect as the one this release killed, in the only nested
/// structure the step surface has.
#[test]
fn an_invented_field_inside_a_fill_form_item_is_refused() {
    let (code, out) = run_step(
        r##"{"cmd":"fill-form","fields":[{"target":"#a","value":"x","chave_inventada_no_item":1}]}"##,
    );
    assert_eq!(code, 2, "an invented key inside an item must fail: {out}");
    assert!(
        out.contains("chave_inventada_no_item"),
        "the message must name the offending key: {out}"
    );
    // The level is the point. `on step cmd=fill-form` and `in fields[0] of
    // cmd=fill-form` send the reader to different places, and a caller with a
    // dozen fields needs the second one.
    assert!(
        out.contains("fields[0]"),
        "the message must locate the item by index, not just the step: {out}"
    );
}

/// The index has to be the REAL one, or it points at the wrong field.
///
/// An implementation that reported the array name without the offset, or that
/// always reported `[0]`, would pass the test above and still send the caller
/// to the wrong line of their script.
#[test]
fn the_reported_item_index_is_the_offending_one() {
    let (code, out) = run_step(
        r##"{"cmd":"fill-form","fields":[{"target":"#a","value":"1"},{"target":"#b","value":"2"},{"target":"#c","value":"3","typo_no_terceiro":1}]}"##,
    );
    assert_eq!(code, 2, "{out}");
    assert!(
        out.contains("fields[2]"),
        "the third item is at index 2 and the message must say so: {out}"
    );
}

/// The rejection must NOT kill the spellings the handler reads.
///
/// `uid`, `ref` and `text` are read inside a `fill-form` item and appear in NO
/// published schema, so they are exactly the spellings a validator built from
/// the schema would have destroyed. This is the half that matters as much as
/// the rejection: closing a fail-open by breaking working scripts is not a fix.
#[test]
fn every_item_spelling_the_handler_reads_still_passes_validation() {
    let items = [
        r##"{"target":"#a","value":"x"}"##,
        r##"{"uid":"#a","value":"x"}"##,
        r##"{"selector":"#a","value":"x"}"##,
        r##"{"ref":"@e1","value":"x"}"##,
        r##"{"target":"#a","text":"x"}"##,
    ];
    let mut refused = Vec::new();
    for item in items {
        let out = run_step_preflight_only(&format!(r#"{{"cmd":"fill-form","fields":[{item}]}}"#));
        if out.contains("unknown field") {
            refused.push(format!("{item}: {}", out.replace('\n', " ")));
        }
    }
    assert!(
        refused.is_empty(),
        "spellings the fill-form item handler reads are refused by the validator:\n{}",
        refused.join("\n")
    );
}

/// The JSON-string form of the array is validated too.
///
/// `fields_json` is the CLI long name and the dispatcher parses the string, so
/// a validator that only walked the array form would leave the same hole under
/// a documented spelling.
#[test]
fn the_json_string_form_of_fields_is_validated_as_well() {
    let (code, out) = run_step(
        r##"{"cmd":"fill-form","fields_json":"[{\"target\":\"#a\",\"value\":\"x\",\"typo_na_string\":1}]"}"##,
    );
    assert_eq!(code, 2, "the string form must be validated too: {out}");
    assert!(out.contains("typo_na_string"), "{out}");
}

/// The same nested fail-open, in the second of the three steps that had it.
///
/// Measured 2026-08-31 before the fix: a `cookie set` carrying an invented key
/// inside a cookie object returned exit 0 with `ok: true`, and the `cookie
/// list` that followed FOUND the cookie stored. The write happened and the key
/// vanished, which is the `fill-form` defect in a different step.
#[test]
fn an_invented_field_inside_a_cookie_item_is_refused() {
    let (code, out) = run_step(
        r##"{"cmd":"cookie","action":"set","cookies":[{"name":"n","value":"v","url":"https://ex.test","chave_inventada_no_item":1}]}"##,
    );
    assert_eq!(code, 2, "an invented key inside a cookie must fail: {out}");
    assert!(out.contains("chave_inventada_no_item"), "{out}");
    // `cookies` is the synonym and `json` the canonical key, so the message
    // names the spelling the caller actually wrote.
    //
    // This line is why the test is worth more than the assertion above it. The
    // first implementation reported the CANONICAL key, so a step written with
    // `cookies` was answered with `in json[0]` — sending the reader to look for
    // a key absent from their own script, which is the exact misdirection this
    // release exists to remove, reintroduced inside its own error text.
    //
    // It was caught because the assertion was written against the CALLER's
    // spelling rather than against what the code produces. A test written from
    // the implementation would have asserted `json[0]`, passed, and frozen the
    // defect. That is the difference between a test that VERIFIES and one that
    // merely CONFIRMS, and it is not obvious from reading either one.
    assert!(
        out.contains("cookies[0]"),
        "the message must locate the cookie by index: {out}"
    );
}

/// The canonical spelling and the string form reach the same validation.
///
/// `json` is the canonical key, `cookies` its synonym, and the payload may
/// arrive as a JSON STRING because `cookie_set_payload` accepts both. A
/// validator that walked only one of the three would leave the hole open under
/// the other two, which is the shape of the original defect.
#[test]
fn every_cookie_payload_spelling_reaches_the_item_validation() {
    for (label, step) in [
        (
            "json array",
            r##"{"cmd":"cookie","action":"set","json":[{"name":"n","value":"v","url":"https://ex.test","typo_json":1}]}"##,
        ),
        (
            "cookies string",
            r##"{"cmd":"cookie","action":"set","cookies":"[{\"name\":\"n\",\"value\":\"v\",\"url\":\"https://ex.test\",\"typo_str\":1}]"}"##,
        ),
    ] {
        let (code, out) = run_step(step);
        assert_eq!(code, 2, "{label} must be validated: {out}");
        assert!(
            out.contains("typo_json") || out.contains("typo_str"),
            "{label}: {out}"
        );
    }
}

/// The rejection must not kill a cookie the product itself emits.
///
/// The allowed set is `native::cookies::Cookie` plus `url`, so what `cookie
/// list` writes round-trips into `cookie set`. If this test fails, the round
/// trip is broken and a script that stores cookies between runs stops working.
#[test]
fn a_cookie_shaped_like_the_products_own_output_still_passes() {
    let full = r##"{"cmd":"cookie","action":"set","cookies":[{"name":"n","value":"v","domain":".ex.test","path":"/","expires":-1,"size":8,"httpOnly":true,"secure":true,"session":true,"sameSite":"Lax"}]}"##;
    let out = run_step_preflight_only(full);
    assert!(
        !out.contains("unknown field"),
        "a cookie carrying every field the product serialises must pass: {out}"
    );
}

/// The third and deepest of the nested fail-opens: two levels down.
///
/// `drag` puts its array inside an OBJECT, so the path is
/// `synthetic_payload.items` and not a bare step key. Measured 2026-08-31
/// before the fix: an invented key inside an item returned exit 0 with the drag
/// reported as successful.
#[test]
fn an_invented_field_inside_a_drag_item_is_refused() {
    let (code, out) = run_step(
        r##"{"cmd":"drag","from":"#a","to":"#b","synthetic_payload":{"items":[{"mimeType":"text/plain","data":"x","chave_inventada_no_item":1}]}}"##,
    );
    assert_eq!(code, 2, "an invented key two levels down must fail: {out}");
    assert!(out.contains("chave_inventada_no_item"), "{out}");
    // The path has to name BOTH levels, or the caller cannot tell an item key
    // from a payload key.
    assert!(
        out.contains("synthetic_payload.items[0]"),
        "the message must carry the full path: {out}"
    );
}

/// The object level was fail-open too, and its typo CHANGED behaviour.
///
/// `dragOperationsMsk` fell through to the default mask of 1 and the drop ran,
/// so the step reported success for a drag configured differently from what the
/// caller wrote. A missing `items` is NOT in this class: it already failed
/// loudly in `normalize_drag_data`, which is why only the optional keys needed
/// this.
#[test]
fn an_invented_field_on_the_drag_payload_object_is_refused() {
    let (code, out) = run_step(
        r##"{"cmd":"drag","from":"#a","to":"#b","synthetic_payload":{"items":[{"mimeType":"text/plain","data":"x"}],"dragOperationsMsk":3}}"##,
    );
    assert_eq!(code, 2, "a typo on the payload object must fail: {out}");
    assert!(out.contains("dragOperationsMsk"), "{out}");
    assert!(
        out.contains("synthetic_payload"),
        "the message must name the object: {out}"
    );
}

/// The `data` wrapper is a documented spelling and reaches the same validation.
///
/// `normalize_drag_data` reads `payload.data` when it is an object and falls
/// back to the payload itself, so BOTH shapes reach the browser. Validating one
/// and not the other would leave the hole open under the form a page actually
/// emits.
#[test]
fn the_drag_data_wrapper_form_is_validated_too() {
    let (code, out) = run_step(
        r##"{"cmd":"drag","from":"#a","to":"#b","synthetic_payload":{"data":{"items":[{"mimeType":"text/plain","data":"x","typo_no_wrapper":1}]}}}"##,
    );
    assert_eq!(code, 2, "the wrapper form must be validated: {out}");
    assert!(out.contains("typo_no_wrapper"), "{out}");
    assert!(
        out.contains("synthetic_payload.data.items[0]"),
        "the wrapper path must be reported in full: {out}"
    );
}

/// The rejection must not narrow the protocol.
///
/// `title` and `baseURL` are CDP `DragDataItem` fields that the published
/// formula and the tests never use. A validator built from what the repo
/// happens to exercise would refuse them, and Chrome accepts them — this
/// catches typos, it does not shrink the protocol.
#[test]
fn every_drag_payload_spelling_the_protocol_defines_still_passes() {
    let cases = [
        r##"{"synthetic_payload":{"items":[{"mimeType":"text/plain","data":"x"}]}}"##,
        r##"{"synthetic_payload":{"items":[{"mimeType":"text/uri-list","data":"u","title":"t","baseURL":"https://ex.test"}]}}"##,
        r##"{"synthetic_payload":{"items":[{"mimeType":"text/plain","data":"x"}],"dragOperationsMask":3,"files":[]}}"##,
        r##"{"syntheticPayload":{"items":[{"mimeType":"text/plain","data":"x"}]}}"##,
        r##"{"synthetic_payload":{"data":{"items":[{"mimeType":"text/plain","data":"x"}]}}}"##,
    ];
    let mut refused = Vec::new();
    for payload in cases {
        // Strip BOTH braces: keeping the trailing one produced a step with an
        // extra `}` and a JSON parse error, which the sentinel assertion caught
        // as exit 65 instead of 2 — a broken test, not a broken validator.
        let inner = payload
            .strip_prefix('{')
            .and_then(|s| s.strip_suffix('}'))
            .expect("case is a brace-wrapped object");
        let step = format!(r##"{{"cmd":"drag","from":"#a","to":"#b",{inner}}}"##);
        let out = run_step_preflight_only(&step);
        if out.contains("unknown field") {
            refused.push(format!("{payload}: {}", out.replace('\n', " ")));
        }
    }
    assert!(
        refused.is_empty(),
        "payload shapes the protocol defines are refused by the validator:\n{}",
        refused.join("\n")
    );
}
