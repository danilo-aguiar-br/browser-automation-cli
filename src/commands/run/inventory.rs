// SPDX-License-Identifier: MIT OR Apache-2.0
//! Run/exec command inventory (GAP-001 / GAP-017 single source of truth).

/// Commands dispatched by `run`/`exec` (GAP-001 / GAP-017 single source of truth).
///
/// # What this list got wrong until 2026-08-31
///
/// It was written by hand beside a dispatcher that matches on slices of its
/// own, so the two drifted in BOTH directions and neither direction was
/// visible from either side.
///
/// It advertised `devtools3p_list`, `devtools3p_exec`, `webmcp_list` and
/// `webmcp_exec` — four underscore spellings the dispatcher never matched. A
/// step naming one of them was answered with `unknown script cmd`, quoting a
/// `Supported:` line that had just listed it. And it omitted `submit`,
/// `click`, `fill`, `screenshot`, `devtools3p` and `webmcp`, six spellings the
/// dispatcher runs, so the same suggestion told an agent that a working
/// command did not exist.
///
/// The unit test `dispatched_cmds_match_inventory` in `execute` now compares
/// this list against the union of the dispatcher slices. That comparison is
/// the only reason the list can be read as an inventory rather than as a
/// plausible-looking copy.
pub const RUN_DISPATCHED_CMDS: &[&str] = &[
    "goto",
    "wait",
    "hover",
    "drag",
    "fill-form",
    "fill_form",
    "select-option",
    "select_option",
    "pick",
    "upload",
    "submit",
    "back",
    "forward",
    "reload",
    "view",
    "press",
    "click",
    "write",
    "fill",
    "keys",
    "type",
    "click-at",
    "click_at",
    "eval",
    "grab",
    "screenshot",
    "print-pdf",
    "print_pdf",
    "extract",
    "text",
    "scroll",
    "cookie",
    "attr",
    "assert",
    "console",
    "net",
    "page",
    "dialog",
    "scrape",
    "emulate",
    "resize",
    "perf",
    "lighthouse",
    "screencast",
    "heap",
    "extension",
    "devtools3p",
    "devtools3p-list",
    "devtools3p-exec",
    "webmcp",
    "webmcp-list",
    "webmcp-exec",
];

/// Top-level browser-adjacent commands intentionally excluded from `run` (GAP-007 / GAP-017).
/// Each entry is `(cmd, reason)`.
pub const INTENTIONAL_RUN_EXCLUDE: &[(&str, &str)] = &[
    (
        "extension-install",
        "install requires Chrome relaunch with --load-extension; use top-level extension install",
    ),
    (
        "extension-uninstall",
        "uninstall is top-level one-shot; use extension uninstall outside run",
    ),
    ("doctor", "meta command; not a browser step"),
    ("commands", "meta discovery; not a browser step"),
    ("schema", "meta discovery; not a browser step"),
    ("version", "meta; not a browser step"),
    ("config", "XDG config; not a browser step"),
    ("completions", "shell completions; not a browser step"),
    ("man", "man page generation; not a browser step"),
    (
        "mitm",
        "MITM is a separate one-shot surface; use mitm capture-url or --mitm with browser cmds",
    ),
    (
        "workflow",
        "workflow journal is top-level; not an in-session browser step",
    ),
    (
        "batch-scrape",
        "batch-scrape is top-level HTTP/browser pool; use scrape steps or top-level batch-scrape",
    ),
    (
        "crawl",
        "crawl is top-level; use top-level crawl or multi-step goto/scrape",
    ),
    ("map", "map is top-level discovery; not an in-session step"),
    ("search", "search is top-level; not an in-session step"),
    ("parse", "path-light parse; not a browser session step"),
    ("qr", "path-light QR; not a browser session step"),
    (
        "image",
        "path-light image pipeline; not a browser session step",
    ),
    (
        "video",
        "path-light video pipeline; not a browser session step",
    ),
    (
        "audio",
        "path-light audio pipeline; not a browser session step",
    ),
    (
        "find-paths",
        "path-light discovery; not a browser session step",
    ),
    (
        "sg-scan",
        "path-light structural scan; not a browser session step",
    ),
    (
        "sg-rewrite",
        "path-light rewrite; not a browser session step",
    ),
    (
        "sheet-write",
        "path-light sheet; not a browser session step",
    ),
    ("monitor", "monitor check is top-level one-shot"),
    (
        "record",
        "record owns the whole session for its recording window; use it top-level and replay its NDJSON with run --script",
    ),
    ("run", "nested run is not supported"),
    ("exec", "nested exec is not supported"),
];

