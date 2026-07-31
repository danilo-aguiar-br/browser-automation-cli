// SPDX-License-Identifier: MIT OR Apache-2.0
//! CDP network/emulation helpers (headers, offline, conditions, CPU, setContent).
use rustc_hash::FxHashMap;
use serde_json::{json, Value};

use crate::native::cdp::client::CdpClient;

/// Set headers appended to every request of the session.
///
/// Replaces the whole set: CDP has no "add one header" call, so the caller must
/// pass the complete map each time.
pub async fn set_extra_headers(
    client: &CdpClient,
    session_id: &str,
    headers: &FxHashMap<String, String>,
) -> Result<(), String> {
    let headers_value: Value = headers
        .iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect::<serde_json::Map<String, Value>>()
        .into();

    client
        .send_command(
            "Network.setExtraHTTPHeaders",
            Some(json!({ "headers": headers_value })),
            Some(session_id),
        )
        .await?;

    Ok(())
}

/// Toggle offline emulation for the session.
pub async fn set_offline(
    client: &CdpClient,
    session_id: &str,
    offline: bool,
) -> Result<(), String> {
    set_network_conditions(client, session_id, offline, 0.0, -1.0, -1.0).await
}

/// Activate `Network.emulateNetworkConditions` (latency ms; throughput bytes/s, -1 = unlimited).
pub async fn set_network_conditions(
    client: &CdpClient,
    session_id: &str,
    offline: bool,
    latency_ms: f64,
    download_throughput: f64,
    upload_throughput: f64,
) -> Result<(), String> {
    client
        .send_command(
            "Network.emulateNetworkConditions",
            Some(json!({
                "offline": offline,
                "latency": latency_ms,
                "downloadThroughput": download_throughput,
                "uploadThroughput": upload_throughput,
            })),
            Some(session_id),
        )
        .await?;
    Ok(())
}

/// Slow the renderer by a multiplier, where `1.0` is no throttling.
///
/// Emulation only: it does not change the machine, so a result measured under
/// throttling is comparable across runs but not to wall-clock on real hardware.
pub async fn set_cpu_throttling_rate(
    client: &CdpClient,
    session_id: &str,
    rate: f64,
) -> Result<(), String> {
    client
        .send_command(
            "Emulation.setCPUThrottlingRate",
            Some(json!({ "rate": rate })),
            Some(session_id),
        )
        .await?;
    Ok(())
}

/// Replace the document of the current frame with `html`.
///
/// Resolves the frame from the frame tree first, so it acts on whatever frame
/// the session is pointed at rather than assuming the main frame.
pub async fn set_content(client: &CdpClient, session_id: &str, html: &str) -> Result<(), String> {
    // Get current frame ID
    let tree_result = client
        .send_command_no_params("Page.getFrameTree", Some(session_id))
        .await?;

    let frame_id = tree_result
        .get("frameTree")
        .and_then(|t| t.get("frame"))
        .and_then(|f| f.get("id"))
        .and_then(|id| id.as_str())
        .ok_or("Could not determine frame ID")?;

    client
        .send_command(
            "Page.setDocumentContent",
            Some(json!({
                "frameId": frame_id,
                "html": html,
            })),
            Some(session_id),
        )
        .await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Domain filter
// ---------------------------------------------------------------------------
