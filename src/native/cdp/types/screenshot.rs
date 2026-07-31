// SPDX-License-Identifier: MIT OR Apache-2.0
//! CDP screenshot / callFunctionOn / browser version types.
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Mirrors CDP `Page.captureScreenshot` request.
///
/// Capture page screenshot.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureScreenshotParams {
    /// Image compression format (defaults to png).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Compression quality from range \[0..100\] (jpeg only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<i32>,
    /// Capture the screenshot of a given region only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clip: Option<Viewport>,
    /// Capture the screenshot from the surface, rather than the view. Defaults to true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_surface: Option<bool>,
    /// Capture the screenshot beyond the viewport. Defaults to false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture_beyond_viewport: Option<bool>,
}

/// Mirrors CDP `Page.Viewport`.
///
/// Viewport for capturing screenshot.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Viewport {
    /// X offset in device independent pixels (dip).
    pub x: f64,
    /// Y offset in device independent pixels (dip).
    pub y: f64,
    /// Rectangle width in device independent pixels (dip).
    pub width: f64,
    /// Rectangle height in device independent pixels (dip).
    pub height: f64,
    /// Page scale factor applied to the clip.
    pub scale: f64,
}

/// Mirrors CDP `Page.captureScreenshot` response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureScreenshotResult {
    /// Base64-encoded image bytes in the requested format.
    pub data: String,
}

// ---------------------------------------------------------------------------
// Runtime.callFunctionOn
// ---------------------------------------------------------------------------

/// Mirrors CDP `Runtime.callFunctionOn` request.
///
/// Calls function with given declaration on the given object. Object group of the result is
/// inherited from the target object.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CallFunctionOnParams {
    /// Source of the function to call, as a declaration such as `function() { … }`.
    pub function_declaration: String,
    /// Identifier of the object to call function on. Either objectId or executionContextId
    /// should be specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<String>,
    /// Call arguments. All call arguments must belong to the same JavaScript world as the
    /// target object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<CallArgument>>,
    /// Whether the result is expected to be a JSON object which should be sent by value. Can be
    /// overriden by `serializationOptions`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_by_value: Option<bool>,
    /// Whether execution should `await` for resulting value and return once awaited promise is
    /// resolved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub await_promise: Option<bool>,
}

/// Mirrors CDP `Runtime.CallArgument`.
///
/// Represents function call argument. Either remote object id `objectId`, primitive `value`,
/// unserializable primitive value or neither of (for undefined) them should be specified.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CallArgument {
    /// Primitive value or serializable javascript object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    /// Remote object handle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Version info (from /json/version)
// ---------------------------------------------------------------------------

/// Body of the HTTP `/json/version` endpoint.
///
/// Not a CDP domain type: this is served over HTTP before any websocket exists,
/// and it is how discovery finds the socket to connect to. Its keys do not
/// follow the protocol's camelCase convention, hence the explicit renames.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserVersionInfo {
    /// Websocket URL of the browser endpoint, which is the target of the connect.
    #[serde(rename = "webSocketDebuggerUrl")]
    pub web_socket_debugger_url: Option<String>,
    /// Browser product string, for example `Chrome/126.0.6478.126`.
    #[serde(rename = "Browser")]
    pub browser: Option<String>,
}