/// camelCase spellings a `run --script` step accepts beside its schema key.
///
/// # Why a table instead of a literal at each read site
///
/// Eight of these aliases were read inline in the capture step handler and
/// `docs/schemas/*.json` declared NONE of them. That is not an oversight in the
/// schema files: they are projected from the clap parser, and an alias reached
/// with `step.get("pageIdx")` is invisible to clap by construction. The result
/// was a tolerance the published contract never mentioned — it worked, and
/// nothing an agent could read said so.
///
/// This table is the shared contract whose absence produced that gap, and it is
/// the same defect class as the audit that created it: a producer and a consumer
/// agreeing on a key name without a place where the agreement is written down.
/// The step handler resolves aliases FROM here via [`step_key_aliases`], and
/// `schema --cmd` publishes them FROM here into `step_key_aliases` on the
/// property. A spelling therefore cannot be accepted without being documented,
/// because both sides read this one list.
///
/// Entries are `(cmd, step_key, alias)`. `include_preserved` appears twice with
/// DIFFERENT aliases because `console` and `net` each named the buffer they
/// preserve; that asymmetry is why the table is keyed by command and not by a
/// mechanical snake-to-camel conversion, which would have silently produced
/// `includePreserved` for both and matched neither.
pub const STEP_KEY_ALIASES: &[(&str, &str, &str)] = &[
    ("console", "include_preserved", "includePreservedMessages"),
    ("console", "page_idx", "pageIdx"),
    ("console", "page_size", "pageSize"),
    ("console", "service_worker_id", "serviceWorkerId"),
    ("net", "include_preserved", "includePreservedRequests"),
    ("net", "page_idx", "pageIdx"),
    ("net", "page_size", "pageSize"),
    ("net", "resource_types", "resourceTypes"),
];

/// Aliases declared for one `(cmd, step_key)` pair, in table order.
///
/// Returns an empty slice-backed iterator for a key with no alias, which is the
/// common case: only optional keys of the capture family carry one.
pub fn step_key_aliases(cmd: &str, step_key: &str) -> Vec<&'static str> {
    STEP_KEY_ALIASES
        .iter()
        .filter(|(c, k, _)| *c == cmd && *k == step_key)
        .map(|(_, _, alias)| *alias)
        .collect()
}

/// Spelling of `cmd` under which a step's field table is registered.
///
/// `run` accepts several spellings of the same command — `fill_form` for
/// `fill-form`, `click` for `press`, `screenshot` for `grab` — and every one of
/// them reaches the SAME handler, so every one of them accepts the same fields.
/// Folding them here is what keeps [`STEP_FIELDS`] one row per behaviour
/// instead of one row per spelling, which is the shape that would let two
/// spellings of one command drift apart.
pub fn canonical_step_cmd(cmd: &str) -> &str {
    match cmd {
        "fill_form" => "fill-form",
        "select_option" | "pick" => "select-option",
        "click" => "press",
        "fill" => "write",
        "click_at" => "click-at",
        "print_pdf" => "print-pdf",
        "screenshot" => "grab",
        "devtools3p-list" | "devtools3p-exec" => "devtools3p",
        "webmcp-list" | "webmcp-exec" => "webmcp",
        other => other,
    }
}

/// Canonical step keys each command reads; see `super::step_fields`.
pub use super::step_fields::STEP_FIELDS;

