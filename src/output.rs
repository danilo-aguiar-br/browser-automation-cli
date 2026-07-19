// SPDX-License-Identifier: MIT OR Apache-2.0
//! Canonical stdout/stderr writers for the agent contract.
//!
//! Rules (`rules_rust_cli_stdin_stdout`):
//! - **stdout** = structured data only (JSON envelope, NDJSON steps, human one-liners)
//! - **stderr** = logs / progress / human diagnostics
//! - Never mix streams; always `write_all` + explicit flush on record boundaries
//! - Map `ErrorKind::BrokenPipe` → exit **141**
//!
//! All agent-consumable lines should go through this module (not raw `println!`).

use std::io::{self, Write};

use crate::error::{CliError, ErrorKind};

/// Map an I/O error from stdout/stderr into a typed [`CliError`].
///
/// Borrows the error (kind + Display only) so callers keep the original if needed
/// (rules: prefer `&T` when the body does not consume ownership).
pub fn map_io_error(err: &io::Error, stream: &str) -> CliError {
    if err.kind() == io::ErrorKind::BrokenPipe {
        CliError::new(
            ErrorKind::BrokenPipe,
            format!("{stream}: broken pipe"),
        )
    } else {
        CliError::new(ErrorKind::Io, format!("{stream}: {err}"))
    }
}

/// Write raw bytes to stdout with `write_all` (no trailing newline).
pub fn write_stdout(bytes: &[u8]) -> Result<(), CliError> {
    let mut out = io::stdout().lock();
    out.write_all(bytes)
        .map_err(|e| map_io_error(&e, "stdout"))?;
    Ok(())
}

/// Write one complete line to stdout (adds `\n`) and flush immediately.
///
/// Use for JSON envelopes, NDJSON steps, and human one-line results.
pub fn writeln_stdout(line: impl AsRef<str>) -> Result<(), CliError> {
    let mut out = io::stdout().lock();
    out.write_all(line.as_ref().as_bytes())
        .map_err(|e| map_io_error(&e, "stdout"))?;
    out.write_all(b"\n")
        .map_err(|e| map_io_error(&e, "stdout"))?;
    out.flush().map_err(|e| map_io_error(&e, "stdout"))?;
    Ok(())
}

/// Write one complete line to stderr (adds `\n`) and flush.
pub fn writeln_stderr(line: impl AsRef<str>) -> Result<(), CliError> {
    let mut err = io::stderr().lock();
    err.write_all(line.as_ref().as_bytes())
        .map_err(|e| map_io_error(&e, "stderr"))?;
    err.write_all(b"\n")
        .map_err(|e| map_io_error(&e, "stderr"))?;
    err.flush().map_err(|e| map_io_error(&e, "stderr"))?;
    Ok(())
}

/// Flush stdout (call before process return when partial buffers may remain).
pub fn flush_stdout() -> Result<(), CliError> {
    io::stdout()
        .flush()
        .map_err(|e| map_io_error(&e, "stdout"))
}

/// Flush stderr.
pub fn flush_stderr() -> Result<(), CliError> {
    io::stderr()
        .flush()
        .map_err(|e| map_io_error(&e, "stderr"))
}

/// Serialize `value` as **compact** JSON (no pretty print) and write one LF line
/// to stdout (flushed). Used for agent envelopes and NDJSON steps.
///
/// Never emits UTF-8 BOM. One complete JSON value per line (NDJSON-safe).
pub fn write_json_line(value: &serde_json::Value) -> Result<(), CliError> {
    write_json_line_ser(value)
}

/// Serialize any [`serde::Serialize`] value as compact JSON and write one LF line.
pub fn write_json_line_ser<T: serde::Serialize>(value: &T) -> Result<(), CliError> {
    let s = crate::json_util::to_compact_string(value)?;
    // Defense: compact encoder must not introduce embedded newlines (would break NDJSON).
    if s.as_bytes().contains(&b'\n') {
        return Err(CliError::new(
            ErrorKind::Software,
            "json encode produced embedded newline (breaks NDJSON/envelope contract)",
        ));
    }
    writeln_stdout(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broken_pipe_maps_to_141() {
        let io_err = io::Error::new(io::ErrorKind::BrokenPipe, "pipe");
        let err = map_io_error(&io_err, "stdout");
        assert_eq!(err.kind(), ErrorKind::BrokenPipe);
        assert_eq!(err.exit_code(), 141);
    }

    #[test]
    fn other_io_maps_to_74() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "nope");
        let err = map_io_error(&io_err, "stdout");
        assert_eq!(err.kind(), ErrorKind::Io);
        assert_eq!(err.exit_code(), 74);
    }
}
