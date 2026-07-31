// SPDX-License-Identifier: MIT OR Apache-2.0

use rustc_hash::FxHashSet;
use serde_json::{json, Value};

use crate::native::cdp::client::CdpClient;
use crate::native::cdp::types::{
    AttachToTargetParams, AttachToTargetResult, CloseTargetParams, CreateTargetParams,
    CreateTargetResult, EvaluateParams,
};
use base64::Engine;

use super::types::{OriginStorage, StorageEntry};

pub(crate) fn collect_frame_origins(tree: &Value, origins: &mut FxHashSet<String>) {
    if let Some(frame) = tree.get("frame") {
        if let Some(url_str) = frame.get("url").and_then(|v| v.as_str()) {
            if let Ok(parsed) = url::Url::parse(url_str) {
                let origin = parsed.origin().ascii_serialization();
                if origin != "null" && !origin.is_empty() {
                    origins.insert(origin);
                }
            }
        }
    }
    if let Some(children) = tree.get("childFrames").and_then(|v| v.as_array()) {
        for child in children {
            collect_frame_origins(child, origins);
        }
    }
}

/// Parse the JS-evaluated origin storage data into an OriginStorage struct.
pub(crate) fn parse_origin_storage(data: &Value) -> Option<OriginStorage> {
    if !data.is_object() {
        return None;
    }
    let origin = data
        .get("origin")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if origin.is_empty() || origin == "null" {
        return None;
    }
    let local_storage: Vec<StorageEntry> = data
        .get("localStorage")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let session_storage: Vec<StorageEntry> = data
        .get("sessionStorage")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    Some(OriginStorage {
        origin,
        local_storage,
        session_storage,
    })
}

/// Evaluate the storage-collection JS snippet and parse the result.
pub(crate) async fn eval_origin_storage(
    client: &CdpClient,
    session_id: &str,
    origin_js: &str,
) -> Option<OriginStorage> {
    let result = client
        .send_command_typed::<_, crate::native::cdp::types::EvaluateResult>(
            "Runtime.evaluate",
            &EvaluateParams {
                expression: origin_js.to_string(),
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(session_id),
        )
        .await
        .ok()?;
    let data = result.result.value.unwrap_or(Value::Null);
    parse_origin_storage(&data)
}

/// Create a temporary CDP target, navigate it to each origin to collect localStorage,
/// then close it. Uses Fetch interception to serve blank HTML instead of making real
/// network requests.
pub(crate) async fn collect_storage_via_temp_target(
    client: &CdpClient,
    origins: &[String],
    origin_js: &str,
) -> Result<Vec<OriginStorage>, String> {
    let create_result: CreateTargetResult = client
        .send_command_typed(
            "Target.createTarget",
            &CreateTargetParams {
                url: "about:blank".to_string(),
                browser_context_id: None,
            },
            None,
        )
        .await?;

    let target_id = create_result.target_id;

    // Ensure the target is closed even if attach or later steps fail
    let result = collect_storage_in_target(client, &target_id, origins, origin_js).await;

    let _ = client
        .send_command_typed::<_, Value>(
            "Target.closeTarget",
            &CloseTargetParams { target_id },
            None,
        )
        .await;

    result
}

pub(crate) async fn collect_storage_in_target(
    client: &CdpClient,
    target_id: &str,
    origins: &[String],
    origin_js: &str,
) -> Result<Vec<OriginStorage>, String> {
    let attach_result: AttachToTargetResult = client
        .send_command_typed(
            "Target.attachToTarget",
            &AttachToTargetParams {
                target_id: target_id.to_string(),
                flatten: true,
            },
            None,
        )
        .await?;

    let temp_session = &attach_result.session_id;

    client
        .send_command_no_params("Page.enable", Some(temp_session))
        .await?;
    client
        .send_command_no_params("Runtime.enable", Some(temp_session))
        .await?;

    // Blank HTML response body, pre-encoded to avoid repeated base64 work per request
    let blank_html_b64 = base64::engine::general_purpose::STANDARD.encode("<html></html>");

    let _ = client
        .send_command(
            "Fetch.enable",
            Some(json!({ "patterns": [{ "urlPattern": "*" }] })),
            Some(temp_session),
        )
        .await;

    let mut event_rx = client.subscribe();
    let mut results = Vec::new();

    for target_origin in origins {
        let nav_url = format!("{}/", target_origin.trim_end_matches('/'));
        if client
            .send_command(
                "Page.navigate",
                Some(json!({ "url": nav_url })),
                Some(temp_session),
            )
            .await
            .is_err()
        {
            continue;
        }

        // Fulfill intercepted requests with blank HTML until the page loads
        let deadline = tokio::time::Instant::now()
            + tokio::time::Duration::from_secs(crate::xdg::policy::policy_u64(
                crate::xdg::policy::key::STATE_COLLECT_DEADLINE_SECS,
            ));
        let mut page_loaded = false;
        while tokio::time::Instant::now() < deadline {
            match tokio::time::timeout(
                tokio::time::Duration::from_secs(crate::xdg::policy::policy_u64(
                    crate::xdg::policy::key::STATE_EVENT_RECV_SECS,
                )),
                event_rx.recv(),
            )
            .await
            {
                Ok(Ok(evt)) if evt.session_id.as_deref() == Some(temp_session) => {
                    if evt.method == "Fetch.requestPaused" {
                        if let Some(request_id) =
                            evt.params.get("requestId").and_then(|v| v.as_str())
                        {
                            let _ = client
                                .send_command(
                                    "Fetch.fulfillRequest",
                                    Some(json!({
                                        "requestId": request_id,
                                        "responseCode": 200,
                                        "responseHeaders": [
                                            { "name": "Content-Type", "value": "text/html" }
                                        ],
                                        "body": &blank_html_b64
                                    })),
                                    Some(temp_session),
                                )
                                .await;
                        }
                    } else if evt.method == "Page.loadEventFired" {
                        page_loaded = true;
                        break;
                    }
                }
                Ok(Ok(_)) => continue,  // event for a different session
                Ok(Err(_)) => continue, // lagged or closed — retry within deadline
                Err(_) => break,        // outer timeout elapsed
            }
        }

        if !page_loaded {
            continue;
        }

        if let Some(storage) = eval_origin_storage(client, temp_session, origin_js).await {
            if !storage.local_storage.is_empty() || !storage.session_storage.is_empty() {
                results.push(storage);
            }
        }
    }

    Ok(results)
}
