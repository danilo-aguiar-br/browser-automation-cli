// SPDX-License-Identifier: MIT OR Apache-2.0
//! Chrome discovery/args unit tests.
use super::args::{
    build_chrome_args, materialize_user_data_dir_sync, merge_proxy_bypass, should_disable_sandbox,
};
use super::tooling::{expand_tilde, find_playwright_chromium};
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
fn a_proxy_never_swallows_the_cdp_control_channel() {
    // The regression this guards: `--proxy http://127.0.0.1:1 goto` failed
    // with "Timed out after 20000ms waiting for Chrome CDP endpoint", because
    // the proxy also captured the loopback WebSocket the CLI drives Chrome
    // with. The message blamed Chrome for the proxy's doing.
    let opts = LaunchOptions {
        proxy: Some("http://127.0.0.1:1".to_string()),
        ..Default::default()
    };
    let result = build_chrome_args(&opts).unwrap();
    materialize_user_data_dir_sync(&result).unwrap();
    let bypass = result
        .args
        .iter()
        .find(|a| a.starts_with("--proxy-bypass-list="))
        .expect("a proxy launch must carry a bypass list");
    assert!(bypass.contains("127.0.0.1"), "{bypass}");
    assert!(bypass.contains("localhost"), "{bypass}");
}

#[test]
fn a_launch_without_a_proxy_has_nothing_to_bypass() {
    let opts = LaunchOptions::default();
    let result = build_chrome_args(&opts).unwrap();
    materialize_user_data_dir_sync(&result).unwrap();
    assert!(!result
        .args
        .iter()
        .any(|a| a.starts_with("--proxy-bypass-list=")));
}

#[test]
fn merging_keeps_the_operator_list_and_adds_loopback_once() {
    // Operator entries keep their order so the argv still reads as what was
    // asked for; loopback is appended, and only what is missing.
    let merged = merge_proxy_bypass(Some("example.com,127.0.0.1"), true).unwrap();
    assert!(merged.starts_with("example.com,127.0.0.1"), "{merged}");
    assert_eq!(merged.matches("127.0.0.1").count(), 1, "{merged}");
    assert!(merged.contains("localhost"), "{merged}");
}

#[test]
fn merging_is_case_insensitive_about_duplicates() {
    let merged = merge_proxy_bypass(Some("LOCALHOST"), true).unwrap();
    assert_eq!(merged.to_ascii_lowercase().matches("localhost").count(), 1);
}

#[test]
fn merging_drops_blank_entries_from_a_sloppy_list() {
    let merged = merge_proxy_bypass(Some("a.test, ,b.test,"), false).unwrap();
    assert_eq!(merged, "a.test,b.test");
}

#[test]
fn merging_with_nothing_to_say_emits_nothing() {
    assert!(merge_proxy_bypass(None, false).is_none());
    assert!(merge_proxy_bypass(Some("  "), false).is_none());
}

#[test]
fn the_loopback_guard_can_be_switched_off() {
    let merged = merge_proxy_bypass(Some("example.com"), false).unwrap();
    assert_eq!(merged, "example.com");
}

