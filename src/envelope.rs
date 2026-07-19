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
    /// Command-specific payload (dynamic by design at the CLI boundary).
    pub data: Value,
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
    /// Structured error.
    pub error: ErrorBody,
    /// Optional partial data (e.g. fail-fast `run` steps already completed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Print a success envelope with arbitrary JSON `data` and flush a single line.
pub fn print_success_json(data: Value) -> Result<(), CliError> {
    let env = SuccessEnvelope {
        schema_version: 1,
        ok: true,
        data,
    };
    output::write_json_line_ser(&env)
}

/// Print an error envelope derived from [`CliError`].
pub fn print_error_json(err: &CliError) -> Result<(), CliError> {
    print_error_json_with_data(err, err.data().cloned())
}

/// Print an error envelope with optional partial `data` (e.g. fail-fast `run` steps).
pub fn print_error_json_with_data(err: &CliError, data: Option<Value>) -> Result<(), CliError> {
    let env = ErrorEnvelope {
        schema_version: 1,
        ok: false,
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
            schema_version: 1,
            ok: true,
            data: json!({"x": 1}),
        };
        let s = crate::json_util::to_compact_string(&env).unwrap();
        let v: Value = crate::json_util::from_str(&s).unwrap();
        assert_eq!(v["schema_version"], 1);
        assert_eq!(v["ok"], true);
        assert_eq!(v["data"]["x"], 1);
        assert!(!s.contains('\n'));
    }

    #[test]
    fn error_envelope_omits_empty_suggestion() {
        let err = CliError::new(ErrorKind::Data, "bad");
        let env = ErrorEnvelope {
            schema_version: 1,
            ok: false,
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
