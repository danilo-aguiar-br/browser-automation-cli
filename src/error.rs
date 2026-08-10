// SPDX-License-Identifier: MIT OR Apache-2.0
//! Typed CLI errors with sysexits-style exit codes.
//!
//! # Error kinds
//!
//! [`ErrorKind`](crate::error::ErrorKind) maps to process exit codes used by the
//! binary and JSON envelopes. Stable machine strings come from
//! [`ErrorKind::as_str`](crate::error::ErrorKind::as_str).
//!
//! # Conversions
//!
//! - [`CliError::exit_code`](crate::error::CliError::exit_code) /
//!   [`ErrorKind::exit_code`](crate::error::ErrorKind::exit_code) — infallible
//!   `u8` mapping (zero-cost)
//! - [`ErrorKind::as_str`](crate::error::ErrorKind::as_str) — static
//!   `&'static str` for JSON `error.kind` (zero-cost)
//! - Prefer these helpers over ad-hoc `as` casts when mapping to process status
//! - There is no fallible `TryFrom` path: every kind has a fixed exit code
//!
//! # Cost
//!
//! | Operation | Cost |
//! |-----------|------|
//! | `exit_code()` | O(1), no allocation |
//! | `as_str()` | O(1), static slice |
//! | `CliError::new` / `with_suggestion` | allocates `String` message (and optional suggestion) |
//!
//! # Examples
//!
//! ```
//! use browser_automation_cli::error::{CliError, ErrorKind};
//!
//! let err = CliError::with_suggestion(
//!     ErrorKind::Unavailable,
//!     "chrome not found",
//!     browser_automation_cli::i18n::suggestion_key("external_binary_path", None),
//! );
//! assert_eq!(err.exit_code(), 69);
//! assert_eq!(err.kind().as_str(), "unavailable");
//! ```
//!
//! # See also
//!
//! - [`crate::envelope`] for JSON success/error envelopes
//! - [`crate::exit_code_for`] for library callers that already hold a
//!   [`CliError`](crate::error::CliError)

use thiserror::Error;

/// High-level failure category mapped to a process exit code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// Invalid usage or clap parse failure (`2`).
    Usage,
    /// The origin served a bot check instead of content (`6`).
    ///
    /// A challenge is an **HTTP 200 with valid HTML**, so every transport-level
    /// field (`status_code`, `http_error`, `http_attempts`) reports success while
    /// the body carries a CAPTCHA. Without this kind the agent quotes the block
    /// page as content and retries against a WAF, which escalates toward a ban.
    /// Remediation is a different engine, a proxy, or waiting -- never a retry of
    /// the same request.
    Blocked,
    /// Capability disabled by policy: argv is correct, a gate flag is missing (`64`).
    ///
    /// Distinct from [`ErrorKind::Usage`] on purpose (GAP-011). The remediation is
    /// to add `--category-*` / `--experimental-*`, never to fix argv. Collapsing
    /// the two sends the agent into a correction loop over an argv that was
    /// already right.
    CapabilityDisabled,
    /// Page or session state does not satisfy the command's precondition (`75`).
    ///
    /// Also distinct from [`ErrorKind::Usage`] (GAP-020 / GAP-041): the argv is
    /// syntactically valid and the fix is an action — navigate first, or answer
    /// the open dialog — not an argv edit.
    Precondition,
    /// Invalid data or payload (`65`).
    Data,
    /// Required input missing (`66`).
    NoInput,
    /// Browser or dependency unavailable (`69`).
    Unavailable,
    /// Internal software failure (`70`).
    Software,
    /// Browser runtime failure (`70`).
    Browser,
    /// CDP/protocol failure (`70`).
    Protocol,
    /// Wall-clock timeout (`124`).
    Timeout,
    /// Cancelled by OS signal (`130`).
    ///
    /// One-shot product contract: **both** SIGINT and SIGTERM map to
    /// [`ErrorKind::Cancelled`] / exit **130** (not sysexits-style 143 for
    /// SIGTERM). Agents and golden tests standardize on 130 for cooperative
    /// cancel. Residual SIGTERM of a *child* Chrome/Lightpanda PID during
    /// FINALIZE is unrelated to this process exit code.
    Cancelled,
    /// Broken pipe on stdout (`141`).
    BrokenPipe,
    /// Configuration error (`78`).
    Config,
    /// I/O failure (`74`).
    Io,
}

