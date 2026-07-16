//! Typed CLI errors with sysexits-style exit codes.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Usage,
    Data,
    NoInput,
    Unavailable,
    Software,
    Browser,
    Protocol,
    Timeout,
    Cancelled,
    BrokenPipe,
    Config,
    Io,
}

impl ErrorKind {
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

#[derive(Debug, Clone)]
pub struct CliError {
    kind: ErrorKind,
    message: String,
    suggestion: Option<String>,
}

impl CliError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            suggestion: None,
        }
    }

    pub fn with_suggestion(
        kind: ErrorKind,
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            suggestion: Some(suggestion.into()),
        }
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn suggestion(&self) -> Option<&str> {
        self.suggestion.as_deref()
    }

    pub fn exit_code(&self) -> u8 {
        self.kind.exit_code()
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CliError {}
