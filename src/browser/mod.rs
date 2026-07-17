//! One-shot browser session: launch-only + actions + reap (PR3–PR6).
//!
//! PROHIBITED on this path: ensure_daemon, connect_cdp, multi-process session.
//! Refs live only inside this process; multi-step scripts share one session via `run`.
//!
//! Method-level docs are expanded over time; clap and skill surfaces document agent usage.
#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use tokio::sync::broadcast;

use crate::error::{CliError, ErrorKind};
use crate::lifecycle::Lifecycle;
use crate::native::browser::{BrowserManager, WaitUntil};
use crate::native::cdp::chrome::LaunchOptions;
use crate::native::cdp::types::CdpEvent;
use crate::native::cookies;
use crate::native::element::{self, RefMap};
use crate::native::interaction;
use crate::native::network;
use crate::native::snapshot::{self, SnapshotOptions};

/// Capture toggles for process-local console/network buffers.
#[derive(Debug, Clone, Copy, Default)]
pub struct CaptureOpts {
    pub console: bool,
    pub network: bool,
}

/// Drop Chrome-internal schemes from capture-network (agent-ready envelope).
fn is_internal_browser_url(url: &str) -> bool {
    url.starts_with("chrome:")
        || url.starts_with("chrome-extension:")
        || url.starts_with("devtools:")
}

/// Drop non-document noise from capture-network (internal + data/blob embeds).
fn is_noise_network_url(url: &str) -> bool {
    is_internal_browser_url(url) || url.starts_with("data:") || url.starts_with("blob:")
}

/// Headless Chrome session owned by a single CLI invocation (or one `run` script).
pub struct OneShotSession {
    manager: BrowserManager,
    ref_map: RefMap,
    iframe_sessions: HashMap<String, String>,
    chrome_pid: Option<u32>,
    capture: CaptureOpts,
    event_rx: broadcast::Receiver<CdpEvent>,
    console_log: Vec<Value>,
    network_log: Vec<Value>,
    perf_active: bool,
    screencast_active: bool,
    heap_chunks: Vec<String>,
    trace_chunks: Vec<String>,
    /// Last written trace path from `perf stop` (for offline insight).
    last_trace_path: Option<PathBuf>,
    /// In-memory NDJSON of last trace (cleared after stop unless kept for insight).
    last_trace_body: Option<String>,
    /// PNG base64 frames from Page.screencastFrame.
    screencast_frames: Vec<String>,
    /// Output directory for screencast frames (set on start).
    screencast_dir: Option<PathBuf>,
    /// Pending screencast frame sessionIds awaiting ack.
    screencast_ack_ids: Vec<i64>,
    /// True while a JS dialog is open (alert/confirm/prompt).
    dialog_open: bool,
    /// HeapProfiler.reportHeapSnapshotProgress finished=true observed.
    heap_snapshot_finished: bool,
    /// Tracing.tracingComplete observed after perf stop.
    tracing_complete: bool,
}

impl OneShotSession {
    /// Launch local Chrome only (no connect, no daemon).
    pub async fn launch_headless() -> Result<Self, CliError> {
        Self::launch_headless_with_capture(CaptureOpts::default()).await
    }

