// SPDX-License-Identifier: MIT OR Apache-2.0
//! The `STEP_FIELDS` table, split out of `run::inventory`.
//!
//! # Why its own module
//!
//! `inventory.rs` reached 560 production lines against the 300-line ceiling
//! `scripts/filesize-check.sh` enforces, and this one table was 293 of them.
//!
//! The split follows the file's own grain rather than a line count: the module
//! holds one table plus the reasoning that table depends on, and the functions
//! that read it stay next to the other tables they also read. `inventory`
//! re-exports the name, so every caller keeps spelling it `inventory::STEP_FIELDS`.

/// Canonical step keys each command reads, beside the universal `cmd`/`action`.
///
/// # Why this table exists
///
/// `reject_unknown_step_fields` used to name four commands and wave the other
/// twenty-four through, so a step like `{"cmd":"net","filter":"…"}` returned
/// every record unfiltered with `ok: true`. That is worse than an error,
/// because the caller then reasons on top of a filter that never ran.
///
/// Closing it needs the accepted key set of EVERY command in one place. The
/// obvious source — the clap projection `schema --cmd` publishes — was tried
/// and reverted: the projection knows `text.target` while the handler also
/// accepts `text.selector`, so deriving from it rejects steps the published
/// cookbook teaches. This table carries the canonical keys and
/// `super::step_synonyms::STEP_KEY_SYNONYMS` carries the alternate
/// spellings, which together are what the handlers actually read.
///
/// Both names are written as plain code, NOT as intra-doc links, and the two
/// steps it took to get there are worth keeping.
///
/// They were bare names — `STEP_KEY_SYNONYMS` and `canonical_step_cmd` — back
/// when this table lived in `inventory`, where both resolved. The split into
/// its own module left them pointing at nothing, and `cargo doc` refused the
/// crate with `unresolved link`.
///
/// Qualifying them was the obvious repair and it was still wrong: `inventory`
/// and `step_synonyms` are PRIVATE modules, so a link out of a `pub` item into
/// either one is `private_intra_doc_links` — a lint that is a WARNING by
/// default and only bites under the `-D warnings` that `scripts/docs-check.sh`
/// passes. Measured 2026-09-01: plain `cargo doc` was happy and the gate was
/// not, which is why the first repair looked green.
///
/// Plain backticks say the same thing to a reader, promise no link rustdoc
/// cannot honour, and match line 224 of `inventory.rs`, which already spells
/// its neighbour that way.
///
/// Rows are keyed by `super::inventory::canonical_step_cmd`. A command with
/// no row is NOT
/// rejected here: it falls through to the dispatcher, which answers with
/// `unknown script cmd`. `every_dispatched_cmd_has_a_field_row` proves that
/// fallthrough can only ever be reached by a command that is about to be
/// rejected anyway.
pub const STEP_FIELDS: &[(&str, &[&str])] = &[
    (
        "goto",
        &[
            "url",
            "init_script",
            "handle_before_unload",
            "navigation_timeout_ms",
        ],
    ),
    ("back", &[]),
    ("forward", &[]),
    (
        "reload",
        &["ignore_cache", "init_script", "handle_before_unload"],
    ),
    (
        "wait",
        &[
            "wait_timeout_ms",
            "text",
            "selector",
            "selectors",
            "state",
            "url",
            "url_contains",
            "navigation",
            "network_idle_ms",
            "dom_stable_ms",
            "min_count",
            "include_snapshot",
        ],
    ),
    ("hover", &["target", "include_snapshot"]),
    (
        "drag",
        &[
            "from",
            "to",
            "to_x",
            "to_y",
            "anchor",
            "synthetic_payload",
            "include_snapshot",
        ],
    ),
    ("fill-form", &["fields", "include_snapshot"]),
    ("select-option", &["target", "option", "include_snapshot"]),
    ("upload", &["target", "path", "include_snapshot"]),
    ("submit", &["target", "timeout_ms", "include_snapshot"]),
    ("view", &["verbose", "allow_empty"]),
    ("press", &["target", "dblclick", "include_snapshot"]),
    ("write", &["target", "value", "include_snapshot"]),
    ("keys", &["key", "target", "include_snapshot"]),
    (
        "type",
        &[
            "target",
            "text",
            "clear",
            "submit",
            "focus_only",
            "include_snapshot",
        ],
    ),
    ("click-at", &["x", "y", "dblclick", "include_snapshot"]),
    (
        "eval",
        &["expression", "args", "dialog_action", "file_path", "typed"],
    ),
    (
        "grab",
        &[
            "path",
            "format",
            "full_page",
            "quality",
            "element",
            "include_base64",
        ],
    ),
    ("extract", &["target", "attr"]),
    ("text", &["target"]),
    (
        "scroll",
        &[
            "target",
            "delta_x",
            "delta_y",
            "to_x",
            "to_y",
            "include_snapshot",
        ],
    ),
    ("cookie", &["url", "json"]),
    ("attr", &["target", "name"]),
    (
        "console",
        &[
            "include_preserved",
            "page_idx",
            "page_size",
            "types",
            "service_worker_id",
            "id",
            "path",
        ],
    ),
    (
        "net",
        &[
            "include_preserved",
            "page_idx",
            "page_size",
            "resource_types",
            "id",
            "request_path",
            "response_path",
        ],
    ),
    (
        "page",
        &[
            "url",
            "background",
            "isolated_context",
            "index",
            "tab_id",
            "bring_to_front",
        ],
    ),
    ("dialog", &["text", "if_present"]),
    (
        "assert",
        &[
            "path",
            "exists",
            "equals",
            "contains",
            "value",
            "url",
            "url_contains",
            "text",
            "text_contains",
            "target",
            "level",
            "max",
            "pattern",
        ],
    ),
    // `engine` is DELIBERATELY absent, the same way `key` is absent from the
    // `press` row above. Both name a field the command cannot honour, and both
    // get a message of their own from `helpers::scrape_engine_refusal` /
    // `press_key_confusion` rather than the generic unknown-field text.
    //
    // Listing it here would be worse than it looks: the unknown-field error
    // builds its suggestion as `Allowed fields for {cmd}` from THIS row, so a
    // caller who mistyped some other key would be told, in the same breath,
    // that `engine` is accepted — while the step refuses it. The help text a
    // caller reads while fixing a script is the last place a stale claim
    // belongs.
    //
    // The two reasons the earlier comment gave for keeping it are both gone:
    // the COOKBOOK example that carried it was rewritten on 2026-08-31, and
    // `perf_steps.rs` no longer discards the field.
    ("scrape", &["url", "formats"]),
    (
        "print-pdf",
        &[
            "url",
            "init_script",
            "handle_before_unload",
            "navigation_timeout_ms",
            "allow_empty",
            "path",
            "landscape",
        ],
    ),
    (
        "perf",
        &["path", "auto_stop", "reload", "name", "insight_set_id"],
    ),
    ("screencast", &["path"]),
    ("heap", &["path", "base", "current", "id", "node"]),
    (
        "lighthouse",
        &["url", "out_dir", "device", "mode", "lighthouse_path"],
    ),
    (
        "emulate",
        &[
            "user_agent",
            "locale",
            "timezone",
            "offline",
            "latitude",
            "longitude",
            "media",
            "network_conditions",
            "cpu_throttling_rate",
            "color_scheme",
            "extra_headers",
            "viewport",
            "screen",
        ],
    ),
    ("resize", &["width", "height", "scale", "mobile", "screen"]),
    ("extension", &["id"]),
    ("devtools3p", &["url", "name", "params"]),
    ("webmcp", &["url", "name", "input"]),
    // GAP-034 pillar 3: expanded by preflight, never dispatched, still has to
    // reject a typo at load time.
    ("include", &["path", "script", "file"]),
    // Row for the OBJECTS inside `fill-form.fields`, not for a command. The
    // `[]` in the key is what keeps it out of reach of a step: `cmd` is checked
    // against `is_dispatchable_cmd` FIRST, so `{"cmd":"fill-form.fields[]"}` is
    // refused as an unknown command before this row is ever consulted.
    ("fill-form.fields[]", &["target", "value"]),
    // Row for the OBJECTS inside `cookie set`'s array.
    //
    // # Why this list and not the full CDP `CookieParam`
    //
    // These are the fields of `native::cookies::Cookie`, the struct the product
    // SERIALISES cookies with, plus `url`, which `set_cookies` fills in when
    // neither `url` nor `domain` is given. That choice makes the round trip
    // work: what `cookie list` emits is exactly what `cookie set` accepts back,
    // which is the shape a script actually moves between two runs.
    //
    // CDP additionally accepts `priority`, `partitionKey`, `sourceScheme`,
    // `sourcePort` and `sameParty`. They are NOT here because the product does
    // not model them anywhere else, and adding them would publish a surface no
    // other part of the CLI knows about. A caller who needs one gets a usage
    // error naming the accepted set, which is a discoverable refusal rather
    // than the silent discard this row replaces.
    (
        "cookie.cookies[]",
        &[
            "name", "value", "url", "domain", "path", "expires", "size", "httpOnly", "secure",
            "session", "sameSite",
        ],
    ),
    // Row for the `drag.synthetic_payload` OBJECT, which is a third shape: not
    // a step and not an array item. `items` is the only required key and its
    // absence already fails loudly in `normalize_drag_data`; the other three
    // were silent. Measured 2026-08-31: a payload carrying `dragOperationsMsk`
    // returned exit 0 and the drop ran with the DEFAULT mask of 1, so a typo
    // changed the drop semantics and reported success.
    //
    // `data` is the wrapper form `normalize_drag_data` accepts — some pages
    // emit the DragData fields one level down — and it is a legitimate spelling
    // rather than a nested payload of its own.
    (
        "drag.synthetic_payload",
        &["items", "dragOperationsMask", "files", "data"],
    ),
    // Row for the OBJECTS inside `synthetic_payload.items`.
    //
    // # Why these four and where they come from
    //
    // Unlike `cookie.cookies[]`, the product models no struct for this: the
    // array is passed through to CDP `Input.dispatchDragEvent` verbatim, so the
    // vocabulary is the protocol's `DragDataItem` — `mimeType` and `data`
    // required, `title` and `baseURL` optional. Taking the four from the
    // protocol rather than from the two the published formula uses is
    // deliberate: rejecting `title` would refuse a payload Chrome accepts, and
    // this validator exists to catch typos, not to narrow a protocol.
    (
        "drag.synthetic_payload.items[]",
        &["mimeType", "data", "title", "baseURL"],
    ),
];
