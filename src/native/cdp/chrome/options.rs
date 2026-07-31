// SPDX-License-Identifier: MIT OR Apache-2.0
//! Launch options shared by CLI → BrowserManager → oxide launch.

/// Everything that has to be decided BEFORE Chrome starts.
///
/// Chrome reads these as process arguments, so none of them can be changed
/// afterwards without relaunching. Options that CAN change on a live browser
/// (viewport, network conditions, user agent overrides per session) belong to
/// the emulate surface instead.
#[derive(Debug, Clone)]
pub struct LaunchOptions {
    /// Run without a visible window. `--headed` is the debugging opt-out.
    pub headless: bool,
    /// Absolute path to the Chrome binary. `None` runs host discovery.
    pub executable_path: Option<String>,
    /// Proxy URL passed to `--proxy-server`.
    pub proxy: Option<String>,
    /// Hosts that bypass the proxy, in Chrome's bypass-list syntax.
    pub proxy_bypass: Option<String>,
    /// Username for a proxy that authenticates. Secret: never logged.
    pub proxy_username: Option<String>,
    /// Password for a proxy that authenticates. Secret: never logged.
    pub proxy_password: Option<String>,
    /// User-data directory. `None` mints an ephemeral marker profile that
    /// FINALIZE reclaims, which is what keeps residual-zero on disk true.
    pub profile: Option<String>,
    /// Extra raw Chrome switches, appended after the ones built here.
    pub args: Vec<String>,
    /// Permit `file://` reads. Off by default: a page that can read local files
    /// can exfiltrate them.
    pub allow_file_access: bool,
    /// Unpacked extension directories to load at startup.
    pub extensions: Option<Vec<String>>,
    /// Storage state (cookies, localStorage) to seed the profile with.
    pub storage_state: Option<String>,
    /// User agent for the whole browser, as opposed to a per-session override.
    pub user_agent: Option<String>,
    /// Continue past TLS errors. Off by default.
    pub ignore_https_errors: bool,
    /// Forces `prefers-color-scheme`: `light` or `dark`.
    pub color_scheme: Option<String>,
    /// Directory downloads are written to.
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
