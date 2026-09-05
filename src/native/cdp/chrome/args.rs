// SPDX-License-Identifier: MIT OR Apache-2.0
//! Chrome CLI arg builder and temp profile materialization.
use std::path::PathBuf;

use super::options::LaunchOptions;

pub(crate) struct ChromeArgs {
    pub args: Vec<String>,
    pub user_data_dir: PathBuf,
    pub temp_user_data_dir: Option<PathBuf>,
}

/// Combine the operator's proxy bypass list with the loopback entries.
///
/// The operator's own entries keep their position and their order: this list
/// is something a human wrote, and reordering it makes the argv harder to
/// compare against what was asked for. Loopback hosts are appended, and only
/// the ones not already present, so passing `--proxy-bypass 127.0.0.1` does
/// not produce a duplicate.
///
/// Returns `None` only when there is nothing at all to emit, which happens
/// when the operator passed no list and the loopback guard is switched off.
pub fn merge_proxy_bypass(operator: Option<&str>, add_loopback: bool) -> Option<String> {
    let mut out: Vec<&str> = operator
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    if add_loopback {
        for host in crate::constants::CDP_PROXY_LOOPBACK_BYPASS.split(',') {
            if !out
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(host))
            {
                out.push(host);
            }
        }
    }

    if out.is_empty() {
        None
    } else {
        Some(out.join(","))
    }
}

/// Create the profile dir on the **current** thread (unit tests / sync callers).
///
/// Production async launch uses [`crate::concurrency::create_dir_all_blocking`] instead.
///
/// # Why `user_data_dir` and not `temp_user_data_dir`
///
/// The two are not the same set. `temp_user_data_dir` marks OWNERSHIP — the
/// profiles this one-shot deletes at FINALIZE — while `user_data_dir` is what
/// Chrome is actually handed. Under an explicit `--profile` the first is `None`
/// and the second still names a directory, so keying materialization off
/// ownership meant nobody created the operator's profile: Chrome received a
/// path the product never made and answered `Failed to create
/// <profile>/SingletonLock: No such file or directory`. Creating a directory
/// and deleting it are different questions, and only the second one is about
/// ownership.
pub(crate) fn materialize_user_data_dir_sync(args: &ChromeArgs) -> Result<(), String> {
    let preexisting = args.user_data_dir.is_dir();
    std::fs::create_dir_all(&args.user_data_dir)
        .map_err(|e| format!("Failed to create profile dir: {e}"))?;
    restrict_named_profile(args, preexisting);
    Ok(())
}

/// Lock a NAMED profile to `0700`, but only the run that created it.
///
/// # Why the ownership test is `temp_user_data_dir.is_none()`
///
/// A throwaway profile already lives under the product's own XDG cache and is
/// deleted at FINALIZE, so tightening it buys nothing. A NAMED one — reached
/// through `--profile` or the `user_data_dir` key — persists on purpose and
/// holds cookies and session tokens, which on a shared host a default umask
/// publishes to every other account.
///
/// # Why only the run that created it
///
/// A directory the operator already had is a directory whose permissions the
/// operator already chose, and silently rewriting them is the CLI modifying a
/// host it was asked to observe. Creating one is a different question, and it
/// is the only one this answers.
///
/// # Why a failure here is not fatal
///
/// The profile is usable at the default mode; only its exposure is worse. A
/// launch refused over a `chmod` would trade a real capability for a warning,
/// and on a filesystem with no Unix modes there is nothing to trade for.
///
/// MEASURED 2026-09-04: the first shape of this guard ran the `chmod` BEFORE
/// `create_dir_all` and skipped it when the directory was absent, so the very
/// first run — the one that creates the directory and receives the first
/// cookie — left it at `0755`. The documentation promised `0700` and the code
/// delivered it only from the second run onward.
fn restrict_named_profile(args: &ChromeArgs, preexisting: bool) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if args.temp_user_data_dir.is_none() && !preexisting {
            let _ = std::fs::set_permissions(
                &args.user_data_dir,
                std::fs::Permissions::from_mode(0o700),
            );
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (args, preexisting);
    }
}

