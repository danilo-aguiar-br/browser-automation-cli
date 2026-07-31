// SPDX-License-Identifier: MIT OR Apache-2.0
//! Wait condition set and the OR algebra that resolves it.

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};
use crate::native::browser::WaitUntil;
use crate::native::element::{self};

use super::super::super::OneShotSession;
use super::request::WaitRequest;

impl OneShotSession {
    /// Full wait surface including network-quiet, minimum match count and DOM
    /// stability (GAP-019/024/032).
    ///
    /// # Condition algebra
    ///
    /// Text, selector, URL, network-quiet and DOM-stability are **OR**-ed: the
    /// first one satisfied returns, and the envelope names it in `waited`.
    /// `min_count` is not a condition of its own — it raises the bar of the
    /// selector condition from "at least one node" to "at least N nodes".
    /// Callers that need AND semantics chain two `wait` steps.
    pub async fn wait_for_conditions(
        &mut self,
        req: WaitRequest<'_>,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        let WaitRequest {
            ms,
            texts,
            selector,
            selectors,
            state,
            url_exact,
            url_contains,
            navigation,
            network_idle_ms,
            min_count,
            dom_stable_ms,
        } = req;
        let mut waited = Vec::new();
        let has_text = !texts.is_empty();
        let has_url = url_exact.is_some() || url_contains.is_some();
        let has_net_idle = network_idle_ms.is_some();
        let has_dom_stable = dom_stable_ms.is_some();
        let want_count = min_count.unwrap_or(1).max(1);

        // Build OR list of CSS selectors (GAP-019).
        let mut sel_list: Vec<String> = Vec::new();
        if let Some(s) = selector {
            let s = s.trim();
            if !s.is_empty() {
                sel_list.push(s.to_string());
                // Also try comma-split parts so a flaky compound still OR-matches.
                if s.contains(',') {
                    for part in s.split(',') {
                        let p = part.trim();
                        if !p.is_empty() && !sel_list.iter().any(|x| x == p) {
                            sel_list.push(p.to_string());
                        }
                    }
                }
            }
        }
        for s in selectors {
            let p = s.trim();
            if !p.is_empty() && !sel_list.iter().any(|x| x == p) {
                sel_list.push(p.to_string());
            }
        }
        let has_sel = !sel_list.is_empty();

        let effective_state = if navigation && state.is_none() {
            Some("load")
        } else {
            state
        };

        if let Some(st) = effective_state {
            let until = WaitUntil::parse_token(st);
            let session_id = self
                .manager
                .active_session_id()
                .map_err(|e| CliError::new(ErrorKind::Browser, e))?
                .to_string();
            self.manager
                .wait_for_lifecycle_external(until, &session_id)
                .await
                .map_err(|e| {
                    CliError::with_suggestion(
                        ErrorKind::Timeout,
                        format!("wait state {st} failed: {e}"),
                        crate::i18n::suggestion_key("use_listed_value", None),
                    )
                })?;
            waited.push(json!({"kind": "state", "state": st}));
        }

        // Any condition that has to be polled rather than awaited once.
        let has_polled = has_text || has_sel || has_url || has_net_idle || has_dom_stable;

        let only_ms = !has_polled && effective_state.is_none();
        if let Some(m) = ms {
            if m > 0 && only_ms {
                let data = self.wait_ms(m).await?;
                return self.attach_snapshot_if(include_snapshot, data).await;
            }
            if m > 0 && !has_polled && effective_state.is_some() {
                let _ = self.wait_ms(m).await?;
                waited.push(json!({"kind": "ms", "ms": m}));
                let data = json!({ "waited": waited, "ok": true });
                return self.attach_snapshot_if(include_snapshot, data).await;
            }
        }

        if !has_polled && effective_state.is_some() {
            let data = json!({ "waited": waited, "ok": true });
            return self.attach_snapshot_if(include_snapshot, data).await;
        }

        if !has_polled && effective_state.is_none() {
            let data = self.wait_ms(ms.unwrap_or(0)).await?;
            return self.attach_snapshot_if(include_snapshot, data).await;
        }