#[test]
fn test_build_args_headless_includes_headless_flag() {
    let opts = LaunchOptions {
        headless: true,
        ..Default::default()
    };
    let result = build_chrome_args(&opts).unwrap();
    materialize_user_data_dir_sync(&result).unwrap();
    assert!(result.args.iter().any(|a| a == "--headless=new"));
    assert!(result.args.iter().any(|a| a == "--hide-scrollbars"));
    // SwiftShader is the software rasteriser, and it names itself through
    // `WEBGL_debug_renderer_info`. Forcing it on hands a bot check the exact
    // string that says "no GPU here", so stealth omits it and the non-stealth
    // path keeps it for deterministic screenshots.
    //
    // Asserted as an INVARIANT against the live policy rather than pinned to
    // one value: `stealth_enabled()` is a process-global, and a test that
    // flipped it would race every other test in the binary.
    assert_eq!(
        result
            .args
            .iter()
            .any(|a| a == "--enable-unsafe-swiftshader"),
        !crate::browser_policy::stealth_enabled(),
        "swiftshader must be present exactly when stealth is off"
    );
    assert!(result.args.iter().any(|a| {
        a == &format!(
            "--window-size={},{}",
            crate::constants::DEFAULT_VIEWPORT_WIDTH,
            crate::constants::DEFAULT_VIEWPORT_HEIGHT
        )
    }));
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
    materialize_user_data_dir_sync(&result).unwrap();
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
    materialize_user_data_dir_sync(&result).unwrap();
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
    // The path must come from a guard, not from a literal under `/tmp`.
    // `materialize_user_data_dir_sync` CREATES the directory in the `--profile`
    // branch — that is the whole point of the second assertion below — so a
    // literal left one behind on every run of this test. `TempDir` removes it on
    // drop, including on the unwind of a failing assertion.
    let profile_guard = tempfile::Builder::new()
        .prefix("bac-chrome-explicit-profile-")
        .tempdir()
        .expect("create explicit profile dir");
    let profile = profile_guard.path().to_string_lossy().into_owned();
    let opts = LaunchOptions {
        profile: Some(profile.clone()),
        ..Default::default()
    };
    let result = build_chrome_args(&opts).unwrap();
    materialize_user_data_dir_sync(&result).unwrap();
    assert!(result.temp_user_data_dir.is_none());
    let expected = format!("--user-data-dir={profile}");
    assert!(result.args.iter().any(|a| a == &expected));
}