/// Create the profile dir off the Tokio worker and stamp ownership at once.
///
/// # Why the marker is written here and not later
///
/// The owner-pid marker is what tells residual GC that a profile has a living
/// owner. A profile that exists WITHOUT one is exactly the shape the sweep is
/// built to collect, so every instruction between the `mkdir` and the stamp is
/// a window in which this launch's own directory looks abandoned. The two
/// belong to one step, and the failure to stamp is now a launch failure rather
/// than a `debug!` line: a profile the GC may reap out from under Chrome is not
/// a degraded launch, it is a launch that has not happened yet.
pub(crate) async fn materialize_profile_dir(args: &ChromeArgs) -> Result<(), String> {
    let preexisting = args.user_data_dir.is_dir();
    crate::concurrency::create_dir_all_blocking(args.user_data_dir.clone())
        .await
        .map_err(|e| format!("Failed to create profile dir: {e}"))?;
    // This is the path a real launch takes; the sync twin above exists for the
    // pre-fork re-assertion. Both must tighten the mode, or the promise holds
    // on one path and not the other.
    restrict_named_profile(args, preexisting);
    // GAP-052: stamp the owning CLI pid so residual GC resolves liveness by
    // exact pid instead of substring-matching whole command lines. Only owned
    // temp profiles are ever swept, so an operator's own `--profile` is left
    // unmarked on purpose.
    if let Some(ref dir) = args.temp_user_data_dir {
        crate::residual::write_owner_pid(dir)
            .map_err(|e| format!("Failed to stamp profile owner pid: {e}"))?;
    }
    Ok(())
}

/// Re-create the profile dir if it vanished, and count it when it did.
///
/// Returns `Ok` unchanged in the overwhelmingly common case where the directory
/// is still there. The counter exists so the race stops being invisible: today
/// it shows up only as a Chrome exit 21 in an unrelated test, which is the
/// worst possible place to read it.
pub(crate) fn reassert_profile_dir(args: &ChromeArgs) -> Result<(), String> {
    if args.user_data_dir.is_dir() {
        return Ok(());
    }
    tracing::warn!(
        target: "browser_automation_cli::launch",
        dir = %args.user_data_dir.display(),
        "profile dir vanished between materialization and fork; recreating"
    );
    PROFILE_DIR_RECREATED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    materialize_user_data_dir_sync(args)?;
    if let Some(ref dir) = args.temp_user_data_dir {
        crate::residual::write_owner_pid(dir)
            .map_err(|e| format!("Failed to stamp profile owner pid: {e}"))?;
    }
    Ok(())
}

/// State of the profile dir at the moment a launch died, as a message suffix.
///
/// # Why measure this at all
///
/// Chrome answers a missing profile with `Failed to create
/// <profile>/SingletonLock: No such file or directory` and exit 21 — the truth,
/// but only about the symptom. Whether the directory was there when the launch
/// gave up is the fact that separates "the product never created it" from
/// "something removed it afterwards", and it is unrecoverable once the process
/// is gone. Reading it here costs one `stat` on a path already at hand, on a
/// path that is already failing.
///
/// This changes no `kind` and no exit code: the launch already failed, and it
/// already failed correctly as `unavailable`.
pub(crate) fn profile_postmortem(args: &ChromeArgs) -> String {
    let present = args.user_data_dir.is_dir();
    let recreated = PROFILE_DIR_RECREATED.load(std::sync::atomic::Ordering::Relaxed);
    format!(
        "\nProfile dir at failure: {} (exists: {present}, recreated_before_fork: {recreated})",
        args.user_data_dir.display()
    )
}