    pub async fn launch_headless_with_capture(capture: CaptureOpts) -> Result<Self, CliError> {
        let options = LaunchOptions {
            headless: true,
            hide_scrollbars: true,
            ..LaunchOptions::default()
        };
        let manager = BrowserManager::launch(options, Some("chrome"))
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Unavailable,
                    format!("Chrome launch failed: {e}"),
                    "Install system Chrome/Chromium or set executable path; re-run doctor",
                )
            })?;
        let chrome_pid = manager.chrome_pid();
        let event_rx = manager.client.subscribe();

        let mut session = Self {
            manager,
            ref_map: RefMap::new(),
            iframe_sessions: HashMap::new(),
            chrome_pid,
            capture,
            event_rx,
            console_log: Vec::new(),
            network_log: Vec::new(),
            perf_active: false,
            screencast_active: false,
            heap_chunks: Vec::new(),
            trace_chunks: Vec::new(),
            last_trace_path: None,
            last_trace_body: None,
            screencast_frames: Vec::new(),
            screencast_dir: None,
            screencast_ack_ids: Vec::new(),
            dialog_open: false,
            heap_snapshot_finished: false,
            tracing_complete: false,
        };
        session.enable_capture_domains().await?;
        Ok(session)
    }

    /// Launch with Chrome extensions loaded (`--load-extension`).
    pub async fn launch_with_extensions(
        capture: CaptureOpts,
        extensions: Vec<String>,
    ) -> Result<Self, CliError> {
        if extensions.is_empty() {
            return Self::launch_headless_with_capture(capture).await;
        }
        for p in &extensions {
            let path = Path::new(p);
            if !path.exists() {
                return Err(CliError::with_suggestion(
                    ErrorKind::NoInput,
                    format!("extension path not found: {p}"),
                    "Pass an unpacked extension directory (contains manifest.json)",
                ));
            }
        }
        // Extensions require a non-headless Chrome product mode (chromiumoxide with_head).
        // build_chrome_args also omits --headless=new when extensions are present.
        let options = LaunchOptions {
            headless: false,
            hide_scrollbars: true,
            extensions: Some(extensions),
            ..LaunchOptions::default()
        };
        let manager = BrowserManager::launch(options, Some("chrome"))
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Unavailable,
                    format!("Chrome launch with extensions failed: {e}"),
                    "Use an unpacked extension dir; ensure Xvfb is available on Linux headed launches",
                )
            })?;
        let chrome_pid = manager.chrome_pid();
        let event_rx = manager.client.subscribe();
        let mut session = Self {
            manager,
            ref_map: RefMap::new(),
            iframe_sessions: HashMap::new(),
            chrome_pid,
            capture,
            event_rx,
            console_log: Vec::new(),
            network_log: Vec::new(),
            perf_active: false,
            screencast_active: false,
            heap_chunks: Vec::new(),
            trace_chunks: Vec::new(),
            last_trace_path: None,
            last_trace_body: None,
            screencast_frames: Vec::new(),
            screencast_dir: None,
            screencast_ack_ids: Vec::new(),
            dialog_open: false,
            heap_snapshot_finished: false,
            tracing_complete: false,
        };
        session.enable_capture_domains().await?;
        Ok(session)
    }

    pub fn chrome_pid(&self) -> Option<u32> {
        self.chrome_pid
    }

    pub fn capture(&self) -> CaptureOpts {
        self.capture
    }

    async fn enable_capture_domains(&mut self) -> Result<(), CliError> {
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();

        // Always enable Page domain for dialogs/screencast and attach page-session listeners
        // (heap chunks, screencast frames, JS dialogs are target-scoped).
        let _ = self
            .manager
            .client
            .send_command_no_params("Page.enable", Some(&session_id))
            .await;
        let _ = self.manager.client.attach_page_session_forwarders().await;

        if self.capture.console {
            self.manager
                .client
                .send_command_no_params("Runtime.enable", Some(&session_id))
                .await
                .map_err(|e| CliError::new(ErrorKind::Protocol, format!("Runtime.enable: {e}")))?;
            // Page-level console listeners (context7): complements browser-level forwarder.
            let _ = self.manager.client.attach_page_console_forwarders().await;
        }
        if self.capture.network {
            self.manager
                .client
                .send_command_no_params("Network.enable", Some(&session_id))
                .await
                .map_err(|e| CliError::new(ErrorKind::Protocol, format!("Network.enable: {e}")))?;
            // Also enable at browser scope (no session) when available.
            let _ = self
                .manager
                .client
                .send_command_no_params("Network.enable", None)
                .await;
            let _ = self.manager.client.attach_page_network_forwarders().await;
        }
        Ok(())
    }

    /// Merge console/network buffers into a result JSON when capture flags are on.
    pub fn with_capture_fields(&mut self, mut data: Value) -> Value {
        self.drain_events();
        if let Some(obj) = data.as_object_mut() {
            if self.capture.console {
                obj.insert("console".to_string(), json!(self.console_log.clone()));
                obj.insert("console_count".to_string(), json!(self.console_log.len()));
            }
            if self.capture.network {
                obj.insert("network".to_string(), json!(self.network_log.clone()));
                obj.insert("network_count".to_string(), json!(self.network_log.len()));
            }
        }
        data
    }

    /// Drain pending CDP events into local buffers (non-blocking).
    pub fn drain_events(&mut self) {
        loop {
            match self.event_rx.try_recv() {
                Ok(evt) => self.ingest_event(evt),
                Err(broadcast::error::TryRecvError::Empty) => break,
                Err(broadcast::error::TryRecvError::Lagged(_)) => continue,
                Err(broadcast::error::TryRecvError::Closed) => break,
            }
        }
    }

    /// Drain events and ack screencast frames (required or Chrome stops sending).
    pub async fn pump_events(&mut self) {
        self.drain_events();
        let acks: Vec<i64> = self.screencast_ack_ids.drain(..).collect();
        if acks.is_empty() {
            return;
        }
        let session_id = self.manager.active_session_id().ok().map(|s| s.to_string());
        for sid in acks {
            let _ = self
                .manager
                .client
                .send_command(
                    "Page.screencastFrameAck",
                    Some(json!({ "sessionId": sid })),
                    session_id.as_deref(),
                )
                .await;
        }
    }

    fn ingest_event(&mut self, evt: CdpEvent) {
        match evt.method.as_str() {
            "Runtime.consoleAPICalled" if self.capture.console => {
                let level = evt
                    .params
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("log")
                    .to_string();
                let raw_args: Vec<Value> = evt
                    .params
                    .get("args")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                let text = network::format_console_args(&raw_args);
                self.console_log.push(json!({
                    "type": level,
                    "text": text,
                    "args": raw_args,
                }));
            }
            "Network.requestWillBeSent" if self.capture.network => {
                let request = evt.params.get("request").cloned().unwrap_or(Value::Null);
                let method = request
                    .get("method")
                    .and_then(|v| v.as_str())
                    .unwrap_or("GET");
                let url = request.get("url").and_then(|v| v.as_str()).unwrap_or("");
                if is_noise_network_url(url) {
                    return;
                }
                let request_id = evt
                    .params
                    .get("requestId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                self.network_log.push(json!({
                    "requestId": request_id,
                    "method": method,
                    "url": url,
                }));
            }
            "HeapProfiler.addHeapSnapshotChunk" => {
                if let Some(chunk) = evt.params.get("chunk").and_then(|v| v.as_str()) {
                    self.heap_chunks.push(chunk.to_string());
                }
            }
            "HeapProfiler.reportHeapSnapshotProgress" => {
                if evt
                    .params
                    .get("finished")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    self.heap_snapshot_finished = true;
                }
            }
            "Tracing.dataCollected" => {
                if let Some(value) = evt.params.get("value") {
                    // CDP sends an array of events; store as one NDJSON line (or expand).
                    if let Some(arr) = value.as_array() {
                        for item in arr {
                            self.trace_chunks
                                .push(serde_json::to_string(item).unwrap_or_default());
                        }
                    } else {
                        self.trace_chunks
                            .push(serde_json::to_string(value).unwrap_or_default());
                    }
                }
            }
            "Tracing.tracingComplete" => {
                self.tracing_complete = true;
            }
            "Page.screencastFrame" => {
                if let Some(data) = evt.params.get("data").and_then(|v| v.as_str()) {
                    // Cap buffer to avoid unbounded memory in long screencasts.
                    if self.screencast_frames.len() < 600 {
                        self.screencast_frames.push(data.to_string());
                    }
                }
                if let Some(sid) = evt.params.get("sessionId").and_then(|v| v.as_i64()) {
                    self.screencast_ack_ids.push(sid);
                }
            }
            "Page.javascriptDialogOpening" => {
                self.dialog_open = true;
            }
            "Page.javascriptDialogClosed" => {
                self.dialog_open = false;
            }
            _ => {}
        }
    }

    /// Navigate and wait for load (same process). Honors robots when policy is Honor.
    pub async fn goto(
        &mut self,
        url: &str,
        robots: crate::robots::RobotsPolicy,
    ) -> Result<Value, CliError> {
        crate::robots::enforce_robots(url, robots, "browser-automation-cli").await?;
        self.ref_map.clear();
        self.manager
            .navigate(url, WaitUntil::Load)
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Browser,
                    format!("Navigation failed: {e}"),
                    "Check URL scheme and network; try about:blank for smoke",
                )
            })?;
        // Give console/network a brief window after load.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        self.drain_events();
        let page_url = self
            .manager
            .get_url()
            .await
            .unwrap_or_else(|_| url.to_string());
        let title = self.manager.get_title().await.unwrap_or_default();
        let data = json!({
            "url": page_url,
            "title": title,
            "robots_policy": robots.as_str(),
        });
        Ok(self.with_capture_fields(data))
    }

    /// Minimal scrape: navigate, return source_url + body text + robots_policy.
    pub async fn scrape(
        &mut self,
        url: &str,
        robots: crate::robots::RobotsPolicy,
    ) -> Result<Value, CliError> {
        let nav = self.goto(url, robots).await?;
        let text_val = self
            .eval(
                "String((document.body && document.body.innerText) || '')",
                None,
                Some("accept"),
                None,
            )
            .await
            .unwrap_or_else(|_| json!({"result": ""}));
        let text_s = match text_val.get("result") {
            Some(Value::String(s)) => s.clone(),
            Some(other) => other.to_string(),
            None => String::new(),
        };
        let text_s = if text_s.is_empty() {
            String::new()
        } else {
            text_s
        };
        Ok(json!({
            "source_url": nav.get("url").cloned().unwrap_or(Value::String(url.to_string())),
            "title": nav.get("title").cloned().unwrap_or(Value::String(String::new())),
            "robots_policy": robots.as_str(),
            "text": text_s,
        }))
    }

    /// Accessibility tree with agent-facing `@eN` refs.
    pub async fn view(&mut self, verbose: bool) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();

        let options = SnapshotOptions {
            interactive: false,
            compact: !verbose,
            ..SnapshotOptions::default()
        };

        self.ref_map.clear();
        let tree = snapshot::take_snapshot(
            &self.manager.client,
            &session_id,
            &options,
            &mut self.ref_map,
            None,
            &self.iframe_sessions,
        )
        .await
        .map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Browser,
                format!("view/snapshot failed: {e}"),
                "Ensure the page finished loading; try goto then view in the same run",
            )
        })?;

        let tree_at = tree_to_at_refs(&tree);
        let url = self.manager.get_url().await.unwrap_or_default();
        let title = self.manager.get_title().await.unwrap_or_default();

        let entries = self.ref_map.entries_sorted();
        let ref_count = entries.len();
        let refs: serde_json::Map<String, Value> = entries
            .into_iter()
            .map(|(ref_id, entry)| {
                let key = format!("@{ref_id}");
                (
                    key,
                    json!({
                        "role": entry.role,
                        "name": entry.name,
                        "id": ref_id,
                    }),
                )
            })
            .collect();

        Ok(json!({
            "tree": tree_at,
            "url": url,
            "title": title,
            "refs": refs,
            "ref_count": ref_count,
        }))
    }

    /// Optionally attach a slim accessibility snapshot to a JSON result.
    pub(crate) async fn attach_snapshot_if(
        &mut self,
        include: bool,
        mut data: Value,
    ) -> Result<Value, CliError> {
        if !include {
            return Ok(data);
        }
        let snap = self.view(false).await?;
        if let Some(obj) = data.as_object_mut() {
            obj.insert("snapshot".to_string(), snap);
            obj.insert("include_snapshot".to_string(), json!(true));
        }
        Ok(data)
    }

    pub async fn press(
        &mut self,
        target: &str,
        dblclick: bool,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();

        let result = if dblclick {
            interaction::dblclick(
                &self.manager.client,
                &session_id,
                &self.ref_map,
                target,
                &self.iframe_sessions,
            )
            .await
        } else {
            interaction::click(
                &self.manager.client,
                &session_id,
                &self.ref_map,
                target,
                "left",
                1,
                &self.iframe_sessions,
            )
            .await
        }
        .map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Browser,
                format!("press failed: {e}"),
                "Use a CSS selector or @eN from view in the same process (run script)",
            )
        })?;

        self.drain_events();
        let data = json!({
            "pressed": target,
            "dblclick": dblclick,
            "dialog_opened": result.dialog_opened,
        });
        self.attach_snapshot_if(include_snapshot, data).await
    }

    pub async fn write(
        &mut self,
        target: &str,
        value: &str,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();

        interaction::fill_smart(
            &self.manager.client,
            &session_id,
            &self.ref_map,
            target,
            value,
            &self.iframe_sessions,
        )
        .await
        .map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Browser,
                format!("write failed: {e}"),
                "Use a CSS selector or @eN from view; select/checkbox/radio use fill_smart semantics",
            )
        })?;

        self.drain_events();
        let data = json!({
            "written": target,
            "value_len": value.len(),
            "fill_mode": "smart",
        });
        self.attach_snapshot_if(include_snapshot, data).await
    }

    /// Click at absolute page coordinates (requires experimental vision flag at CLI).
    pub async fn click_at(
        &mut self,
        x: f64,
        y: f64,
        dblclick: bool,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        let result = interaction::click_at(&self.manager.client, &session_id, x, y, dblclick)
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Browser,
                    format!("click-at failed: {e}"),
                    "Coordinates are page CSS pixels; enable --experimental-vision",
                )
            })?;
        self.drain_events();
        let data = json!({
            "clicked_at": { "x": x, "y": y },
            "dblclick": dblclick,
            "dialog_opened": result.dialog_opened,
        });
        self.attach_snapshot_if(include_snapshot, data).await
    }

    pub async fn keys(&mut self, key: &str, include_snapshot: bool) -> Result<Value, CliError> {
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();

        interaction::press_key(&self.manager.client, &session_id, key)
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Browser,
                    format!("keys failed: {e}"),
                    "Pass a CDP key name such as Enter, Tab, Escape, or ArrowDown",
                )
            })?;

        self.drain_events();
        let data = json!({ "key": key });
        self.attach_snapshot_if(include_snapshot, data).await
    }

    pub async fn type_text(
        &mut self,
        target: Option<&str>,
        text: &str,
        clear: bool,
        submit: Option<&str>,
        focus_only: bool,
    ) -> Result<Value, CliError> {
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();

        let typed_target = if focus_only || target.is_none() {
            // tool-ref type_text: type into currently focused element
            if clear {
                // Select-all then type (best-effort clear of focused field)
                let _ =
                    interaction::press_key(&self.manager.client, &session_id, "Control+a").await;
                let _ =
                    interaction::press_key(&self.manager.client, &session_id, "Backspace").await;
            }
            interaction::type_text_into_active_context(
                &self.manager.client,
                &session_id,
                text,
                None,
            )
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Browser,
                    format!("type (focus-only) failed: {e}"),
                    "Focus an input first or pass a CSS/@eN target",
                )
            })?;
            target.unwrap_or("(focused)").to_string()
        } else {
            let t = match target {
                Some(s) => s,
                None => {
                    return Err(CliError::new(
                        ErrorKind::Usage,
                        "type requires --target or --focus-only",
                    ));
                }
            };
            interaction::type_text(
                &self.manager.client,
                &session_id,
                &self.ref_map,
                t,
                text,
                clear,
                None,
                &self.iframe_sessions,
            )
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Browser,
                    format!("type failed: {e}"),
                    "Use a CSS selector or @eN from view in the same process",
                )
            })?;
            t.to_string()
        };

        if let Some(key) = submit {
            interaction::press_key(&self.manager.client, &session_id, key)
                .await
                .map_err(|e| {
                    CliError::with_suggestion(
                        ErrorKind::Browser,
                        format!("type --submit key failed: {e}"),
                        "Pass a CDP key such as Enter",
                    )
                })?;
        }

        self.drain_events();
        Ok(json!({
            "typed": typed_target,
            "text_len": text.len(),
            "cleared": clear,
            "submit": submit,
            "focus_only": focus_only || target.is_none(),
        }))
    }

    pub async fn eval(
        &mut self,
        expression: &str,
        args_json: Option<&str>,
        dialog_action: Option<&str>,
        file_path: Option<&Path>,
    ) -> Result<Value, CliError> {
        use crate::native::cdp::types::{EvaluateParams, EvaluateResult};

        // dialogAction: accept | dismiss | prompt text (default accept)
        let action = dialog_action.unwrap_or("accept");
        let _ = action; // applied via auto-accept path below; dismiss handled when needed
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();

        // Build expression: call bare functions once; never re-invoke IIFEs.
        // Bug: wrapping `(() => 1)()` as `((() => 1)())()` yields "is not a function".
        let expr = normalize_eval_expression(expression, args_json)?;

        let client = std::sync::Arc::clone(&self.manager.client);
        let expr_owned = expr.clone();
        let mut eval_fut = Box::pin(async move {
            let result: EvaluateResult = client
                .send_command_typed(
                    "Runtime.evaluate",
                    &EvaluateParams {
                        expression: expr_owned,
                        return_by_value: Some(true),
                        await_promise: Some(true),
                    },
                    Some(&session_id),
                )
                .await?;
            if let Some(ref details) = result.exception_details {
                let msg = details
                    .exception
                    .as_ref()
                    .and_then(|e| e.description.as_deref())
                    .unwrap_or(&details.text);
                return Err(format!("Evaluation error: {msg}"));
            }
            Ok(result.result.value.unwrap_or(Value::Null))
        });
        let v = loop {
            tokio::select! {
                res = &mut eval_fut => {
                    break res.map_err(|e| {
                        CliError::with_suggestion(
                            ErrorKind::Browser,
                            format!("eval failed: {e}"),
                            "Check the JS expression; use return-by-value expressions",
                        )
                    })?;
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(40)) => {
                    self.drain_events();
                    if self.dialog_open {
                        let accept = !action.eq_ignore_ascii_case("dismiss");
                        let prompt = if action.eq_ignore_ascii_case("accept")
                            || action.eq_ignore_ascii_case("dismiss")
                        {
                            None
                        } else {
                            Some(action)
                        };
                        let _ = self.manager.handle_dialog(accept, prompt).await;
                        self.dialog_open = false;
                    }
                }
            }
        };
        // Allow consoleAPICalled events to land after evaluate.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        self.drain_events();
        let data = self.with_capture_fields(json!({ "result": v }));
        if let Some(path) = file_path {
            let body = serde_json::to_vec_pretty(&data).map_err(|e| {
                CliError::new(ErrorKind::Io, format!("eval serialize for file: {e}"))
            })?;
            std::fs::write(path, body).map_err(|e| {
                CliError::new(ErrorKind::Io, format!("eval write {}: {e}", path.display()))
            })?;
        }
        Ok(data)
    }

    pub async fn wait_ms(&mut self, ms: u64) -> Result<Value, CliError> {
        // Pump in slices so screencast FrameAck keeps frames flowing during waits.
        let mut remaining = ms;
        while remaining > 0 {
            let slice = remaining.min(50);
            tokio::time::sleep(std::time::Duration::from_millis(slice)).await;
            remaining = remaining.saturating_sub(slice);
            if self.screencast_active {
                self.pump_events().await;
            } else {
                self.drain_events();
            }
        }
        Ok(json!({ "waited_ms": ms }))
    }

    pub async fn grab(
        &mut self,
        path: Option<&Path>,
        format: &str,
        full_page: bool,
        quality: Option<i32>,
        element: Option<&str>,
    ) -> Result<Value, CliError> {
        use crate::native::screenshot::{take_screenshot, ScreenshotOptions};

        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();

        let out_path = path.map(|p| p.to_path_buf()).unwrap_or_else(|| {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            std::path::PathBuf::from(format!("grab-{stamp}.{format}"))
        });

        let options = ScreenshotOptions {
            path: Some(out_path.to_string_lossy().into_owned()),
            format: format.to_string(),
            full_page,
            quality,
            selector: element.map(|s| s.to_string()),
            ..ScreenshotOptions::default()
        };

        let result = take_screenshot(
            &self.manager.client,
            &session_id,
            &self.ref_map,
            &options,
            &self.iframe_sessions,
        )
        .await
        .map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Browser,
                format!("grab failed: {e}"),
                "Ensure page is loaded; check write permissions for path",
            )
        })?;

        let path_str = if result.path.is_empty() {
            out_path.to_string_lossy().into_owned()
        } else {
            result.path
        };
        let path_buf = std::path::PathBuf::from(&path_str);
        let written = path_buf.exists();
        let magic_ok = written && verify_image_magic(&path_buf, format);
        let byte_size = std::fs::metadata(&path_buf).map(|m| m.len()).unwrap_or(0);

        Ok(json!({
            "path": path_str,
            "format": format,
            "written": written,
            "magic_ok": magic_ok,
            "byte_size": byte_size,
            "full_page": full_page,
            "quality": quality,
            "element": element,
        }))
    }

    // --- Camada B ---

    pub async fn extract(&mut self, target: &str, attr: Option<&str>) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();

        if let Some(name) = attr {
            let v = element::get_element_attribute(
                &self.manager.client,
                &session_id,
                &self.ref_map,
                target,
                name,
                &self.iframe_sessions,
            )
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Browser,
                    format!("extract attr failed: {e}"),
                    "Use --ref @eN from view in the same run",
                )
            })?;
            Ok(json!({ "target": target, "attr": name, "value": v }))
        } else {
            let text = element::get_element_text(
                &self.manager.client,
                &session_id,
                &self.ref_map,
                target,
                &self.iframe_sessions,
            )
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Browser,
                    format!("extract text failed: {e}"),
                    "Use --ref @eN from view in the same run",
                )
            })?;
            Ok(json!({ "target": target, "text": text }))
        }
    }

    pub async fn attr(&mut self, target: &str, name: &str) -> Result<Value, CliError> {
        self.extract(target, Some(name)).await
    }

    /// PRD §7 `text`: extract visible text from a target.
    pub async fn text(&mut self, target: &str) -> Result<Value, CliError> {
        self.extract(target, None).await
    }

    /// PRD §7 `scroll`: scroll window or element by delta pixels.
    pub async fn scroll(
        &mut self,
        target: Option<&str>,
        delta_x: f64,
        delta_y: f64,
    ) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        interaction::scroll(
            &self.manager.client,
            &session_id,
            &self.ref_map,
            target,
            delta_x,
            delta_y,
            &self.iframe_sessions,
        )
        .await
        .map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Browser,
                format!("scroll failed: {e}"),
                "Pass --target @eN from view, or omit target for window scroll",
            )
        })?;
        Ok(json!({
            "ok": true,
            "target": target,
            "delta_x": delta_x,
            "delta_y": delta_y,
        }))
    }

    pub async fn cookie_list(&mut self, url: Option<&str>) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        let cookies = if let Some(u) = url {
            cookies::get_cookies(&self.manager.client, &session_id, Some(vec![u.to_string()])).await
        } else {
            cookies::get_all_cookies(&self.manager.client, &session_id).await
        }
        .map_err(|e| CliError::new(ErrorKind::Browser, format!("cookie list failed: {e}")))?;
        Ok(json!({
            "cookies": cookies,
            "count": cookies.len(),
            "url_filter": url,
        }))
    }

    pub async fn cookie_set(&mut self, cookies_json: &str) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        let parsed: Value = serde_json::from_str(cookies_json).map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Usage,
                format!("cookie set JSON invalid: {e}"),
                r#"Use --json '[{"name":"a","value":"b","url":"https://example.com"}]'"#,
            )
        })?;
        let arr = parsed.as_array().ok_or_else(|| {
            CliError::with_suggestion(
                ErrorKind::Usage,
                "cookie set requires a JSON array",
                r#"Use --json '[{"name":"a","value":"b","url":"https://example.com"}]'"#,
            )
        })?;
        let current_url = self.manager.get_url().await.ok();
        cookies::set_cookies(
            &self.manager.client,
            &session_id,
            arr.clone(),
            current_url.as_deref(),
        )
        .await
        .map_err(|e| CliError::new(ErrorKind::Browser, format!("cookie set failed: {e}")))?;
        Ok(json!({ "ok": true, "set_count": arr.len() }))
    }

    pub async fn cookie_clear(&mut self) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        cookies::clear_cookies(&self.manager.client, &session_id)
            .await
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("cookie clear failed: {e}")))?;
        Ok(json!({ "ok": true, "cleared": true }))
    }

    pub async fn page_info(&mut self) -> Result<Value, CliError> {
        self.drain_events();
        let url = self.manager.get_url().await.unwrap_or_default();
        let title = self.manager.get_title().await.unwrap_or_default();
        Ok(json!({ "url": url, "title": title }))
    }

    pub async fn assert_url(&mut self, value: &str, contains: bool) -> Result<Value, CliError> {
        self.drain_events();
        let url = self.manager.get_url().await.unwrap_or_default();
        let ok = if contains {
            url.contains(value)
        } else {
            url == value
        };
        if !ok {
            return Err(CliError::with_suggestion(
                ErrorKind::Data,
                format!(
                    "assert url failed: got={url:?} expected contains={contains} value={value:?}"
                ),
                "Navigate first with goto in the same run",
            ));
        }
        Ok(json!({ "assert": "url", "ok": true, "url": url, "value": value, "contains": contains }))
    }

    pub async fn assert_text(
        &mut self,
        value: &str,
        target: Option<&str>,
    ) -> Result<Value, CliError> {
        self.drain_events();
        let haystack = if let Some(t) = target {
            let session_id = self
                .manager
                .active_session_id()
                .map_err(|e| CliError::new(ErrorKind::Browser, e))?
                .to_string();
            element::get_element_text(
                &self.manager.client,
                &session_id,
                &self.ref_map,
                t,
                &self.iframe_sessions,
            )
            .await
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("assert text: {e}")))?
        } else {
            let v = self
                .manager
                .evaluate("document.body ? document.body.innerText : ''", None)
                .await
                .map_err(|e| CliError::new(ErrorKind::Browser, format!("assert text: {e}")))?;
            v.as_str().unwrap_or("").to_string()
        };

        if !haystack.contains(value) {
            return Err(CliError::with_suggestion(
                ErrorKind::Data,
                format!("assert text failed: value not found: {value:?}"),
                "Check view/extract in the same run; text match is substring",
            ));
        }
        Ok(json!({ "assert": "text", "ok": true, "value": value, "target": target }))
    }

    pub async fn assert_console(&mut self, level: &str, max: u64) -> Result<Value, CliError> {
        if !self.capture.console {
            return Err(CliError::with_suggestion(
                ErrorKind::Usage,
                "assert console requires --capture-console on the same invocation",
                "browser-automation-cli --capture-console run --script audit.jsonl",
            ));
        }
        self.drain_events();
        let level_l = level.to_ascii_lowercase();
        let count = self
            .console_log
            .iter()
            .filter(|m| {
                m.get("type")
                    .and_then(|v| v.as_str())
                    .map(|t| t.eq_ignore_ascii_case(&level_l))
                    .unwrap_or(false)
            })
            .count() as u64;
        if count > max {
            return Err(CliError::with_suggestion(
                ErrorKind::Data,
                format!("assert console failed: level={level} count={count} max={max}"),
                "Fix page console noise or raise --max",
            ));
        }
        Ok(json!({
            "assert": "console",
            "ok": true,
            "level": level,
            "count": count,
            "max": max,
        }))
    }

    pub fn console_list(
        &mut self,
        page_idx: Option<usize>,
        page_size: Option<usize>,
        types: Option<&str>,
        include_preserved: bool,
        service_worker_id: Option<&str>,
    ) -> Result<Value, CliError> {
        if !self.capture.console {
            return Err(CliError::with_suggestion(
                ErrorKind::Usage,
                "console list requires --capture-console",
                "Pass --capture-console before run/console",
            ));
        }
        self.drain_events();
        let mut messages: Vec<Value> = self.console_log.clone();
        let _ = include_preserved; // one-shot: log is process-local; flag accepted for tool-ref parity
        if let Some(types_csv) = types {
            let wanted: Vec<String> = types_csv
                .split(',')
                .map(|s| s.trim().to_ascii_lowercase())
                .filter(|s| !s.is_empty())
                .collect();
            if !wanted.is_empty() {
                messages.retain(|m| {
                    let level = m
                        .get("level")
                        .or_else(|| m.get("type"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    wanted.iter().any(|w| level.contains(w))
                });
            }
        }
        if let Some(sw) = service_worker_id {
            messages.retain(|m| {
                m.get("service_worker_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s == sw)
                    .unwrap_or(false)
            });
        }
        let total = messages.len();
        let page = page_idx.unwrap_or(0);
        let size = page_size.unwrap_or(total.max(1));
        let start = page.saturating_mul(size).min(total);
        let end = (start + size).min(total);
        let page_msgs = messages[start..end].to_vec();
        Ok(json!({
            "messages": page_msgs,
            "count": page_msgs.len(),
            "total": total,
            "page_idx": page,
            "page_size": size,
        }))
    }

    pub fn console_get(&mut self, id: usize) -> Result<Value, CliError> {
        // Full unpaginated list for get-by-id
        let list = self.console_list(None, None, None, true, None)?;
        let total = list.get("total").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        // Prefer original buffer for stable ids
        self.drain_events();
        self.console_log
            .get(id)
            .cloned()
            .map(|m| json!({ "id": id, "message": m }))
            .ok_or_else(|| {
                CliError::with_suggestion(
                    ErrorKind::Data,
                    format!("console message id {id} not found (count={total})"),
                    "Use console list to inspect ids (0-based index)",
                )
            })
    }

    pub fn console_clear(&mut self) -> Result<Value, CliError> {
        if !self.capture.console {
            return Err(CliError::with_suggestion(
                ErrorKind::Usage,
                "console clear requires --capture-console",
                "Pass --capture-console before run/console",
            ));
        }
        self.drain_events();
        let n = self.console_log.len();
        self.console_log.clear();
        Ok(json!({ "cleared": n }))
    }

    pub fn console_dump(&mut self, path: &Path) -> Result<Value, CliError> {
        let data = self.console_list(None, None, None, true, None)?;
        // Dump full buffer, not just first page
        let messages = self.console_log.clone();
        let _ = data;
        let mut body = String::new();
        for m in &messages {
            body.push_str(&serde_json::to_string(m).unwrap_or_default());
            body.push('\n');
        }
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    CliError::new(ErrorKind::Io, format!("console dump mkdir: {e}"))
                })?;
            }
        }
        std::fs::write(path, body.as_bytes())
            .map_err(|e| CliError::new(ErrorKind::Io, format!("console dump write: {e}")))?;
        Ok(json!({
            "path": path.to_string_lossy(),
            "count": messages.len(),
        }))
    }

    pub fn net_list(
        &mut self,
        page_idx: Option<usize>,
        page_size: Option<usize>,
        resource_types: Option<&str>,
        include_preserved: bool,
    ) -> Result<Value, CliError> {
        if !self.capture.network {
            return Err(CliError::with_suggestion(
                ErrorKind::Usage,
                "net list requires --capture-network",
                "Pass --capture-network before run/net",
            ));
        }
        self.drain_events();
        let mut requests: Vec<Value> = self.network_log.clone();
        let _ = include_preserved; // process-local buffer; accepted for tool-ref parity
        if let Some(types_csv) = resource_types {
            let wanted: Vec<String> = types_csv
                .split(',')
                .map(|s| s.trim().to_ascii_lowercase())
                .filter(|s| !s.is_empty())
                .collect();
            if !wanted.is_empty() {
                requests.retain(|r| {
                    let rt = r
                        .get("resource_type")
                        .or_else(|| r.get("type"))
                        .or_else(|| r.get("resourceType"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    wanted.iter().any(|w| rt.contains(w))
                });
            }
        }
        let total = requests.len();
        let page = page_idx.unwrap_or(0);
        let size = page_size.unwrap_or(total.max(1));
        let start = page.saturating_mul(size).min(total);
        let end = (start + size).min(total);
        let page_reqs = requests[start..end].to_vec();
        Ok(json!({
            "requests": page_reqs,
            "count": page_reqs.len(),
            "total": total,
            "page_idx": page,
            "page_size": size,
        }))
    }

    /// Resolve a network entry by 0-based index or CDP `requestId` string.
    pub fn net_get(
        &mut self,
        id: &str,
        request_path: Option<&Path>,
        response_path: Option<&Path>,
    ) -> Result<Value, CliError> {
        let _ = self.net_list(None, None, None, true)?;
        let requests = self.network_log.clone();
        let (index, req) = if let Ok(idx) = id.parse::<usize>() {
            let req = requests.get(idx).cloned().ok_or_else(|| {
                CliError::with_suggestion(
                    ErrorKind::Data,
                    format!(
                        "network request index {idx} not found (count={})",
                        requests.len()
                    ),
                    "Use net list; pass 0-based index or requestId string",
                )
            })?;
            (idx, req)
        } else {
            let (idx, req) = requests
                .iter()
                .enumerate()
                .find(|(_, r)| {
                    r.get("requestId")
                        .and_then(|v| v.as_str())
                        .map(|rid| rid == id)
                        .unwrap_or(false)
                })
                .map(|(i, r)| (i, r.clone()))
                .ok_or_else(|| {
                    CliError::with_suggestion(
                        ErrorKind::Data,
                        format!(
                            "network requestId {id} not found (count={})",
                            requests.len()
                        ),
                        "Use net list; pass 0-based index or exact requestId",
                    )
                })?;
            (idx, req)
        };
        let request_id = req
            .get("requestId")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if let Some(p) = request_path {
            if let Some(parent) = p.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        CliError::new(ErrorKind::Io, format!("net get request-path mkdir: {e}"))
                    })?;
                }
            }
            let body = serde_json::to_vec_pretty(&req)
                .map_err(|e| CliError::new(ErrorKind::Io, format!("net get serialize: {e}")))?;
            std::fs::write(p, body)
                .map_err(|e| CliError::new(ErrorKind::Io, format!("net get request-path: {e}")))?;
        }
        if let Some(p) = response_path {
            if let Some(parent) = p.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        CliError::new(ErrorKind::Io, format!("net get response-path mkdir: {e}"))
                    })?;
                }
            }
            let body = serde_json::to_vec_pretty(&req)
                .map_err(|e| CliError::new(ErrorKind::Io, format!("net get serialize: {e}")))?;
            std::fs::write(p, body)
                .map_err(|e| CliError::new(ErrorKind::Io, format!("net get response-path: {e}")))?;
        }
        Ok(json!({
            "id": index,
            "requestId": request_id,
            "request": req,
            "request_path": request_path.map(|p| p.to_string_lossy().to_string()),
            "response_path": response_path.map(|p| p.to_string_lossy().to_string()),
        }))
    }

    pub async fn dialog(
        &mut self,
        accept: bool,
        prompt_text: Option<&str>,
    ) -> Result<Value, CliError> {
        self.manager
            .handle_dialog(accept, prompt_text)
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Browser,
                    format!("dialog failed: {e}"),
                    "Dialog must be open; use after press that triggers alert/confirm/prompt",
                )
            })?;
        Ok(json!({
            "dialog": if accept { "accept" } else { "dismiss" },
            "prompt_text": prompt_text,
        }))
    }

    pub async fn hover(&mut self, target: &str, include_snapshot: bool) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        interaction::hover(
            &self.manager.client,
            &session_id,
            &self.ref_map,
            target,
            &self.iframe_sessions,
        )
        .await
        .map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Browser,
                format!("hover failed: {e}"),
                "Use a CSS selector or @eN from view in the same process (run script)",
            )
        })?;
        self.drain_events();
        let data = json!({ "hovered": target });
        self.attach_snapshot_if(include_snapshot, data).await
    }

    pub async fn drag(
        &mut self,
        from: &str,
        to: &str,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        interaction::drag(
            &self.manager.client,
            &session_id,
            &self.ref_map,
            from,
            to,
            &self.iframe_sessions,
        )
        .await
        .map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Browser,
                format!("drag failed: {e}"),
                "Use two CSS selectors or @eN refs in the same frame",
            )
        })?;
        self.drain_events();
        let data = json!({ "dragged_from": from, "dragged_to": to });
        self.attach_snapshot_if(include_snapshot, data).await
    }

    pub async fn fill_form(
        &mut self,
        fields: &[(String, String)],
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        let mut filled = Vec::new();
        for (target, value) in fields {
            self.write(target, value, false).await?;
            filled.push(json!({ "target": target, "value_len": value.len() }));
        }
        let data = json!({ "filled": filled, "count": filled.len() });
        self.attach_snapshot_if(include_snapshot, data).await
    }

    pub async fn upload(
        &mut self,
        target: &str,
        path: &Path,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        self.drain_events();
        if !path.is_file() {
            return Err(CliError::with_suggestion(
                ErrorKind::Usage,
                format!("upload path is not a regular file: {}", path.display()),
                "Pass a single regular file path (not a directory)",
            ));
        }
        let abs = path
            .canonicalize()
            .map_err(|e| CliError::new(ErrorKind::Io, format!("upload canonicalize: {e}")))?;
        self.manager
            .upload_files(
                target,
                &[abs.to_string_lossy().to_string()],
                &self.ref_map,
                &self.iframe_sessions,
            )
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Browser,
                    format!("upload failed: {e}"),
                    "Target must be a file input; use CSS selector or @eN",
                )
            })?;
        self.drain_events();
        let data = json!({
            "uploaded": target,
            "path": abs.to_string_lossy(),
        });
        self.attach_snapshot_if(include_snapshot, data).await
    }

    pub async fn back(&mut self) -> Result<Value, CliError> {
        self.history_nav("back").await
    }

    pub async fn forward(&mut self) -> Result<Value, CliError> {
        self.history_nav("forward").await
    }

    pub async fn reload(&mut self, ignore_cache: bool) -> Result<Value, CliError> {
        self.drain_events();
        let script = if ignore_cache {
            "location.reload(true); 'ok'"
        } else {
            "location.reload(); 'ok'"
        };
        self.manager
            .evaluate(script, None)
            .await
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("reload failed: {e}")))?;
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        self.drain_events();
        let url = self.manager.get_url().await.unwrap_or_default();
        let title = self.manager.get_title().await.unwrap_or_default();
        Ok(json!({
            "reloaded": true,
            "ignore_cache": ignore_cache,
            "url": url,
            "title": title,
        }))
    }

    async fn history_nav(&mut self, direction: &str) -> Result<Value, CliError> {
        self.drain_events();
        let script = match direction {
            "back" => "history.back(); 'ok'",
            "forward" => "history.forward(); 'ok'",
            _ => "null",
        };
        self.manager
            .evaluate(script, None)
            .await
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("{direction} failed: {e}")))?;
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        self.drain_events();
        let url = self.manager.get_url().await.unwrap_or_default();
        let title = self.manager.get_title().await.unwrap_or_default();
        Ok(json!({
            "navigation": direction,
            "url": url,
            "title": title,
        }))
    }

    pub async fn page_list(&mut self) -> Result<Value, CliError> {
        self.drain_events();
        let pages: Vec<Value> = self
            .manager
            .pages_list()
            .into_iter()
            .map(|p| {
                json!({
                    "tab_id": p.tab_id,
                    "label": p.label,
                    "url": p.url,
                    "title": p.title,
                    "target_type": p.target_type,
                })
            })
            .collect();
        let active = self.manager.active_tab_id();
        Ok(json!({
            "pages": pages,
            "count": pages.len(),
            "active_tab_id": active,
        }))
    }

    pub async fn page_new(
        &mut self,
        url: Option<&str>,
        background: bool,
        isolated_context: bool,
    ) -> Result<Value, CliError> {
        self.drain_events();
        let mut isolation_note = None;
        let mut isolation_limitation = None;
        let ctx_id = if isolated_context {
            match self.manager.create_browser_context().await {
                Ok(id) => {
                    isolation_note = Some(
                        "isolated BrowserContext created for cookie/storage isolation within this one-shot process"
                            .to_string(),
                    );
                    Some(id)
                }
                Err(e) => {
                    // Some Chromium builds reject Browser.createBrowserContext (-32601).
                    // Fall back to shared context with an explicit limitation for agents.
                    isolation_limitation =
                        Some("isolated_context_unsupported_on_this_browser".to_string());
                    isolation_note = Some(format!(
                        "isolatedContext requested but Browser.createBrowserContext unavailable ({e}); tab uses shared browser context"
                    ));
                    None
                }
            }
        } else {
            None
        };
        let mut data = self
            .manager
            .tab_new_in_context(url, None, ctx_id)
            .await
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("page new failed: {e}")))?;
        // tool-ref background: do not switch focus when true (best-effort)
        if !background {
            if let Some(idx) = data.get("index").and_then(|v| v.as_u64()) {
                let _ = self.manager.tab_switch(idx as usize).await;
            }
        }
        if let Some(obj) = data.as_object_mut() {
            obj.insert("background".into(), json!(background));
            obj.insert("isolated_context".into(), json!(isolated_context));
            if let Some(n) = isolation_note {
                obj.insert("note".into(), json!(n));
            }
            if let Some(lim) = isolation_limitation {
                obj.insert("limitation".into(), json!(lim));
            }
        }
        self.drain_events();
        Ok(data)
    }

    pub async fn page_select(
        &mut self,
        index: usize,
        bring_to_front: bool,
    ) -> Result<Value, CliError> {
        self.drain_events();
        let mut data =
            self.manager.tab_switch(index).await.map_err(|e| {
                CliError::new(ErrorKind::Browser, format!("page select failed: {e}"))
            })?;
        if bring_to_front {
            if let Ok(session_id) = self.manager.active_session_id() {
                let _ = self
                    .manager
                    .client
                    .send_command("Page.bringToFront", None, Some(session_id))
                    .await;
            }
        }
        if let Some(obj) = data.as_object_mut() {
            obj.insert("bring_to_front".into(), json!(bring_to_front));
        }
        self.ref_map.clear();
        self.drain_events();
        Ok(data)
    }

    pub async fn page_close(&mut self, index: Option<usize>) -> Result<Value, CliError> {
        self.drain_events();
        let data =
            self.manager.tab_close(index).await.map_err(|e| {
                CliError::new(ErrorKind::Browser, format!("page close failed: {e}"))
            })?;
        self.ref_map.clear();
        self.drain_events();
        Ok(data)
    }

    pub async fn wait_for(
        &mut self,
        ms: Option<u64>,
        text: Option<&str>,
        selector: Option<&str>,
        state: Option<&str>,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        let mut waited = Vec::new();

        if let Some(st) = state {
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
                        "Use --state load|domcontentloaded|networkidle|none",
                    )
                })?;
            waited.push(json!({"kind": "state", "state": st}));
        }

        if let Some(m) = ms {
            if m > 0 && text.is_none() && selector.is_none() && state.is_none() {
                let data = self.wait_ms(m).await?;
                return self.attach_snapshot_if(include_snapshot, data).await;
            }
            if m > 0 && text.is_none() && selector.is_none() && state.is_some() {
                // pure ms after state already done above
                if m > 0 {
                    let _ = self.wait_ms(m).await?;
                    waited.push(json!({"kind": "ms", "ms": m}));
                }
                let data = json!({ "waited": waited, "ok": true });
                return self.attach_snapshot_if(include_snapshot, data).await;
            }
        }

        if text.is_none() && selector.is_none() && state.is_some() {
            let data = json!({ "waited": waited, "ok": true });
            return self.attach_snapshot_if(include_snapshot, data).await;
        }

        if text.is_none() && selector.is_none() && state.is_none() {
            let data = self.wait_ms(ms.unwrap_or(0)).await?;
            return self.attach_snapshot_if(include_snapshot, data).await;
        }

        let deadline = std::time::Instant::now()
            + std::time::Duration::from_millis(ms.unwrap_or(10_000).max(1));
        loop {
            self.drain_events();
            if let Some(sel) = selector {
                let session_id = self
                    .manager
                    .active_session_id()
                    .map_err(|e| CliError::new(ErrorKind::Browser, e))?
                    .to_string();
                if element::get_element_count(&self.manager.client, &session_id, sel)
                    .await
                    .unwrap_or(0)
                    > 0
                {
                    waited.push(json!({"kind": "selector", "selector": sel}));
                    let data = json!({ "waited": waited, "ok": true });
                    return self.attach_snapshot_if(include_snapshot, data).await;
                }
            }
            if let Some(t) = text {
                let body = self
                    .manager
                    .evaluate("document.body ? document.body.innerText : ''", None)
                    .await
                    .unwrap_or(json!(""));
                let hay = body.as_str().unwrap_or("");
                if hay.contains(t) {
                    waited.push(json!({"kind": "text", "text": t}));
                    let data = json!({ "waited": waited, "ok": true });
                    return self.attach_snapshot_if(include_snapshot, data).await;
                }
            }
            if std::time::Instant::now() >= deadline {
                return Err(CliError::with_suggestion(
                    ErrorKind::Timeout,
                    "wait condition not met before deadline",
                    "Increase --ms, set --state, or ensure page content is present",
                ));
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn emulate(
        &mut self,
        user_agent: Option<&str>,
        locale: Option<&str>,
        timezone: Option<&str>,
        offline: bool,
        latitude: Option<f64>,
        longitude: Option<f64>,
        media: Option<&str>,
        network_conditions: Option<&str>,
        cpu_throttling_rate: Option<f64>,
        color_scheme: Option<&str>,
        extra_headers_json: Option<&str>,
        viewport: Option<&str>,
    ) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        if let Some(ua) = user_agent {
            if ua.is_empty() {
                // clear override with empty UA not portable; skip
            } else {
                self.manager
                    .set_user_agent(ua)
                    .await
                    .map_err(|e| CliError::new(ErrorKind::Browser, format!("emulate ua: {e}")))?;
            }
        }
        if let Some(loc) = locale {
            self.manager
                .set_locale(loc)
                .await
                .map_err(|e| CliError::new(ErrorKind::Browser, format!("emulate locale: {e}")))?;
        }
        if let Some(tz) = timezone {
            self.manager
                .set_timezone(tz)
                .await
                .map_err(|e| CliError::new(ErrorKind::Browser, format!("emulate timezone: {e}")))?;
        }

        let mut applied_network = None;
        if let Some(name) = network_conditions {
            let preset = crate::constants::network_preset_by_name(name).ok_or_else(|| {
                CliError::with_suggestion(
                    ErrorKind::Usage,
                    format!("unknown network conditions: {name}"),
                    format!(
                        "Use one of: {}",
                        crate::constants::network_preset_names().join(", ")
                    ),
                )
            })?;
            network::set_network_conditions(
                &self.manager.client,
                &session_id,
                preset.offline,
                preset.latency_ms,
                preset.download_throughput,
                preset.upload_throughput,
            )
            .await
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("emulate network: {e}")))?;
            applied_network = Some(preset.name);
        } else if offline {
            network::set_offline(&self.manager.client, &session_id, true)
                .await
                .map_err(|e| CliError::new(ErrorKind::Browser, format!("emulate offline: {e}")))?;
            applied_network = Some("Offline");
        }

        if let Some(rate) = cpu_throttling_rate {
            let rate = rate.clamp(1.0, 20.0);
            network::set_cpu_throttling_rate(&self.manager.client, &session_id, rate)
                .await
                .map_err(|e| {
                    CliError::new(ErrorKind::Browser, format!("emulate cpu throttle: {e}"))
                })?;
        }

        if let (Some(lat), Some(lon)) = (latitude, longitude) {
            self.manager
                .set_geolocation(lat, lon, Some(1.0))
                .await
                .map_err(|e| CliError::new(ErrorKind::Browser, format!("emulate geo: {e}")))?;
        }

        if let Some(scheme) = color_scheme {
            let value = match scheme.to_ascii_lowercase().as_str() {
                "dark" => "dark",
                "light" => "light",
                "auto" => "",
                other => {
                    return Err(CliError::with_suggestion(
                        ErrorKind::Usage,
                        format!("invalid color-scheme: {other}"),
                        "Use dark, light, or auto",
                    ));
                }
            };
            self.manager
                .set_emulated_media(
                    media,
                    Some(vec![("prefers-color-scheme".into(), value.into())]),
                )
                .await
                .map_err(|e| {
                    CliError::new(ErrorKind::Browser, format!("emulate color-scheme: {e}"))
                })?;
        } else if let Some(m) = media {
            self.manager
                .set_emulated_media(Some(m), None)
                .await
                .map_err(|e| CliError::new(ErrorKind::Browser, format!("emulate media: {e}")))?;
        }

        if let Some(headers_raw) = extra_headers_json {
            let map: HashMap<String, String> = if headers_raw.trim().is_empty() {
                HashMap::new()
            } else {
                serde_json::from_str(headers_raw).map_err(|e| {
                    CliError::with_suggestion(
                        ErrorKind::Usage,
                        format!("invalid extra-headers JSON: {e}"),
                        r#"Pass object JSON e.g. {"X-Custom":"1"}"#,
                    )
                })?
            };
            network::set_extra_headers(&self.manager.client, &session_id, &map)
                .await
                .map_err(|e| CliError::new(ErrorKind::Browser, format!("emulate headers: {e}")))?;
        }

        let mut applied_viewport = None;
        if let Some(vp) = viewport {
            let spec = crate::constants::parse_viewport_spec(vp).map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Usage,
                    e,
                    "Format: WxHxDPR[,mobile][,touch][,landscape]",
                )
            })?;
            self.manager
                .set_viewport(
                    spec.width,
                    spec.height,
                    spec.device_scale_factor,
                    spec.mobile,
                )
                .await
                .map_err(|e| CliError::new(ErrorKind::Browser, format!("emulate viewport: {e}")))?;
            applied_viewport = Some(json!({
                "width": spec.width,
                "height": spec.height,
                "device_scale_factor": spec.device_scale_factor,
                "mobile": spec.mobile,
                "has_touch": spec.has_touch,
                "is_landscape": spec.is_landscape,
            }));
        }

        Ok(json!({
            "emulated": true,
            "user_agent": user_agent,
            "locale": locale,
            "timezone": timezone,
            "offline": offline || applied_network == Some("Offline"),
            "latitude": latitude,
            "longitude": longitude,
            "media": media,
            "network_conditions": applied_network,
            "cpu_throttling_rate": cpu_throttling_rate,
            "color_scheme": color_scheme,
            "extra_headers": extra_headers_json.is_some(),
            "viewport": applied_viewport,
        }))
    }

    pub async fn resize(
        &mut self,
        width: i32,
        height: i32,
        scale: f64,
        mobile: bool,
    ) -> Result<Value, CliError> {
        self.drain_events();
        self.manager
            .set_viewport(width, height, scale, mobile)
            .await
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("resize failed: {e}")))?;
        Ok(json!({
            "width": width,
            "height": height,
            "scale": scale,
            "mobile": mobile,
        }))
    }

    pub async fn perf_start(
        &mut self,
        path: Option<&Path>,
        reload: bool,
        auto_stop: bool,
    ) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        self.trace_chunks.clear();
        self.manager
            .client
            .send_command(
                "Tracing.start",
                Some(json!({
                    "categories": "devtools.timeline,v8.execute,blink.user_timing,disabled-by-default-devtools.timeline",
                    "transferMode": "ReportEvents",
                })),
                None,
            )
            .await
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("perf start: {e}")))?;
        let _ = self
            .manager
            .client
            .send_command_no_params("Performance.enable", Some(&session_id))
            .await;
        self.perf_active = true;
        if reload {
            let _ = self.reload(false).await?;
        }
        let mut out = json!({
            "perf": "start",
            "path": path.map(|p| p.to_string_lossy().to_string()),
            "reload": reload,
            "auto_stop": auto_stop,
        });
        if auto_stop {
            // tool-ref autoStop: stop after load/reload settles
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let stop = self.perf_stop(path).await?;
            if let Some(obj) = out.as_object_mut() {
                obj.insert("auto_stopped".into(), json!(true));
                obj.insert("stop".into(), stop);
            }
        }
        Ok(out)
    }

    pub async fn perf_stop(&mut self, path: Option<&Path>) -> Result<Value, CliError> {
        self.pump_events().await;
        self.tracing_complete = false;
        if self.perf_active {
            let _ = self
                .manager
                .client
                .send_command("Tracing.end", None, None)
                .await;
            self.perf_active = false;
        }
        // Wait for dataCollected + tracingComplete (up to ~5s).
        for _ in 0..100 {
            self.pump_events().await;
            if self.tracing_complete && !self.trace_chunks.is_empty() {
                for _ in 0..5 {
                    self.pump_events().await;
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                }
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        let body = self.trace_chunks.join("\n");
        let chunks = self.trace_chunks.len();
        self.last_trace_body = Some(body.clone());
        let mut out_path = path.map(|p| p.to_path_buf());
        if out_path.is_none() {
            // Default artifact so insight can always read a file after stop.
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            out_path = Some(PathBuf::from(format!("trace-{stamp}.ndjson")));
        }
        if let Some(ref p) = out_path {
            if let Some(parent) = p.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        CliError::new(ErrorKind::Io, format!("perf stop mkdir: {e}"))
                    })?;
                }
            }
            std::fs::write(p, body.as_bytes())
                .map_err(|e| CliError::new(ErrorKind::Io, format!("perf stop write: {e}")))?;
            self.last_trace_path = Some(p.clone());
        }
        self.trace_chunks.clear();
        self.tracing_complete = false;
        // Synthetic insight sets for tool-ref performance_analyze_insight flow
        let set_id = format!(
            "set-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        );
        Ok(json!({
            "perf": "stop",
            "path": out_path.map(|p| p.to_string_lossy().to_string()),
            "events": chunks,
            "available_insight_sets": [{
                "insight_set_id": set_id,
                "insights": [
                    "DocumentLatency",
                    "LCPBreakdown",
                    "CLSCulprits",
                    "INPBreakdown",
                    "RenderBlocking",
                    "ThirdParties"
                ]
            }],
        }))
    }

    pub async fn perf_insight(
        &mut self,
        name: Option<&str>,
        insight_set_id: Option<&str>,
    ) -> Result<Value, CliError> {
        self.pump_events().await;
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        let live_metrics = self
            .manager
            .client
            .send_command("Performance.getMetrics", None, Some(&session_id))
            .await
            .ok();

        let offline = if let Some(ref p) = self.last_trace_path {
            crate::native::perf_insight::analyze_file(p, name).ok()
        } else if let Some(ref body) = self.last_trace_body {
            crate::native::perf_insight::analyze_text(body, name, None).ok()
        } else {
            None
        };

        Ok(json!({
            "perf": "insight",
            "name": name,
            "insight_name": name,
            "insight_set_id": insight_set_id,
            "live_metrics": live_metrics,
            "trace_insight": offline,
            "trace_path": self.last_trace_path.as_ref().map(|p| p.to_string_lossy().to_string()),
        }))
    }

    /// Offline insight from a previously written trace path (no browser required).
    pub fn perf_insight_file(path: &Path, name: Option<&str>) -> Result<Value, CliError> {
        crate::native::perf_insight::analyze_file(path, name).map_err(|e| {
            CliError::with_suggestion(ErrorKind::Io, e, "Pass a path produced by perf stop --path")
        })
    }

    pub async fn screencast_start(&mut self, path: Option<&Path>) -> Result<Value, CliError> {
        self.pump_events().await;
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        self.screencast_frames.clear();
        self.screencast_ack_ids.clear();
        let dir = path.map(|p| p.to_path_buf()).unwrap_or_else(|| {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            PathBuf::from(format!("screencast-{stamp}"))
        });
        std::fs::create_dir_all(&dir)
            .map_err(|e| CliError::new(ErrorKind::Io, format!("screencast dir: {e}")))?;
        self.screencast_dir = Some(dir.clone());
        // Page domain must be enabled for screencast frames.
        let _ = self
            .manager
            .client
            .send_command_no_params("Page.enable", Some(&session_id))
            .await;
        self.manager
            .client
            .send_command(
                "Page.startScreencast",
                Some(json!({
                    "format": "png",
                    "quality": 60,
                    "maxWidth": 1280,
                    "maxHeight": 720,
                    "everyNthFrame": 1,
                })),
                Some(&session_id),
            )
            .await
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("screencast start: {e}")))?;
        self.screencast_active = true;
        // Pump a few frames immediately so FrameAck unblocks the pipeline.
        for _ in 0..15 {
            self.pump_events().await;
            tokio::time::sleep(std::time::Duration::from_millis(40)).await;
        }
        Ok(json!({
            "screencast": "start",
            "dir": dir.to_string_lossy(),
            "note": "Frames buffered in process; stop writes PNG files + manifest.json",
            "frames_buffered": self.screencast_frames.len(),
        }))
    }

    pub async fn screencast_stop(&mut self, path: Option<&Path>) -> Result<Value, CliError> {
        for _ in 0..40 {
            self.pump_events().await;
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        }
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        if self.screencast_active {
            let _ = self
                .manager
                .client
                .send_command("Page.stopScreencast", None, Some(&session_id))
                .await;
            self.screencast_active = false;
        }
        self.pump_events().await;

        let dir = self.screencast_dir.clone().unwrap_or_else(|| {
            PathBuf::from(format!(
                "screencast-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0)
            ))
        });
        std::fs::create_dir_all(&dir)
            .map_err(|e| CliError::new(ErrorKind::Io, format!("screencast stop mkdir: {e}")))?;

        use base64::Engine;
        let engine = base64::engine::general_purpose::STANDARD;
        let mut written = 0u64;
        let mut paths: Vec<String> = Vec::new();
        for (i, b64) in self.screencast_frames.iter().enumerate() {
            let bytes = match engine.decode(b64) {
                Ok(b) => b,
                Err(_) => continue,
            };
            let name = format!("frame-{:05}.png", i + 1);
            let out = dir.join(&name);
            if std::fs::write(&out, &bytes).is_ok() {
                written += 1;
                paths.push(out.to_string_lossy().into_owned());
            }
        }
        let video_path = path.map(|p| p.to_path_buf()).or_else(|| {
            // If start path looked like a video file, encode there
            self.screencast_dir.as_ref().and_then(|d| {
                let s = d.to_string_lossy();
                if s.ends_with(".webm") || s.ends_with(".mp4") {
                    Some(d.clone())
                } else {
                    None
                }
            })
        });
        let mut video_out: Option<String> = None;
        let mut encode_note: Option<String> = None;
        if let Some(ref vp) = video_path {
            let ext = vp
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("mp4")
                .to_ascii_lowercase();
            let is_video = ext == "webm" || ext == "mp4";
            if is_video && written > 0 {
                if let Some(parent) = vp.parent() {
                    if !parent.as_os_str().is_empty() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                }
                let pattern = dir.join("frame-%05d.png");
                let vcodec = if ext == "webm" {
                    "libvpx-vp9"
                } else {
                    "libx264"
                };
                let mut cmd = std::process::Command::new("ffmpeg");
                cmd.arg("-y")
                    .arg("-framerate")
                    .arg("10")
                    .arg("-i")
                    .arg(&pattern)
                    .arg("-c:v")
                    .arg(vcodec)
                    .arg("-pix_fmt")
                    .arg("yuv420p")
                    .arg(vp);
                match cmd.output() {
                    Ok(out) if out.status.success() => {
                        video_out = Some(vp.to_string_lossy().into_owned());
                        encode_note = Some("encoded via ffmpeg".into());
                    }
                    Ok(out) => {
                        encode_note = Some(format!(
                            "ffmpeg failed: {}",
                            String::from_utf8_lossy(&out.stderr)
                        ));
                    }
                    Err(e) => {
                        encode_note =
                            Some(format!("ffmpeg not available: {e}; PNG frames kept in dir"));
                    }
                }
            }
        }
        let manifest = json!({
            "format": "png",
            "frame_count": written,
            "frames": paths,
            "video": video_out,
            "encode_note": encode_note,
            "ffmpeg_hint": format!(
                "ffmpeg -y -framerate 10 -i {}/frame-%05d.png -c:v libx264 -pix_fmt yuv420p {}.mp4",
                dir.display(),
                dir.display()
            ),
        });
        let manifest_path = dir.join("manifest.json");
        let _ = std::fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).unwrap_or_default(),
        );
        self.screencast_frames.clear();
        self.screencast_ack_ids.clear();
        Ok(json!({
            "screencast": "stop",
            "dir": dir.to_string_lossy(),
            "frame_count": written,
            "manifest": manifest_path.to_string_lossy(),
            "video": video_out,
            "encode_note": encode_note,
        }))
    }

    pub async fn heap_take(&mut self, path: &Path) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        self.heap_chunks.clear();
        self.heap_snapshot_finished = false;
        let _ = self
            .manager
            .client
            .send_command_no_params("HeapProfiler.enable", Some(&session_id))
            .await;
        self.manager
            .client
            .send_command(
                "HeapProfiler.takeHeapSnapshot",
                Some(json!({ "reportProgress": true })),
                Some(&session_id),
            )
            .await
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("heap take: {e}")))?;
        // Wait for chunks + progress finished (up to ~10s).
        for _ in 0..200 {
            self.drain_events();
            if self.heap_snapshot_finished && !self.heap_chunks.is_empty() {
                // Drain a few more ticks for trailing chunks.
                for _ in 0..10 {
                    self.drain_events();
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                }
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        // Final drain
        for _ in 0..20 {
            self.drain_events();
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| CliError::new(ErrorKind::Io, format!("heap take mkdir: {e}")))?;
            }
        }
        let body = self.heap_chunks.join("");
        let bytes = body.len();
        if bytes == 0 {
            return Err(CliError::with_suggestion(
                ErrorKind::Browser,
                "heap take produced empty snapshot (no HeapProfiler chunks received)",
                "Ensure Chrome supports HeapProfiler; re-run doctor; check event forwarders",
            ));
        }
        std::fs::write(path, body.as_bytes())
            .map_err(|e| CliError::new(ErrorKind::Io, format!("heap take write: {e}")))?;
        self.heap_chunks.clear();
        self.heap_snapshot_finished = false;
        Ok(json!({
            "heap": "take",
            "path": path.to_string_lossy(),
            "bytes": bytes,
        }))
    }

    pub fn heap_file_summary(path: &Path) -> Result<Value, CliError> {
        crate::native::heap_snapshot::summarize(path).map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Io,
                e,
                "Pass a path produced by heap take (.heapsnapshot JSON)",
            )
        })
    }

    pub fn heap_close(path: &Path) -> Result<Value, CliError> {
        crate::native::heap_snapshot::close_snapshot(path).map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Io,
                e,
                "Pass a path produced by heap take (.heapsnapshot JSON)",
            )
        })
    }

    pub fn heap_compare(base: &Path, current: &Path) -> Result<Value, CliError> {
        crate::native::heap_snapshot::compare(base, current).map_err(|e| {
            CliError::with_suggestion(ErrorKind::Io, e, "Pass two paths produced by heap take")
        })
    }

    pub fn heap_details(path: &Path) -> Result<Value, CliError> {
        crate::native::heap_snapshot::details(path).map_err(|e| {
            CliError::with_suggestion(ErrorKind::Io, e, "Pass a valid .heapsnapshot path")
        })
    }

    pub fn heap_dup_strings(path: &Path) -> Result<Value, CliError> {
        crate::native::heap_snapshot::duplicate_strings(path).map_err(|e| {
            CliError::with_suggestion(ErrorKind::Io, e, "Pass a valid .heapsnapshot path")
        })
    }

    pub fn heap_class_nodes(path: &Path, id: u64) -> Result<Value, CliError> {
        crate::native::heap_snapshot::class_nodes(path, id).map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Io,
                e,
                "Pass a valid .heapsnapshot path and class id",
            )
        })
    }

    pub fn heap_node_op(path: &Path, node: u64, op: &str) -> Result<Value, CliError> {
        crate::native::heap_snapshot::node_op(path, node, op).map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Io,
                e,
                "Pass a valid .heapsnapshot path and node id (or 0-based index)",
            )
        })
    }

    /// Offline object details for one node id (distance, retained size, detachedness).
    pub fn heap_object_details(path: &Path, node: u64) -> Result<Value, CliError> {
        crate::native::heap_snapshot::object_details(path, node).map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Io,
                e,
                "Pass a valid .heapsnapshot path and node id (or 0-based index)",
            )
        })
    }

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

    /// Reload extension service worker target by id prefix (one-shot CDP).
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
                "Run extension list after extension install <unpacked-dir>",
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
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
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
                "Use extension list; trigger requires a service_worker target",
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

    /// Discover third-party developer tools via `devtoolstooldiscovery` CustomEvent.
    pub async fn devtools3p_list(&mut self) -> Result<Value, CliError> {
        self.pump_events().await;
        let expr = r#"(() => {
          return new Promise((resolve) => {
            if (!window.__dtmcp) window.__dtmcp = {};
            window.__dtmcp.toolGroups = [];
            const groups = [];
            const event = new CustomEvent('devtoolstooldiscovery');
            event.respondWith = (toolGroup) => {
              if (!toolGroup || typeof toolGroup.name !== 'string' || !Array.isArray(toolGroup.tools)) {
                return;
              }
              const tools = [];
              for (const tool of toolGroup.tools) {
                if (!tool || typeof tool.name !== 'string') continue;
                tools.push({
                  name: tool.name,
                  description: typeof tool.description === 'string' ? tool.description : '',
                  inputSchema: tool.inputSchema || {},
                });
              }
              const g = {
                name: toolGroup.name,
                description: typeof toolGroup.description === 'string' ? toolGroup.description : '',
                tools,
              };
              groups.push(g);
              window.__dtmcp.toolGroups.push({
                name: g.name,
                description: g.description,
                tools: toolGroup.tools,
              });
              if (!window.__dtmcp.executeTool) {
                window.__dtmcp.executeTool = async (toolName, args) => {
                  for (const group of (window.__dtmcp.toolGroups || [])) {
                    const t = (group.tools || []).find((x) => x.name === toolName);
                    if (t && typeof t.execute === 'function') {
                      return await t.execute(args || {});
                    }
                  }
                  throw new Error('Tool ' + toolName + ' not found');
                };
              }
            };
            window.dispatchEvent(event);
            setTimeout(() => resolve(groups), 0);
          });
        })()"#;
        let result = self.eval(expr, None, Some("accept"), None).await?;
        let groups = result
            .get("result")
            .cloned()
            .or_else(|| result.get("value").cloned())
            .unwrap_or(result);
        let tools_flat: Vec<Value> = groups
            .as_array()
            .map(|arr| {
                arr.iter()
                    .flat_map(|g| {
                        g.get("tools")
                            .and_then(|t| t.as_array())
                            .cloned()
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(json!({
            "groups": groups,
            "tools": tools_flat,
            "count": tools_flat.len(),
            "available": true,
        }))
    }

    pub async fn devtools3p_exec(
        &mut self,
        name: &str,
        params_json: Option<&str>,
    ) -> Result<Value, CliError> {
        let _ = self.devtools3p_list().await?;
        let params = params_json.unwrap_or("{}");
        // Validate JSON object
        let parsed: Value = serde_json::from_str(params).map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Usage,
                format!("invalid params JSON: {e}"),
                r#"Pass --params '{"key":"value"}'"#,
            )
        })?;
        if !parsed.is_object() {
            return Err(CliError::with_suggestion(
                ErrorKind::Usage,
                "params must be a JSON object",
                r#"Pass --params '{"key":"value"}'"#,
            ));
        }
        let name_js = serde_json::to_string(name).unwrap_or_else(|_| "\"\"".into());
        let params_js = parsed.to_string();
        let expr = format!(
            r#"(async () => {{
              if (!window.__dtmcp || typeof window.__dtmcp.executeTool !== 'function') {{
                throw new Error('No third-party tools discovered on page');
              }}
              const out = await window.__dtmcp.executeTool({name_js}, {params_js});
              try {{ return JSON.parse(JSON.stringify(out)); }} catch (_) {{ return String(out); }}
            }})()"#
        );
        let result = self.eval(&expr, None, Some("accept"), None).await?;
        if result.get("exceptionDetails").is_some() {
            return Err(CliError::with_suggestion(
                ErrorKind::NoInput,
                format!("devtools3p exec {name} failed"),
                "List tools with browser-automation-cli --category-third-party devtools3p list --url <page>",
            ));
        }
        let value = result
            .get("result")
            .cloned()
            .or_else(|| result.get("value").cloned())
            .unwrap_or(result);
        Ok(json!({
            "name": name,
            "result": value,
            "ok": true,
        }))
    }

    /// List WebMCP / declarative tool forms on the page (Chrome 149+ features).
    pub async fn webmcp_list(&mut self) -> Result<Value, CliError> {
        self.pump_events().await;
        let expr = r#"(() => {
          const tools = [];
          // Declarative form-based tools (test harness / early WebMCP)
          document.querySelectorAll('form[toolname]').forEach((form) => {
            tools.push({
              name: form.getAttribute('toolname') || '',
              description: form.getAttribute('tooldescription') || '',
              source: 'form',
            });
          });
          // Future navigator surface (best-effort)
          try {
            if (navigator.modelContext && typeof navigator.modelContext.listTools === 'function') {
              // sync list not always available; ignore
            }
          } catch (_) {}
          if (window.__webmcpTools && Array.isArray(window.__webmcpTools)) {
            for (const t of window.__webmcpTools) {
              if (t && t.name) tools.push({ name: t.name, description: t.description || '', source: 'window' });
            }
          }
          return tools;
        })()"#;
        let result = self.eval(expr, None, Some("accept"), None).await?;
        let tools = result
            .get("result")
            .cloned()
            .or_else(|| result.get("value").cloned())
            .unwrap_or(result);
        let count = tools.as_array().map(|a| a.len()).unwrap_or(0);
        Ok(json!({
            "tools": tools,
            "count": count,
            "available": true,
            "note": "Requires Chrome with WebMCP/DevToolsWebMCPSupport for full surface; form[toolname] always listed",
        }))
    }

    pub async fn webmcp_exec(
        &mut self,
        name: &str,
        input_json: Option<&str>,
    ) -> Result<Value, CliError> {
        let input = input_json.unwrap_or("{}");
        let parsed: Value = serde_json::from_str(input).map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Usage,
                format!("invalid input JSON: {e}"),
                r#"Pass --input '{"key":"value"}'"#,
            )
        })?;
        let name_js = serde_json::to_string(name).unwrap_or_else(|_| "\"\"".into());
        let input_js = parsed.to_string();
        let expr = format!(
            r#"(async () => {{
              const toolName = {name_js};
              const input = {input_js};
              // Form-based tools
              const form = document.querySelector('form[toolname="' + CSS.escape(toolName) + '"]')
                || document.querySelector('form[toolname="' + toolName + '"]');
              if (form) {{
                return await new Promise((resolve, reject) => {{
                  const handler = (event) => {{
                    event.preventDefault();
                    try {{
                      if (typeof event.respondWith === 'function') {{
                        // page may set respondWith on submit
                      }}
                    }} catch (_) {{}}
                  }};
                  form.addEventListener('submit', handler, {{ once: true }});
                  // Prefer page-defined onsubmit
                  if (typeof form.onsubmit === 'function') {{
                    const fake = {{
                      preventDefault() {{}},
                      respondWith(v) {{ resolve({{ status: 'Completed', output: v }}); }},
                    }};
                    try {{
                      form.onsubmit(fake);
                      setTimeout(() => resolve({{ status: 'Completed', output: null }}), 0);
                    }} catch (e) {{
                      reject(e);
                    }}
                    return;
                  }}
                  form.requestSubmit ? form.requestSubmit() : form.submit();
                  setTimeout(() => resolve({{ status: 'Completed', output: null, note: 'form submitted' }}), 50);
                }});
              }}
              if (window.__webmcpTools) {{
                const t = window.__webmcpTools.find((x) => x.name === toolName);
                if (t && typeof t.execute === 'function') {{
                  const out = await t.execute(input);
                  return {{ status: 'Completed', output: out }};
                }}
              }}
              throw new Error('Tool ' + toolName + ' not found');
            }})()"#
        );
        let result = self.eval(&expr, None, Some("accept"), None).await?;
        if result.get("exceptionDetails").is_some() {
            let msg = result
                .pointer("/exceptionDetails/exception/description")
                .or_else(|| result.pointer("/exceptionDetails/text"))
                .and_then(|v| v.as_str())
                .unwrap_or("tool not found");
            return Err(CliError::with_suggestion(
                ErrorKind::NoInput,
                format!("webmcp exec {name}: {msg}"),
                "List tools first; page must expose form[toolname] or __webmcpTools",
            ));
        }
        let value = result
            .get("result")
            .cloned()
            .or_else(|| result.get("value").cloned())
            .unwrap_or(result);
        Ok(json!({
            "name": name,
            "result": value,
            "ok": true,
        }))
    }

    /// Close CDP + wait/kill child (FINALIZE core).
    pub async fn shutdown(mut self) -> Result<(), CliError> {
        self.console_log.clear();
        self.network_log.clear();
        self.heap_chunks.clear();
        self.trace_chunks.clear();
        self.screencast_frames.clear();
        self.screencast_ack_ids.clear();
        self.screencast_dir = None;
        self.last_trace_body = None;
        self.ref_map.clear();
        self.manager.close().await.map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Browser,
                format!("Browser close failed: {e}"),
                "Process reaped by chromiumoxide finalize or Lightpanda process Drop",
            )
        })
    }
}

