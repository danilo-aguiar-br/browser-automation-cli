// SPDX-License-Identifier: MIT OR Apache-2.0
//! Unit tests for native browser host.

use super::launch::{
    lightpanda_target_init_timeout, remaining_until, run_with_lightpanda_deadline,
};
use super::navigate::poll_network_idle;
use super::types::{
    active_page_index_after_removal, is_internal_chrome_target, should_track_target,
    update_page_target_info_in_pages,
};
use super::validate::validate_lightpanda_options;
use super::*;
use crate::native::cdp::chrome::LaunchOptions;
use crate::native::cdp::types::*;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tokio::time::sleep;

#[test]
fn test_format_tab_id() {
    assert_eq!(format_tab_id(1), "t1");
    assert_eq!(format_tab_id(42), "t42");
}

#[test]
fn test_parse_tab_ref_id() {
    assert_eq!(TabRef::parse("t1"), Ok(TabRef::Id(1)));
    assert_eq!(TabRef::parse("t42"), Ok(TabRef::Id(42)));
    assert_eq!(TabRef::parse("T7"), Ok(TabRef::Id(7)));
}

#[test]
fn test_parse_tab_ref_label() {
    assert_eq!(TabRef::parse("docs"), Ok(TabRef::Label("docs".to_string())));
    assert_eq!(
        TabRef::parse("app-2"),
        Ok(TabRef::Label("app-2".to_string()))
    );
    assert_eq!(
        TabRef::parse("my_tab"),
        Ok(TabRef::Label("my_tab".to_string()))
    );
}

#[test]
fn test_parse_tab_ref_rejects_bare_integer() {
    let err = TabRef::parse("2").unwrap_err();
    assert!(
        err.contains("positional integers are not accepted"),
        "error should teach the user to use `t<N>`: {err}"
    );
    assert!(err.contains("t2"));
}

#[test]
fn test_parse_tab_ref_rejects_empty() {
    assert!(TabRef::parse("").is_err());
    assert!(TabRef::parse("   ").is_err());
}

#[test]
fn test_parse_tab_ref_rejects_zero() {
    let err = TabRef::parse("t0").unwrap_err();
    assert!(err.contains("start at t1"));
}

#[test]
fn test_parse_tab_ref_rejects_invalid_label() {
    assert!(TabRef::parse("2docs").is_err());
    assert!(TabRef::parse("-docs").is_err());
    assert!(TabRef::parse("docs!").is_err());
    assert!(TabRef::parse("docs space").is_err());
}

#[test]
fn test_is_valid_label() {
    assert!(is_valid_label("docs"));
    assert!(is_valid_label("Docs"));
    assert!(is_valid_label("app-2"));
    assert!(is_valid_label("my_tab"));
    assert!(!is_valid_label(""));
    assert!(!is_valid_label("2docs"));
    assert!(!is_valid_label("-docs"));
    assert!(!is_valid_label("docs!"));
}

#[test]
fn test_should_track_popup_target_with_empty_url() {
    let target = TargetInfo {
        target_id: "popup-1".to_string(),
        target_type: "page".to_string(),
        title: String::new(),
        url: String::new(),
        attached: None,
        browser_context_id: None,
    };

    assert!(should_track_target(&target));
}

#[test]
fn test_should_not_track_internal_chrome_target() {
    let target = TargetInfo {
        target_id: "chrome-tab".to_string(),
        target_type: "page".to_string(),
        title: "New Tab".to_string(),
        url: "chrome://newtab/".to_string(),
        attached: None,
        browser_context_id: None,
    };

    assert!(!should_track_target(&target));
}