/// Object arrays a step carries: `(cmd, array_path, item_row)`.
///
/// `array_path` is DOTTED because the nesting depth is not uniform across
/// steps: `fill-form` and `cookie` put the array directly under a step key,
/// `drag` puts it inside an object.
///
/// # The depth is NOT uniform, and the first version of this comment was wrong
///
/// This said the surface had ONE level, on the evidence that
/// `rg 'as_array()' src/commands/run/execute/` returns a single hit. That
/// measurement was true and the conclusion drawn from it was false: the two
/// other steps carrying nested objects leave the executor as OPAQUE payload —
/// `cookie set` through `v.to_string()` and `drag` through `.cloned()` — so
/// `as_array()` proves the EXECUTOR does not descend, not that nothing is
/// nested. Reading an absence as coverage is the same mistake as reading
/// `ok: true` as success.
///
/// Measured the same day, with the top level already closed: `fill-form.fields`
/// and `cookie set`'s array sit one level down, and `drag` puts its array
/// INSIDE an object, two levels down. Hence dotted paths — the depth travels as
/// data, so a new nesting is a new row rather than a new walker.
///
/// This is still a table and not a recursion, and that is a decision rather
/// than an oversight: a recursion needs a rule for objects with no declared
/// row, and every such rule either rejects a payload the browser accepts or
/// accepts anything. The day a step nests an array of objects inside an array
/// of objects, a dotted path cannot address it and the walk MUST become
/// recursive. Without this paragraph the chosen depth reads as arbitrary and
/// the next audit reopens the question from scratch.
///
/// # What it closes
///
/// `reject_unknown_step_fields` inspected only the step's TOP-LEVEL keys.
/// Measured 2026-08-31, after the top level was closed:
/// `{"cmd":"fill-form","fields":[{"target":"#a","value":"x","chave_inventada":1}]}`
/// returned exit 0 with `ok: true` and filled the field, discarding the
/// invented key in silence. The same defect the release was opened to kill,
/// one level below where it was killed.
pub const STEP_ITEM_ARRAYS: &[(&str, &str, &str)] = &[
    ("fill-form", "fields", "fill-form.fields[]"),
    // `json` is the canonical key and `cookies` its synonym; the walker resolves
    // the FIRST segment through `step_key_reads`, so the row names only the
    // canonical one.
    ("cookie", "json", "cookie.cookies[]"),
    // Two levels: the array lives inside an object. The path is dotted rather
    // than a second table because the depth is data, not structure — adding a
    // third level is another row, not another walker.
    (
        "drag",
        "synthetic_payload.items",
        "drag.synthetic_payload.items[]",
    ),
    // The wrapper form. `normalize_drag_data` reads `payload.data` when it is
    // an object and falls back to `payload` itself, so BOTH paths reach the
    // browser and both have to be validated. Declared rather than probed: a
    // walker that guessed the wrapper would be implicit behaviour nobody can
    // read off the table.
    (
        "drag",
        "synthetic_payload.data.items",
        "drag.synthetic_payload.items[]",
    ),
];

/// Object nodes nested inside a step: `(cmd, object_path, row)`.
///
/// # Why arrays were not enough
///
/// [`STEP_ITEM_ARRAYS`] validates the OBJECTS INSIDE an array. `drag` carries
/// an object that is not inside any array — `synthetic_payload` itself — and
/// its optional keys were discarded in silence. Measured 2026-08-31:
/// `{"synthetic_payload":{"items":[…],"dragOperationsMsk":3}}` returned exit 0
/// and the drop ran with the default mask, so the typo changed behaviour and
/// reported success. A missing `items` is NOT in this class: it already fails
/// loudly in `normalize_drag_data`.
pub const STEP_OBJECT_NODES: &[(&str, &str, &str)] = &[
    ("drag", "synthetic_payload", "drag.synthetic_payload"),
    // The wrapper's inner object has the same shape as the outer one.
    ("drag", "synthetic_payload.data", "drag.synthetic_payload"),
];