fn verify_image_magic(path: &Path, format: &str) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    match format {
        "png" => bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]),
        "jpeg" | "jpg" => bytes.starts_with(&[0xFF, 0xD8, 0xFF]),
        "webp" => {
            bytes.len() >= 12
                && bytes[0..4] == [0x52, 0x49, 0x46, 0x46]
                && bytes[8..12] == [0x57, 0x45, 0x42, 0x50]
        }
        _ => !bytes.is_empty(),
    }
}

/// Rewrite native `[ref=eN]` markers to agent-facing `[@eN]`.
pub fn tree_to_at_refs(tree: &str) -> String {
    let mut out = String::with_capacity(tree.len());
    let bytes = tree.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(b"ref=") && i + 4 < bytes.len() && bytes[i + 4] == b'e' {
            out.push('@');
            i += 4;
            while i < bytes.len() && bytes[i].is_ascii_alphanumeric() {
                out.push(bytes[i] as char);
                i += 1;
            }
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// Normalize JS for `Runtime.evaluate`.
///
/// - With `--args`, always call as `({expr})(arg0,…)`.
/// - Bare function / arrow: call once as `({expr})()`.
/// - Already-invoked IIFE ending in `)()`: leave as-is (never double-call).
/// - Plain expressions: leave as-is.
fn normalize_eval_expression(
    expression: &str,
    args_json: Option<&str>,
) -> Result<String, CliError> {
    if let Some(args_raw) = args_json {
        let uids: Vec<String> = serde_json::from_str(args_raw).map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Usage,
                format!("eval --args must be a JSON array of uids: {e}"),
                r#"Example: --args '["@e1","@e2"]'"#,
            )
        })?;
        let args_js: Vec<String> = uids
            .iter()
            .map(|u| {
                let cleaned = u.trim().trim_start_matches('@');
                format!("\"{cleaned}\"")
            })
            .collect();
        let joined = args_js.join(",");
        return Ok(format!("({expression})({joined})"));
    }

    let trimmed = expression.trim();
    // Strip a single trailing semicolon for IIFE detection only.
    let for_detect = trimmed.trim_end_matches(';').trim_end();
    // Already invoked: `(...)()` or `(async ...)()` — re-wrapping yields "is not a function".
    if for_detect.ends_with(")()") {
        return Ok(expression.to_string());
    }

    let head = trimmed.trim_start();
    let is_bare_callable = head.starts_with("function")
        || head.starts_with("async function")
        || (head.starts_with("async") && trimmed.contains("=>"))
        || (head.starts_with('(') && trimmed.contains("=>"));

    if is_bare_callable {
        // Bare function / arrow needs a single call site.
        return Ok(format!("({expression})()"));
    }

    Ok(expression.to_string())
}

