// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fs;
use std::path::PathBuf;

use rustc_hash::FxHashSet;

use serde_json::{json, Value};

use crate::native::cdp::client::CdpClient;
use crate::native::cdp::types::EvaluateParams;
use crate::native::cookies;
use crate::validation::{is_valid_session_name, session_name_error};

use super::collect::{collect_frame_origins, collect_storage_via_temp_target, eval_origin_storage};
use super::crypto::{decrypt_data, encrypt_data};
use super::fs_ops::{get_sessions_dir, is_encrypted_state};
use super::types::StorageState;

/// Capture cookies and web storage from the live browser into a state file.
///
/// # Errors
///
/// Fails with the cookie read error from
/// [`get_all_cookies`](crate::native::cookies::get_all_cookies), with
/// `"Failed to serialize state: …"`, with
/// `"Failed to create state directory <dir>: …"` when `path` is `None` and the
/// sessions directory cannot be made, with the session-name error when
/// `session_name` is not a valid one, with the encryption error when
/// `encryption_key` is configured, and with
/// `"Failed to write state to <path>: …"` on the final disk write.
///
/// Storage collection is deliberately best-effort: a refused
/// `Page.getFrameTree`, an origin whose storage cannot be read, and a failed
/// temp-target sweep are all skipped, so the state file is written with fewer
/// origins rather than not written at all.
pub async fn save_state(
    client: &CdpClient,
    session_id: &str,
    path: Option<&str>,
    session_name: Option<&str>,
    session_id_str: &str,
    visited_origins: &FxHashSet<String>,
) -> Result<String, String> {
    let cookies = cookies::get_all_cookies(client, session_id).await?;

    let origin_js = r#"(() => {
        const result = { origin: location.origin, localStorage: [], sessionStorage: [] };
        try {
            for (let i = 0; i < localStorage.length; i++) {
                const key = localStorage.key(i);
                result.localStorage.push({ name: key, value: localStorage.getItem(key) });
            }
        } catch(e) {}
        try {
            for (let i = 0; i < sessionStorage.length; i++) {
                const key = sessionStorage.key(i);
                result.sessionStorage.push({ name: key, value: sessionStorage.getItem(key) });
            }
        } catch(e) {}
        return result;
    })()"#;

    // Merge visited origins with current frame tree origins
    let mut all_origins = visited_origins.clone();
    if let Ok(tree_result) = client
        .send_command_no_params("Page.getFrameTree", Some(session_id))
        .await
    {
        if let Some(tree) = tree_result.get("frameTree") {
            collect_frame_origins(tree, &mut all_origins);
        }
    }

    // 1. Collect localStorage from the current page
    let mut origins = Vec::new();
    let mut current_origin = String::new();

    if let Some(storage) = eval_origin_storage(client, session_id, origin_js).await {
        current_origin = storage.origin.clone();
        if !storage.local_storage.is_empty() || !storage.session_storage.is_empty() {
            origins.push(storage);
        }
    }

    // 2. Collect localStorage from remaining origins via a disposable temp target
    all_origins.remove(&current_origin);
    if !all_origins.is_empty() {
        let remaining: Vec<String> = all_origins.into_iter().collect();
        if let Ok(temp_origins) =
            collect_storage_via_temp_target(client, &remaining, origin_js).await
        {
            origins.extend(temp_origins);
        }
    }

    let state = StorageState { cookies, origins };
    let json_str = serde_json::to_string_pretty(&state)
        .map_err(|e| format!("Failed to serialize state: {e}"))?;

    let mut save_path = match path {
        Some(p) => p.to_string(),
        None => {
            let dir = get_sessions_dir();
            // PAR-81: mkdir off async worker.
            crate::concurrency::create_dir_all_blocking(dir.clone())
                .await
                .map_err(|e| format!("Failed to create state directory {}: {e}", dir.display()))?;
            let name = session_name.unwrap_or("default");
            if !is_valid_session_name(name) {
                return Err(session_name_error(name));
            }
            dir.join(format!("{name}-{session_id_str}.json"))
                .to_string_lossy()
                .to_string()
        }
    };

    // PAR-60: disk write off the async worker (docsrs spawn_blocking / std::fs).
    if let Some(key) = crate::xdg::encryption_key() {
        let encrypted = encrypt_data(json_str.as_bytes(), &key)?;
        save_path.push_str(".enc");
        crate::concurrency::write_bytes_blocking(PathBuf::from(&save_path), encrypted)
            .await
            .map_err(|e| format!("Failed to write state to {save_path}: {e}"))?;
    } else {
        crate::concurrency::write_bytes_blocking(PathBuf::from(&save_path), json_str.into_bytes())
            .await
            .map_err(|e| format!("Failed to write state to {save_path}: {e}"))?;
    }

    Ok(save_path)
}