/// Field names an object inside one of [`STEP_ITEM_ARRAYS`] may carry.
///
/// # The rule every row here obeys: never NARROWER than the layer below
///
/// An allowlist may be equal to or wider than what the layer underneath
/// accepts, and never narrower. Narrower does not close a fail-open; it trades
/// "accepted and discarded" for "refuses what used to work", and the second is
/// worse because it breaks a caller who was already correct. This validator
/// exists to catch typos, not to shrink a protocol.
///
/// # Why the two item rows have DIFFERENT provenance, and must not be uniformed
///
/// `cookie.cookies[]` is derived from `native::cookies::Cookie`, the struct the
/// product itself serialises cookies with. That is the right source there
/// BECAUSE the product models the cookie: it emits one in `cookie list` and
/// consumes one in `cookie set`, so the round trip is the contract and the
/// struct IS the contract.
///
/// `drag.synthetic_payload.items[]` is taken from the CDP `DragDataItem`
/// specification. That is the right source there for the opposite reason: the
/// product models NOTHING. Measured 2026-08-31 — there is no `DragDataItem` in
/// `src/`, `items` is passed through opaque, and the only `mimeType` in the
/// whole tree is an example string in `commands/nav/input/keyboard.rs`. With no
/// struct to derive from, the protocol is the only contract available, and it
/// is the layer that decides what actually works.
///
/// Two rows, two sources, and the difference is load-bearing. Uniforming them
/// breaks one: deriving the drag row from what the repo happens to exercise
/// would drop `title` and `baseURL`, which Chrome accepts today.
///
/// Unlike [`step_allowed_fields`] this prepends nothing: `cmd`, `action` and
/// `kind` belong to a STEP, and an item is not one.
pub fn step_item_allowed_fields(item_row: &str) -> Option<Vec<&'static str>> {
    let (_, keys) = STEP_FIELDS.iter().find(|(c, _)| *c == item_row)?;
    let mut out: Vec<&'static str> = Vec::new();
    for key in *keys {
        out.extend(step_key_reads(item_row, key));
    }
    out.sort_unstable();
    out.dedup();
    Some(out)
}

/// Alternate spellings a step key answers to; see `super::step_synonyms`.
pub use super::step_synonyms::STEP_KEY_SYNONYMS;

/// Every spelling of one step key, canonical first, then synonyms and aliases.
///
/// The navigation handlers pass this straight to `first_present`, so for them
/// the spellings a handler reads and the spellings the validator allows are
/// literally the same list, and neither direction can drift.
///
/// That mechanism does NOT reach every handler, and this doc used to claim it
/// did. `first_present` lives in `execute::nav_steps::fields` and is
/// `pub(super)`, so `perf_steps` and `page_steps` hand-roll their reads and the
/// invariant holds there only by attention. Measured 2026-08-31 it had already
/// slipped: `dir` is declared a synonym of `path` for the whole `screencast`
/// command, `start` read it and `stop` did not, so a `stop` step carrying `dir`
/// was accepted by the validator and discarded by the handler.
///
/// Stating the real scope is the point. A doc that promises an invariant the
/// code does not enforce is worse than one that names the gap, because the next
/// reader stops looking.
pub fn step_key_reads(cmd: &str, step_key: &str) -> Vec<&'static str> {
    // `step_key` is borrowed from the caller; the canonical spelling has to be
    // re-found in the table to reach a `'static` lifetime for the first entry.
    let canonical = canonical_step_cmd(cmd);
    let mut out: Vec<&'static str> = STEP_FIELDS
        .iter()
        .filter(|(c, _)| *c == canonical)
        .flat_map(|(_, keys)| keys.iter().copied())
        .filter(|k| *k == step_key)
        .collect();
    out.extend(
        STEP_KEY_SYNONYMS
            .iter()
            .filter(|(c, k, _)| *c == canonical && *k == step_key)
            .map(|(_, _, syn)| *syn),
    );
    out.extend(step_key_aliases(canonical, step_key));
    out
}

/// Every field name a step of this command may carry, or `None` for no row.
///
/// `None` means the command has no dispatch arm either, so the caller leaves
/// the rejection to the dispatcher's `unknown script cmd`, which says something
/// more useful than a field list would.
pub fn step_allowed_fields(cmd: &str) -> Option<Vec<&'static str>> {
    let canonical = canonical_step_cmd(cmd);
    let (_, keys) = STEP_FIELDS.iter().find(|(c, _)| *c == canonical)?;
    // `cmd` names the step; `action` is both the sub-selector of the eight
    // action-taking commands AND the fallback spelling of `cmd` itself (see
    // `stream.rs`). `kind` is the second spelling of `action` that
    // `helpers::step_action` reads, and it reads it for EVERY command, not just
    // for `assert` — so `{"cmd":"heap","kind":"summary"}` selects a capability
    // row and must not be refused by a list that only knew about `action`.
    // All three are universal rather than per-command for that reason.
    let mut out: Vec<&'static str> = vec!["cmd", "action", "kind"];
    for key in *keys {
        out.extend(step_key_reads(canonical, key));
    }
    out.sort_unstable();
    out.dedup();
    Some(out)
}