impl ErrorKind {
    /// Process exit code for this kind.
    pub fn exit_code(self) -> u8 {
        match self {
            ErrorKind::Usage => 2,
            ErrorKind::Blocked => 6,
            ErrorKind::CapabilityDisabled => 64,
            ErrorKind::Data => 65,
            ErrorKind::NoInput => 66,
            ErrorKind::Unavailable => 69,
            ErrorKind::Software | ErrorKind::Browser | ErrorKind::Protocol => 70,
            ErrorKind::Precondition => 75,
            ErrorKind::Config => 78,
            ErrorKind::Io => 74,
            ErrorKind::Timeout => 124,
            ErrorKind::Cancelled => 130,
            ErrorKind::BrokenPipe => 141,
        }
    }

    /// Stable machine-readable kind string for JSON envelopes.
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorKind::Usage => "usage",
            ErrorKind::Blocked => "blocked",
            ErrorKind::CapabilityDisabled => "capability-disabled",
            ErrorKind::Precondition => "precondition",
            ErrorKind::Data => "data",
            ErrorKind::NoInput => "no-input",
            ErrorKind::Unavailable => "unavailable",
            ErrorKind::Software => "software",
            ErrorKind::Browser => "browser",
            ErrorKind::Protocol => "protocol",
            ErrorKind::Timeout => "timeout",
            ErrorKind::Cancelled => "cancelled",
            ErrorKind::BrokenPipe => "broken-pipe",
            ErrorKind::Config => "config",
            ErrorKind::Io => "io",
        }
    }

    /// Every variant, for exhaustiveness tests and round-trip parsing.
    pub const ALL: &'static [ErrorKind] = &[
        ErrorKind::Usage,
        ErrorKind::Blocked,
        ErrorKind::CapabilityDisabled,
        ErrorKind::Precondition,
        ErrorKind::Data,
        ErrorKind::NoInput,
        ErrorKind::Unavailable,
        ErrorKind::Software,
        ErrorKind::Browser,
        ErrorKind::Protocol,
        ErrorKind::Timeout,
        ErrorKind::Cancelled,
        ErrorKind::BrokenPipe,
        ErrorKind::Config,
        ErrorKind::Io,
    ];

    /// Parse a kind back from its [`ErrorKind::as_str`] form.
    ///
    /// Returns `None` for an unknown string so the caller decides what an
    /// unrecognized kind means. Callers that need to re-raise a kind carried
    /// through JSON **must** use this instead of a hand-written `match`: a local
    /// match with a `_ => Software` arm silently downgrades every kind the author
    /// forgot, which is how `precondition` (75) and `capability-disabled` (64)
    /// were reported as `software` (70) in the `run` envelope.
    // Not `std::str::FromStr`: that trait returns `Result<Self, Err>`, while an
    // unknown kind here is a legitimate `None` (callers fall back explicitly).
    #[allow(clippy::should_implement_trait)]
    #[must_use]
    pub fn from_str(kind: &str) -> Option<ErrorKind> {
        ErrorKind::ALL.iter().copied().find(|k| k.as_str() == kind)
    }
}

/// Typed CLI error with optional remediation hint.
#[derive(Debug, Clone, Error)]
#[error("{message}")]
pub struct CliError {
    kind: ErrorKind,
    message: String,
    suggestion: Option<String>,
    /// Optional partial JSON payload (e.g. fail-fast `run` steps).
    data: Option<serde_json::Value>,
}