/// How many times [`reassert_profile_dir`] had to rebuild a vanished profile.
///
/// Process-local, and reported by the launch diagnostics rather than logged
/// once and forgotten.
pub(crate) static PROFILE_DIR_RECREATED: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

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
    // `TranslateUI` is the switch chromiumoxide's DEFAULT_ARGS used; `Translate`
    // is the newer name. Both are listed because the rename is version-dependent
    // and an unknown feature name is ignored rather than rejected.
    let mut disable_features: Vec<String> =
        vec!["Translate".to_string(), "TranslateUI".to_string()];

    // `AutomationControlled` is the switch that removes `navigator.webdriver`
    // at the source. This is deliberately NOT done with a JS getter override:
    // that leaves a patched `Function.prototype.toString` behind, which is
    // itself a marker, whereas killing the property leaves nothing to find.
    //
    // Paired with `--disable-blink-features=AutomationControlled` below because
    // the two switches gate different layers and Chrome versions have moved the
    // behaviour between them.
    if crate::browser_policy::stealth_enabled() {
        disable_features.push("AutomationControlled".to_string());
    }
    let has_extensions = options
        .extensions
        .as_ref()
        .is_some_and(|exts| !exts.is_empty());
    if has_extensions {
        // Chrome 127+ gates --load-extension behind this feature (must disable the gate).
        disable_features.push("DisableLoadExtensionCommandLineSwitch".to_string());
    }

    // ── Parity with chromiumoxide DEFAULT_ARGS (24 switches) ─────────────
    //
    // `Browser::launch` injected its own DEFAULT_ARGS on top of ours. The
    // self-spawn path passes exactly this argv, so each of those 24 switches was
    // audited. ADOPT means it is emitted below; REJECT records why it is not.
    //
    // | # | chromiumoxide switch                              | Verdict | Note |
    // |---|---------------------------------------------------|---------|------|
    // | 1 | disable-background-networking                     | ADOPT   | already present |
    // | 2 | enable-features=NetworkService,…InProcess         | ADOPT   | already in `enable_features` |
    // | 3 | disable-background-timer-throttling               | ADOPT   | added: hidden tabs must keep timers for scraping |
    // | 4 | disable-backgrounding-occluded-windows            | ADOPT   | already present |
    // | 5 | disable-breakpad                                  | ADOPT   | already present |
    // | 6 | disable-client-side-phishing-detection            | ADOPT   | already present |
    // | 7 | disable-component-extensions-with-background-pages| ADOPT   | already present |
    // | 8 | disable-default-apps                              | ADOPT   | already present |
    // | 9 | disable-dev-shm-usage                             | ADOPT   | conditional via `should_disable_dev_shm` |
    // |10 | disable-features=TranslateUI                      | ADOPT   | merged into the single disable list |
    // |11 | disable-hang-monitor                              | ADOPT   | already present |
    // |12 | disable-ipc-flooding-protection                   | ADOPT   | added: CDP bursts trip the throttle |
    // |13 | disable-popup-blocking                            | ADOPT   | already present |
    // |14 | disable-prompt-on-repost                          | ADOPT   | already present |
    // |15 | disable-renderer-backgrounding                    | ADOPT   | added: pairs with #3 for offscreen work |
    // |16 | disable-sync                                      | ADOPT   | already present |
    // |17 | force-color-profile=srgb                          | ADOPT   | added: screenshot bytes must not vary by host profile |
    // |18 | metrics-recording-only                            | REJECT  | GAP-016 / PRD §5F forbid enabling the metrics subsystem at all; we hard-disable it instead |
    // |19 | enable-automation                                 | REJECT  | sets `navigator.webdriver` and an infobar; the product deliberately does not announce itself |
    // |20 | password-store=basic                              | ADOPT   | conditional on `!use_real_keychain` |
    // |21 | use-mock-keychain                                 | ADOPT   | same condition as #20 |
    // |22 | enable-blink-features=IdleDetection               | REJECT  | enables an API no command uses; pure attack surface |
    // |23 | lang=en_US                                        | REJECT  | would pin `Accept-Language` and change scraped content, overriding the product `--lang` |
    let mut args = vec![
        "--remote-debugging-port=0".to_string(),
        "--no-first-run".to_string(),
        // #3 / #12 / #15 / #17: adopted from chromiumoxide DEFAULT_ARGS.
        "--disable-background-timer-throttling".to_string(),
        "--disable-ipc-flooding-protection".to_string(),
        "--disable-renderer-backgrounding".to_string(),
        "--force-color-profile=srgb".to_string(),
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

    if crate::browser_policy::stealth_enabled() {
        args.push("--disable-blink-features=AutomationControlled".to_string());
        // The automation infobar is visible proof of a driven browser on any
        // headed launch, and a page can measure the viewport it steals.
        args.push("--disable-infobars".to_string());
        // QUIC runs over UDP and bypasses an HTTP proxy entirely, so a run with
        // `--proxy` would leak part of its traffic around the egress the caller
        // chose. Disabling it also removes a transport fingerprint this product
        // cannot shape.
        args.push("--disable-quic".to_string());
    }

    // NOT XDG keys, decided 2026-08-31 after an audit proposed making both
    // configurable.
    //
    // `--disable-quic` above is a security decision with its reasoning attached:
    // QUIC runs over UDP and goes around an HTTP proxy, so a configurable
    // version of it is a switch whose ON position silently leaks traffic past
    // the egress the caller chose. A knob that can only be set wrongly is not a
    // knob.
    //
    // The four ANGLE flags below are one BUNDLE, not four values. `vulkan`,
    // `swiftshader` as the Vulkan implementation, `swiftshader` as the WebGPU
    // adapter and the surface being disabled only make sense together; setting
    // one to something else yields a combination no real browser ships. Same
    // argument `native::stealth` records for `GpuProfile`, where vendor,
    // renderer, adapter and `hardware_concurrency` are correlated and detectors
    // cross-check them — breaking the bundle desynchronises the very pair the
    // check compares.
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
        // SwiftShader is the software rasteriser headless falls back to, and it
        // reports itself through `WebGL_debug_renderer_info`. Forcing it on is
        // handing a bot check the exact string that says "no GPU here".
        //
        // Left in place when stealth is off, because that path exists for
        // deterministic screenshots where a stable software renderer is the
        // point. With stealth on, the GPU the host actually has is the more
        // defensible answer, and `Fingerprint::NativeGPU` reports it honestly.
        if !crate::browser_policy::stealth_enabled() {
            args.push("--enable-unsafe-swiftshader".to_string());
        }
    }

    if let Some(ref proxy) = options.proxy {
        args.push(format!("--proxy-server={proxy}"));
        // Loopback is bypassed whether or not the operator asked, because the
        // CDP control channel is loopback and a proxied control channel is a
        // browser that never answers. See `CDP_PROXY_LOOPBACK_BYPASS`.
        let merged = merge_proxy_bypass(
            options.proxy_bypass.as_deref(),
            crate::xdg::resolve_cdp_proxy_bypass_loopback(),
        );
        if let Some(list) = merged {
            args.push(format!("--proxy-bypass-list={list}"));
        }
    } else if let Some(ref bypass) = options.proxy_bypass {
        // No proxy to bypass. Emitted anyway so the flag is never silently
        // dropped: Chrome ignores it, and the argv stays a faithful record of
        // what the operator asked for.
        args.push(format!("--proxy-bypass-list={bypass}"));
    }

    // Precedence: `--profile` on argv, then the `user_data_dir` XDG key, then a
    // throwaway profile. The middle step exists because a one-shot CLI cannot
    // satisfy a detector that attests SESSION — fifty invocations present as
    // fifty machines, and the tokens the first one earned die at DIE. Naming
    // the directory once in the config file is what makes them one machine.
    //
    // Both named branches yield `temp_user_data_dir: None`, which is not an
    // omission: `None` is exactly how this crate says "the operator owns this
    // directory, never sweep it". Residual GC already reads it that way, so an
    // operator profile reached through the key inherits the same protection the
    // flag has always had, with no second vocabulary.
    let named_profile = options.profile.clone().or_else(crate::xdg::user_data_dir);
    let (user_data_dir, temp_user_data_dir) = if let Some(ref profile) = named_profile {
        let expanded = super::tooling::expand_tilde(profile);
        let dir = PathBuf::from(&expanded);
        // Mode is NOT set here. This function only builds argv; the directory
        // does not exist yet on a first run, so a `chmod` at this point is a
        // no-op on exactly the run that matters. It lives in
        // `restrict_named_profile`, called right after creation on both the
        // async launch path and its sync pre-fork twin.
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

    // Publish what this launch ACTUALLY passed, once per process.
    //
    // # The defect this closes
    //
    // An audit tried the experiment that decides whether a launch switch hurts
    // fidelity — remove one, repeat the measurement — and could not run it,
    // because the switches were invisible from the product's own surface. The
    // CLI exposed the SYMPTOM, since the flags show up in `ps`, and hid the
    // CONTROL. Reading argv out of `ps` is not a contract; this is.
    //
    // # Why publishing beats adding a knob per flag
    //
    // The refusal recorded above still holds: `--disable-quic` is a security
    // decision whose ON position leaks traffic past the caller's proxy, and the
    // four ANGLE switches are one correlated BUNDLE that a detector
    // cross-checks. A knob that can only be set wrongly is not a knob. What the
    // operator actually lacked was the ability to SEE the set and correlate it
    // with the flags that already govern it — `--no-stealth` removes the QUIC
    // decision, `--webgpu` gates the ANGLE bundle.
    //
    // `--ozone-override-screen-size`, the third switch that audit named, is
    // absent from this tree entirely as of 2026-09-04.
    //
    // `set` and not `get_or_init`: a second call in one process would mean two
    // launches with possibly different argv, and silently keeping the first is
    // how a witness starts lying. The value is read by `doctor --fingerprint`.
    let _ = LAUNCH_ARGS.set(args.clone());
    Ok(ChromeArgs {
        args,
        user_data_dir,
        temp_user_data_dir,
    })
}

/// The argv this process handed Chrome, or `None` before any launch.
static LAUNCH_ARGS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();

/// What [`build_chrome_args`] produced for this process.
///
/// `None` means no launch has happened yet, which is a different answer from
/// "launched with no flags" and must stay distinguishable.
pub(crate) fn launch_args() -> Option<&'static [String]> {
    LAUNCH_ARGS.get().map(Vec::as_slice)
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