/// Human-readable list of dispatched cmds for suggestions (GAP-017).
pub fn run_supported_suggestion() -> String {
    format!("Supported: {}", RUN_DISPATCHED_CMDS.join(" "))
}

/// Suggestion for an unknown / excluded `run` script cmd (agent-native).
///
/// When `cmd` is in [`INTENTIONAL_RUN_EXCLUDE`], explain why and point to
/// top-level use; otherwise list dispatched cmds.
pub fn run_unknown_cmd_suggestion(cmd: &str) -> String {
    if let Some((_, reason)) = INTENTIONAL_RUN_EXCLUDE.iter().find(|(c, _)| *c == cmd) {
        return format!(
            "{cmd} is intentionally excluded from run: {reason}. {}",
            run_supported_suggestion()
        );
    }
    run_supported_suggestion()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Without this, a command added to the dispatcher and forgotten here would
    /// silently keep the old fail-open behaviour: `step_allowed_fields` returns
    /// `None`, the validator waves the step through, and the class this table
    /// was written to close reopens for exactly one command.
    #[test]
    fn every_dispatched_cmd_has_a_field_row() {
        let missing: Vec<&str> = RUN_DISPATCHED_CMDS
            .iter()
            .copied()
            .filter(|cmd| step_allowed_fields(cmd).is_none())
            .collect();
        assert!(
            missing.is_empty(),
            "dispatched cmds with no STEP_FIELDS row (their steps would accept any field): \
             {missing:?}"
        );
    }

    /// A synonym row pointing at a key no command declares is dead: nothing
    /// reads it and nothing allows it, so it documents a tolerance that does
    /// not exist.
    #[test]
    fn every_synonym_points_at_a_declared_key() {
        for (cmd, key, syn) in STEP_KEY_SYNONYMS {
            let row = STEP_FIELDS
                .iter()
                .find(|(c, _)| c == cmd)
                .unwrap_or_else(|| panic!("synonym {syn} names cmd {cmd} with no STEP_FIELDS row"));
            assert!(
                row.1.contains(key),
                "synonym `{syn}` is declared for {cmd}.{key}, and {cmd} declares no `{key}`"
            );
        }
    }

    /// The tables are keyed by canonical cmd, so a row spelled with an alias
    /// would never be found by `step_allowed_fields`.
    #[test]
    fn tables_are_keyed_by_canonical_cmd_only() {
        for (cmd, _) in STEP_FIELDS {
            assert_eq!(canonical_step_cmd(cmd), *cmd, "STEP_FIELDS row {cmd}");
        }
        for (cmd, _, _) in STEP_KEY_SYNONYMS {
            assert_eq!(canonical_step_cmd(cmd), *cmd, "STEP_KEY_SYNONYMS row {cmd}");
        }
    }

    /// Reads and the allowlist come from the same table, so every spelling a
    /// handler can reach is a spelling the validator accepts.
    #[test]
    fn every_read_spelling_is_an_allowed_field() {
        for (cmd, keys) in STEP_FIELDS {
            let allowed = step_allowed_fields(cmd).expect("row exists");
            for key in *keys {
                for spelling in step_key_reads(cmd, key) {
                    assert!(
                        allowed.contains(&spelling),
                        "{cmd} reads `{spelling}` for `{key}` and the allowlist rejects it"
                    );
                }
            }
        }
    }

    /// An alias spelling reaches the same row as the canonical one, which is
    /// what makes `click` a true alias of `press` rather than a second command
    /// with a field list of its own.
    #[test]
    fn alias_spellings_share_the_canonical_field_list() {
        for (alias, canonical) in [
            ("click", "press"),
            ("fill", "write"),
            ("fill_form", "fill-form"),
            ("pick", "select-option"),
            ("screenshot", "grab"),
            ("click_at", "click-at"),
            ("print_pdf", "print-pdf"),
        ] {
            assert_eq!(
                step_allowed_fields(alias),
                step_allowed_fields(canonical),
                "{alias} must accept exactly what {canonical} accepts"
            );
        }
    }
}
