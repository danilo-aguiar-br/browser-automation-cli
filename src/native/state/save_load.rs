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

/// [`save_state`] written atomically for the auto-save path.
///
/// Auto-save runs at shutdown, where a crash mid-write would leave a truncated
/// file in place of a working session. Writing to a temp file and renaming means
/// the old state survives a failed save instead of being destroyed by it.
pub async fn save_auto_state_transactional(
    client: &CdpClient,
    session_id: &str,
    session_name: &str,
    session_id_str: &str,
    visited_origins: &FxHashSet<String>,
) -> Result<String, String> {
    if !is_valid_session_name(session_name) {
        return Err(session_name_error(session_name));
    }

    let dir = get_sessions_dir();
    // PAR-80: mkdir/rename off async worker.
    crate::concurrency::create_dir_all_blocking(dir.clone())
        .await
        .map_err(|e| format!("Failed to create state directory {}: {e}", dir.display()))?;

    let tmp_dir = dir.join(".tmp");
    crate::concurrency::create_dir_all_blocking(tmp_dir.clone())
        .await
        .map_err(|e| {
            format!(
                "Failed to create temporary state directory {}: {e}",
                tmp_dir.display()
            )
        })?;

    let base_name = format!("{session_name}-{session_id_str}");
    let final_json_path = dir.join(format!("{base_name}.json"));
    let final_path = if crate::xdg::encryption_key().is_some() {
        PathBuf::from(format!("{}.enc", final_json_path.to_string_lossy()))
    } else {
        final_json_path
    };
    let candidate_json_path = tmp_dir.join(format!(
        "{}-candidate-{}.json",
        base_name,
        std::process::id()
    ));
    let candidate_arg = candidate_json_path.to_string_lossy().to_string();

    let candidate_path = save_state(
        client,
        session_id,
        Some(&candidate_arg),
        Some(session_name),
        session_id_str,
        visited_origins,
    )
    .await?;

    let candidate_for_validate = candidate_path.clone();
    if let Err(err) =
        tokio::task::spawn_blocking(move || validate_state_file(&candidate_for_validate))
            .await
            .map_err(|e| format!("state validate join: {e}"))?
    {
        let _ = fs::remove_file(&candidate_path);
        return Err(err);
    }

    let previous_path = PathBuf::from(format!("{}.previous", final_path.to_string_lossy()));
    let final_exists = final_path.exists();
    if final_exists {
        let _ = fs::remove_file(&previous_path);
        crate::concurrency::rename_blocking(final_path.clone(), previous_path.clone())
            .await
            .map_err(|e| {
                format!(
                    "Failed to rotate previous state {} to {}: {e}",
                    final_path.display(),
                    previous_path.display()
                )
            })?;
    }

    let candidate = PathBuf::from(&candidate_path);
    if let Err(err) =
        crate::concurrency::rename_blocking(candidate.clone(), final_path.clone()).await
    {
        if previous_path.exists() && !final_path.exists() {
            let _ = crate::concurrency::rename_blocking(previous_path.clone(), final_path.clone())
                .await;
        }
        return Err(format!(
            "Failed to promote state {} to {}: {err}",
            candidate.display(),
            final_path.display()
        ));
    }
    if previous_path.exists() {
        let _ = fs::remove_file(&previous_path);
    }

    Ok(final_path.to_string_lossy().to_string())
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

/// Check that a file parses as a state file before anything acts on it.
pub fn validate_state_file(path: &str) -> Result<(), String> {
    let json_str = read_state_json(path)?;
    let _: StorageState =
        crate::json_util::from_str(&json_str).map_err(|e| format!("Invalid state file: {e}"))?;
    Ok(())
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
