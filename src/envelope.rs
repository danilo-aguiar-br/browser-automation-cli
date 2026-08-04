// SPDX-License-Identifier: MIT OR Apache-2.0
//! JSON envelope helpers (`schema_version = 1`).
//!
//! Success and error envelopes are written to **stdout** for agent parsing.
//! Human diagnostics stay on stderr.
//!
//! Wire format is a **single compact JSON object per line** (RFC 8259; CLI
//! semantics of `application/json`). Unknown fields on **input** follow
//! Must-Ignore at the clap boundary; envelopes themselves are fully typed
//! on serialize.
//!
//! # Success shape
//!
//! ```json
//! {"schema_version":1,"ok":true,"data":{}}
//! ```
//!
//! # Error shape
//!
//! ```json
//! {"schema_version":1,"ok":false,"error":{"kind":"unavailable","message":"...","exit_code":69}}
//! ```

use serde::Serialize;
use serde_json::Value;

use crate::error::CliError;
use crate::output;

/// Success envelope (`ok: true`) — typed wire contract for agents.
#[derive(Debug, Serialize)]
pub struct SuccessEnvelope {
    /// Envelope schema version (currently `1`).
    pub schema_version: u32,
    /// Always `true` for this shape.
    pub ok: bool,
    /// Optional agent correlation id (from `--correlation-id`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    /// Command-specific payload (dynamic by design at the CLI boundary).
    pub data: Value,
    /// What the universal data operations did, when any of them ran.
    ///
    /// Omitted entirely when no flag was passed, so an envelope nobody asked to
    /// reduce keeps its exact previous shape. When present it carries
    /// `truncated`, which is the difference between a short payload and a cut one
    /// — a distinction the agent cannot recover from the payload itself.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_ops: Option<crate::agent_ops::AgentOpsReport>,
}

/// Error object nested under an error envelope.
#[derive(Debug, Serialize)]
pub struct ErrorBody {
    /// Machine-stable error kind (`unavailable`, `data`, …).
    pub kind: String,
    /// Human/agent message (English technical).
    pub message: String,
    /// Sysexits-style process exit code.
    pub exit_code: u8,
    /// Optional recovery hint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

/// Error envelope (`ok: false`) — typed wire contract for agents.
#[derive(Debug, Serialize)]
pub struct ErrorEnvelope {
    /// Envelope schema version (currently `1`).
    pub schema_version: u32,
    /// Always `false` for this shape.
    pub ok: bool,
    /// Optional agent correlation id (from `--correlation-id`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    /// Structured error.
    pub error: ErrorBody,
    /// Optional partial data (e.g. fail-fast `run` steps already completed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Print a success envelope with arbitrary JSON `data` and flush a single line.
///
/// # Errors
///
/// Returns [`CliError`] with [`ErrorKind::BrokenPipe`](crate::error::ErrorKind::BrokenPipe)
/// when stdout is closed mid-write (exit **141** for agents).
///
/// # Examples
///
/// ```no_run
/// use browser_automation_cli::envelope::print_success_json;
/// use serde_json::json;
///
/// print_success_json(json!({"ok_detail": true})).expect("stdout");
/// ```
pub fn print_success_json(data: Value) -> Result<(), CliError> {
    // Payload reduction happens HERE, once, for all 69 commands. Doing it per
    // command produced eight inconsistent implementations and left the most
    // invoked diagnostic — `doctor`, at 26_277 bytes — with none at all.
    let (data, agent_ops) = crate::agent_ops::apply_process_ops(data)?;
    let env = SuccessEnvelope {
        schema_version: crate::constants::ENVELOPE_SCHEMA_VERSION,
        ok: true,
        correlation_id: crate::agent_context::correlation_id(),
        data,
        agent_ops,
    };
    output::write_json_line_ser(&env)
}

/// Print an error envelope derived from [`CliError`].
///
/// # Errors
///
/// Propagates stdout write failures as [`CliError`] (typically broken pipe).
pub fn print_error_json(err: &CliError) -> Result<(), CliError> {
    print_error_json_with_data(err, err.data().cloned())
}

/// Print an error envelope with optional partial `data` (e.g. fail-fast `run` steps).
///
/// # Errors
///
/// Propagates stdout write failures as [`CliError`] (typically broken pipe).
///
/// # Examples
///
/// ```no_run
/// use browser_automation_cli::envelope::print_error_json_with_data;
/// use browser_automation_cli::error::{CliError, ErrorKind};
/// use serde_json::json;
///
/// let err = CliError::new(ErrorKind::Unavailable, "chrome not found");
/// print_error_json_with_data(&err, Some(json!({"steps": []}))).ok();
/// ```
pub fn print_error_json_with_data(err: &CliError, data: Option<Value>) -> Result<(), CliError> {
    let env = ErrorEnvelope {
        schema_version: crate::constants::ENVELOPE_SCHEMA_VERSION,
        ok: false,
        correlation_id: crate::agent_context::correlation_id(),
        error: ErrorBody {
            kind: err.kind().as_str().to_string(),
            message: err.message().to_string(),
            exit_code: err.exit_code(),
            suggestion: err.suggestion().map(|s| s.to_string()),
        },
        data,
    };
    output::write_json_line_ser(&env)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;
    use serde_json::json;

    #[test]
    fn success_envelope_roundtrip_shape() {
        let env = SuccessEnvelope {
            schema_version: crate::constants::ENVELOPE_SCHEMA_VERSION,
            ok: true,
            correlation_id: None,
            data: json!({"x": 1}),
            agent_ops: None,
        };
        let s = crate::json_util::to_compact_string(&env).unwrap();
        let v: Value = crate::json_util::from_str(&s).unwrap();
        assert_eq!(
            v["schema_version"],
            crate::constants::ENVELOPE_SCHEMA_VERSION
        );
        assert_eq!(v["ok"], true);
        assert_eq!(v["data"]["x"], 1);
        assert!(!s.contains('\n'));
    }

    #[test]
    fn error_envelope_omits_empty_suggestion() {
        let err = CliError::new(ErrorKind::Data, "bad");
        let env = ErrorEnvelope {
            schema_version: crate::constants::ENVELOPE_SCHEMA_VERSION,
            ok: false,
            correlation_id: None,
            error: ErrorBody {
                kind: err.kind().as_str().to_string(),
                message: err.message().to_string(),
                exit_code: err.exit_code(),
                suggestion: None,
            },
            data: None,
        };
        let s = crate::json_util::to_compact_string(&env).unwrap();
        let v: Value = crate::json_util::from_str(&s).unwrap();
        assert!(v.get("data").is_none());
        assert!(v["error"].get("suggestion").is_none());
    }
}
