// SPDX-License-Identifier: MIT OR Apache-2.0
//! Annotation collection and element rect lookup.

use super::super::types::{RawAnnotation, Rect};
use super::geometry::*;
use crate::native::cdp::client::CdpClient;
use crate::native::cdp::types::*;
use crate::native::element::RefMap;

pub(crate) async fn collect_annotations(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
) -> Result<Vec<RawAnnotation>, String> {
    let entries = ref_map.entries_sorted();
    if entries.is_empty() {
        return Ok(Vec::new());
    }

    // Collect entries that have backend_node_ids for batch resolution.
    let with_backend_ids: Vec<(String, crate::native::element::RefEntry, i64)> = entries
        .iter()
        .filter_map(|(ref_id, entry)| {
            entry
                .backend_node_id
                .map(|bid| (ref_id.clone(), entry.clone(), bid))
        })
        .collect();

    if with_backend_ids.is_empty() {
        return Ok(Vec::new());
    }

    // Bounded concurrent CDP resolve (rules_rust_paralelismo: no unbounded join_all).
    let cdp_limit = crate::concurrency::effective_limit_capped(crate::concurrency::CDP_FANOUT_CAP);
    let resolve_futures: Vec<_> = with_backend_ids
        .iter()
        .map(|(_, _, backend_node_id)| {
            client.send_command(
                "DOM.resolveNode",
                Some(serde_json::json!({
                    "backendNodeId": backend_node_id,
                    "objectGroup": "browser-automation-cli-annotate"
                })),
                Some(session_id),
            )
        })
        .collect();

    let resolve_results =
        crate::concurrency::join_bounded_ordered(resolve_futures, cdp_limit).await;

    // Collect resolved object IDs paired with their ref info.
    let mut resolved: Vec<(String, crate::native::element::RefEntry, String)> = Vec::new();
    for (i, result) in resolve_results.into_iter().enumerate() {
        if let Ok(val) = result {
            if let Some(oid) = val
                .get("object")
                .and_then(|o| o.get("objectId"))
                .and_then(|v| v.as_str())
            {
                let (ref_id, entry, _) = &with_backend_ids[i];
                resolved.push((ref_id.clone(), entry.clone(), oid.to_string()));
            }
        }
    }

    if resolved.is_empty() {
        return Ok(Vec::new());
    }

    // Batch-get bounding rects (bounded concurrent CDP).
    let rect_futures: Vec<_> = resolved
        .iter()
        .map(|(_, _, object_id)| get_rect_for_object(client, session_id, object_id))
        .collect();

    let rect_results = crate::concurrency::join_bounded_ordered(rect_futures, cdp_limit).await;

    let mut annotations = Vec::new();
    for (i, rect_result) in rect_results.into_iter().enumerate() {
        let rect = match rect_result {
            Ok(Some(r)) if r.width > 0.0 && r.height > 0.0 => r,
            _ => continue,
        };

        let (ref_id, entry, _) = &resolved[i];
        let number = ref_id
            .strip_prefix('e')
            .and_then(|n| n.parse::<u64>().ok())
            .unwrap_or(0);

        annotations.push(RawAnnotation {
            ref_id: ref_id.clone(),
            number,
            role: entry.role.clone(),
            name: (!entry.name.is_empty()).then_some(entry.name.clone()),
            rect,
        });
    }

    Ok(annotations)
}

pub(crate) async fn get_rect_for_selector(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector: &str,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
) -> Result<Option<Rect>, String> {
    let (object_id, effective_session_id) = crate::native::element::resolve_element_object_id(
        client,
        session_id,
        ref_map,
        selector,
        iframe_sessions,
    )
    .await?;
    get_rect_for_object(client, &effective_session_id, &object_id).await
}

pub(crate) async fn get_rect_for_object(
    client: &CdpClient,
    session_id: &str,
    object_id: &str,
) -> Result<Option<Rect>, String> {
    let result: EvaluateResult = client
        .send_command_typed(
            "Runtime.callFunctionOn",
            &CallFunctionOnParams {
                function_declaration: r#"function() {
                    const rect = this.getBoundingClientRect();
                    return { x: rect.x, y: rect.y, width: rect.width, height: rect.height };
                }"#
                .to_string(),
                object_id: Some(object_id.to_string()),
                arguments: None,
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(session_id),
        )
        .await?;

    Ok(result.result.value.as_ref().and_then(parse_rect))
}
