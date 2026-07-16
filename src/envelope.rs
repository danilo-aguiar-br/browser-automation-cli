//! JSON envelope helpers (schema_version = 1).

use serde_json::{json, Value};

use crate::error::CliError;

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

pub fn print_error_json(err: &CliError) -> Result<(), CliError> {
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