fn mark_launched(life: &Lifecycle, pid: Option<u32>) {
    if let Ok(mut ledger) = life.ledger.lock() {
        ledger.chrome_launched = true;
        ledger.chrome_pid = pid;
    }
}

fn mark_closed(life: &Lifecycle) {
    if let Ok(mut ledger) = life.ledger.lock() {
        ledger.chrome_launched = false;
        ledger.chrome_pid = None;
        ledger.profile_dir = None;
    }
}

async fn launch_marked(life: &Lifecycle, capture: CaptureOpts) -> Result<OneShotSession, CliError> {
    let session = OneShotSession::launch_headless_with_capture(capture).await?;
    mark_launched(life, session.chrome_pid());
    Ok(session)
}

#[cfg(test)]
mod eval_normalize_tests {
    use super::normalize_eval_expression;

    #[test]
    fn leaves_invoked_iife_alone() {
        let e = "(() => { return 9; })()";
        assert_eq!(normalize_eval_expression(e, None).unwrap(), e);
        let e2 = "(async () => 1)()";
        assert_eq!(normalize_eval_expression(e2, None).unwrap(), e2);
        let e3 = "(function(){ return 2; })()";
        assert_eq!(normalize_eval_expression(e3, None).unwrap(), e3);
    }

    #[test]
    fn wraps_bare_arrow_once() {
        assert_eq!(
            normalize_eval_expression("() => 7", None).unwrap(),
            "(() => 7)()"
        );
        assert_eq!(
            normalize_eval_expression("async () => 3", None).unwrap(),
            "(async () => 3)()"
        );
    }

