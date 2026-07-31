// SPDX-License-Identifier: MIT OR Apache-2.0
//! The capability / precondition rows themselves (GAP-010 / GAP-029 / GAP-041).

use super::{Capability, Precondition};

/// One row of the declarative gate table.
#[derive(Debug, Clone, Copy)]
pub struct CapabilityRow {
    /// Command name, kebab-case (`heap`, `click-at`).
    pub cmd: &'static str,
    /// Optional action within the command (`summary`, `start`).
    ///
    /// `None` is the catch-all row for the command.
    pub action: Option<&'static str>,
    /// Gate flags that must be enabled.
    pub capabilities: &'static [Capability],
    /// Page/session state that must hold.
    pub preconditions: &'static [Precondition],
}

impl CapabilityRow {
    /// True when this row governs `cmd` (+ `action`), both already normalized.
    #[must_use]
    pub fn matches_key(&self, cmd: &str, action: Option<&str>) -> bool {
        self.cmd == cmd && self.action == action
    }
}

const MEMORY: &[Capability] = &[Capability::Memory];
const EXTENSIONS: &[Capability] = &[Capability::Extensions];
const THIRD_PARTY: &[Capability] = &[Capability::ThirdParty];
const WEBMCP: &[Capability] = &[Capability::Webmcp];
const VISION: &[Capability] = &[Capability::Vision];
const SCREENCAST: &[Capability] = &[Capability::Screencast];
const CONSOLE: &[Capability] = &[Capability::CaptureConsole];
const NETWORK: &[Capability] = &[Capability::CaptureNetwork];
const NONE: &[Capability] = &[];

const BLOCKED: &[Precondition] = &[Precondition::NoDialogOpen];
const FREE: &[Precondition] = &[];

/// The gate table.
///
/// # Dialog guard (GAP-041)
///
/// `NoDialogOpen` mirrors `blockedByDialog: true` in the reference surface, whose
/// declarations live in `snapshot.ts`, `script.ts`, `screenshot.ts`,
/// `performance.ts`, `network.ts`, `memory.ts`, `input.ts`, `emulation.ts` and
/// `lighthouse.ts`. Deliberately **not** guarded:
///
/// - `dialog` — it is the remedy; guarding it would deadlock the agent
/// - `goto` / `back` / `forward` / `reload` — navigation dismisses the dialog and
///   already clears the flag
/// - `page`, `cookie`, `console`, `net` listing, `assert`, `wait` — read session
///   state that stays truthful behind a dialog
///
/// # Heap (GAP-010)
///
/// Only snapshot capture (`take`) is free. The other eleven actions are gated,
/// matching the reference surface. `summary` and `close` used to escape the gate
/// because the rule was an inline `matches!` rather than this table.
#[must_use]
pub fn capability_rows() -> &'static [CapabilityRow] {
    const ROWS: &[CapabilityRow] = &[
        // --- memory (GAP-010): only `take` is free ---
        row("heap", Some("take"), NONE, BLOCKED),
        row("heap", None, MEMORY, BLOCKED),
        // --- other policy gates ---
        row("extension", None, EXTENSIONS, FREE),
        row("devtools3p", None, THIRD_PARTY, FREE),
        row("devtools3p-list", None, THIRD_PARTY, FREE),
        row("devtools3p-exec", None, THIRD_PARTY, FREE),
        row("webmcp", None, WEBMCP, FREE),
        row("webmcp-list", None, WEBMCP, FREE),
        row("webmcp-exec", None, WEBMCP, FREE),
        row("screencast", None, SCREENCAST, FREE),
        row("click-at", None, VISION, BLOCKED),
        // --- capture buffers (GAP-029) ---
        row("console", None, CONSOLE, FREE),
        row("net", None, NETWORK, BLOCKED),
        row("assert", Some("console"), CONSOLE, FREE),
        row("assert", Some("console-empty"), CONSOLE, FREE),
        row("assert", Some("console-no-match"), CONSOLE, FREE),
        // --- dialog-blocked surfaces with no capability gate (GAP-041) ---
        row("view", None, NONE, BLOCKED),
        row("eval", None, NONE, BLOCKED),
        row("grab", None, NONE, BLOCKED),
        row("screenshot", None, NONE, BLOCKED),
        row("perf", None, NONE, BLOCKED),
        row("lighthouse", None, NONE, BLOCKED),
        row("emulate", None, NONE, BLOCKED),
        row("resize", None, NONE, BLOCKED),
        row("press", None, NONE, BLOCKED),
        row("click", None, NONE, BLOCKED),
        row("write", None, NONE, BLOCKED),
        row("fill", None, NONE, BLOCKED),
        row("fill-form", None, NONE, BLOCKED),
        row("type", None, NONE, BLOCKED),
        row("keys", None, NONE, BLOCKED),
        row("hover", None, NONE, BLOCKED),
        row("drag", None, NONE, BLOCKED),
        row("submit", None, NONE, BLOCKED),
        row("upload", None, NONE, BLOCKED),
        row("scroll", None, NONE, BLOCKED),
        row("select-option", None, NONE, BLOCKED),
        row("pick", None, NONE, BLOCKED),
        row("text", None, NONE, BLOCKED),
        row("attr", None, NONE, BLOCKED),
        row("extract", None, NONE, BLOCKED),
    ];
    ROWS
}

/// Commands whose effect can invalidate previously issued `@eN` refs (GAP-042).
///
/// The snapshot stays opt-in via `--include-snapshot`, which is what keeps the
/// envelope small. This list is the missing half of that trade: the agent is told
/// its tree went stale instead of discovering it a step later, on the wrong
/// command.
const REF_INVALIDATING: &[&str] = &[
    // DOM interaction
    "press",
    "click",
    "click-at",
    "write",
    "fill",
    "fill-form",
    "type",
    "keys",
    "hover",
    "drag",
    "submit",
    "upload",
    "select-option",
    "pick",
    "scroll",
    // Navigation replaces the document outright
    "goto",
    "back",
    "forward",
    "reload",
    // Arbitrary script may mutate anything
    "eval",
    // Answering a dialog resumes script execution that can mutate the DOM
    "dialog",
];

/// True when `cmd` may invalidate `@eN` refs taken before it (GAP-042).
#[must_use]
pub fn invalidates_refs(cmd: &str) -> bool {
    let cmd = cmd.trim().to_ascii_lowercase().replace('_', "-");
    REF_INVALIDATING.contains(&cmd.as_str())
}

/// `const fn` row constructor so the table stays a single readable literal.
const fn row(
    cmd: &'static str,
    action: Option<&'static str>,
    capabilities: &'static [Capability],
    preconditions: &'static [Precondition],
) -> CapabilityRow {
    CapabilityRow {
        cmd,
        action,
        capabilities,
        preconditions,
    }
}
