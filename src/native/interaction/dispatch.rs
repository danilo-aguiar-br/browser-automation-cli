// SPDX-License-Identifier: MIT OR Apache-2.0

use super::kinematics;
use super::types::{ClickResult, PendingRelease};
use crate::native::cdp::client::CdpClient;
use crate::native::cdp::types::*;
use serde_json::Value;

/// Last pointer position for this process, so a move starts where the previous
/// one ended instead of teleporting from the origin every time.
///
/// # Concurrency
///
/// `Relaxed` on two independent coordinates: an interleaved read can only mix
/// two plausible start points, which costs nothing -- the destination is exact
/// either way, and this is a cosmetic trajectory origin, not a correctness input.
static CURSOR_X: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static CURSOR_Y: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn cursor_position() -> (f64, f64) {
    use std::sync::atomic::Ordering;
    (
        f64::from_bits(CURSOR_X.load(Ordering::Relaxed)),
        f64::from_bits(CURSOR_Y.load(Ordering::Relaxed)),
    )
}

fn remember_cursor(x: f64, y: f64) {
    use std::sync::atomic::Ordering;
    CURSOR_X.store(x.to_bits(), Ordering::Relaxed);
    CURSOR_Y.store(y.to_bits(), Ordering::Relaxed);
}

/// Move the pointer to `(x, y)` along an interpolated path.
///
/// Returns `Ok(true)` when a JavaScript dialog interrupted the move. Under
/// `direct` this collapses to the single `mouseMoved` released before 0.1.8.
pub(super) async fn travel_to(
    client: &CdpClient,
    session_id: &str,
    accept_sessions: &[&str],
    x: f64,
    y: f64,
) -> Result<bool, String> {
    let mut k = kinematics::active();
    let path = k.path(cursor_position(), (x, y));
    for (px, py) in path {
        let opened = dispatch_mouse_or_dialog(
            client,
            session_id,
            accept_sessions,
            &DispatchMouseEventParams {
                event_type: "mouseMoved".to_string(),
                x: px,
                y: py,
                button: None,
                buttons: None,
                click_count: None,
                delta_x: None,
                delta_y: None,
                modifiers: None,
            },
        )
        .await?;
        remember_cursor(px, py);
        if opened {
            return Ok(true);
        }
        let gap = k.move_gap_ms();
        if gap > 0 && k.profile().is_human() {
            tokio::time::sleep(std::time::Duration::from_millis(gap)).await;
        }
    }
    Ok(false)
}

/// Dispatches one mouse event and waits for the browser to ack it, but
/// returns Ok(true) if a JavaScript dialog opens first. A synchronous dialog
/// (confirm/prompt/alert in the event handler) blocks the renderer's main
/// thread, so the input ack cannot arrive until the dialog is resolved;
/// without this the command hangs until the client read timeout and the agent
/// never sees the pending-dialog warning.
pub(super) async fn dispatch_mouse_or_dialog(
    client: &CdpClient,
    session_id: &str,
    accept_sessions: &[&str],
    params: &DispatchMouseEventParams,
) -> Result<bool, String> {
    use tokio::sync::broadcast::error::RecvError;

    // Subscribe before sending so the dialog event cannot slip past us.
    let mut events = client.subscribe();
    let send =
        client.send_command_typed::<_, Value>("Input.dispatchMouseEvent", params, Some(session_id));
    tokio::pin!(send);
    loop {
        tokio::select! {
            res = &mut send => {
                res?;
                return Ok(false);
            }
            event = events.recv() => {
                match event {
                    Ok(e) if e.method == "Page.javascriptDialogOpening" => {
                        // Only a dialog on this click's frame/page session
                        // aborts it; a background-tab dialog must not. A
                        // session-less event has no flat session and is
                        // treated as the top-level page (i.e. ours).
                        let ours = match e.session_id.as_deref() {
                            Some(sid) => accept_sessions.contains(&sid),
                            None => true,
                        };
                        if ours {
                            return Ok(true);
                        }
                        continue;
                    }
                    Ok(_) => continue,
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => {
                        (&mut send).await?;
                        return Ok(false);
                    }
                }
            }
        }
    }
}

pub(super) async fn dispatch_click(
    client: &CdpClient,
    session_id: &str,
    accept_sessions: &[&str],
    x: f64,
    y: f64,
    button: &str,
    click_count: i32,
) -> Result<ClickResult, String> {
    // Jitter the contact point: without it every click on one element lands on
    // the exact geometric centre, so N clicks share N identical coordinates.
    let mut k = kinematics::active();
    let (x, y) = k.jitter_target(x, y);

    // Travel there instead of teleporting. A cursor that materializes on the
    // target with no prior movement is not a gesture a hand can produce.
    if travel_to(client, session_id, accept_sessions, x, y).await? {
        // No button was pressed yet, nothing to release.
        return Ok(ClickResult {
            dialog_opened: true,
            pending_release: None,
        });
    }

    let button_value = match button {
        "right" => 2,
        "middle" => 4,
        _ => 1,
    };

    // Press
    if dispatch_mouse_or_dialog(
        client,
        session_id,
        accept_sessions,
        &DispatchMouseEventParams {
            event_type: "mousePressed".to_string(),
            x,
            y,
            button: Some(button.to_string()),
            buttons: Some(button_value),
            click_count: Some(click_count),
            delta_x: None,
            delta_y: None,
            modifiers: None,
        },
    )
    .await?
    {
        // Dialog opened from the mousedown handler: the button is held and the
        // release will never arrive on its own. Hand the caller what it needs
        // to release once the dialog is resolved.
        return Ok(ClickResult {
            dialog_opened: true,
            pending_release: Some(PendingRelease {
                session_id: session_id.to_string(),
                x,
                y,
                button: button.to_string(),
            }),
        });
    }

    // Hold the button. Press and release in the same millisecond at the same
    // pixel is the shape of a synthesized click, not of a pressed finger.
    let dwell = k.click_dwell_ms();
    if dwell > 0 {
        tokio::time::sleep(std::time::Duration::from_millis(dwell)).await;
    }

    // Release. A dialog here fired from the click/mouseup handler, which runs
    // after the button is already up, so there is nothing left to release.
    let dialog_opened = dispatch_mouse_or_dialog(
        client,
        session_id,
        accept_sessions,
        &DispatchMouseEventParams {
            event_type: "mouseReleased".to_string(),
            x,
            y,
            button: Some(button.to_string()),
            buttons: Some(0),
            click_count: Some(click_count),
            delta_x: None,
            delta_y: None,
            modifiers: None,
        },
    )
    .await?;
    Ok(ClickResult {
        dialog_opened,
        pending_release: None,
    })
}
