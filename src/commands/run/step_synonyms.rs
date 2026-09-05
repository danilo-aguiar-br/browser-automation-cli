// SPDX-License-Identifier: MIT OR Apache-2.0
//! The `STEP_KEY_SYNONYMS` table, split out of `run::inventory`.
//!
//! # Why its own module
//!
//! The second table extracted for the 300-line ceiling, on the same grain as
//! `step_fields`: one table plus the reasoning it depends on, with the readers
//! staying beside the other tables they also consult.
//!
//! `inventory` re-exports the name, so callers keep spelling it
//! `inventory::STEP_KEY_SYNONYMS`.

/// Alternate spellings a step key answers to: `(cmd, step_key, synonym)`.
///
/// # Why this is a second table and not more rows in `STEP_KEY_ALIASES`
///
/// [`STEP_KEY_ALIASES`] carries a promise these entries cannot keep:
/// `schema --cmd` publishes it, and `tests/step_key_alias_gate.rs` fails any
/// entry that `docs/schemas/*.json` does not declare. The spellings below —
/// `ref`, `sel`, `trigger`, `uid`, `js`, `function` among them — appear in no
/// published schema, and putting them there without regenerating those files
/// would only move the inconsistency into the gate.
///
/// So the two tables answer two different questions. `STEP_KEY_ALIASES` is
/// "camelCase spellings the schema must publish". This one is "every spelling
/// the handler reads", which is exactly what an allowlist needs and a strictly
/// larger set. Both feed [`step_key_reads`], so a handler cannot read a
/// spelling this file does not list.
///
/// Order matters: [`step_key_reads`] yields the canonical key first and then
/// these in table order, and `first_present` short-circuits on the first key
/// that is present at all.
pub const STEP_KEY_SYNONYMS: &[(&str, &str, &str)] = &[
    ("goto", "init_script", "initScript"),
    ("goto", "handle_before_unload", "handleBeforeUnload"),
    // `navigationTimeoutMs` and `timeoutMs` were in the old reject-list and in
    // NO reader: the validator accepted them and the handler dropped them, so
    // a step asking for a longer navigation timeout got the default and no
    // warning. They are read from here now.
    ("goto", "navigation_timeout_ms", "navigationTimeoutMs"),
    ("goto", "navigation_timeout_ms", "timeout"),
    ("goto", "navigation_timeout_ms", "timeout_ms"),
    ("goto", "navigation_timeout_ms", "timeoutMs"),
    ("reload", "ignore_cache", "ignoreCache"),
    ("reload", "init_script", "initScript"),
    ("reload", "handle_before_unload", "handleBeforeUnload"),
    ("wait", "wait_timeout_ms", "waitTimeoutMs"),
    ("wait", "wait_timeout_ms", "ms"),
    ("wait", "wait_timeout_ms", "timeout_ms"),
    ("wait", "wait_timeout_ms", "timeoutMs"),
    ("wait", "selector", "sel"),
    ("wait", "url_contains", "urlContains"),
    ("wait", "network_idle_ms", "networkIdleMs"),
    ("wait", "network_idle_ms", "network_idle"),
    ("wait", "network_idle_ms", "networkIdle"),
    ("wait", "network_idle_ms", "idle_ms"),
    ("wait", "network_idle_ms", "idleMs"),
    ("wait", "dom_stable_ms", "domStableMs"),
    ("wait", "dom_stable_ms", "dom_stable"),
    ("wait", "dom_stable_ms", "domStable"),
    ("wait", "min_count", "minCount"),
    ("wait", "include_snapshot", "includeSnapshot"),
    ("hover", "target", "ref"),
    ("hover", "target", "selector"),
    ("hover", "include_snapshot", "includeSnapshot"),
    ("drag", "to_x", "toX"),
    ("drag", "to_y", "toY"),
    ("drag", "synthetic_payload", "syntheticPayload"),
    ("drag", "include_snapshot", "includeSnapshot"),
    ("fill-form", "fields", "json"),
    ("fill-form", "fields", "fields_json"),
    ("fill-form", "fields", "fieldsJson"),
    ("fill-form", "include_snapshot", "includeSnapshot"),
    ("select-option", "target", "ref"),
    ("select-option", "target", "selector"),
    ("select-option", "target", "trigger"),
    ("select-option", "option", "value"),
    ("select-option", "option", "text"),
    ("select-option", "include_snapshot", "includeSnapshot"),
    ("upload", "target", "ref"),
    ("upload", "target", "selector"),
    ("upload", "target", "uid"),
    ("upload", "include_snapshot", "includeSnapshot"),
    ("submit", "target", "ref"),
    ("submit", "target", "selector"),
    ("submit", "timeout_ms", "timeoutMs"),
    ("submit", "include_snapshot", "includeSnapshot"),
    ("view", "verbose", "detailed"),
    ("view", "allow_empty", "allowEmpty"),
    ("press", "target", "ref"),
    ("press", "target", "selector"),
    ("press", "include_snapshot", "includeSnapshot"),
    ("write", "target", "ref"),
    ("write", "target", "selector"),
    ("write", "value", "text"),
    ("write", "include_snapshot", "includeSnapshot"),
    ("keys", "target", "ref"),
    ("keys", "target", "selector"),
    ("keys", "include_snapshot", "includeSnapshot"),
    ("type", "target", "ref"),
    ("type", "target", "selector"),
    ("type", "target", "uid"),
    // `exec type <target> <text>` writes BOTH `value` and `text` into the step,
    // and the handler read only `text`. The pairing is `argv_to_step`'s, so the
    // spelling has to be honoured here or the argv surface contradicts itself.
    ("type", "text", "value"),
    ("type", "focus_only", "focusOnly"),
    ("type", "include_snapshot", "includeSnapshot"),
    ("click-at", "include_snapshot", "includeSnapshot"),
    ("eval", "expression", "function"),
    ("eval", "expression", "js"),
    ("eval", "dialog_action", "dialogAction"),
    ("eval", "file_path", "filePath"),
    ("grab", "full_page", "fullPage"),
    ("grab", "element", "selector"),
    ("grab", "element", "ref"),
    ("grab", "include_base64", "includeBase64"),
    ("extract", "target", "ref"),
    ("extract", "target", "selector"),
    ("text", "target", "ref"),
    ("text", "target", "selector"),
    ("scroll", "target", "ref"),
    ("scroll", "target", "selector"),
    ("scroll", "delta_x", "deltaX"),
    ("scroll", "delta_x", "dx"),
    ("scroll", "delta_y", "deltaY"),
    ("scroll", "delta_y", "dy"),
    ("scroll", "to_x", "toX"),
    ("scroll", "to_y", "toY"),
    ("scroll", "include_snapshot", "includeSnapshot"),
    ("cookie", "json", "cookies"),
    // `--cookies-json` is the flag name on the CLI surface, so an author who
    // learned the flag first reaches for `cookies_json` in a step and gets
    // exit 2 after already paying for a launch. One row is cheaper than the
    // round trip it saves.
    ("cookie", "json", "cookies_json"),
    ("attr", "target", "ref"),
    ("attr", "target", "selector"),
    ("attr", "name", "attr"),
    ("console", "id", "msgid"),
    ("console", "id", "index"),
    ("page", "index", "page_id"),
    ("page", "index", "pageId"),
    ("page", "isolated_context", "isolatedContext"),
    ("page", "bring_to_front", "bringToFront"),
    ("dialog", "if_present", "ifPresent"),
    ("assert", "path", "json_path"),
    ("assert", "path", "jsonPath"),
    ("assert", "target", "ref"),
    ("scrape", "formats", "format"),
    ("print-pdf", "init_script", "initScript"),
    ("print-pdf", "handle_before_unload", "handleBeforeUnload"),
    ("print-pdf", "navigation_timeout_ms", "timeout_ms"),
    ("print-pdf", "allow_empty", "allowEmpty"),
    ("perf", "auto_stop", "autoStop"),
    ("perf", "name", "insight_name"),
    ("perf", "name", "insightName"),
    ("perf", "insight_set_id", "insightSetId"),
    ("screencast", "path", "dir"),
    ("lighthouse", "out_dir", "outDir"),
    ("lighthouse", "lighthouse_path", "lighthousePath"),
    // Spellings of the OBJECTS inside `fill-form.fields`. They were inline at
    // `forms.rs:40` and `:44` until 2026-08-31, which is why `uid`, `ref` and
    // `text` were read by the handler, published by no schema, and allowed by
    // nothing. Listing them here is what lets the validator accept exactly the
    // set the handler reads — the rejection must NOT kill these three.
    ("fill-form.fields[]", "target", "uid"),
    ("fill-form.fields[]", "target", "selector"),
    ("fill-form.fields[]", "target", "ref"),
    ("fill-form.fields[]", "value", "text"),
];
