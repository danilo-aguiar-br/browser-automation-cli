//! Chromiumoxide launch + FINALIZE helpers (PRD: Browser::launch only, no connect).
#![allow(missing_docs)]
//!
//! System Chrome/Chromium only — no BrowserFetcher embutido (PRD L56 / L387).
//! Launch flags come from `build_chrome_args` so proxy/webgpu/extensions are live.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::Handler as OxideHandler;
use tokio::sync::Mutex;

use super::chrome::{build_chrome_args, find_chrome, LaunchOptions};

/// Launch result before the CDP client takes ownership of the handler task.
pub struct OxideLaunch {
    pub browser: Browser,
    pub handler: OxideHandler,
    pub executable: Option<PathBuf>,
    pub ws_url: String,
    /// Temp user-data-dir created for this one-shot (cleanup after FINALIZE).
    pub temp_user_data_dir: Option<PathBuf>,
}

/// Launch headless Chrome via chromiumoxide using system or explicit executable.
///
/// PROIBIDO: attach/connect to external CDP in MVP. PROIBIDO: BrowserFetcher download automático.
/// Handler must be polled (via CdpClient::from_browser) for commands to complete.
pub async fn launch_with_oxide(options: &LaunchOptions) -> Result<OxideLaunch, String> {
    let chrome_args = build_chrome_args(options)?;

    let mut builder = BrowserConfig::builder();

    let executable = options
        .executable_path
        .as_ref()
        .map(PathBuf::from)
        .or_else(find_chrome);

    if let Some(ref exe) = executable {
        builder = builder.chrome_executable(exe);
    }

    // Headed vs headless: builder tracks product mode; flags also carry --headless=new.
    // Chromiumoxide refuses extensions in headless product mode — force with_head when loading.
    let has_extensions = options.extensions.as_ref().is_some_and(|e| !e.is_empty());
    if !options.headless || has_extensions {
        builder = builder.with_head();
    }

    // Register extensions on BrowserConfig so it does NOT inject --disable-extensions.
    if let Some(ref exts) = options.extensions {
        if !exts.is_empty() {
            builder = builder.extensions(exts.clone());
        }
    }

    builder = builder.user_data_dir(chrome_args.user_data_dir.clone());

    // Apply full flag set (proxy, webgpu, extensions, sandbox, window-size, …).
    // Skip flags already owned by BrowserConfig to avoid duplicate conflicts.
    for a in &chrome_args.args {
        if a.starts_with("--user-data-dir=") {
            continue;
        }
        if a == "--headless=new" || a == "--headless" || a.starts_with("--headless=") {
            // BrowserConfig owns headless mode via with_head / default.
            continue;
        }
        // Extension load paths are owned by BrowserConfig.extensions — avoid double --load-extension.
        if a.starts_with("--load-extension=") || a.starts_with("--disable-extensions-except=") {
            continue;
        }
        builder = builder.arg(a.as_str());
    }

    let config = builder
        .build()
        .map_err(|e| format!("chromiumoxide BrowserConfig: {e}"))?;

    let (browser, handler) = Browser::launch(config)
        .await
        .map_err(|e| format!("chromiumoxide Browser::launch: {e}"))?;

    let ws_url = browser.websocket_address().clone();

    Ok(OxideLaunch {
        browser,
        handler,
        executable,
        ws_url,
        temp_user_data_dir: chrome_args.temp_user_data_dir,
    })
}

/// FINALIZE: close + wait + kill fallback on a shared browser mutex.
pub async fn finalize_browser(browser: Arc<Mutex<Browser>>) -> Result<(), String> {
    let mut browser = match Arc::try_unwrap(browser) {
        Ok(m) => m.into_inner(),
        Err(shared) => {
            let mut guard = shared.lock().await;
            if let Err(e) = guard.close().await {
                let _ = guard.kill().await;
                return Err(format!("chromiumoxide close: {e}"));
            }
            match tokio::time::timeout(Duration::from_secs(5), guard.wait()).await {
                Ok(Ok(_)) => {}
                Ok(Err(e)) => {
                    let _ = guard.kill().await;
                    return Err(format!("chromiumoxide wait: {e}"));
                }
                Err(_) => {
                    let _ = guard.kill().await;
                }
            }
            return Ok(());
        }
    };

    if let Err(e) = browser.close().await {
        let _ = browser.kill().await;
        return Err(format!("chromiumoxide close: {e}"));
    }

    match tokio::time::timeout(Duration::from_secs(5), browser.wait()).await {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => {
            let _ = browser.kill().await;
            return Err(format!("chromiumoxide wait: {e}"));
        }
        Err(_) => {
            let _ = browser.kill().await;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_options_default_is_headless_path() {
        let o = LaunchOptions::default();
        assert!(o.headless);
    }

    #[test]
    fn build_args_feed_proxy_into_oxide_flag_list() {
        let o = LaunchOptions {
            proxy: Some("http://127.0.0.1:8080".to_string()),
            ..Default::default()
        };
        let args = build_chrome_args(&o).unwrap();
        assert!(args
            .args
            .iter()
            .any(|a| a == "--proxy-server=http://127.0.0.1:8080"));
        if let Some(ref dir) = args.temp_user_data_dir {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}