        let started = std::time::Instant::now();
        let deadline = started + std::time::Duration::from_millis(ms.unwrap_or(10_000).max(1));
        // DOM-stability state: fingerprint of the last poll plus when it changed.
        let mut dom_signature: Option<String> = None;
        let mut dom_unchanged_since = started;
        loop {
            self.drain_events();

            // GAP-032: network quiet — zero in-flight requests for the whole window.
            if let Some(window_ms) = network_idle_ms {
                if self.network_is_quiet(window_ms, started) {
                    waited.push(json!({
                        "kind": "network_idle",
                        "idle_ms": window_ms,
                        "requests_started": self.net_started,
                    }));
                    let data = json!({
                        "waited": waited,
                        "ok": true,
                        "requests_started": self.net_started,
                    });
                    return self.attach_snapshot_if(include_snapshot, data).await;
                }
            }

            // GAP-032: DOM stability — serialized document unchanged for the window.
            if let Some(window_ms) = dom_stable_ms {
                if let Some(sig) = self.dom_signature().await {
                    let now = std::time::Instant::now();
                    if dom_signature.as_deref() != Some(sig.as_str()) {
                        dom_signature = Some(sig);
                        dom_unchanged_since = now;
                    } else if now.duration_since(dom_unchanged_since)
                        >= std::time::Duration::from_millis(window_ms)
                    {
                        waited.push(json!({
                            "kind": "dom_stable",
                            "dom_stable_ms": window_ms,
                        }));
                        let data = json!({ "waited": waited, "ok": true });
                        return self.attach_snapshot_if(include_snapshot, data).await;
                    }
                }
            }

            // GAP-024: URL conditions
            if has_url {
                let href = self
                    .manager
                    .evaluate("location.href", None)
                    .await
                    .ok()
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_default();
                if let Some(exact) = url_exact {
                    if href == exact {
                        waited.push(json!({"kind": "url", "url": exact, "match": "exact"}));
                        let data = json!({ "waited": waited, "ok": true, "href": href });
                        return self.attach_snapshot_if(include_snapshot, data).await;
                    }
                }
                if let Some(sub) = url_contains {
                    if href.contains(sub) {
                        waited.push(json!({
                            "kind": "url_contains",
                            "url_contains": sub,
                            "href": href
                        }));
                        let data = json!({ "waited": waited, "ok": true, "href": href });
                        return self.attach_snapshot_if(include_snapshot, data).await;
                    }
                }
            }

            // GAP-019: selector OR list (compound + split parts)
            if has_sel {
                let session_id = self
                    .manager
                    .active_session_id()
                    .map_err(|e| CliError::new(ErrorKind::Browser, e))?
                    .to_string();
                for sel in &sel_list {
                    match element::get_element_count(&self.manager.client, &session_id, sel).await {
                        // GAP-032: `min_count` raises the bar from "any node" to N nodes.
                        Ok(n) if n as u64 >= want_count => {
                            waited.push(json!({
                                "kind": "selector",
                                "selector": sel,
                                "matched_selector": sel,
                                "count": n,
                                "min_count": want_count
                            }));
                            let data = json!({
                                "waited": waited,
                                "ok": true,
                                "matched_selector": sel,
                                "count": n
                            });
                            return self.attach_snapshot_if(include_snapshot, data).await;
                        }
                        Ok(_) => {}
                        Err(e) => {
                            // Invalid selector should not be a silent timeout.
                            if e.contains("SyntaxError")
                                || e.contains(" DomException")
                                || e.contains("is not a valid")
                            {
                                return Err(CliError::with_suggestion(
                                    ErrorKind::Usage,
                                    format!("wait selector invalid: {sel}: {e}"),
                                    crate::i18n::suggestion_key("target_ref_from_view", None),
                                ));
                            }
                        }
                    }
                }
            }

            if has_text {
                let body = self
                    .manager
                    .evaluate("document.body ? document.body.innerText : ''", None)
                    .await
                    .unwrap_or(json!(""));
                let hay = body.as_str().unwrap_or("");
                if let Some(t) = texts.iter().find(|t| hay.contains(t.as_str())) {
                    waited.push(json!({"kind": "text", "text": t, "match": "any"}));
                    let data = json!({ "waited": waited, "ok": true });
                    return self.attach_snapshot_if(include_snapshot, data).await;
                }
            }
            if std::time::Instant::now() >= deadline {
                // Name the conditions: "not met" alone forces the agent to guess.
                let mut pending: Vec<String> = Vec::new();
                if has_text {
                    pending.push(format!("text({})", texts.join("|")));
                }
                if has_sel {
                    pending.push(format!("selector({}) >= {want_count}", sel_list.join("|")));
                }
                if has_url {
                    pending.push("url".to_string());
                }
                if let Some(w) = network_idle_ms {
                    pending.push(format!(
                        "network_idle({w}ms, in_flight={})",
                        self.net_inflight
                    ));
                }
                if let Some(w) = dom_stable_ms {
                    pending.push(format!("dom_stable({w}ms)"));
                }
                return Err(CliError::with_suggestion(
                    ErrorKind::Timeout,
                    format!(
                        "wait condition not met before deadline: {}",
                        pending.join(", ")
                    ),
                    crate::i18n::suggestion_key("raise_timeout", None),
                ));
            }
            tokio::time::sleep(std::time::Duration::from_millis(
                crate::xdg::policy::policy_u64(
                    crate::xdg::policy::key::DEFAULT_NAV_MICRO_SETTLE_MS,
                ),
            ))
            .await;
        }
    }
}
