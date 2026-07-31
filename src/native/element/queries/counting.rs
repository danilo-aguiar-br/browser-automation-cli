// SPDX-License-Identifier: MIT OR Apache-2.0
//! Selector match counting (page-level `Runtime.evaluate`, no element resolve).

use crate::native::cdp::client::CdpClient;
use crate::native::cdp::types::{EvaluateParams, EvaluateResult};

use super::super::js::build_count_elements_js;

/// Number of elements matching a CSS selector.
///
/// Selector only: unlike its neighbours this takes no ref, because a `@eN` ref
/// denotes ONE element and counting it would always answer one.
pub async fn get_element_count(
    client: &CdpClient,
    session_id: &str,
    selector: &str,
) -> Result<i64, String> {
    let js = build_count_elements_js(selector);

    let result: EvaluateResult = client
        .send_command_typed(
            "Runtime.evaluate",
            &EvaluateParams {
                expression: js,
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(session_id),
        )
        .await?;

    Ok(result.result.value.and_then(|v| v.as_i64()).unwrap_or(0))
}