    #[test]
    fn leaves_plain_expression() {
        assert_eq!(normalize_eval_expression("1+1", None).unwrap(), "1+1");
        assert_eq!(normalize_eval_expression("(1+1)", None).unwrap(), "(1+1)");
        assert_eq!(
            normalize_eval_expression("document.title", None).unwrap(),
            "document.title"
        );
    }

    #[test]
    fn args_force_call() {
        let out = normalize_eval_expression("(el) => el", Some(r#"["@e1"]"#)).unwrap();
        assert_eq!(out, r#"((el) => el)("e1")"#);
    }
}

async fn finish(
    life: &Lifecycle,
    session: OneShotSession,
    work_res: Result<Value, CliError>,
) -> Result<Value, CliError> {
    let close_res = session.shutdown().await;
    mark_closed(life);
    match (work_res, close_res) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), _) => Err(e),
        (Ok(_), Err(e)) => Err(e),
    }
}

pub async fn run_goto(life: &Lifecycle, url: &str) -> Result<Value, CliError> {
    run_goto_with_robots(
        life,
        url,
        CaptureOpts::default(),
        crate::robots::RobotsPolicy::Honor,
    )
    .await
}

pub async fn run_goto_with_robots(
    life: &Lifecycle,
    url: &str,
    capture: CaptureOpts,
    robots: crate::robots::RobotsPolicy,
) -> Result<Value, CliError> {
    let mut session = launch_marked(life, capture).await?;
    let work = session.goto(url, robots).await;
    finish(life, session, work).await
}