#[test]
fn test_update_page_target_info_in_pages_updates_existing_page() {
    let mut pages = vec![PageInfo {
        tab_id: 1,
        label: None,
        target_id: "popup-1".to_string(),
        session_id: "session-1".to_string(),
        url: String::new(),
        title: String::new(),
        target_type: "page".to_string(),
    }];
    let target = TargetInfo {
        target_id: "popup-1".to_string(),
        target_type: "page".to_string(),
        title: "Popup".to_string(),
        url: "https://example.com/popup".to_string(),
        attached: None,
        browser_context_id: None,
    };

    assert!(update_page_target_info_in_pages(&mut pages, &target));
    assert_eq!(pages[0].url, "https://example.com/popup");
    assert_eq!(pages[0].title, "Popup");
}

#[test]
fn test_active_page_index_after_removal_shifts_when_earlier_tab_is_removed() {
    assert_eq!(active_page_index_after_removal(2, 0, 3), 1);
}

#[test]
fn test_active_page_index_after_removal_keeps_same_slot_when_later_tab_is_removed() {
    assert_eq!(active_page_index_after_removal(1, 2, 3), 1);
}

#[test]
fn test_active_page_index_after_removal_clamps_when_active_last_tab_is_removed() {
    assert_eq!(active_page_index_after_removal(3, 3, 3), 2);
}

#[test]
fn test_active_page_index_after_removal_resets_when_last_page_disappears() {
    assert_eq!(active_page_index_after_removal(0, 0, 0), 0);
}

#[test]
fn test_validate_launch_options_extensions_and_cdp() {
    let ext = vec!["/path/to/ext".to_string()];
    assert!(validate_launch_options(Some(&ext), true, None, None, false, None,).is_err());
}

#[test]
fn test_validate_launch_options_profile_and_cdp() {
    assert!(validate_launch_options(None, true, Some("/path"), None, false, None,).is_err());
}

#[test]
fn test_validate_launch_options_storage_state_and_profile() {
    assert!(validate_launch_options(
        None,
        false,
        Some("/profile"),
        Some("/state.json"),
        false,
        None,
    )
    .is_err());
}

#[test]
fn test_validate_launch_options_storage_state_and_extensions() {
    let ext = vec!["/ext".to_string()];
    assert!(
        validate_launch_options(Some(&ext), false, None, Some("/state.json"), false, None,)
            .is_err()
    );
}

#[test]
fn test_validate_launch_options_allow_file_access_firefox() {
    assert!(
        validate_launch_options(None, false, None, None, true, Some("/usr/bin/firefox"),).is_err()
    );
}

#[test]
fn test_validate_launch_options_valid() {
    assert!(validate_launch_options(None, false, None, None, false, None,).is_ok());
}

#[test]
fn test_to_ai_friendly_error_strict_mode() {
    assert_eq!(
        to_ai_friendly_error("Strict mode violation: multiple elements"),
        "Element matched multiple results. Use a more specific selector."
    );
}

#[test]
fn test_to_ai_friendly_error_not_visible() {
    assert_eq!(
        to_ai_friendly_error("element is not visible"),
        "Element exists but is not visible. Wait for it to become visible or scroll it into view."
    );
}

#[test]
fn test_to_ai_friendly_error_intercept() {
    assert_eq!(
        to_ai_friendly_error("element intercepted by another element"),
        "Another element is covering the target element. Try scrolling or closing overlays."
    );
}

#[test]
fn test_to_ai_friendly_error_timeout() {
    assert_eq!(
        to_ai_friendly_error("Timeout waiting for element"),
        "Operation timed out. The page may still be loading or the element may not exist."
    );
}

#[test]
fn test_to_ai_friendly_error_not_found() {
    assert_eq!(
        to_ai_friendly_error("Element not found"),
        "Element not found. Verify the selector is correct and the element exists in the DOM."
    );
}

#[test]
fn test_to_ai_friendly_error_unknown() {
    let msg = "Some custom error message";
    assert_eq!(to_ai_friendly_error(msg), msg);
}

