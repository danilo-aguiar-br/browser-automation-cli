// SPDX-License-Identifier: MIT OR Apache-2.0
//! Wait step: time, text, selector, load state, URL, and navigation predicates.

use serde_json::Value;

use crate::browser::OneShotSession;
use crate::error::CliError;

use super::fields::{first_present, first_str, include_snapshot};

pub(super) async fn wait(session: &mut OneShotSession, step: &Value) -> Result<Value, CliError> {
    // GAP-053: `wait_timeout_ms` is the PUBLIC name — it is the `--wait-timeout-ms`
    // flag, the published schema field and the name in the agent skill. It was
    // missing here, so a `run` step carrying it was discarded in silence and the
    // wait fell back to the built-in default. The default happens to equal the
    // documented value, which is why the drop produced plausible timings and went
    // unnoticed.
    //
    // Order matters: `wait_timeout_ms` comes FIRST because the CLI resolves
    // `wait_timeout_ms.or(ms)` (see `commands::nav::wait`), so the explicit
    // condition timeout wins over the plain sleep when a step carries both.
    let ms = first_present(
        step,
        &[
            "wait_timeout_ms",
            "waitTimeoutMs",
            "ms",
            "timeout_ms",
            "timeoutMs",
        ],
    )
    .and_then(|v| v.as_u64());
    let texts: Vec<String> = match step.get("text") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        Some(Value::String(s)) => vec![s.clone()],
        _ => Vec::new(),
    };
    // GAP-019: selector string and/or array of selectors (OR).
    let selector = first_str(step, &["selector", "sel"]);
    let selectors: Vec<String> = match step.get("selectors") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        Some(Value::String(s)) => vec![s.clone()],
        _ => match first_present(step, &["selector", "sel"]) {
            Some(Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            _ => Vec::new(),
        },
    };
    let state = step.get("state").and_then(|v| v.as_str());
    // GAP-024: wait for URL / navigation complete.
    let url_exact = step
        .get("url")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty());
    let url_contains = first_str(step, &["url_contains", "urlContains"]).filter(|s| !s.is_empty());
    let navigation = step
        .get("navigation")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let include_snap = include_snapshot(step);

    // GAP-032: quiet-network and DOM-stability windows join the same OR set as the
    // other conditions. `0` means "use the built-in window" rather than "no wait",
    // because a zero-length quiet window would be satisfied instantly.
    let network_idle_ms = first_present(
        step,
        &[
            "network_idle_ms",
            "networkIdleMs",
            "network_idle",
            "networkIdle",
            "idle_ms",
            "idleMs",
        ],
    )
    .and_then(coerce_window_ms)
    .map(|v| {
        if v == 0 {
            crate::xdg::policy::policy_u64(crate::xdg::policy::key::DEFAULT_NETWORK_IDLE_WINDOW_MS)
        } else {
            v
        }
    });
    let dom_stable_ms = first_present(
        step,
        &["dom_stable_ms", "domStableMs", "dom_stable", "domStable"],
    )
    .and_then(coerce_window_ms)
    .map(|v| {
        if v == 0 {
            crate::xdg::policy::policy_u64(crate::xdg::policy::key::DEFAULT_DOM_STABLE_WINDOW_MS)
        } else {
            v
        }
    });
    // `min_count` is not a condition of its own: it raises the selector condition
    // from "at least one node" to "at least N nodes".
    let min_count = first_present(step, &["min_count", "minCount"]).and_then(|v| v.as_u64());

    let has_sel = selector.is_some() || !selectors.is_empty();
    let has_url = url_exact.is_some() || url_contains.is_some();
    let has_quiet = network_idle_ms.is_some() || dom_stable_ms.is_some();
    if texts.is_empty()
        && !has_sel
        && state.is_none()
        && !has_url
        && !navigation
        && !has_quiet
        && !include_snap
    {
        session.wait_ms(ms.unwrap_or(0)).await
    } else {
        session
            .wait_for_conditions(
                crate::browser::WaitRequest {
                    ms,
                    texts: &texts,
                    selector,
                    selectors: &selectors,
                    state,
                    url_exact,
                    url_contains,
                    navigation,
                    network_idle_ms,
                    min_count,
                    dom_stable_ms,
                },
                include_snap,
            )
            .await
    }
}

/// Accept a window as a number or as `true` (meaning "use the built-in window").
fn coerce_window_ms(v: &Value) -> Option<u64> {
    match v {
        Value::Bool(true) => Some(0),
        Value::Bool(false) => None,
        other => other.as_u64(),
    }
}