impl CliError {
    /// Create an error without a suggestion.
    ///
    /// Marked `#[cold]`: success paths dominate; keeps error construction out of
    /// the predicted hot branch layout (rules_rust_eficiencia_e_performance).
    #[cold]
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            suggestion: None,
            data: None,
        }
    }

    /// Create an error with a short remediation suggestion for agents.
    #[cold]
    pub fn with_suggestion(
        kind: ErrorKind,
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            suggestion: Some(suggestion.into()),
            data: None,
        }
    }

    /// Attach partial JSON `data` (kept on error envelopes).
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Error category.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Human-readable message (also used in JSON `error.message`).
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Optional remediation hint.
    pub fn suggestion(&self) -> Option<&str> {
        self.suggestion.as_deref()
    }

    /// Optional partial data payload.
    pub fn data(&self) -> Option<&serde_json::Value> {
        self.data.as_ref()
    }

    /// Process exit code for this error.
    pub fn exit_code(&self) -> u8 {
        self.kind.exit_code()
    }
}

#[cfg(test)]
mod kind_roundtrip_tests {
    use super::ErrorKind;

    /// Every kind must survive `as_str` -> `from_str` unchanged.
    ///
    /// This is what stops a new kind from silently degrading to `software` when
    /// an envelope carries it across a process or module boundary.
    #[test]
    fn every_kind_round_trips_through_its_string() {
        for kind in ErrorKind::ALL {
            let s = kind.as_str();
            assert_eq!(
                ErrorKind::from_str(s),
                Some(*kind),
                "kind {s} does not round-trip"
            );
        }
    }

    /// `ALL` must list every variant; a missing one breaks the round-trip.
    #[test]
    fn all_covers_every_variant() {
        // Exhaustive match: adding a variant without listing it fails to compile.
        for kind in ErrorKind::ALL {
            match kind {
                ErrorKind::Usage
                | ErrorKind::Blocked
                | ErrorKind::CapabilityDisabled
                | ErrorKind::Precondition
                | ErrorKind::Data
                | ErrorKind::NoInput
                | ErrorKind::Unavailable
                | ErrorKind::Software
                | ErrorKind::Browser
                | ErrorKind::Protocol
                | ErrorKind::Timeout
                | ErrorKind::Cancelled
                | ErrorKind::BrokenPipe
                | ErrorKind::Config
                | ErrorKind::Io => {}
            }
        }
        // Completeness in the other direction, which the loop above cannot prove:
        // every variant that exists must APPEAR in `ALL`. A frozen `ALL.len()`
        // literal was checked here before and could not tell "all listed" from
        // "one missing plus one duplicated"; `kind_strings_are_unique` covers the
        // duplicate half, and this covers the missing half.
        for kind in [
            ErrorKind::Usage,
            ErrorKind::Blocked,
            ErrorKind::CapabilityDisabled,
            ErrorKind::Precondition,
            ErrorKind::Data,
            ErrorKind::NoInput,
            ErrorKind::Unavailable,
            ErrorKind::Software,
            ErrorKind::Browser,
            ErrorKind::Protocol,
            ErrorKind::Timeout,
            ErrorKind::Cancelled,
            ErrorKind::BrokenPipe,
            ErrorKind::Config,
            ErrorKind::Io,
        ] {
            assert!(
                ErrorKind::ALL.contains(&kind),
                "{} is missing from ErrorKind::ALL",
                kind.as_str()
            );
        }
    }

    /// Kind strings must be unique, or `from_str` would be ambiguous.
    #[test]
    fn kind_strings_are_unique() {
        let mut seen: Vec<&str> = ErrorKind::ALL.iter().map(|k| k.as_str()).collect();
        seen.sort_unstable();
        let before = seen.len();
        seen.dedup();
        assert_eq!(before, seen.len(), "duplicate kind string");
    }

    /// The two kinds this taxonomy work added keep their PRD exit codes.
    #[test]
    fn new_kinds_keep_their_exit_codes() {
        assert_eq!(ErrorKind::CapabilityDisabled.exit_code(), 64);
        assert_eq!(ErrorKind::Precondition.exit_code(), 75);
        assert_eq!(ErrorKind::Usage.exit_code(), 2);
    }

    #[test]
    fn unknown_kind_is_none_not_a_silent_default() {
        assert_eq!(ErrorKind::from_str("not-a-kind"), None);
    }
}
