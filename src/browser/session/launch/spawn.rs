// SPDX-License-Identifier: MIT OR Apache-2.0
//! Chrome launch paths: headless, proxied, and with unpacked extensions.

use std::path::Path;

use rustc_hash::FxHashMap;

use crate::error::{CliError, ErrorKind};
use crate::native::browser::BrowserManager;
use crate::native::cdp::chrome::LaunchOptions;
use crate::native::element::RefMap;

use super::super::{CaptureOpts, OneShotSession};

impl OneShotSession {
    /// Launch a headless Chrome session with default capture off.
    pub async fn launch_headless() -> Result<Self, CliError> {
        Self::launch_headless_with_capture(CaptureOpts::default()).await
    }

    /// Launch headless Chrome with the given console/network capture options.
    pub async fn launch_headless_with_capture(capture: CaptureOpts) -> Result<Self, CliError> {
        Self::launch_headless_with_options(capture, None).await
    }

    /// Launch headless Chrome optionally routed through a local MITM proxy (GAP-011).
    pub async fn launch_headless_with_proxy(
        capture: CaptureOpts,
        proxy_server: &str,
    ) -> Result<Self, CliError> {
        Self::launch_headless_with_options(capture, Some(proxy_server)).await
    }

    async fn launch_headless_with_options(
        capture: CaptureOpts,
        proxy_server: Option<&str>,
    ) -> Result<Self, CliError> {
        // A local MITM proxy is an explicit route the caller asked for, so it
        // outranks the egress proxy from policy. Chaining the two would need an
        // upstream hop the MITM handler does not implement, and silently
        // dropping one of them would send traffic somewhere nobody chose.
        let mitm_proxy = proxy_server.map(|s| s.to_string());
        let egress_proxy = crate::browser_policy::proxy().map(str::to_string);
        let proxy = mitm_proxy.clone().or(egress_proxy);

        let options = LaunchOptions {
            // Read from policy instead of hard-coding `true`. Before this, the
            // `--headed` flag parsed and was dropped on the floor.
            headless: crate::browser_policy::mode().launches_headless(),
            hide_scrollbars: true,
            proxy,
            proxy_bypass: crate::browser_policy::proxy_bypass().map(str::to_string),
            no_xvfb: crate::browser_policy::no_xvfb(),
            // Trust MITM CA via ignore certs for one-shot local intercept (PRD §5E).
            ignore_https_errors: mitm_proxy.is_some(),
            ..LaunchOptions::default()
        };
        let manager = BrowserManager::launch(options, Some("chrome"))
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Unavailable,
                    format!("Chrome launch failed: {e}"),
                    crate::i18n::suggestion_key("external_binary_path", None),
                )
            })?;
        let chrome_pid = manager.chrome_pid();
        let event_rx = manager.client.subscribe();

        let mut session = Self {
            manager,
            ref_map: RefMap::new(),
            iframe_sessions: FxHashMap::default(),
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
            dialog_open: FxHashMap::default(),
            dialog_suppress_open: FxHashMap::default(),
            heap_snapshot_finished: false,
            tracing_complete: false,
            console_preserved: Vec::new(),
            network_preserved: Vec::new(),
            loaded_extension_ids: Vec::new(),
            named_contexts: FxHashMap::default(),
            drag_intercepted: None,
            net_inflight: 0,
            net_started: 0,
            net_last_activity: None,
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
                    crate::i18n::suggestion_key("extension_unpacked_dir", None),
                ));
            }
        }
        // Extensions require a non-headless Chrome product mode (chromiumoxide with_head).
        // build_chrome_args also omits --headless=new when extensions are present.
        let options = LaunchOptions {
            headless: false,
            hide_scrollbars: true,
            extensions: Some(extensions.clone()),
            proxy: crate::browser_policy::proxy().map(str::to_string),
            proxy_bypass: crate::browser_policy::proxy_bypass().map(str::to_string),
            no_xvfb: crate::browser_policy::no_xvfb(),
            ..LaunchOptions::default()
        };
        let manager = BrowserManager::launch(options, Some("chrome"))
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Unavailable,
                    format!("Chrome launch with extensions failed: {e}"),
                    crate::i18n::suggestion_key("extension_unpacked_dir", None),
                )
            })?;
        let chrome_pid = manager.chrome_pid();
        let event_rx = manager.client.subscribe();
        let mut session = Self {
            manager,
            ref_map: RefMap::new(),
            iframe_sessions: FxHashMap::default(),
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
            dialog_open: FxHashMap::default(),
            dialog_suppress_open: FxHashMap::default(),
            heap_snapshot_finished: false,
            tracing_complete: false,
            console_preserved: Vec::new(),
            network_preserved: Vec::new(),
            loaded_extension_ids: Vec::new(),
            named_contexts: FxHashMap::default(),
            drag_intercepted: None,
            net_inflight: 0,
            net_started: 0,
            net_last_activity: None,
        };
        // Best-effort: record extension path basenames; real ids come from list after launch.
        session.loaded_extension_ids = extensions
            .iter()
            .filter_map(|p| {
                Path::new(p)
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
            })
            .collect();
        session.enable_capture_domains().await?;
        // Populate loaded ids from live targets when available.
        if let Ok(list) = session.extension_list().await {
            if let Some(arr) = list.get("extensions").and_then(|v| v.as_array()) {
                for t in arr {
                    if let Some(id) = t.get("id").and_then(|v| v.as_str()) {
                        if !id.is_empty() && !session.loaded_extension_ids.iter().any(|x| x == id) {
                            session.loaded_extension_ids.push(id.to_string());
                        }
                    }
                }
            }
        }
        Ok(session)
    }
}