pub async fn run_scrape(
    life: &Lifecycle,
    url: &str,
    robots: crate::robots::RobotsPolicy,
    capture: CaptureOpts,
) -> Result<Value, CliError> {
    let mut session = launch_marked(life, capture).await?;
    let work = session.scrape(url, robots).await;
    finish(life, session, work).await
}

pub async fn run_goto_capture(
    life: &Lifecycle,
    url: &str,
    capture: CaptureOpts,
) -> Result<Value, CliError> {
    run_goto_with_robots(life, url, capture, crate::robots::RobotsPolicy::Honor).await
}

pub async fn run_view(
    life: &Lifecycle,
    verbose: bool,
    capture: CaptureOpts,
) -> Result<Value, CliError> {
    let mut session = launch_marked(life, capture).await?;
    let work = async {
        let _ = session
            .goto("about:blank", crate::robots::RobotsPolicy::Honor)
            .await?;
        session.view(verbose).await
    }
    .await;
    finish(life, session, work).await
}

pub async fn run_press(
    life: &Lifecycle,
    target: &str,
    dblclick: bool,
    include_snapshot: bool,
    capture: CaptureOpts,
) -> Result<Value, CliError> {
    let mut session = launch_marked(life, capture).await?;
    let work = async {
        let _ = session
            .goto("about:blank", crate::robots::RobotsPolicy::Honor)
            .await?;
        session.press(target, dblclick, include_snapshot).await
    }
    .await;
    finish(life, session, work).await
}

