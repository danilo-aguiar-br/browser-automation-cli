// SPDX-License-Identifier: MIT OR Apache-2.0
//! Console and network capture-buffer steps.
#![allow(missing_docs, unused_imports)]

use std::path::Path;

use serde_json::{json, Value};

use crate::browser::OneShotSession;
use crate::error::{CliError, ErrorKind};
use crate::robots::RobotsPolicy;

use super::super::super::RunFlags;
use super::super::helpers::step_beforeunload_action;
pub(super) async fn handle(
    session: &mut OneShotSession,
    cmd: &str,
    step: &Value,
) -> Result<Value, CliError> {
    match cmd {
        "console" => {
            let action = step
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("list");
            match action {
                "list" => {
                    let page_idx = step
                        .get("page_idx")
                        .or_else(|| step.get("pageIdx"))
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize);
                    let page_size = step
                        .get("page_size")
                        .or_else(|| step.get("pageSize"))
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize);
                    let types = step.get("types").and_then(|v| v.as_str());
                    let include_preserved = step
                        .get("include_preserved")
                        .or_else(|| step.get("includePreservedMessages"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let sw = step
                        .get("service_worker_id")
                        .or_else(|| step.get("serviceWorkerId"))
                        .and_then(|v| v.as_str());
                    session.console_list(page_idx, page_size, types, include_preserved, sw)
                }
                "get" => {
                    let id = step
                        .get("id")
                        .or_else(|| step.get("msgid"))
                        .or_else(|| step.get("index"))
                        .and_then(|v| v.as_u64())
                        .ok_or_else(|| {
                            CliError::new(
                                ErrorKind::Usage,
                                "console get requires id|msgid|index (0-based)",
                            )
                        })? as usize;
                    session.console_get(id)
                }
                "clear" => session.console_clear(),
                "dump" => {
                    let path = step.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
                        CliError::new(ErrorKind::Usage, "console dump requires path")
                    })?;
                    session.console_dump(Path::new(path)).await
                }
                other => Err(CliError::new(
                    ErrorKind::Usage,
                    format!("unknown console action: {other}"),
                )),
            }
        }
        "net" => {
            let action = step
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("list");
            match action {
                "list" => {
                    let page_idx = step
                        .get("page_idx")
                        .or_else(|| step.get("pageIdx"))
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize);
                    let page_size = step
                        .get("page_size")
                        .or_else(|| step.get("pageSize"))
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize);
                    let resource_types = step
                        .get("resource_types")
                        .or_else(|| step.get("resourceTypes"))
                        .and_then(|v| v.as_str());
                    let include_preserved = step
                        .get("include_preserved")
                        .or_else(|| step.get("includePreservedRequests"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    session.net_list(page_idx, page_size, resource_types, include_preserved)
                }
                "get" => {
                    let id = step
                        .get("id")
                        .map(|v| {
                            if let Some(s) = v.as_str() {
                                s.to_string()
                            } else if let Some(n) = v.as_u64() {
                                n.to_string()
                            } else {
                                String::new()
                            }
                        })
                        .filter(|s| !s.is_empty())
                        .ok_or_else(|| {
                            CliError::new(
                                ErrorKind::Usage,
                                "net get requires id (index or requestId)",
                            )
                        })?;
                    let request_path = step
                        .get("request_path")
                        .and_then(|v| v.as_str())
                        .map(Path::new);
                    let response_path = step
                        .get("response_path")
                        .and_then(|v| v.as_str())
                        .map(Path::new);
                    session.net_get(&id, request_path, response_path).await
                }
                other => Err(CliError::new(
                    ErrorKind::Usage,
                    format!("unknown net action: {other}"),
                )),
            }
        }
        other => Err(CliError::new(
            ErrorKind::Usage,
            format!("internal: unexpected cmd in this family: {other}"),
        )),
    }
}
