// SPDX-License-Identifier: MIT OR Apache-2.0
//! Domain / API classification over captures (CPU-bound via map_cpu).

use serde_json::{json, Value};

use crate::error::CliError;

use super::store::resolve_capture_path;
use super::types::MitmCapture;

/// List unique hosts.
pub fn domains(capture_path: Option<&str>) -> Result<Value, CliError> {
    let (path, explicit) = resolve_capture_path(capture_path)?;
    let cap = MitmCapture::load_scoped(&path, true, explicit)?;
    // PAR-56: host extract is pure CPU over items → map_cpu when large.
    let hosts_list = crate::concurrency::map_cpu(&cap.items, |e| e.host.clone());
    let mut hosts = std::collections::BTreeSet::new();
    for h in hosts_list.into_iter().flatten() {
        hosts.insert(h);
    }
    let list: Vec<String> = hosts.into_iter().collect();
    let count = list.len();
    Ok(json!({ "hosts": list, "count": count }))
}

/// Discover REST/GraphQL-ish endpoints from capture.
pub fn apis(kind: Option<&str>, capture_path: Option<&str>) -> Result<Value, CliError> {
    let (path, explicit) = resolve_capture_path(capture_path)?;
    let cap = MitmCapture::load_scoped(&path, true, explicit)?;
    let kind_owned = kind.map(|s| s.to_string());
    // PAR-56: classify endpoints in parallel when capture is large.
    let mut out: Vec<Value> = crate::concurrency::map_cpu(&cap.items, |e| {
        let url_l = e.url.to_ascii_lowercase();
        let is_gql = url_l.contains("graphql")
            || e.request_body
                .as_deref()
                .map(|b| b.contains("\"query\"") || b.contains("query "))
                .unwrap_or(false);
        let is_rest = url_l.contains("/api")
            || url_l.contains("/v1")
            || url_l.contains("/v2")
            || e.content_type
                .as_deref()
                .map(|c| c.contains("json"))
                .unwrap_or(false);
        let k = if is_gql {
            "graphql"
        } else if is_rest {
            "rest"
        } else {
            "other"
        };
        if let Some(ref filter) = kind_owned {
            if filter != k {
                return None;
            }
        }
        Some(json!({
            "id": e.id,
            "kind": k,
            "method": e.method,
            "url": e.url,
            "status": e.status,
        }))
    })
    .into_iter()
    .flatten()
    .collect();
    // Stable agent order by id when present (PAR-105: sort_by_cpu when large).
    crate::concurrency::sort_by_cpu(&mut out, |a, b| {
        let ia = a.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
        let ib = b.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
        ia.cmp(&ib)
    });
    Ok(json!({ "count": out.len(), "apis": out }))
}
