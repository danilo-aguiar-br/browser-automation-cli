// SPDX-License-Identifier: MIT OR Apache-2.0
//! Fail when a canonical `run --script` step key is accepted but undiscoverable.
//!
//! # Why this exists
//!
//! Step keys fall into two classes and only one of them was covered. A key that
//! mirrors a clap flag is documented for free: `docs/schemas/<cmd>.schema.json`
//! is projected from the parser and carries a `step_key` field for every
//! property, and `doc-coverage-check.sh` holds that flag surface against the
//! docs. A key that exists ONLY in the step surface has no such home by
//! construction, so nothing looked at it.
//!
//! Measured 2026-08-31: of the 119 distinct keys in `STEP_FIELDS`, twelve had no
//! schema counterpart. `landscape` was the sharp one. The step read it, the CDP
//! call honoured it once the parameter actually reached `Page.printToPDF`, and
//! the entire published corpus mentioned the word exactly once — inside
//! `emulate`'s viewport prose, which is screen orientation and not paper. A
//! capability nobody can find is a capability that does not exist.
//!
//! [`step_key_alias_gate`](../step_key_alias_gate.rs) already holds every
//! declared ALIAS against its command schema. This is the missing half:
//! canonical keys.
//!
//! # Why this reads the table instead of parsing its source
//!
//! The first version of this gate was a shell script that re-derived
//! `STEP_FIELDS` with a regex over the `.rs` file, then subtracted the command
//! names to recover the keys. That works until the table's formatting moves,
//! and it also silently DROPPED every key whose spelling happens to match a
//! command name — `text`, `reload`, `submit` and `write` are all both. Reading
//! the table as data has neither failure mode: a row is already a `(cmd, keys)`
//! pair, so nothing has to be recovered.
//!
//! # Why the schema match is structural
//!
//! Substring search over a multi-command corpus lies. Three successive searches
//! called `landscape` documented, every one of them matching prose in a schema
//! description belonging to a different command. Each schema property publishes
//! its own `step_key`, so membership is decided against that field and never
//! against the file's text.

use std::collections::BTreeSet;

use browser_automation_cli::commands::run::STEP_FIELDS;

mod common;
use common::root;

/// Every `step_key` value published anywhere inside one schema document.
///
/// Schemas nest: a command's own arguments sit at the top level and each action
/// carries a second level beneath it. Recursing covers both without this gate
/// having to encode the shape, which is the same reason the alias gate walks
/// rather than indexes.
fn published_step_keys(v: &serde_json::Value, out: &mut BTreeSet<String>) {
    match v {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::String(k)) = map.get("step_key") {
                out.insert(k.clone());
            }
            for child in map.values() {
                published_step_keys(child, out);
            }
        }
        serde_json::Value::Array(items) => {
            for child in items {
                published_step_keys(child, out);
            }
        }
        _ => {}
    }
}

/// The `step_key` set of one command, or `None` when it publishes no schema.
///
/// A row keyed by a nested shape — `cookie.cookies[]`, `drag.synthetic_payload`
/// — names no command and therefore no schema file. That is not a defect: those
/// rows describe objects INSIDE a step, and their vocabulary is answered by the
/// cookbook arm below.
fn schema_keys(cmd: &str) -> Option<BTreeSet<String>> {
    let path = root()
        .join("docs/schemas")
        .join(format!("{cmd}.schema.json"));
    let raw = std::fs::read_to_string(&path).ok()?;
    let doc: serde_json::Value =
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
    let mut out = BTreeSet::new();
    published_step_keys(&doc, &mut out);
    Some(out)
}

/// Whether a cookbook carries `key` as a JSON token or a backticked identifier.
///
/// Both spellings are anchored on purpose. The bare word matches prose about
/// something else, which is precisely how `landscape` passed for documented
/// three times before the search was tightened.
fn cookbook_mentions(text: &str, key: &str) -> bool {
    text.contains(&format!("\"{key}\"")) || text.contains(&format!("`{key}`"))
}

#[test]
fn every_canonical_step_key_is_discoverable() {
    let root = root();
    let en = std::fs::read_to_string(root.join("docs/COOKBOOK.md")).expect("read COOKBOOK.md");
    let pt = std::fs::read_to_string(root.join("docs/COOKBOOK.pt-BR.md"))
        .expect("read COOKBOOK.pt-BR.md");

    // An empty sweep must fail rather than pass. A key-extraction pipeline
    // reported green from an emptied list twice while this gate was being
    // written, so the shape of the input is asserted before it is trusted.
    assert!(
        STEP_FIELDS.len() >= 40,
        "STEP_FIELDS collapsed to {} rows; the table is the input to this gate and an empty \
         input would make it pass having checked nothing",
        STEP_FIELDS.len()
    );

    let mut undocumented: Vec<String> = Vec::new();
    let mut checked = 0usize;

    for (cmd, keys) in STEP_FIELDS {
        let published = schema_keys(cmd);
        for key in *keys {
            checked += 1;
            if published.as_ref().is_some_and(|set| set.contains(*key)) {
                continue;
            }
            if cookbook_mentions(&en, key) && cookbook_mentions(&pt, key) {
                continue;
            }
            let where_en = cookbook_mentions(&en, key);
            let where_pt = cookbook_mentions(&pt, key);
            undocumented.push(format!(
                "{cmd}.{key} (schema={}, cookbook_en={where_en}, cookbook_pt={where_pt})",
                published.is_some()
            ));
        }
    }

    assert!(
        undocumented.is_empty(),
        "{} of {checked} step keys are accepted by the validator and documented nowhere a caller \
         reads. A key is discoverable when its command's schema publishes it as a `step_key`, or \
         when BOTH cookbooks carry it as \"key\" or `key`. Undocumented: {undocumented:#?}",
        undocumented.len()
    );
}
