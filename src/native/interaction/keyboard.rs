// SPDX-License-Identifier: MIT OR Apache-2.0

use super::keys::{char_to_key_info, key_text, named_key_info};
use crate::native::cdp::client::CdpClient;
use crate::native::cdp::types::*;
use crate::native::element::{resolve_element_object_id, RefMap};
use serde_json::Value;

#[allow(clippy::too_many_arguments)]
/// Type into an element by synthesising per-character key events.
///
/// Slower than assigning `value`, and that is the point: a page that reacts to
/// `keydown` (autocomplete, input masking, validation) only sees this route.
///
/// Under `--input-profile human` a printable character is wrapped in a real
/// `keydown` / `keyup` pair around [`Input.insertText`]. Before 0.1.8 printables
/// went through `insertText` alone, so the docstring above described the opposite
/// of what the code did: an autocomplete listening for `keydown` saw nothing.
/// `direct` keeps the bare `insertText`, which is the Electron-safe path.
///
/// [`Input.insertText`]: https://chromedevtools.github.io/devtools-protocol/tot/Input/#method-insertText
pub async fn type_text(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    text: &str,
    clear: bool,
    delay_ms: Option<u64>,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
) -> Result<(), String> {
    let (object_id, effective_session_id) = resolve_element_object_id(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
    )
    .await?;

    // Focus
    client
        .send_command_typed::<_, Value>(
            "Runtime.callFunctionOn",
            &CallFunctionOnParams {
                function_declaration: "function() { this.focus(); }".to_string(),
                object_id: Some(object_id.clone()),
                arguments: None,
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(&effective_session_id),
        )
        .await?;

    if clear {
        client
            .send_command_typed::<_, Value>(
                "Runtime.callFunctionOn",
                &CallFunctionOnParams {
                    function_declaration: r#"function() {
                        this.select && this.select();
                        this.value = '';
                        this.dispatchEvent(new Event('input', { bubbles: true }));
                    }"#
                    .to_string(),
                    object_id: Some(object_id),
                    arguments: None,
                    return_by_value: Some(true),
                    await_promise: Some(false),
                },
                Some(&effective_session_id),
            )
            .await?;
    }

    type_text_into_active_context(client, session_id, text, delay_ms).await
}

/// [`type_text`] aimed at whatever currently holds focus.
///
/// For typing into an element that cannot be addressed by ref or selector,
/// such as one inside a shadow root the snapshot did not reach.
pub async fn type_text_into_active_context(
    client: &CdpClient,
    session_id: &str,
    text: &str,
    delay_ms: Option<u64>,
) -> Result<(), String> {
    let mut k = super::kinematics::active();

    for ch in text.chars() {
        if matches!(ch, '\n' | '\r' | '\t') {
            let (key, code, key_code) = char_to_key_info(ch);
            let text_str = key_text(&key);
            client
                .send_command_typed::<_, Value>(
                    "Input.dispatchKeyEvent",
                    &DispatchKeyEventParams {
                        event_type: "keyDown".to_string(),
                        key: Some(key.clone()),
                        code: Some(code.clone()),
                        text: text_str.clone(),
                        unmodified_text: text_str,
                        windows_virtual_key_code: Some(key_code),
                        native_virtual_key_code: Some(key_code),
                        modifiers: None,
                    },
                    Some(session_id),
                )
                .await?;

            client
                .send_command_typed::<_, Value>(
                    "Input.dispatchKeyEvent",
                    &DispatchKeyEventParams {
                        event_type: "keyUp".to_string(),
                        key: Some(key),
                        code: Some(code),
                        text: None,
                        unmodified_text: None,
                        windows_virtual_key_code: Some(key_code),
                        native_virtual_key_code: Some(key_code),
                        modifiers: None,
                    },
                    Some(session_id),
                )
                .await?;
        } else {
            // VS Code/Electron webviews reject repeated dispatchKeyEvent calls
            // that CARRY printable `text`. The rejection is about the payload,
            // not about the events existing, so a keydown/keyup pair with `text`
            // left empty brackets the insertion without reintroducing that bug:
            // the page gets the events it listens for, `insertText` still owns
            // the character. `direct` skips the pair entirely.
            let (key, code, key_code) = char_to_key_info(ch);
            let bracket = k.profile().is_human();

            if bracket {
                client
                    .send_command_typed::<_, Value>(
                        "Input.dispatchKeyEvent",
                        &DispatchKeyEventParams {
                            event_type: "keyDown".to_string(),
                            key: Some(key.clone()),
                            code: Some(code.clone()),
                            text: None,
                            unmodified_text: None,
                            windows_virtual_key_code: Some(key_code),
                            native_virtual_key_code: Some(key_code),
                            modifiers: None,
                        },
                        Some(session_id),
                    )
                    .await?;
            }

            client
                .send_command_typed::<_, Value>(
                    "Input.insertText",
                    &InsertTextParams {
                        text: ch.to_string(),
                    },
                    Some(session_id),
                )
                .await?;

            if bracket {
                let dwell = k.key_dwell_ms();
                if dwell > 0 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(dwell)).await;
                }
                client
                    .send_command_typed::<_, Value>(
                        "Input.dispatchKeyEvent",
                        &DispatchKeyEventParams {
                            event_type: "keyUp".to_string(),
                            key: Some(key),
                            code: Some(code),
                            text: None,
                            unmodified_text: None,
                            windows_virtual_key_code: Some(key_code),
                            native_virtual_key_code: Some(key_code),
                            modifiers: None,
                        },
                        Some(session_id),
                    )
                    .await?;
            }
        }

        let delay = k.type_delay_ms(delay_ms);
        if delay > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
        }
    }

    Ok(())
}