/// Errors containing "not found" but NOT "element" should pass through unchanged.
#[test]
fn test_to_ai_friendly_error_ignores_non_element_not_found() {
    let err = "Chrome not found. Install Chrome or use --executable-path.";
    assert_eq!(to_ai_friendly_error(err), err);
}

#[test]
fn test_to_ai_friendly_error_catches_no_element() {
    let mapped =
        "Element not found. Verify the selector is correct and the element exists in the DOM.";
    assert_eq!(to_ai_friendly_error("No element found for css 'x'"), mapped);
}

#[test]
fn test_remaining_until_returns_none_for_past_deadline() {
    let deadline = Instant::now()
        .checked_sub(Duration::from_millis(1))
        .expect("past instant should be representable");
    assert!(remaining_until(deadline).is_none());
}

#[tokio::test]
async fn test_run_with_lightpanda_deadline_enforces_timeout() {
    let deadline = Instant::now() + Duration::from_millis(25);
    let err = tokio::time::timeout(
        Duration::from_secs(1),
        run_with_lightpanda_deadline(
            deadline,
            async {
                sleep(Duration::from_millis(100)).await;
                Ok::<(), String>(())
            },
            "Target domain initialization attempt exceeded the remaining startup deadline",
        ),
    )
    .await
    .expect("outer timeout should not fire")
    .unwrap_err();

    assert!(
        err.contains("Timed out after 10000ms waiting for Lightpanda Target domain to initialize")
    );
    assert!(err.contains("remaining startup deadline"));
}

#[tokio::test]
async fn test_run_with_lightpanda_deadline_returns_operation_error() {
    let deadline = Instant::now() + Duration::from_secs(1);
    let err = run_with_lightpanda_deadline(
        deadline,
        async { Err::<(), String>("Target.getTargets failed".to_string()) },
        "unused timeout context",
    )
    .await
    .unwrap_err();

    assert_eq!(err, "Target.getTargets failed");
}

#[test]
fn test_lightpanda_target_init_timeout_includes_last_error() {
    let err = lightpanda_target_init_timeout(Some("Target.setDiscoverTargets failed"));
    assert!(
        err.contains("Timed out after 10000ms waiting for Lightpanda Target domain to initialize")
    );
    assert!(err.contains("Target.setDiscoverTargets failed"));
}

#[test]
fn test_validate_lightpanda_rejects_webgpu() {
    let options = LaunchOptions {
        webgpu: true,
        ..Default::default()
    };
    let err = validate_lightpanda_options(&options).unwrap_err();
    assert!(err.contains("WebGPU"));
    assert!(validate_lightpanda_options(&LaunchOptions::default()).is_ok());
}

#[test]
fn test_is_internal_chrome_target() {
    assert!(is_internal_chrome_target("chrome://newtab/"));
    assert!(is_internal_chrome_target(
        "chrome://omnibox-popup.top-chrome/"
    ));
    assert!(is_internal_chrome_target(
        "chrome-extension://abc123/popup.html"
    ));
    assert!(is_internal_chrome_target(
        "devtools://devtools/bundled/inspector.html"
    ));
    assert!(!is_internal_chrome_target("https://example.com"));
    assert!(!is_internal_chrome_target("http://localhost:3000"));
    assert!(!is_internal_chrome_target(crate::constants::ABOUT_BLANK));
}

// -----------------------------------------------------------------------
// poll_network_idle tests
// -----------------------------------------------------------------------

fn cdp_event(method: &str, session_id: &str, params: Value) -> CdpEvent {
    CdpEvent {
        method: method.to_string(),
        params,
        session_id: Some(session_id.to_string()),
    }
}