pub(crate) fn read_state_json(path: &str) -> Result<String, String> {
    if is_encrypted_state(std::path::Path::new(path)) {
        let key = crate::xdg::encryption_key().ok_or_else(|| {
            "Encrypted state file requires config set encryption_key (XDG config)".to_string()
        })?;
        let data = fs::read(path).map_err(|e| format!("Failed to read state from {path}: {e}"))?;
        let decrypted = decrypt_data(&data, &key)?;
        Ok(String::from_utf8(decrypted)
            .map_err(|e| format!("Decrypted state is not valid UTF-8: {e}"))?)
    } else {
        match fs::read_to_string(path) {
            Ok(s) => Ok(s),
            Err(e) => {
                if let Some(key) = crate::xdg::encryption_key() {
                    let enc_path = format!("{path}.enc");
                    if let Ok(data) = fs::read(&enc_path) {
                        let decrypted = decrypt_data(&data, &key)?;
                        Ok(String::from_utf8(decrypted)
                            .map_err(|de| format!("Decrypted state is not valid UTF-8: {de}"))?)
                    } else {
                        Err(format!("Failed to read state from {path}: {e}"))
                    }
                } else {
                    Err(format!("Failed to read state from {path}: {e}"))
                }
            }
        }
    }
}

/// Async read of state JSON (PAR-77: disk off the async worker).
pub(crate) async fn read_state_json_async(path: &str) -> Result<String, String> {
    let path = path.to_string();
    tokio::task::spawn_blocking(move || read_state_json(&path))
        .await
        .map_err(|e| format!("state read join: {e}"))?
}

/// Restore cookies and web storage from a state file into the live browser.
///
/// Storage is replayed per origin, which requires navigating to each one: the
/// browser refuses writes to another origin's storage.
///
/// # Errors
///
/// Fails with the read error from the state file — including
/// `"Encrypted state file requires config set encryption_key (XDG config)"`,
/// a wrong key, and `"Decrypted state is not valid UTF-8: …"` — with
/// `"Invalid state file: …"` when the JSON does not deserialize into a
/// `StorageState`, with the cookie write error from
/// [`set_cookies`](crate::native::cookies::set_cookies), and with the CDP
/// error raised by the `Page.navigate` that visits each origin.
///
/// Individual `localStorage.setItem` / `sessionStorage.setItem` evaluations
/// are best-effort: an origin that refuses storage — third-party cookies
/// blocked, or a quota exceeded — is skipped silently, so restore can report
/// success with less state than the file held.
pub async fn load_state(client: &CdpClient, session_id: &str, path: &str) -> Result<(), String> {
    // PAR-77: never fs::read on async worker for state restore.
    let json_str = read_state_json_async(path).await?;

    let state: StorageState =
        crate::json_util::from_str(&json_str).map_err(|e| format!("Invalid state file: {e}"))?;

    // Load cookies
    if !state.cookies.is_empty() {
        let cookie_values: Vec<Value> = state
            .cookies
            .iter()
            .map(|c| serde_json::to_value(c).unwrap_or(Value::Null))
            .collect();
        cookies::set_cookies(client, session_id, cookie_values, None).await?;
    }

    // Load storage per origin — sequential by design (N-143 / PAR-54):
    // single CDP session cannot navigate multiple origins concurrently.
    for origin in &state.origins {
        if origin.local_storage.is_empty() && origin.session_storage.is_empty() {
            continue;
        }

        // Navigate to origin to set storage
        let navigate_url = format!("{}/", origin.origin.trim_end_matches('/'));
        client
            .send_command(
                "Page.navigate",
                Some(json!({ "url": navigate_url })),
                Some(session_id),
            )
            .await?;

        // Brief wait for navigation
        tokio::time::sleep(tokio::time::Duration::from_millis(
            crate::xdg::policy::policy_u64(crate::xdg::policy::key::STATE_LOAD_SETTLE_MS),
        ))
        .await;

        for entry in &origin.local_storage {
            let js = format!(
                "localStorage.setItem({}, {})",
                serde_json::to_string(&entry.name).unwrap_or_default(),
                serde_json::to_string(&entry.value).unwrap_or_default(),
            );
            let _ = client
                .send_command_typed::<_, crate::native::cdp::types::EvaluateResult>(
                    "Runtime.evaluate",
                    &EvaluateParams {
                        expression: js,
                        return_by_value: Some(true),
                        await_promise: Some(false),
                    },
                    Some(session_id),
                )
                .await;
        }

        for entry in &origin.session_storage {
            let js = format!(
                "sessionStorage.setItem({}, {})",
                serde_json::to_string(&entry.name).unwrap_or_default(),
                serde_json::to_string(&entry.value).unwrap_or_default(),
            );
            let _ = client
                .send_command_typed::<_, crate::native::cdp::types::EvaluateResult>(
                    "Runtime.evaluate",
                    &EvaluateParams {
                        expression: js,
                        return_by_value: Some(true),
                        await_promise: Some(false),
                    },
                    Some(session_id),
                )
                .await;
        }
    }

    Ok(())
}
