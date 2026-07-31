// SPDX-License-Identifier: MIT OR Apache-2.0
//! Chrome discovery/args unit tests.
use super::args::{build_chrome_args, materialize_temp_user_data_dir_sync, should_disable_sandbox};
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
fn test_build_args_headless_includes_headless_flag() {
    let opts = LaunchOptions {
        headless: true,
        ..Default::default()
    };
    let result = build_chrome_args(&opts).unwrap();
    materialize_temp_user_data_dir_sync(&result).unwrap();
    assert!(result.args.iter().any(|a| a == "--headless=new"));
    assert!(result.args.iter().any(|a| a == "--hide-scrollbars"));
    assert!(result
        .args
        .iter()
        .any(|a| a == "--enable-unsafe-swiftshader"));
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
    materialize_temp_user_data_dir_sync(&result).unwrap();
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
    materialize_temp_user_data_dir_sync(&result).unwrap();
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
    materialize_temp_user_data_dir_sync(&result).unwrap();
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
    materialize_temp_user_data_dir_sync(&result).unwrap();
    let default_size = format!(
        "--window-size={},{}",
        crate::constants::DEFAULT_VIEWPORT_WIDTH,
        crate::constants::DEFAULT_VIEWPORT_HEIGHT
    );
    assert!(!result.args.iter().any(|a| a == &default_size));
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
    materialize_temp_user_data_dir_sync(&result).unwrap();
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
    materialize_temp_user_data_dir_sync(&result).unwrap();
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
    materialize_temp_user_data_dir_sync(&result).unwrap();
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
    materialize_temp_user_data_dir_sync(&result).unwrap();
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
    materialize_temp_user_data_dir_sync(&result).unwrap();
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
    materialize_temp_user_data_dir_sync(&result).unwrap();
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
    materialize_temp_user_data_dir_sync(&result).unwrap();
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
    materialize_temp_user_data_dir_sync(&result).unwrap();
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
    materialize_temp_user_data_dir_sync(&result).unwrap();
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
    materialize_temp_user_data_dir_sync(&result).unwrap();
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
    materialize_temp_user_data_dir_sync(&result).unwrap();
    assert!(!result
        .args
        .iter()
        .any(|a| a == "--ignore-certificate-errors"));
    if let Some(ref dir) = result.temp_user_data_dir {
        let _ = std::fs::remove_dir_all(dir);
    }
}
