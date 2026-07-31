// SPDX-License-Identifier: MIT OR Apache-2.0
//! Shared step-field helpers for run/exec dispatch.

use serde_json::Value;

use crate::browser::OneShotSession;
use crate::capability::Capability;
use crate::error::{CliError, ErrorKind};

use super::RunFlags;

/// The `action` sub-selector of a step, if present (`heap` → `summary`).
///
/// Capability lookup is specific-first, so this is what lets one `heap` row gate
/// eleven actions while `take` stays free.
pub(super) fn step_action(step: &Value) -> Option<&str> {
    step.get("action")
        .or_else(|| step.get("kind"))
        .and_then(|v| v.as_str())
}

/// Parse `format` / `formats` from a run step (GAP-057; mirrors top-level scrape).
///
/// Accepts a string (optionally CSV), an array of strings, or absence (empty →
/// session default `text`).
pub(super) fn scrape_formats_from_step(step: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let push_token = |raw: &str, out: &mut Vec<String>| {
        for part in raw.split(',') {
            let t = part.trim();
            if !t.is_empty() {
                out.push(t.to_string());
            }
        }
    };
    if let Some(v) = step.get("formats").or_else(|| step.get("format")) {
        match v {
            Value::String(s) => push_token(s, &mut out),
            Value::Array(arr) => {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        push_token(s, &mut out);
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// True when `cap` is enabled for this `run` invocation.
///
/// Policy gates come from `RunFlags`; capture buffers come from how the session
/// was launched, because `--capture-*` shapes the launch rather than the dispatch.
pub(super) fn step_capability_enabled(
    cap: Capability,
    flags: RunFlags,
    session: &OneShotSession,
) -> bool {
    match cap {
        Capability::Memory => flags.category_memory,
        Capability::Extensions => flags.category_extensions,
        Capability::ThirdParty => flags.category_third_party,
        Capability::Webmcp => flags.category_webmcp,
        Capability::Vision => flags.experimental_vision,
        Capability::Screencast => flags.experimental_screencast,
        Capability::CaptureConsole => session.capture().console,
        Capability::CaptureNetwork => session.capture().network,
    }
}

pub(super) fn step_beforeunload_action(step: &Value) -> Option<&'static str> {
    let v = step
        .get("handle_before_unload")
        .or_else(|| step.get("handleBeforeUnload"));
    match v {
        Some(Value::Bool(true)) => Some("accept"),
        Some(Value::Bool(false)) => None,
        Some(Value::String(s)) => {
            let s = s.trim().to_ascii_lowercase();
            match s.as_str() {
                "accept" | "true" | "1" | "yes" => Some("accept"),
                "dismiss" | "cancel" => Some("dismiss"),
                "off" | "false" | "0" | "no" | "none" => None,
                _ => Some("accept"),
            }
        }
        _ => None,
    }
}

/// Backward-compatible bool form (true when any auto-handle is requested).
#[allow(dead_code)]
pub(super) fn step_wants_beforeunload_handle(step: &Value) -> bool {
    step_beforeunload_action(step).is_some()
}

pub(crate) fn reject_unknown_step_fields(cmd: &str, step: &Value) -> Result<(), CliError> {
    let Some(obj) = step.as_object() else {
        return Ok(());
    };
    let allowed: &[&str] = match cmd {
        "scroll" => &[
            "cmd",
            "action",
            "target",
            "selector",
            "delta_x",
            "delta_y",
            "deltaX",
            "deltaY",
            "dx",
            "dy",
            "to_x",
            "toX",
            "to_y",
            "toY",
            "include_snapshot",
            "includeSnapshot",
        ],
        "drag" => &[
            "cmd",
            "action",
            "from",
            "to",
            "to_x",
            "toX",
            "to_y",
            "toY",
            "anchor",
            "synthetic_payload",
            "syntheticPayload",
            "include_snapshot",
            "includeSnapshot",
        ],
        // GAP-034 pillar 3: `include` is expanded by preflight and never reaches
        // dispatch, but it still has to reject typo fields at load time.
        "include" => &["cmd", "action", "path", "script", "file"],
        "submit" => &[
            "cmd",
            "action",
            "target",
            "selector",
            "timeout_ms",
            "timeoutMs",
            "include_snapshot",
            "includeSnapshot",
        ],
        "goto" => &[
            "cmd",
            "action",
            "url",
            "init_script",
            "initScript",
            "handle_before_unload",
            "handleBeforeUnload",
            "navigation_timeout_ms",
            "navigationTimeoutMs",
            "timeout_ms",
            "timeoutMs",
        ],
        _ => return Ok(()),
    };
    for key in obj.keys() {
        if !allowed.iter().any(|a| a == key) {
            return Err(CliError::with_suggestion(
                ErrorKind::Usage,
                format!("unknown field `{key}` on step cmd={cmd}"),
                format!("Allowed fields for {cmd}: {}", allowed.join(", ")),
            ));
        }
    }
    Ok(())
}
