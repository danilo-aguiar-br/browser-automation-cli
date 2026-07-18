//! Typed CLI errors with sysexits-style exit codes.
//!
//! # Error kinds
//!
//! `ErrorKind` maps to process exit codes used by the binary and JSON envelopes.
//!
//! # Examples
//!
//! ```
//! use browser_automation_cli::error::{CliError, ErrorKind};
//!
//! let err = CliError::with_suggestion(
//!     ErrorKind::Unavailable,
//!     "chrome not found",
//!     "install Chromium or Google Chrome",
//! );
//! assert_eq!(err.exit_code(), 69);
//! assert_eq!(err.kind().as_str(), "unavailable");
//! ```

use thiserror::Error;

/// High-level failure category mapped to a process exit code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// Invalid usage or clap parse failure (`2`).
    Usage,
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
    /// Cancelled by signal (`130`).
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
            ErrorKind::Data => 65,
            ErrorKind::NoInput => 66,
            ErrorKind::Unavailable => 69,
            ErrorKind::Software | ErrorKind::Browser | ErrorKind::Protocol => 70,
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
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            suggestion: None,
            data: None,
        }
    }

    /// Create an error with a short remediation suggestion for agents.
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

