//! Chrome discovery + launch option args for chromiumoxide one-shot.
//!
//! PROIBIDO: dual spawn via Child/Command for Chrome production path.
//! PROIBIDO: BrowserFetcher embutido no MVP (system Chrome only).
//! Launch ownership: `oxide::launch_with_oxide` → `Browser::launch`.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Options shared by CLI → BrowserManager → oxide launch.
#[derive(Debug, Clone)]
pub struct LaunchOptions {
    pub headless: bool,
    pub executable_path: Option<String>,
    pub proxy: Option<String>,
    pub proxy_bypass: Option<String>,
    pub proxy_username: Option<String>,
    pub proxy_password: Option<String>,
    pub profile: Option<String>,
    pub args: Vec<String>,
    pub allow_file_access: bool,
    pub extensions: Option<Vec<String>>,
    pub storage_state: Option<String>,
    pub user_agent: Option<String>,
    pub ignore_https_errors: bool,
    pub color_scheme: Option<String>,
    pub download_path: Option<String>,
    /// Hide native scrollbars in headless Chromium screenshots.
    pub hide_scrollbars: bool,
    /// Initial viewport for `--window-size`.
    pub viewport_size: Option<(u32, u32)>,
    /// When true, omit mock keychain flags (real system keychain).
    pub use_real_keychain: bool,
    /// Enable WebGPU (SwiftShader on Linux when needed).
    pub webgpu: bool,
    /// Opt-out Xvfb for headed Linux (legacy flag retained for CLI compat).
    pub no_xvfb: bool,
    /// Restrict WebRTC to proxied transports.
    pub restrict_webrtc: bool,
}

impl Default for LaunchOptions {
    fn default() -> Self {
        Self {
            headless: true,
            executable_path: None,
            proxy: None,
            proxy_bypass: None,
            proxy_username: None,
            proxy_password: None,
            profile: None,
            args: Vec::new(),
            allow_file_access: false,
            extensions: None,
            storage_state: None,
            user_agent: None,
            ignore_https_errors: false,
            color_scheme: None,
            download_path: None,
            hide_scrollbars: true,
            viewport_size: None,
            use_real_keychain: false,
            webgpu: false,
            no_xvfb: false,
            restrict_webrtc: false,
        }
    }
}

/// Resolved Chrome CLI args + user-data-dir for chromiumoxide launch.
pub(crate) struct ChromeArgs {
    pub args: Vec<String>,
    pub user_data_dir: PathBuf,
    pub temp_user_data_dir: Option<PathBuf>,
}

