//! JSON envelope helpers (`schema_version = 1`).
//!
//! Success and error envelopes are written to **stdout** for agent parsing.
//! Human diagnostics stay on stderr.
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

use serde_json::{json, Value};

use crate::error::CliError;

/// Print a success envelope with arbitrary JSON `data` and flush a single line.
pub fn print_success_json(data: Value) -> Result<(), CliError> {
    let env = json!({
        "schema_version": 1,
        "ok": true,
        "data": data,
    });
    println!(
        "{}",
        serde_json::to_string(&env).map_err(|e| {
            CliError::new(
                crate::error::ErrorKind::Software,
                format!("json encode: {e}"),
            )
        })?
    );
    Ok(())
}

/// Print an error envelope derived from [`CliError`].
pub fn print_error_json(err: &CliError) -> Result<(), CliError> {
    print_error_json_with_data(err, err.data().cloned())
}

/// Print an error envelope with optional partial `data` (e.g. fail-fast `run` steps).
pub fn print_error_json_with_data(err: &CliError, data: Option<Value>) -> Result<(), CliError> {
    let mut env = json!({
        "schema_version": 1,
        "ok": false,
        "error": {
            "kind": err.kind().as_str(),
            "message": err.message(),
            "exit_code": err.exit_code(),
        }
    });
    if let Some(s) = err.suggestion() {
        env["error"]["suggestion"] = json!(s);
    }
    if let Some(d) = data {
        env["data"] = d;
    }
    println!(
        "{}",
        serde_json::to_string(&env).map_err(|e| {
            CliError::new(
                crate::error::ErrorKind::Software,
                format!("json encode: {e}"),
            )
        })?
    );
    Ok(())
}
