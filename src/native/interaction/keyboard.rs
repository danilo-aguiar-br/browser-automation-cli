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
///
/// # Errors
///
/// Propagates
/// [`resolve_element_object_id`]
/// — unknown `@eN` ref, or a selector matching nothing — then the CDP error
/// raised by the `Runtime.callFunctionOn` that focuses the element, by the
/// second one that clears it when `clear` is set, and finally by
/// [`type_text_into_active_context`]. A failure after the clear leaves the
/// field empty.
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
///
/// # Errors
///
/// Fails with the CDP error raised by any `Input.dispatchKeyEvent` or
/// `Input.insertText` in the per-character sequence. Typing stops at the first
/// refusal, so the field keeps the prefix already inserted. Nothing here
/// checks that anything holds focus: with no focused element the events are
/// accepted and go nowhere.
pub async fn type_text_into_active_context(
    client: &CdpClient,
    session_id: &str,
    text: &str,
    delay_ms: Option<u64>,
) -> Result<(), String> {
    let mut k = super::kinematics::active();

    // Collected instead of streamed because the gap AFTER a character is a
    // property of the pair it forms with the next one, and an iterator that has
    // already yielded cannot say what comes after.
    let chars: Vec<char> = text.chars().collect();
    for (index, ch) in chars.iter().copied().enumerate() {
        let next = chars.get(index + 1).copied();
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
                        native_virtual_key_code: None,
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
                        native_virtual_key_code: None,
                        modifiers: None,
                    },
                    Some(session_id),
                )
                .await?;
        } else {
            // A typo is typed, noticed and erased BEFORE the intended
            // character, so the field still ends up holding exactly `text`.
            // Governed by `input_typo_permille`, which is zero unless the
            // caller asks for it: this is the one humanisation a page can read
            // as a different VALUE rather than as different timing.
            if let Some(wrong) = k.maybe_typo(ch) {
                insert_printable(client, session_id, &mut k, wrong).await?;
                let notice = k.type_delay_ms(None);
                if notice > 0 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(notice)).await;
                }
                press_key(client, session_id, "Backspace").await?;
                let resume = k.type_delay_ms(None);
                if resume > 0 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(resume)).await;
                }
            }
            insert_printable(client, session_id, &mut k, ch).await?;
        }

        // `type_delay_after` and not `type_delay_ms`: the character decides
        // whether this gap may also carry a long pause. Punctuation and word
        // boundaries are where a writer stops to think, and that stop is what
        // puts the right tail on the interval distribution.
        let delay = k.type_delay_after(ch, next, delay_ms);
        if delay > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
        }
    }

    Ok(())
}

/// Emit one printable character: the bracketing key events plus the insertion.
///
/// Split out of [`type_text_into_active_context`] when typo correction landed.
/// A mistyped character has to travel EXACTLY the path the intended one takes,
/// and two copies of this sequence would be two chances to drift apart — which
/// is the shape of leak a detector reads as "these two keystrokes came from
/// different code".
///
/// # Errors
///
/// Propagates the CDP error raised by `Input.dispatchKeyEvent` or
/// `Input.insertText`.
async fn insert_printable(
    client: &CdpClient,
    session_id: &str,
    k: &mut super::kinematics::Kinematics,
    ch: char,
) -> Result<(), String> {
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
                    native_virtual_key_code: None,
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
                    native_virtual_key_code: None,
                    modifiers: None,
                },
                Some(session_id),
            )
            .await?;
    }
    Ok(())
}

/// Press and release one named key, such as `Enter` or `Tab`.
///
/// # Errors
///
/// Propagates [`press_key_with_modifiers`] with no modifiers: the CDP error
/// raised by either `Input.dispatchKeyEvent`.
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
///
/// # Errors
///
/// Fails with the CDP error raised by the `keyDown` or the `keyUp`
/// `Input.dispatchKeyEvent`. A failure between the two leaves the key logically
/// held down for the page. An unknown `key` name is **not** an error: it is
/// mapped to a best-effort key/code pair and dispatched.
pub async fn press_key_with_modifiers(
    client: &CdpClient,
    session_id: &str,
    key: &str,
    modifiers: Option<i32>,
) -> Result<(), String> {
    // Chord bits carried IN the key string ("Control+a") are merged with any
    // passed explicitly, rather than one shape winning: both spellings reach
    // this function from real callers and dropping either would fix one and
    // break the other.
    let (base_key, chord_modifiers) = super::keys::parse_chord(key);
    let modifiers = match (modifiers, chord_modifiers) {
        (None, 0) => None,
        (explicit, chord) => Some(explicit.unwrap_or(0) | chord),
    };
    let (key_name, code, key_code) = named_key_info(&base_key);

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
                native_virtual_key_code: None,
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
                native_virtual_key_code: None,
                modifiers,
            },
            Some(session_id),
        )
        .await?;

    Ok(())
}