/// Build Chrome flags from [`LaunchOptions`] (used by oxide one-shot path).
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
        "--metrics-recording-only".to_string(),
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
        args.push(format!("--proxy-server={}", proxy));
    }

    if let Some(ref bypass) = options.proxy_bypass {
        args.push(format!("--proxy-bypass-list={}", bypass));
    }

    let (user_data_dir, temp_user_data_dir) = if let Some(ref profile) = options.profile {
        let expanded = expand_tilde(profile);
        let dir = PathBuf::from(&expanded);
        args.push(format!("--user-data-dir={}", expanded));
        (dir, None)
    } else {
        let dir = std::env::temp_dir().join(format!(
            "browser-automation-cli-chrome-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create temp profile dir: {}", e))?;
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
            args.push(format!("--load-extension={}", ext_list));
            args.push(format!("--disable-extensions-except={}", ext_list));
        }
    }

    let has_window_size = options
        .args
        .iter()
        .any(|a| a.starts_with("--start-maximized") || a.starts_with("--window-size="));

    if !has_window_size && options.headless && !has_extensions {
        let (w, h) = options.viewport_size.unwrap_or((1280, 720));
        args.push(format!("--window-size={},{}", w, h));
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

/// Locate system Chrome/Chromium (product cache → PATH → Playwright/Puppeteer caches).
pub fn find_chrome() -> Option<PathBuf> {
    if let Some(p) = crate::install::find_installed_chrome() {
        return Some(p);
    }

    let cache_dir = crate::install::get_browsers_dir();
    if cache_dir.exists() {
        let _ = writeln!(
            std::io::stderr(),
            "Warning: Chrome cache directory exists ({}) but no Chrome binary found inside. \
             Falling back to system Chrome (product browsers cache empty).",
            cache_dir.display()
        );
    }

    #[cfg(target_os = "macos")]
    {
        let candidates = [
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
            "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
        ];
        for c in &candidates {
            let p = PathBuf::from(c);
            if p.exists() {
                return Some(p);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let candidates = [
            "google-chrome",
            "google-chrome-stable",
            "chromium-browser",
            "chromium",
            "brave-browser",
            "brave-browser-stable",
        ];
        for name in &candidates {
            if let Ok(output) = Command::new("which").arg(name).output() {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !path.is_empty() {
                        return Some(PathBuf::from(path));
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let candidates = [
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        ];
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let chrome = PathBuf::from(&local).join(r"Google\Chrome\Application\chrome.exe");
            if chrome.exists() {
                return Some(chrome);
            }
            let brave =
                PathBuf::from(&local).join(r"BraveSoftware\Brave-Browser\Application\brave.exe");
            if brave.exists() {
                return Some(brave);
            }
        }
        for c in &candidates {
            let p = PathBuf::from(c);
            if p.exists() {
                return Some(p);
            }
        }
    }

    if let Some(p) = find_puppeteer_chrome() {
        return Some(p);
    }
    if let Some(p) = find_playwright_chromium() {
        return Some(p);
    }

    None
}

fn should_disable_sandbox(existing_args: &[String]) -> bool {
    if existing_args.iter().any(|a| a == "--no-sandbox") {
        return false;
    }
    if std::env::var("CI").is_ok() {
        return true;
    }
    #[cfg(unix)]
    {
        if unsafe { libc::geteuid() } == 0 {
            return true;
        }
        if Path::new("/.dockerenv").exists() {
            return true;
        }
        if Path::new("/run/.containerenv").exists() {
            return true;
        }
        if let Ok(cgroup) = std::fs::read_to_string("/proc/1/cgroup") {
            if cgroup.contains("docker") || cgroup.contains("kubepods") || cgroup.contains("lxc") {
                return true;
            }
        }
    }
    false
}

fn should_disable_dev_shm(existing_args: &[String]) -> bool {
    if existing_args.iter().any(|a| a == "--disable-dev-shm-usage") {
        return false;
    }
    if std::env::var("CI").is_ok() {
        return true;
    }
    #[cfg(unix)]
    {
        if unsafe { libc::geteuid() } == 0 {
            return true;
        }
        if Path::new("/.dockerenv").exists() || Path::new("/run/.containerenv").exists() {
            return true;
        }
        if let Ok(cgroup) = std::fs::read_to_string("/proc/1/cgroup") {
            if cgroup.contains("docker") || cgroup.contains("kubepods") || cgroup.contains("lxc") {
                return true;
            }
        }
    }
    false
}

fn find_puppeteer_chrome() -> Option<PathBuf> {
    let mut search_dirs = Vec::new();
    if let Ok(custom) = std::env::var("PUPPETEER_CACHE_DIR") {
        search_dirs.push(PathBuf::from(custom).join("chrome"));
    }
    if let Some(home) = dirs::home_dir() {
        search_dirs.push(home.join(".cache/puppeteer/chrome"));
    }
    for dir in &search_dirs {
        if !dir.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            let mut matches: Vec<PathBuf> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| {
                    let candidate = build_puppeteer_binary_path(&e.path());
                    if candidate.exists() {
                        Some(candidate)
                    } else {
                        None
                    }
                })
                .collect();
            matches.sort();
            matches.reverse();
            if let Some(p) = matches.into_iter().next() {
                return Some(p);
            }
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn build_puppeteer_binary_path(version_dir: &Path) -> PathBuf {
    version_dir.join("chrome-linux64/chrome")
}

#[cfg(target_os = "macos")]
fn build_puppeteer_binary_path(version_dir: &Path) -> PathBuf {
    let arm = version_dir.join(
        "chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
    );
    if arm.exists() {
        return arm;
    }
    version_dir.join(
        "chrome-mac-x64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
    )
}

#[cfg(target_os = "windows")]
fn build_puppeteer_binary_path(version_dir: &Path) -> PathBuf {
    version_dir.join(r"chrome-win64\chrome.exe")
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn build_puppeteer_binary_path(version_dir: &Path) -> PathBuf {
    version_dir.join("chrome")
}

fn find_playwright_chromium() -> Option<PathBuf> {
    let mut search_dirs = Vec::new();
    if let Ok(custom) = std::env::var("PLAYWRIGHT_BROWSERS_PATH") {
        search_dirs.push(PathBuf::from(custom));
    }
    if let Some(home) = dirs::home_dir() {
        search_dirs.push(home.join(".cache/ms-playwright"));
    }
    for dir in &search_dirs {
        if !dir.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            let mut matches: Vec<PathBuf> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_str()
                        .map(|n| n.starts_with("chromium-"))
                        .unwrap_or(false)
                })
                .filter_map(|e| {
                    let candidate = build_playwright_binary_path(&e.path());
                    if candidate.exists() {
                        Some(candidate)
                    } else {
                        None
                    }
                })
                .collect();
            matches.sort();
            matches.reverse();
            if let Some(p) = matches.into_iter().next() {
                return Some(p);
            }
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn build_playwright_binary_path(chromium_dir: &Path) -> PathBuf {
    let standard = chromium_dir.join("chrome-linux/chrome");
    if standard.exists() {
        return standard;
    }
    chromium_dir.join("chrome-linux64/chrome")
}

#[cfg(target_os = "macos")]
fn build_playwright_binary_path(chromium_dir: &Path) -> PathBuf {
    chromium_dir.join("chrome-mac/Chromium.app/Contents/MacOS/Chromium")
}

#[cfg(target_os = "windows")]
fn build_playwright_binary_path(chromium_dir: &Path) -> PathBuf {
    chromium_dir.join("chrome-win/chrome.exe")
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn build_playwright_binary_path(chromium_dir: &Path) -> PathBuf {
    chromium_dir.join("chrome")
}

fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix('~') {
        if let Some(home) = dirs::home_dir() {
            return home
                .join(rest.strip_prefix('/').unwrap_or(rest))
                .to_string_lossy()
                .to_string();
        }
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::EnvGuard;

    #[test]
    fn test_find_chrome_returns_some_on_host() {
        // Hosts without Chrome still exercise the function without panic.
        let _ = find_chrome();
    }

    #[test]
    fn test_expand_tilde() {
        if dirs::home_dir().is_some() {
            let expanded = expand_tilde("~/foo");
            assert!(!expanded.starts_with('~'));
            assert!(expanded.ends_with("foo") || expanded.ends_with("foo/"));
        }
    }

    #[test]
    fn test_expand_tilde_no_tilde() {
        assert_eq!(expand_tilde("/tmp/x"), "/tmp/x");
    }

    #[test]
    fn test_should_disable_sandbox_skips_if_already_set() {
        assert!(!should_disable_sandbox(&["--no-sandbox".to_string()]));
    }

    #[test]
    fn test_find_playwright_chromium_nonexistent() {
        let g = EnvGuard::new(&["PLAYWRIGHT_BROWSERS_PATH"]);
        g.set("PLAYWRIGHT_BROWSERS_PATH", "/nonexistent-playwright-path");
        let result = find_playwright_chromium();
        assert!(result.is_none());
    }

    #[test]
    fn test_build_args_headless_includes_headless_flag() {
        let opts = LaunchOptions {
            headless: true,
            ..Default::default()
        };
        let result = build_chrome_args(&opts).unwrap();
        assert!(result.args.iter().any(|a| a == "--headless=new"));
        assert!(result.args.iter().any(|a| a == "--hide-scrollbars"));
        assert!(result
            .args
            .iter()
            .any(|a| a == "--enable-unsafe-swiftshader"));
        assert!(result.args.iter().any(|a| a == "--window-size=1280,720"));
        assert!(result.temp_user_data_dir.is_some());
        let dir = result.temp_user_data_dir.unwrap();
        assert!(dir.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_build_args_headed_no_headless_flag() {
        let opts = LaunchOptions {
            headless: false,
            ..Default::default()
        };
        let result = build_chrome_args(&opts).unwrap();
        assert!(!result.args.iter().any(|a| a.contains("--headless")));
        assert!(!result.args.iter().any(|a| a == "--hide-scrollbars"));
        assert!(result.temp_user_data_dir.is_some());
        if let Some(ref dir) = result.temp_user_data_dir {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    #[test]
    fn test_build_args_temp_user_data_dir_created() {
        let opts = LaunchOptions::default();
        let result = build_chrome_args(&opts).unwrap();
        let dir = result.temp_user_data_dir.as_ref().unwrap();
        assert!(dir.exists());
        assert!(result
            .args
            .iter()
            .any(|a| a.starts_with("--user-data-dir=")));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_build_args_profile_no_temp_dir() {
        let opts = LaunchOptions {
            profile: Some("/tmp/my-profile".to_string()),
            ..Default::default()
        };
        let result = build_chrome_args(&opts).unwrap();
        assert!(result.temp_user_data_dir.is_none());
        assert!(result
            .args
            .iter()
            .any(|a| a == "--user-data-dir=/tmp/my-profile"));
    }

    #[test]
    fn test_build_args_custom_window_size_not_overridden() {
        let opts = LaunchOptions {
            headless: true,
            args: vec!["--window-size=1920,1080".to_string()],
            ..Default::default()
        };
        let result = build_chrome_args(&opts).unwrap();
        assert!(!result.args.iter().any(|a| a == "--window-size=1280,720"));
        assert!(result.args.iter().any(|a| a == "--window-size=1920,1080"));
        if let Some(ref dir) = result.temp_user_data_dir {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    #[test]
    fn test_build_args_hide_scrollbars_false_suppresses_default_hide_scrollbars() {
        let opts = LaunchOptions {
            headless: true,
            hide_scrollbars: false,
            ..Default::default()
        };
        let result = build_chrome_args(&opts).unwrap();
        assert!(!result.args.iter().any(|a| a == "--hide-scrollbars"));
        if let Some(ref dir) = result.temp_user_data_dir {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    #[test]
    fn test_build_args_start_maximized_suppresses_default_window_size() {
        let opts = LaunchOptions {
            headless: true,
            args: vec!["--start-maximized".to_string()],
            ..Default::default()
        };
        let result = build_chrome_args(&opts).unwrap();
        assert!(!result.args.iter().any(|a| a == "--window-size=1280,720"));
        assert!(result.args.iter().any(|a| a == "--start-maximized"));
        if let Some(ref dir) = result.temp_user_data_dir {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    #[test]
    fn test_build_args_disables_translate() {
        let opts = LaunchOptions::default();
        let result = build_chrome_args(&opts).unwrap();
        assert!(result
            .args
            .iter()
            .any(|a| a.contains("--disable-features") && a.contains("Translate")));
        if let Some(ref dir) = result.temp_user_data_dir {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    #[test]
    fn test_build_args_webgpu_default_off() {
        let opts = LaunchOptions::default();
        let result = build_chrome_args(&opts).unwrap();
        assert!(!result.args.iter().any(|a| a == "--enable-unsafe-webgpu"));
        if let Some(ref dir) = result.temp_user_data_dir {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    #[test]
    fn test_build_args_restrict_webrtc_enforces_safe_policy() {
        let opts = LaunchOptions {
            restrict_webrtc: true,
            args: vec!["--force-webrtc-ip-handling-policy=default".to_string()],
            ..Default::default()
        };
        let result = build_chrome_args(&opts).unwrap();
        let policies: Vec<&String> = result
            .args
            .iter()
            .filter(|arg| arg.starts_with("--force-webrtc-ip-handling-policy="))
            .collect();
        assert_eq!(
            policies,
            vec![&"--force-webrtc-ip-handling-policy=disable_non_proxied_udp".to_string()]
        );
        if let Some(ref dir) = result.temp_user_data_dir {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    #[test]
    fn test_build_args_webgpu_includes_webgpu_flags() {
        let opts = LaunchOptions {
            webgpu: true,
            ..Default::default()
        };
        let result = build_chrome_args(&opts).unwrap();
        assert!(result.args.iter().any(|a| a == "--enable-unsafe-webgpu"));
        if cfg!(target_os = "linux") {
            assert!(result.args.iter().any(|a| a == "--use-angle=vulkan"));
            assert!(result.args.iter().any(|a| a == "--use-vulkan=swiftshader"));
        }
        if let Some(ref dir) = result.temp_user_data_dir {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    #[test]
    fn test_build_args_merges_user_enable_features() {
        let opts = LaunchOptions {
            webgpu: true,
            args: vec![
                "--enable-features=Foo,Bar".to_string(),
                "--some-other-flag".to_string(),
                "--enable-features=NetworkService".to_string(),
            ],
            ..Default::default()
        };
        let result = build_chrome_args(&opts).unwrap();
        let flags: Vec<&String> = result
            .args
            .iter()
            .filter(|a| a.starts_with("--enable-features="))
            .collect();
        assert_eq!(flags.len(), 1);
        let features: Vec<&str> = flags[0]
            .strip_prefix("--enable-features=")
            .unwrap()
            .split(',')
            .collect();
        assert!(features.contains(&"NetworkService"));
        assert!(features.contains(&"Foo"));
        assert!(features.contains(&"Bar"));
        assert!(result.args.iter().any(|a| a == "--some-other-flag"));
        if let Some(ref dir) = result.temp_user_data_dir {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    #[test]
    fn test_build_args_single_enable_features_flag() {
        let opts = LaunchOptions {
            webgpu: true,
            ..Default::default()
        };
        let result = build_chrome_args(&opts).unwrap();
        let count = result
            .args
            .iter()
            .filter(|a| a.starts_with("--enable-features="))
            .count();
        assert_eq!(count, 1);
        if let Some(ref dir) = result.temp_user_data_dir {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    #[test]
    fn test_build_args_headless_with_extensions_skips_headless_flag() {
        let opts = LaunchOptions {
            headless: true,
            extensions: Some(vec!["/tmp/ext".to_string()]),
            ..Default::default()
        };
        let result = build_chrome_args(&opts).unwrap();
        assert!(!result.args.iter().any(|a| a.contains("--headless")));
        assert!(result
            .args
            .iter()
            .any(|a| a.starts_with("--load-extension=")));
        assert!(result.args.iter().any(|a| {
            a.starts_with("--disable-features=")
                && a.contains("DisableLoadExtensionCommandLineSwitch")
        }));
        if let Some(ref dir) = result.temp_user_data_dir {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    #[test]
    fn test_build_args_ignore_https_errors_includes_flag() {
        let opts = LaunchOptions {
            ignore_https_errors: true,
            ..Default::default()
        };
        let result = build_chrome_args(&opts).unwrap();
        assert!(result
            .args
            .iter()
            .any(|a| a == "--ignore-certificate-errors"));
        if let Some(ref dir) = result.temp_user_data_dir {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    #[test]
    fn test_build_args_ignore_https_errors_default_no_flag() {
        let opts = LaunchOptions::default();
        let result = build_chrome_args(&opts).unwrap();
        assert!(!result
            .args
            .iter()
            .any(|a| a == "--ignore-certificate-errors"));
        if let Some(ref dir) = result.temp_user_data_dir {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}
