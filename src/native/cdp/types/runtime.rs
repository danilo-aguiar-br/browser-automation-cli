// SPDX-License-Identifier: MIT OR Apache-2.0
//! CDP Runtime domain types.
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Mirrors CDP `Runtime.evaluate` request.
///
/// Evaluates expression on global object.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateParams {
    /// JavaScript source evaluated in the page's global scope.
    pub expression: String,
    /// Whether the result is expected to be a JSON object that should be sent by value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_by_value: Option<bool>,
    /// Whether execution should `await` for resulting value and return once awaited promise is
    /// resolved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub await_promise: Option<bool>,
}

/// Mirrors CDP `Runtime.evaluate` response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateResult {
    /// Evaluation result.
    pub result: RemoteObject,
    /// Set when the expression threw. `result` is still present in that case, so
    /// ignoring this field turns a thrown error into a silent value.
    pub exception_details: Option<ExceptionDetails>,
}

/// Mirrors CDP `Runtime.RemoteObject`.
///
/// Mirror object referencing original JavaScript object.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteObject {
    /// JavaScript type: `object`, `function`, `undefined`, `string`, `number`,
    /// `boolean`, `symbol`, `bigint`.
    ///
    /// Renamed because `type` is a Rust keyword; the wire name stays `type`.
    #[serde(rename = "type")]
    pub object_type: String,
    /// Object subtype hint. Specified for `object` type values only. NOTE: If you change
    /// anything here, make sure to also update `subtype` in `ObjectPreview` and
    /// `PropertyPreview` below.
    pub subtype: Option<String>,
    /// Remote object value in case of primitive values or JSON values (if it was requested).
    pub value: Option<Value>,
    /// String representation of the object.
    pub description: Option<String>,
    /// Unique object identifier (for non-primitive values).
    pub object_id: Option<String>,
    /// Object class (constructor) name. Specified for `object` type values only.
    pub class_name: Option<String>,
    /// Primitive value which can not be JSON-stringified does not have `value`, but gets this
    /// property.
    pub unserializable_value: Option<String>,
    /// Structured preview Chrome sends for objects that were not returned by
    /// value. Left opaque because nothing here walks it.
    pub preview: Option<Value>,
}

/// Mirrors CDP `Runtime.ExceptionDetails`.
///
/// Detailed information about exception (or error) that was thrown during script compilation or
/// execution.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionDetails {
    /// Exception text, which should be used together with exception object when available.
    pub text: String,
    /// Exception object if available.
    pub exception: Option<RemoteObject>,
    /// Line number of the exception location (0-based).
    pub line_number: Option<i64>,
    /// Column number of the exception location (0-based).
    pub column_number: Option<i64>,
}

/// Mirrors CDP `Runtime.consoleAPICalled` event.
///
/// Issued when console API was called.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleApiCalledEvent {
    /// Console method that was called: `log`, `warn`, `error`, `debug`, `info`, …
    ///
    /// This is what `console list --level` filters on. Renamed because `type` is
    /// a Rust keyword; the wire name stays `type`.
    #[serde(rename = "type")]
    pub call_type: String,
    /// Call arguments, one remote object per argument passed to the console call.
    pub args: Vec<RemoteObject>,
    /// Call timestamp in milliseconds since epoch.
    pub timestamp: Option<f64>,
}

// Runtime.exceptionThrown
/// Mirrors CDP `Runtime.exceptionThrown` event.
///
/// Issued when exception was thrown and unhandled.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionThrownEvent {
    /// Timestamp of the exception.
    pub timestamp: f64,
    /// Details of the exception that went unhandled.
    pub exception_details: ExceptionDetails,
}

// ---------------------------------------------------------------------------
// Accessibility domain
// ---------------------------------------------------------------------------
