// SPDX-License-Identifier: MIT OR Apache-2.0
//! Declarative capability and precondition table (GAP-010 / GAP-029 / GAP-041).
//!
//! Every command that needs a gate flag, a capture flag, or a page-state
//! precondition declares it **here**, once. The three consumers read the same
//! table instead of re-encoding the rule:
//!
//! | Consumer | Reads |
//! |----------|-------|
//! | `commands::dispatch::gates` | top-level argv dispatch |
//! | `commands::run::execute` | per-step dispatch inside `run` / `exec` |
//! | `commands::run::preflight` | whole-script check before BORN |
//!
//! Applying a gate command-by-command is what produced GAP-010: `heap take`,
//! `heap summary` and `heap close` silently escaped `--category-memory` because
//! the rule lived in an inline `matches!` instead of a table anyone could audit.
//!
//! # Keys
//!
//! A key is `command` or `command.action`, always kebab-case. Lookup falls back
//! from the specific key to the bare command, so `heap.summary` resolves through
//! its own row while an unlisted `heap.foo` inherits the `heap` row.
//!
//! # Blank-page policy for the read family (GAP-020)
//!
//! The inconsistency GAP-020 reports — `view` refusing while `cookie list`
//! succeeds with an empty list — was never a stated rule. This is the rule:
//!
//! | Command shape | On a blank page |
//! |---------------|-----------------|
//! | Returns a **collection** (`cookie`, `console`, `net`) | succeed, report `empty: true` |
//! | Returns the **`@eN` tree** (`view`) | refuse with [`Precondition`](crate::error::ErrorKind::Precondition), opt out via `--allow-empty` |
//! | Resolves a **`@eN` target** (`text`, `attr`) | fail on the ref lookup, already not a silent success |
//!
//! The split is not arbitrary. An empty collection is a truthful answer. An empty
//! `@eN` tree is not an answer at all: every later step naming a ref from it is
//! already broken, so returning `ok` buys a failure attributed to the wrong
//! command several steps downstream.
//!
//! Refusal is `Precondition` (75), never `Usage` (2): the argv was correct and the
//! remediation is to navigate.

mod table;

#[cfg(test)]
mod tests;

pub use table::{capability_rows, invalidates_refs, CapabilityRow};

use crate::error::{CliError, ErrorKind};

/// A gate flag that must be enabled for a command to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Capability {
    /// `--category-memory`
    Memory,
    /// `--category-extensions`
    Extensions,
    /// `--category-third-party`
    ThirdParty,
    /// `--category-webmcp`
    Webmcp,
    /// `--experimental-vision`
    Vision,
    /// `--experimental-screencast`
    Screencast,
    /// `--capture-console`
    CaptureConsole,
    /// `--capture-network`
    CaptureNetwork,
}

impl Capability {
    /// CLI flag that enables this capability.
    #[must_use]
    pub fn flag(self) -> &'static str {
        match self {
            Capability::Memory => "--category-memory",
            Capability::Extensions => "--category-extensions",
            Capability::ThirdParty => "--category-third-party",
            Capability::Webmcp => "--category-webmcp",
            Capability::Vision => "--experimental-vision",
            Capability::Screencast => "--experimental-screencast",
            Capability::CaptureConsole => "--capture-console",
            Capability::CaptureNetwork => "--capture-network",
        }
    }

    /// Stable machine name for `commands` / `schema` envelopes.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Capability::Memory => "category-memory",
            Capability::Extensions => "category-extensions",
            Capability::ThirdParty => "category-third-party",
            Capability::Webmcp => "category-webmcp",
            Capability::Vision => "experimental-vision",
            Capability::Screencast => "experimental-screencast",
            Capability::CaptureConsole => "capture-console",
            Capability::CaptureNetwork => "capture-network",
        }
    }

    /// i18n suggestion key naming the remediation flag.
    #[must_use]
    pub fn suggestion_key(self) -> &'static str {
        match self {
            Capability::Memory => "category_memory",
            Capability::Extensions => "category_extensions",
            Capability::ThirdParty => "third_party_flag",
            Capability::Webmcp => "webmcp_flag",
            Capability::Vision => "vision_required",
            Capability::Screencast => "screencast_flag",
            Capability::CaptureConsole => "capture_console",
            Capability::CaptureNetwork => "capture_network",
        }
    }

    /// True when the capability is a capture buffer rather than a policy gate.
    ///
    /// Capture flags shape how the session is launched, so `run` preflight has to
    /// check them against [`crate::browser::CaptureOpts`] instead of `RunFlags`.
    #[must_use]
    pub fn is_capture(self) -> bool {
        matches!(
            self,
            Capability::CaptureConsole | Capability::CaptureNetwork
        )
    }

    /// Error for a command invoked while this capability is disabled (GAP-011).
    #[must_use]
    pub fn disabled_error(self, cmd: &str) -> CliError {
        CliError::with_suggestion(
            ErrorKind::CapabilityDisabled,
            format!("{cmd} requires {}", self.flag()),
            crate::i18n::suggestion_key(self.suggestion_key(), None),
        )
    }
}

/// A page/session state condition that must hold before a command runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precondition {
    /// No JavaScript dialog may be open on the active page (GAP-041).
    ///
    /// Mirrors `blockedByDialog: true` in the reference tool surface. A command
    /// dispatched behind an open dialog either does nothing or reads a frozen
    /// page, and returns `ok` either way — the silent-success class this removes.
    NoDialogOpen,
}

impl Precondition {
    /// Stable machine name for `commands` / `schema` envelopes.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Precondition::NoDialogOpen => "no-dialog-open",
        }
    }

    /// Error for a command invoked while this precondition is unmet (GAP-020/041).
    #[must_use]
    pub fn unmet_error(self, cmd: &str) -> CliError {
        match self {
            Precondition::NoDialogOpen => CliError::with_suggestion(
                ErrorKind::Precondition,
                format!("{cmd} is blocked while a JavaScript dialog is open"),
                crate::i18n::suggestion_key("dialog_open_required", None),
            ),
        }
    }
}

/// Look up the row governing `cmd` (optionally `action`).
///
/// Resolution is specific-first: `heap` + `summary` tries `heap.summary`, then
/// falls back to `heap`. Returns `None` when the command declares no gate.
#[must_use]
pub fn lookup(cmd: &str, action: Option<&str>) -> Option<&'static CapabilityRow> {
    let cmd = normalize(cmd);
    if let Some(action) = action {
        let action = normalize(action);
        if let Some(row) = capability_rows()
            .iter()
            .find(|r| r.matches_key(&cmd, Some(&action)))
        {
            return Some(row);
        }
    }
    capability_rows().iter().find(|r| r.matches_key(&cmd, None))
}

/// Capabilities required by `cmd`/`action`, or an empty slice.
#[must_use]
pub fn required_capabilities(cmd: &str, action: Option<&str>) -> &'static [Capability] {
    lookup(cmd, action).map_or(&[], |r| r.capabilities)
}

/// Preconditions required by `cmd`/`action`, or an empty slice.
#[must_use]
pub fn required_preconditions(cmd: &str, action: Option<&str>) -> &'static [Precondition] {
    lookup(cmd, action).map_or(&[], |r| r.preconditions)
}

/// Normalize a command or action token: lowercase, `_` folded to `-`.
fn normalize(token: &str) -> String {
    token.trim().to_ascii_lowercase().replace('_', "-")
}
