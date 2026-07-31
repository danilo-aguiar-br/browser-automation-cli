// SPDX-License-Identifier: MIT OR Apache-2.0
//! Engine dispatch and one-shot browser launch.

use rustc_hash::FxHashSet;
use serde_json::json;
use std::sync::Arc;

use crate::native::browser::validate::{validate_launch_options, validate_lightpanda_options};
use crate::native::browser::{BrowserManager, BrowserProcess};
use crate::native::cdp::chrome::LaunchOptions;
use crate::native::cdp::client::CdpClient;
use crate::native::cdp::lightpanda::{launch_lightpanda, LightpandaLaunchOptions};

use super::lightpanda::initialize_lightpanda_manager;

impl BrowserManager {
    /// PID of the Chrome this invocation launched, when it launched one.
    ///
    /// `None` for a connection to a browser someone else owns, which is also
    /// the signal that FINALIZE must not kill it.
    pub fn chrome_pid(&self) -> Option<u32> {
        if let Some(ref process) = self.browser_process {
            return process.id();
        }
        None
    }

    /// Temporary user-data-dir created for this one-shot launch, if any.
    pub fn temp_user_data_dir(&self) -> Option<&std::path::Path> {
        self.temp_user_data_dir.as_deref()
    }

    /// BORN: start an engine and connect to it.
    ///
    /// `engine` selects Chrome or Lightpanda; `None` uses the default. On
    /// return the manager owns whatever was spawned and must be shut down.
    pub async fn launch(mut options: LaunchOptions, engine: Option<&str>) -> Result<Self, String> {
        let engine = engine.unwrap_or("chrome");

        match engine {
            "chrome" => {
                validate_launch_options(
                    options.extensions.as_deref(),
                    false,
                    options.profile.as_deref(),
                    options.storage_state.as_deref(),
                    options.allow_file_access,
                    options.executable_path.as_deref(),
                )?;
            }
            "lightpanda" => {
                validate_lightpanda_options(&options)?;
            }
            _ => {
                return Err(format!(
                    "Unknown engine '{engine}'. Supported engines: chrome, lightpanda"
                ));
            }
        }

        let ignore_https_errors = options.ignore_https_errors;
        // Post-CDP fields are not read by build_chrome_args / lightpanda spawn:
        // take ownership once (rules: move > clone on owned LaunchOptions).
        let user_agent = options.user_agent.take();
        let color_scheme = options.color_scheme.take();
        let download_path = options.download_path.take();

        let manager = if engine == "lightpanda" {
            let lp_options = LightpandaLaunchOptions {
                executable_path: options.executable_path.take(),
                proxy: options.proxy.take(),
                port: None,
            };
            let mut lp = launch_lightpanda(&lp_options).await?;
            // Process keeps empty ws_url after take; manager owns the connection string.
            let ws_url = std::mem::take(&mut lp.ws_url);
            let process = BrowserProcess::Lightpanda(lp);
            let mut manager = initialize_lightpanda_manager(ws_url, process).await?;
            manager.owns_oxide_browser = false;
            manager.download_path = download_path;
            manager
        } else {
            // Chrome one-shot: single chromiumoxide stack (Browser::launch only — no dual WS).
            let launched = crate::native::cdp::oxide::launch_with_oxide(&options).await?;
            let crate::native::cdp::oxide::OxideLaunch {
                browser,
                handler,
                ws_url,
                temp_user_data_dir,
                ..
            } = launched;
            let client = Arc::new(CdpClient::from_browser(browser, handler).await?);
            let mut manager = Self {
                client,
                browser_process: None,
                owns_oxide_browser: true,
                ws_url,
                pages: Vec::new(),
                active_page_index: 0,
                default_timeout_ms: 25_000,
                download_path,
                ignore_https_errors,
                visited_origins: FxHashSet::default(),
                next_tab_id: 1,
                direct_page: false,
                temp_user_data_dir,
            };
            manager.discover_and_attach_targets().await?;
            manager
        };

        let session_id = manager.active_session_id()?.to_string();

        if ignore_https_errors {
            let _ = manager
                .client
                .send_command(
                    "Security.setIgnoreCertificateErrors",
                    Some(json!({ "ignore": true })),
                    Some(&session_id),
                )
                .await;
        }

        if let Some(ref ua) = user_agent {
            let _ = manager
                .client
                .send_command(
                    "Emulation.setUserAgentOverride",
                    Some(json!({ "userAgent": ua })),
                    Some(&session_id),
                )
                .await;
        }

        if let Some(ref scheme) = color_scheme {
            let _ = manager
                .client
                .send_command(
                    "Emulation.setEmulatedMedia",
                    Some(json!({ "features": [{ "name": "prefers-color-scheme", "value": scheme }] })),
                    Some(&session_id),
                )
                .await;
        }

        if let Some(ref path) = manager.download_path {
            let _ = manager
                .client
                .send_command(
                    "Browser.setDownloadBehavior",
                    Some(json!({ "behavior": "allow", "downloadPath": path })),
                    None,
                )
                .await;
        }

        Ok(manager)
    }

    /// CDP session of the active tab, or an error when no tab is attached.
    pub fn active_session_id(&self) -> Result<&str, String> {
        self.pages
            .get(self.active_page_index)
            .map(|p| p.session_id.as_str())
            .ok_or_else(|| "No active page".to_string())
    }
}