#[test]
fn test_build_args_custom_window_size_not_overridden() {
    let opts = LaunchOptions {
        headless: true,
        args: vec!["--window-size=2560,1440".to_string()],
        ..Default::default()
    };
    let result = build_chrome_args(&opts).unwrap();
    materialize_user_data_dir_sync(&result).unwrap();
    let default_size = format!(
        "--window-size={},{}",
        crate::constants::DEFAULT_VIEWPORT_WIDTH,
        crate::constants::DEFAULT_VIEWPORT_HEIGHT
    );
    assert!(!result.args.iter().any(|a| a == &default_size));
    assert!(result.args.iter().any(|a| a == "--window-size=2560,1440"));
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
    materialize_user_data_dir_sync(&result).unwrap();
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
    materialize_user_data_dir_sync(&result).unwrap();
    assert!(!result.args.iter().any(|a| a.starts_with("--window-size=")));
    assert!(result.args.iter().any(|a| a == "--start-maximized"));
    if let Some(ref dir) = result.temp_user_data_dir {
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[test]
fn test_build_args_disables_translate() {
    let opts = LaunchOptions::default();
    let result = build_chrome_args(&opts).unwrap();
    materialize_user_data_dir_sync(&result).unwrap();
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
    materialize_user_data_dir_sync(&result).unwrap();
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
    materialize_user_data_dir_sync(&result).unwrap();
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
    materialize_user_data_dir_sync(&result).unwrap();
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
    materialize_user_data_dir_sync(&result).unwrap();
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
    materialize_user_data_dir_sync(&result).unwrap();
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
    materialize_user_data_dir_sync(&result).unwrap();
    assert!(!result.args.iter().any(|a| a.contains("--headless")));
    assert!(result
        .args
        .iter()
        .any(|a| a.starts_with("--load-extension=")));
    assert!(result.args.iter().any(|a| {
        a.starts_with("--disable-features=") && a.contains("DisableLoadExtensionCommandLineSwitch")
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
    materialize_user_data_dir_sync(&result).unwrap();
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
    materialize_user_data_dir_sync(&result).unwrap();
    assert!(!result
        .args
        .iter()
        .any(|a| a == "--ignore-certificate-errors"));
    if let Some(ref dir) = result.temp_user_data_dir {
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// An explicit `--profile` is materialized too, not only an owned temp profile.
///
/// Materialization used to key off `temp_user_data_dir`, which is `None` on this
/// branch, so nobody created the operator's directory: Chrome was handed a path
/// that did not exist and answered `Failed to create <profile>/SingletonLock: No
/// such file or directory`. Creating a directory and deleting it at FINALIZE are
/// different questions, and only the second one is about ownership.
#[test]
fn an_explicit_profile_directory_is_created_before_launch() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let profile = tmp
        .path()
        .join("nested")
        .join("profile-that-does-not-exist");
    assert!(!profile.exists(), "fixture must start absent");

    let opts = LaunchOptions {
        profile: Some(profile.display().to_string()),
        ..Default::default()
    };
    let result = build_chrome_args(&opts).unwrap();
    materialize_user_data_dir_sync(&result).unwrap();

    assert!(
        profile.is_dir(),
        "an explicit profile must exist before Chrome is forked"
    );
    assert!(
        result.temp_user_data_dir.is_none(),
        "an operator's own profile is never owned, so it is never deleted"
    );
}

/// A profile that vanished between materialization and the fork is rebuilt.
///
/// `Lifecycle::new` sweeps `chrome_profiles_dir` at the BORN of every
/// invocation and `cargo test` runs binaries concurrently, so the window
/// between the two is real ground that other processes walk over. This asserts
/// the precondition is re-established rather than merely likely.
#[test]
fn a_vanished_profile_is_recreated_before_the_fork() {
    let opts = LaunchOptions::default();
    let result = build_chrome_args(&opts).unwrap();
    materialize_user_data_dir_sync(&result).unwrap();
    assert!(result.user_data_dir.is_dir());

    std::fs::remove_dir_all(&result.user_data_dir).expect("simulate the sweep");
    assert!(!result.user_data_dir.exists(), "control: it is really gone");

    let before = super::args::PROFILE_DIR_RECREATED.load(std::sync::atomic::Ordering::Relaxed);
    super::args::reassert_profile_dir(&result).expect("must rebuild, not fail");
    let after = super::args::PROFILE_DIR_RECREATED.load(std::sync::atomic::Ordering::Relaxed);

    assert!(result.user_data_dir.is_dir(), "the dir must be back");
    assert_eq!(after, before + 1, "the race must be counted, not silent");
    let _ = std::fs::remove_dir_all(&result.user_data_dir);
}

/// Re-asserting a directory that is still there is a no-op and counts nothing.
#[test]
fn reasserting_a_live_profile_changes_nothing() {
    let opts = LaunchOptions::default();
    let result = build_chrome_args(&opts).unwrap();
    materialize_user_data_dir_sync(&result).unwrap();

    let before = super::args::PROFILE_DIR_RECREATED.load(std::sync::atomic::Ordering::Relaxed);
    super::args::reassert_profile_dir(&result).expect("no-op must succeed");
    let after = super::args::PROFILE_DIR_RECREATED.load(std::sync::atomic::Ordering::Relaxed);

    assert_eq!(
        after, before,
        "the common path must not inflate the counter"
    );
    let _ = std::fs::remove_dir_all(&result.user_data_dir);
}

/// The post-mortem names the profile and whether it was there at the failure.
#[test]
fn the_postmortem_reports_the_profile_state() {
    let opts = LaunchOptions::default();
    let result = build_chrome_args(&opts).unwrap();
    materialize_user_data_dir_sync(&result).unwrap();

    let present = super::args::profile_postmortem(&result);
    assert!(present.contains("exists: true"), "got {present}");
    assert!(
        present.contains(&result.user_data_dir.display().to_string()),
        "must name the path: {present}"
    );

    std::fs::remove_dir_all(&result.user_data_dir).expect("remove");
    let absent = super::args::profile_postmortem(&result);
    assert!(
        absent.contains("exists: false"),
        "a missing profile is the fact worth reading: {absent}"
    );
}