/// Press and release one named key, such as `Enter` or `Tab`.
pub async fn press_key(client: &CdpClient, session_id: &str, key: &str) -> Result<(), String> {
    press_key_with_modifiers(client, session_id, key, None).await
}

/// Dispatch a keyDown+keyUp sequence for `key` with an optional CDP modifier bitmask.
///
/// Modifier values follow the CDP `Input.dispatchKeyEvent` spec:
/// 1 = Alt, 2 = Control, 4 = Meta (Cmd), 8 = Shift.
///
/// Callers that need a platform-appropriate modifier (e.g. Cmd on macOS,
/// Ctrl elsewhere) must choose the value themselves -- see `cfg!(target_os)`.
pub async fn press_key_with_modifiers(
    client: &CdpClient,
    session_id: &str,
    key: &str,
    modifiers: Option<i32>,
) -> Result<(), String> {
    let (key_name, code, key_code) = named_key_info(key);

    // Suppress text insertion when Control (2) or Meta (4) modifiers are active,
    // since these are command chords (e.g. Ctrl+A = select-all), not text input.
    let has_command_modifier = modifiers.is_some_and(|m| m & (2 | 4) != 0);
    let text = if has_command_modifier {
        None
    } else {
        key_text(&key_name)
    };

    client
        .send_command_typed::<_, Value>(
            "Input.dispatchKeyEvent",
            &DispatchKeyEventParams {
                event_type: "keyDown".to_string(),
                key: Some(key_name.clone()),
                code: Some(code.clone()),
                text: text.clone(),
                unmodified_text: text.clone(),
                windows_virtual_key_code: Some(key_code),
                native_virtual_key_code: Some(key_code),
                modifiers,
            },
            Some(session_id),
        )
        .await?;

    // Hold the key. A zero-length press cannot trigger auto-repeat and reads as
    // instantaneous to any handler that measures duration.
    let dwell = super::kinematics::active().key_dwell_ms();
    if dwell > 0 {
        tokio::time::sleep(tokio::time::Duration::from_millis(dwell)).await;
    }

    client
        .send_command_typed::<_, Value>(
            "Input.dispatchKeyEvent",
            &DispatchKeyEventParams {
                event_type: "keyUp".to_string(),
                key: Some(key_name),
                code: Some(code),
                text: None,
                unmodified_text: None,
                windows_virtual_key_code: Some(key_code),
                native_virtual_key_code: Some(key_code),
                modifiers,
            },
            Some(session_id),
        )
        .await?;

    Ok(())
}