pub async fn run_write(
    life: &Lifecycle,
    target: &str,
    value: &str,
    include_snapshot: bool,
    capture: CaptureOpts,
) -> Result<Value, CliError> {
    let mut session = launch_marked(life, capture).await?;
    let work = async {
        let _ = session
            .goto("about:blank", crate::robots::RobotsPolicy::Honor)
            .await?;
        session.write(target, value, include_snapshot).await
    }
    .await;
    finish(life, session, work).await
}

pub async fn run_keys(
    life: &Lifecycle,
    key: &str,
    include_snapshot: bool,
    capture: CaptureOpts,
) -> Result<Value, CliError> {
    let mut session = launch_marked(life, capture).await?;
    let work = async {
        let _ = session
            .goto("about:blank", crate::robots::RobotsPolicy::Honor)
            .await?;
        session.keys(key, include_snapshot).await
    }
    .await;
    finish(life, session, work).await
}

pub async fn run_type(
    life: &Lifecycle,
    target: Option<&str>,
    text: &str,
    clear: bool,
    submit: Option<&str>,
    focus_only: bool,
    capture: CaptureOpts,
) -> Result<Value, CliError> {
    let mut session = launch_marked(life, capture).await?;
    let work = async {
        let _ = session
            .goto("about:blank", crate::robots::RobotsPolicy::Honor)
            .await?;
        session
            .type_text(target, text, clear, submit, focus_only)
            .await
    }
    .await;
    finish(life, session, work).await
}