/// Regression test for #846: when no network events arrive at all (e.g.
/// page fully served from cache), poll_network_idle must NOT return
/// instantly.  It should observe at least 500 ms of idle before resolving.
#[tokio::test]
async fn test_network_idle_no_events_does_not_return_instantly() {
    let (tx, mut rx) = broadcast::channel::<CdpEvent>(16);
    let session = "s1";

    let start = tokio::time::Instant::now();
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        poll_network_idle(session, &mut rx, Duration::from_secs(5)),
    )
    .await
    .expect("outer timeout should not fire");

    assert!(result.is_ok());
    let elapsed = start.elapsed();
    assert!(
        elapsed >= Duration::from_millis(500),
        "network idle returned in {elapsed:?}, expected >= 500ms"
    );

    drop(tx);
}

/// Normal flow: requests start and finish, idle is detected after the last
/// request completes and 500 ms of silence passes.
#[tokio::test]
async fn test_network_idle_after_requests_complete() {
    let (tx, mut rx) = broadcast::channel::<CdpEvent>(16);
    let session = "s1";

    let _keep_alive = tx.clone();
    tokio::spawn(async move {
        sleep(Duration::from_millis(50)).await;
        let _ = tx.send(cdp_event(
            "Network.requestWillBeSent",
            session,
            json!({ "requestId": "r1" }),
        ));
        sleep(Duration::from_millis(100)).await;
        let _ = tx.send(cdp_event(
            "Network.loadingFinished",
            session,
            json!({ "requestId": "r1" }),
        ));
    });

    let start = tokio::time::Instant::now();
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        poll_network_idle(session, &mut rx, Duration::from_secs(5)),
    )
    .await
    .expect("outer timeout should not fire");

    assert!(result.is_ok());
    let elapsed = start.elapsed();
    assert!(
        elapsed >= Duration::from_millis(500),
        "should wait >= 500ms after last request finishes, got {elapsed:?}"
    );
}

/// A new request arriving during the idle window resets the timer.
#[tokio::test]
async fn test_network_idle_resets_on_new_request() {
    let (tx, mut rx) = broadcast::channel::<CdpEvent>(16);
    let session = "s1";

    let _keep_alive = tx.clone();
    tokio::spawn(async move {
        sleep(Duration::from_millis(50)).await;
        let _ = tx.send(cdp_event(
            "Network.requestWillBeSent",
            session,
            json!({ "requestId": "r1" }),
        ));
        sleep(Duration::from_millis(50)).await;
        let _ = tx.send(cdp_event(
            "Network.loadingFinished",
            session,
            json!({ "requestId": "r1" }),
        ));
        // Wait 200ms (< 500ms idle window), then fire another request
        sleep(Duration::from_millis(200)).await;
        let _ = tx.send(cdp_event(
            "Network.requestWillBeSent",
            session,
            json!({ "requestId": "r2" }),
        ));
        sleep(Duration::from_millis(100)).await;
        let _ = tx.send(cdp_event(
            "Network.loadingFinished",
            session,
            json!({ "requestId": "r2" }),
        ));
    });

    let start = tokio::time::Instant::now();
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        poll_network_idle(session, &mut rx, Duration::from_secs(5)),
    )
    .await
    .expect("outer timeout should not fire");

    assert!(result.is_ok());
    let elapsed = start.elapsed();
    // r2 finishes at ~400ms; idle should be detected at ~900ms
    assert!(
        elapsed >= Duration::from_millis(800),
        "should wait for idle after second request, got {elapsed:?}"
    );
}

/// When the overall timeout expires before idle is reached, the function
/// returns an error.
#[tokio::test]
async fn test_network_idle_overall_timeout() {
    let (tx, mut rx) = broadcast::channel::<CdpEvent>(16);
    let session = "s1";

    // Keep sending requests so idle is never reached
    tokio::spawn(async move {
        for i in 0u64.. {
            let _ = tx.send(cdp_event(
                "Network.requestWillBeSent",
                session,
                json!({ "requestId": format!("r{}", i) }),
            ));
            sleep(Duration::from_millis(100)).await;
        }
    });

    let result = poll_network_idle(session, &mut rx, Duration::from_millis(800)).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("Timeout waiting for networkidle"));
}
