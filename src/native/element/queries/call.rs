// SPDX-License-Identifier: MIT OR Apache-2.0
//! Shared `Runtime.callFunctionOn` helper for element property queries.

use serde_json::Value;

use crate::native::cdp::client::CdpClient;
use crate::native::cdp::types::{CallFunctionOnParams, EvaluateResult};

use super::super::refs::RefMap;
use super::super::resolve::resolve_element_object_id;

/// Resolve a selector/`@eN` ref and evaluate `function_declaration` on it.
///
/// Returns the by-value result of the call, or `None` when the page returned
/// `undefined`. Single source of truth for the resolve-then-call pattern used by
/// every element query in this module.
pub(super) async fn call_on_element(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
    function_declaration: String,
) -> Result<Option<Value>, String> {
    let (object_id, effective_session_id) = resolve_element_object_id(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
    )
    .await?;

    let result: EvaluateResult = client
        .send_command_typed(
            "Runtime.callFunctionOn",
            &CallFunctionOnParams {
                function_declaration,
                object_id: Some(object_id),
                arguments: None,
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(&effective_session_id),
        )
        .await?;

    Ok(result.result.value)
}
