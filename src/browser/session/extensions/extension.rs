// SPDX-License-Identifier: MIT OR Apache-2.0
//! OneShotSession methods (componentized; single-responsibility impl blocks).

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};

use super::super::OneShotSession;

impl OneShotSession {
    /// List loaded Chrome extensions in this session.
    ///
    /// # Errors
    ///
    /// Fails with [`ErrorKind::Browser`] —
    /// `"extension list: …"` — when `Target.getTargets` is refused. A session
    /// with no extensions is not an error: it answers `count: 0`.
    pub async fn extension_list(&mut self) -> Result<Value, CliError> {
        self.pump_events().await;
        let targets = self
            .manager
            .client
            .send_command("Target.getTargets", None, None)
            .await
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("extension list: {e}")))?;
        let list = targets
            .get("targetInfos")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let extensions: Vec<Value> = list
            .into_iter()
            .filter(|t| {
                t.get("url")
                    .and_then(|u| u.as_str())
                    .map(|u| u.starts_with("chrome-extension://"))
                    .unwrap_or(false)
                    || t.get("type").and_then(|x| x.as_str()) == Some("service_worker")
            })
            .map(|t| {
                let url = t.get("url").and_then(|u| u.as_str()).unwrap_or("");
                let id = url
                    .strip_prefix("chrome-extension://")
                    .and_then(|rest| rest.split('/').next())
                    .unwrap_or("")
                    .to_string();
                json!({
                    "id": id,
                    "url": url,
                    "type": t.get("type"),
                    "title": t.get("title"),
                    "targetId": t.get("targetId"),
                })
            })
            .collect();
        Ok(json!({ "extensions": extensions, "count": extensions.len() }))
    }

    /// Unload extension targets in this process (GAP-007).
    /// Uninstall a loaded extension by id.
    ///
    /// # Errors
    ///
    /// Propagates [`extension_list`](Self::extension_list): a refused
    /// `Target.getTargets`.
    ///
    /// An `id` that matches no loaded extension is **not** an error: it is
    /// dropped from this process's load list and reported as
    /// `effect: "metadata_only"`, because there is nothing to unload and
    /// failing would punish a caller who asked for a state that already holds.
    /// The individual `Target.closeTarget` calls are best-effort too, so a
    /// target that refuses to close still counts as closed.
    pub async fn extension_uninstall(&mut self, id: &str) -> Result<Value, CliError> {
        self.pump_events().await;
        let listed = self.extension_list().await?;
        let targets = listed
            .get("extensions")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let matches: Vec<Value> = targets
            .into_iter()
            .filter(|t| {
                t.get("id")
                    .and_then(|v| v.as_str())
                    .map(|s| s == id || s.starts_with(id) || id.contains(s))
                    .unwrap_or(false)
            })
            .collect();
        if matches.is_empty() {
            // Cross-process / not loaded: honest metadata effect.
            self.loaded_extension_ids
                .retain(|x| x != id && !id.contains(x));
            return Ok(json!({
                "uninstalled": id,
                "effect": "metadata_only",
                "persistent": false,
                "ok": true,
                "note": "no matching extension target in this process; omitted from next load path",
            }));
        }
        // PAR-96: multi-target close is independent CDP I/O → join_bounded.
        let target_ids: Vec<String> = matches
            .iter()
            .filter_map(|t| {
                t.get("targetId")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .collect();
        let cdp_limit =
            crate::concurrency::effective_limit_capped(crate::concurrency::CDP_ATTACH_FANOUT_CAP);
        let client = self.manager.client.clone();
        let close_futs: Vec<_> = target_ids
            .into_iter()
            .map(|tid| {
                let client = client.clone();
                async move {
                    let _ = client
                        .send_command(
                            "Target.closeTarget",
                            Some(json!({ "targetId": tid.clone() })),
                            None,
                        )
                        .await;
                    tid
                }
            })
            .collect();
        let closed = crate::concurrency::join_bounded(close_futs, cdp_limit.max(1)).await;
        self.loaded_extension_ids
            .retain(|x| x != id && !id.contains(x));
        Ok(json!({
            "uninstalled": id,
            "effect": "unloaded",
            "closed_targets": closed,
            "persistent": false,
            "ok": true,
        }))
    }

    /// Reload extension service worker target by id prefix (one-shot CDP).
    /// Reload a loaded extension by id.
    ///
    /// # Errors
    ///
    /// Propagates [`extension_list`](Self::extension_list), then fails with
    /// [`ErrorKind::NoInput`] —
    /// `"extension id not found: <id>"`, carrying the `extension_list_first`
    /// suggestion — when no loaded extension matches, and with
    /// [`ErrorKind::Browser`] when the
    /// matched entry carries no `targetId`.
    ///
    /// The `Target.closeTarget` itself is best-effort: the reload is a
    /// close-and-let-Chrome-respawn, so a refusal is not reported and the
    /// `after` listing is what shows whether the worker came back.
    pub async fn extension_reload(&mut self, id: &str) -> Result<Value, CliError> {
        self.pump_events().await;
        let listed = self.extension_list().await?;
        let targets = listed
            .get("extensions")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let match_t = targets.iter().find(|t| {
            t.get("id")
                .and_then(|v| v.as_str())
                .map(|s| s == id || s.starts_with(id) || id.contains(s))
                .unwrap_or(false)
        });
        let Some(t) = match_t else {
            return Err(CliError::with_suggestion(
                ErrorKind::NoInput,
                format!("extension id not found: {id}"),
                crate::i18n::suggestion_key("extension_list_first", None),
            ));
        };
        let target_id = t
            .get("targetId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CliError::new(ErrorKind::Browser, "missing targetId"))?
            .to_string();
        // Close then rely on Chrome to re-spawn the extension SW on next attach.
        let _ = self
            .manager
            .client
            .send_command(
                "Target.closeTarget",
                Some(json!({ "targetId": target_id })),
                None,
            )
            .await;
        tokio::time::sleep(std::time::Duration::from_millis(
            crate::xdg::resolve_interact_settle_ms(),
        ))
        .await;
        let again = self.extension_list().await?;
        Ok(json!({
            "reloaded": id,
            "closed_target": target_id,
            "after": again,
            "one_shot": true,
            "ok": true,
            "note": "one-shot SW restart via Target.closeTarget; install path is --load-extension on the same invocation",
        }))
    }

    /// Trigger an extension action/command by id.
    ///
    /// # Errors
    ///
    /// Propagates [`extension_list`](Self::extension_list), then fails with
    /// [`ErrorKind::NoInput`] —
    /// `"extension service_worker not found for id: <id>"` — when no target of
    /// type `service_worker` matches, and with
    /// [`ErrorKind::Browser`] when the entry
    /// carries no `targetId` or `Target.attachToTarget` is refused
    /// (`"attach extension SW: …"`).
    ///
    /// The `Runtime.evaluate` that probes `chrome.runtime` is best-effort: its
    /// failure lands in the `evaluate` field as `null` rather than failing the
    /// call, so "attached but the API was unavailable" stays distinguishable
    /// from "could not attach".
    pub async fn extension_trigger(&mut self, id: &str) -> Result<Value, CliError> {
        self.pump_events().await;
        let listed = self.extension_list().await?;
        let targets = listed
            .get("extensions")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let match_t = targets.iter().find(|t| {
            t.get("id")
                .and_then(|v| v.as_str())
                .map(|s| s == id || s.starts_with(id))
                .unwrap_or(false)
                && t.get("type").and_then(|v| v.as_str()) == Some("service_worker")
        });
        let Some(t) = match_t else {
            return Err(CliError::with_suggestion(
                ErrorKind::NoInput,
                format!("extension service_worker not found for id: {id}"),
                crate::i18n::suggestion_key("extension_list_first", None),
            ));
        };
        let target_id = t
            .get("targetId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CliError::new(ErrorKind::Browser, "missing targetId"))?
            .to_string();
        // Attach and try chrome.runtime / action APIs when available.
        let attach = self
            .manager
            .client
            .send_command(
                "Target.attachToTarget",
                Some(json!({ "targetId": target_id, "flatten": true })),
                None,
            )
            .await
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("attach extension SW: {e}")))?;
        let session = attach
            .get("sessionId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let eval = self
            .manager
            .client
            .send_command(
                "Runtime.evaluate",
                Some(json!({
                    "expression": "(() => { try { if (chrome && chrome.runtime) { return { ok: true, id: chrome.runtime.id }; } return { ok: false, reason: 'no chrome.runtime' }; } catch (e) { return { ok: false, reason: String(e) }; } })()",
                    "returnByValue": true,
                    "awaitPromise": true,
                })),
                session.as_deref(),
            )
            .await;
        Ok(json!({
            "triggered": id,
            "targetId": target_id,
            "evaluate": eval.unwrap_or(Value::Null),
            "one_shot": true,
            "ok": true,
            "note": "best-effort SW Runtime.evaluate in the same process; popup UI may need headed Chrome",
        }))
    }
}
