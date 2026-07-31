// SPDX-License-Identifier: MIT OR Apache-2.0
//! Chrome CLI arg builder and temp profile materialization.
use std::path::PathBuf;

use super::options::LaunchOptions;

pub(crate) struct ChromeArgs {
    pub args: Vec<String>,
    pub user_data_dir: PathBuf,
    pub temp_user_data_dir: Option<PathBuf>,
}

/// Create the temp profile dir on the **current** thread (unit tests / sync callers).
///
/// Production async launch uses [`crate::concurrency::create_dir_all_blocking`] instead.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn materialize_temp_user_data_dir_sync(args: &ChromeArgs) -> Result<(), String> {
    if let Some(ref dir) = args.temp_user_data_dir {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("Failed to create temp profile dir: {e}"))?;
    }
    Ok(())
}

/// Build Chrome flags from [`LaunchOptions`] (used by oxide one-shot path).
///
/// Temp profile directories are **not** created here (PAR-92): callers on the
/// async path must await disk materialization off the Tokio worker.
pub(crate) fn build_chrome_args(options: &LaunchOptions) -> Result<ChromeArgs, String> {
    // Chrome only honors the last --enable-features switch.
    let mut enable_features: Vec<String> = vec![
        "NetworkService".to_string(),
        "NetworkServiceInProcess".to_string(),
    ];
    if options.webgpu && cfg!(target_os = "linux") {
        enable_features.push("Vulkan".to_string());
    }

    let mut user_args: Vec<String> = Vec::new();
    for arg in &options.args {
        if let Some(values) = arg.strip_prefix("--enable-features=") {
            for feature in values.split(',').map(str::trim).filter(|f| !f.is_empty()) {
                if !enable_features.iter().any(|f| f == feature) {
                    enable_features.push(feature.to_string());
                }
            }
        } else {
            user_args.push(arg.clone());
        }
    }

    // Chrome only honors the last --disable-features switch — keep a single list.
    let mut disable_features: Vec<String> = vec!["Translate".to_string()];
    let has_extensions = options
        .extensions
        .as_ref()
        .is_some_and(|exts| !exts.is_empty());
    if has_extensions {
        // Chrome 127+ gates --load-extension behind this feature (must disable the gate).
        disable_features.push("DisableLoadExtensionCommandLineSwitch".to_string());
    }

    let mut args = vec![
        "--remote-debugging-port=0".to_string(),
        "--no-first-run".to_string(),
        "--no-default-browser-check".to_string(),
        "--disable-background-networking".to_string(),
        "--disable-backgrounding-occluded-windows".to_string(),
        "--disable-component-update".to_string(),
        "--disable-default-apps".to_string(),
        "--disable-hang-monitor".to_string(),
        "--disable-popup-blocking".to_string(),
        "--disable-prompt-on-repost".to_string(),
        "--disable-sync".to_string(),
        format!("--disable-features={}", disable_features.join(",")),
        format!("--enable-features={}", enable_features.join(",")),
        // GAP-016 / PRD §5F: do not enable metrics subsystem (even "recording-only").
        // Prefer hard-disable of metrics/crash reporter where Chromium honors flags.
        "--disable-metrics".to_string(),
        "--disable-metrics-reporting".to_string(),
        "--disable-breakpad".to_string(),
        "--disable-crash-reporter".to_string(),
        "--disable-domain-reliability".to_string(),
        "--disable-client-side-phishing-detection".to_string(),
        "--disable-component-extensions-with-background-pages".to_string(),
    ];

    if options.webgpu {
        args.push("--enable-unsafe-webgpu".to_string());
        if cfg!(target_os = "linux") {
            args.push("--use-angle=vulkan".to_string());
            args.push("--use-vulkan=swiftshader".to_string());
            args.push("--use-webgpu-adapter=swiftshader".to_string());
            args.push("--disable-vulkan-surface".to_string());
        }
    }

    if !options.use_real_keychain {
        args.push("--password-store=basic".to_string());
        args.push("--use-mock-keychain".to_string());
    }

    if options.headless && !has_extensions {
        args.push("--headless=new".to_string());
        if options.hide_scrollbars {
            args.push("--hide-scrollbars".to_string());
        }
        args.push("--enable-unsafe-swiftshader".to_string());
    }

    if let Some(ref proxy) = options.proxy {
        args.push(format!("--proxy-server={proxy}"));
    }

    if let Some(ref bypass) = options.proxy_bypass {
        args.push(format!("--proxy-bypass-list={bypass}"));
    }

    let (user_data_dir, temp_user_data_dir) = if let Some(ref profile) = options.profile {
        let expanded = super::tooling::expand_tilde(profile);
        let dir = PathBuf::from(&expanded);
        args.push(format!("--user-data-dir={expanded}"));
        (dir, None)
    } else {
        // PAR-92: allocate path only — do **not** `create_dir_all` here.
        // Under XDG cache (not OS temp) so residual GC owns product profiles.
        // Marker prefix keeps residual discover aligned.
        let base = crate::xdg::chrome_profiles_dir().unwrap_or_else(|_| {
            crate::xdg::cache_dir()
                .unwrap_or_else(|_| PathBuf::from("chrome-profiles-unconfigured"))
                .join("chrome-profiles")
        });
        let dir = base.join(format!(
            "browser-automation-cli-chrome-{}",
            uuid::Uuid::new_v4()
        ));
        args.push(format!("--user-data-dir={}", dir.display()));
        (dir.clone(), Some(dir))
    };

    if options.ignore_https_errors {
        args.push("--ignore-certificate-errors".to_string());
    }

    if options.allow_file_access {
        args.push("--allow-file-access-from-files".to_string());
        args.push("--allow-file-access".to_string());
    }

    if let Some(ref exts) = options.extensions {
        if !exts.is_empty() {
            let ext_list = exts.join(",");
            args.push(format!("--load-extension={ext_list}"));
            args.push(format!("--disable-extensions-except={ext_list}"));
        }
    }

    let has_window_size = options
        .args
        .iter()
        .any(|a| a.starts_with("--start-maximized") || a.starts_with("--window-size="));

    if !has_window_size && options.headless && !has_extensions {
        let (w, h) = options.viewport_size.unwrap_or((
            crate::xdg::policy::policy_u32(crate::xdg::policy::key::DEFAULT_VIEWPORT_WIDTH),
            crate::xdg::policy::policy_u32(crate::xdg::policy::key::DEFAULT_VIEWPORT_HEIGHT),
        ));
        args.push(format!("--window-size={w},{h}"));
    }

    args.extend(user_args);

    if options.restrict_webrtc {
        args.retain(|arg| !arg.starts_with("--force-webrtc-ip-handling-policy="));
        args.push("--force-webrtc-ip-handling-policy=disable_non_proxied_udp".to_string());
    }

    if should_disable_sandbox(&args) {
        args.push("--no-sandbox".to_string());
    }

    if should_disable_dev_shm(&args) {
        args.push("--disable-dev-shm-usage".to_string());
    }

    Ok(ChromeArgs {
        args,
        user_data_dir,
        temp_user_data_dir,
    })
}
pub(crate) fn should_disable_sandbox(existing_args: &[String]) -> bool {
    if existing_args.iter().any(|a| a == "--no-sandbox") {
        return false;
    }
    // Container/root detection only (no product CI env var as settings).
    #[cfg(unix)]
    {
        // SAFETY:
        // - Contract: detect effective root to decide Chrome `--no-sandbox`.
        // - Invariant: `geteuid` is always safe; returns the process effective uid.
        // - Root/container cannot run Chromium sandbox reliably; flag is a launch arg only.
        // - See: `man 2 geteuid`; Chromium sandbox docs for containers.
        if unsafe { libc::geteuid() } == 0 {
            return true;
        }
    }
    // Shared multiplatform probe (dockerenv, cgroup, k8s, podman).
    crate::platform::HostEnvironment::detect().container
}

pub(crate) fn should_disable_dev_shm(existing_args: &[String]) -> bool {
    if existing_args.iter().any(|a| a == "--disable-dev-shm-usage") {
        return false;
    }
    #[cfg(unix)]
    {
        // SAFETY:
        // - Contract: detect effective root to decide Chrome `--disable-dev-shm-usage`.
        // - Invariant: `geteuid` is always safe; returns the process effective uid.
        // - Root/container hosts often have tiny `/dev/shm`; flag is a launch arg only.
        // - See: `man 2 geteuid`; Chromium headless container guidance.
        if unsafe { libc::geteuid() } == 0 {
            return true;
        }
    }
    crate::platform::HostEnvironment::detect().container
}