pub async fn run_with_session<F, Fut>(
    life: &Lifecycle,
    capture: CaptureOpts,
    work: F,
) -> Result<Value, CliError>
where
    F: FnOnce(OneShotSession) -> Fut,
    Fut: std::future::Future<Output = Result<(OneShotSession, Value), CliError>>,
{
    let session = launch_marked(life, capture).await?;
    match work(session).await {
        Ok((session, value)) => finish(life, session, Ok(value)).await,
        Err(e) => {
            mark_closed(life);
            Err(e)
        }
    }
}

/// Block on tokio multi-thread runtime for one-shot browser work.
pub fn block_on_browser<F, T>(fut: F) -> Result<T, CliError>
where
    F: std::future::Future<Output = Result<T, CliError>>,
{
    block_on_browser_timeout(fut, 0)
}

/// Like `block_on_browser`, but abort with `ErrorKind::Timeout` when `timeout_secs > 0`.
pub fn block_on_browser_timeout<F, T>(fut: F, timeout_secs: u64) -> Result<T, CliError>
where
    F: std::future::Future<Output = Result<T, CliError>>,
{
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(2)
        .thread_name("bac-browser")
        .build()
        .map_err(|e| {
            CliError::new(
                ErrorKind::Software,
                format!("Failed to create tokio runtime: {e}"),
            )
        })?;
    if timeout_secs == 0 {
        return rt.block_on(fut);
    }
    rt.block_on(async {
        match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), fut).await {
            Ok(inner) => inner,
            Err(_) => Err(CliError::with_suggestion(
                ErrorKind::Timeout,
                format!("operation exceeded --timeout {timeout_secs}s"),
                "Raise --timeout or reduce wait/navigation work",
            )),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{is_internal_browser_url, is_noise_network_url, tree_to_at_refs};
    use crate::native::browser::WaitUntil;
    use serde_json::json;

    #[test]
    fn tree_to_at_refs_rewrites_markers() {
        let raw = r#"- link "Home" [ref=e1]
  - button "Go" [checked=false, ref=e2]
"#;
        let out = tree_to_at_refs(raw);
        assert!(out.contains("[@e1]"), "out={out}");
        assert!(out.contains("@e2"), "out={out}");
    }

    #[test]
    fn internal_browser_urls_filtered() {
        assert!(is_internal_browser_url("chrome://new-tab-page/"));
        assert!(is_internal_browser_url("chrome-extension://abc/x.js"));
        assert!(is_internal_browser_url("devtools://devtools/bundled/"));
        assert!(!is_internal_browser_url("https://example.com/"));
        assert!(!is_internal_browser_url("about:blank"));
        assert!(is_noise_network_url(
            "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7"
        ));
        assert!(is_noise_network_url("blob:https://example.com/uuid"));
        assert!(is_noise_network_url("chrome://new-tab-page/"));
        assert!(!is_noise_network_url("https://example.com/"));
    }

    #[test]
    fn wait_until_tokens_parse() {
        assert_eq!(
            WaitUntil::parse_token("networkidle"),
            WaitUntil::NetworkIdle
        );
        assert_eq!(
            WaitUntil::parse_token("domcontentloaded"),
            WaitUntil::DomContentLoaded
        );
        assert_eq!(WaitUntil::parse_token("load"), WaitUntil::Load);
        assert_eq!(WaitUntil::parse_token("none"), WaitUntil::None);
    }

    #[test]
    fn net_request_id_resolution_logic() {
        let requests = [
            json!({"requestId": "rid-1", "method": "GET", "url": "https://a.example/"}),
            json!({"requestId": "rid-2", "method": "POST", "url": "https://b.example/"}),
        ];
        let by_index = requests.get(1).unwrap();
        assert_eq!(by_index["requestId"], "rid-2");
        let by_rid = requests.iter().find(|r| r["requestId"] == "rid-1").unwrap();
        assert_eq!(by_rid["url"], "https://a.example/");
        // String id that is numeric index
        let idx: usize = "0".parse().unwrap();
        assert_eq!(requests[idx]["requestId"], "rid-1");
    }
}
