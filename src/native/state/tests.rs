// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fs;

use crate::native::cookies::Cookie;

use super::crypto::{decrypt_data, encrypt_data};
use super::fs_ops::{is_encrypted_state, is_state_file};
use super::*;

#[test]
fn test_storage_state_serialization() {
    let state = StorageState {
        cookies: vec![Cookie {
            name: "session".to_string(),
            value: "abc123".to_string(),
            domain: ".example.com".to_string(),
            path: "/".to_string(),
            expires: 0.0,
            size: 0,
            http_only: true,
            secure: false,
            session: true,
            same_site: Some("Lax".to_string()),
        }],
        origins: vec![OriginStorage {
            origin: "https://example.com".to_string(),
            local_storage: vec![StorageEntry {
                name: "key".to_string(),
                value: "val".to_string(),
            }],
            session_storage: vec![],
        }],
    };

    let json = serde_json::to_string_pretty(&state).unwrap();
    let parsed: StorageState = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.cookies.len(), 1);
    assert_eq!(parsed.cookies[0].name, "session");
    assert_eq!(parsed.origins.len(), 1);
    assert_eq!(parsed.origins[0].local_storage.len(), 1);
}

#[test]
fn test_storage_state_empty() {
    let state = StorageState {
        cookies: vec![],
        origins: vec![],
    };
    let json = serde_json::to_string(&state).unwrap();
    let parsed: StorageState = serde_json::from_str(&json).unwrap();
    assert!(parsed.cookies.is_empty());
    assert!(parsed.origins.is_empty());
}

#[test]
fn test_state_show_nonexistent_file() {
    let result = state_show("/tmp/nonexistent-browser-automation-cli-state-file.json");
    assert!(result.is_err());
}

#[test]
fn test_state_clear_nonexistent_file() {
    let result = state_clear(Some(
        "/tmp/nonexistent-browser-automation-cli-state-file.json",
    ));
    assert!(result.is_err());
}

#[test]
fn test_state_file_matcher_includes_transactional_backups() {
    assert!(is_state_file(std::path::Path::new("auth.json")));
    assert!(is_state_file(std::path::Path::new("auth.json.enc")));
    assert!(is_state_file(std::path::Path::new("auth.json.previous")));
    assert!(is_state_file(std::path::Path::new(
        "auth.json.enc.previous"
    )));
    assert!(is_encrypted_state(std::path::Path::new(
        "auth.json.enc.previous"
    )));
}

#[test]
fn test_state_clear_removes_transactional_backups() {
    let guard = crate::test_utils::EnvGuard::new(&["HOME", "BROWSER_AUTOMATION_CLI_NAMESPACE"]);
    let dir = tempfile::tempdir().unwrap();
    guard.set("HOME", dir.path().to_str().unwrap());
    guard.remove("BROWSER_AUTOMATION_CLI_NAMESPACE");

    let sessions = get_sessions_dir();
    fs::create_dir_all(&sessions).unwrap();
    fs::write(sessions.join("auth-test.json"), "{}").unwrap();
    fs::write(sessions.join("auth-test.json.previous"), "{}").unwrap();
    fs::write(sessions.join("auth-test.json.enc.previous"), "encrypted").unwrap();

    let result = state_clear(None).unwrap();

    assert_eq!(result["deleted"], 3);
    assert!(!sessions.join("auth-test.json").exists());
    assert!(!sessions.join("auth-test.json.previous").exists());
    assert!(!sessions.join("auth-test.json.enc.previous").exists());
}

#[test]
fn test_state_rename_nonexistent() {
    let result = state_rename(
        "/tmp/nonexistent-browser-automation-cli-state-file.json",
        "new-name",
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn test_state_list_returns_json() {
    let result = state_list().unwrap();
    assert!(result.get("files").is_some());
    assert!(result.get("directory").is_some());
}

#[test]
fn test_sessions_dir_path() {
    let dir = get_sessions_dir();
    assert!(dir.to_string_lossy().contains("sessions"));
}

#[test]
fn test_get_state_dir_namespace_scopes_sessions() {
    // Namespace is XDG-config only (no product env vars).
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let guard = crate::test_utils::EnvGuard::new(&["HOME", "XDG_CONFIG_HOME", "XDG_STATE_HOME"]);
    guard.set("HOME", home.to_str().unwrap());
    guard.set("XDG_CONFIG_HOME", home.join("config").to_str().unwrap());
    guard.set("XDG_STATE_HOME", home.join("state").to_str().unwrap());

    let cfg = crate::xdg::ProductConfig {
        namespace: Some("Worktree: One".into()),
        ..Default::default()
    };
    crate::xdg::write_config(&cfg).expect("write config under temp XDG");

    let state = get_state_dir();
    assert!(
        state.to_string_lossy().contains("namespaces")
            && state.to_string_lossy().contains("worktree-one"),
        "state dir should scope under namespaces/worktree-one, got {}",
        state.display()
    );
    assert!(get_sessions_dir().ends_with("sessions"));
}

#[test]
fn test_encrypt_decrypt_roundtrip() {
    let plain = b"hello world";
    let key = "test-secret-key";
    let encrypted = encrypt_data(plain, key).unwrap();
    assert!(encrypted.len() > 12);
    assert_ne!(&encrypted[12..], plain);
    let decrypted = decrypt_data(&encrypted, key).unwrap();
    assert_eq!(decrypted, plain);
}

#[test]
fn test_decrypt_wrong_key_fails() {
    let plain = b"secret data";
    let encrypted = encrypt_data(plain, "key1").unwrap();
    let result = decrypt_data(&encrypted, "key2");
    assert!(result.is_err());
}

#[test]
fn test_cookie_serde_roundtrip() {
    let cookie = Cookie {
        name: "test".to_string(),
        value: "123".to_string(),
        domain: ".test.com".to_string(),
        path: "/api".to_string(),
        expires: 1700000000.0,
        size: 7,
        http_only: false,
        secure: true,
        session: false,
        same_site: Some("Strict".to_string()),
    };

    let json = serde_json::to_value(&cookie).unwrap();
    assert_eq!(json["name"], "test");
    assert_eq!(json["httpOnly"], false);
    assert_eq!(json["secure"], true);
    assert_eq!(json["sameSite"], "Strict");
}

#[test]
fn test_dispatch_state_command_routes_state_list() {
    let cmd = serde_json::json!({ "action": "state_list" });
    let result = dispatch_state_command(&cmd);
    assert!(result.is_some());
    assert!(result.unwrap().is_ok());
}

#[test]
fn test_dispatch_state_command_returns_none_for_unknown() {
    let cmd = serde_json::json!({ "action": "navigate" });
    assert!(dispatch_state_command(&cmd).is_none());
}

#[test]
fn test_dispatch_state_command_returns_none_for_missing_action() {
    let cmd = serde_json::json!({});
    assert!(dispatch_state_command(&cmd).is_none());
}

#[test]
fn test_dispatch_state_show_missing_path() {
    let cmd = serde_json::json!({ "action": "state_show" });
    let result = dispatch_state_command(&cmd).unwrap();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Missing 'path' parameter");
}

#[test]
fn test_dispatch_state_rename_missing_params() {
    let cmd = serde_json::json!({ "action": "state_rename" });
    let result = dispatch_state_command(&cmd).unwrap();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Missing 'path' parameter");

    let cmd = serde_json::json!({ "action": "state_rename", "path": "/tmp/test.json" });
    let result = dispatch_state_command(&cmd).unwrap();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Missing 'name' parameter");
}
