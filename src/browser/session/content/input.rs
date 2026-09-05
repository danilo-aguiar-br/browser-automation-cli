// SPDX-License-Identifier: MIT OR Apache-2.0
//! press, write, click_at, keys, type_text

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};
use crate::native::interaction;

use super::super::OneShotSession;

impl OneShotSession {
    /// Click an element.
    ///
    /// # Errors
    ///
    /// Fails when the session has no active page, and with
    /// [`ErrorKind::Browser`] —
    /// `"press failed: …"`, carrying the `target_ref_from_view` suggestion —
    /// when `target` resolves to no element, its centre is covered by another
    /// element, or an input event is refused.
    ///
    /// A JavaScript dialog opened by the click is not an error: it is recorded
    /// on the session and reported as `dialog_opened: true`, so the next step
    /// sees it without racing the CDP event.
    ///
    /// With `include_snapshot`, a failure to take that snapshot also fails
    /// this call, even though the click already landed.
    ///
    /// The result carries `url_before`, `url_after` and `navigated`.
    ///
    /// `navigated: true` proves the page URL changed. The false case proves
    /// NOTHING: the URL is read immediately after the click, so a same-page
    /// submit, a fetch, or a navigation still in flight all leave it false. It
    /// is a positive signal only.
    pub async fn press(
        &mut self,
        target: &str,
        dblclick: bool,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self.session_id()?;
        let url_before = self.current_url().await;

        let result = if dblclick {
            interaction::dblclick(
                &self.manager.client,
                &session_id,
                &self.ref_map,
                target,
                &self.iframe_sessions,
            )
            .await
        } else {
            interaction::click(
                &self.manager.client,
                &session_id,
                &self.ref_map,
                target,
                "left",
                1,
                &self.iframe_sessions,
            )
            .await
        }
        .map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Browser,
                format!("press failed: {e}"),
                crate::i18n::suggestion_key("target_ref_from_view", None),
            )
        })?;

        self.drain_events();
        // GAP-041: the interaction itself reports the dialog synchronously; record
        // it now so the next step's guard sees it without racing the CDP event.
        if result.dialog_opened {
            self.mark_active_page_dialog_open();
        }
        let mut data = json!({
            "pressed": target,
            "dblclick": dblclick,
            "dialog_opened": result.dialog_opened,
        });
        let url_after = self.current_url().await;
        stamp_navigation(&mut data, &url_before, &url_after);
        self.attach_snapshot_if(include_snapshot, data).await
    }

    /// Set a field's value directly, without synthesising key events.
    ///
    /// # Errors
    ///
    /// Fails when the session has no active page, and with
    /// [`ErrorKind::Browser`] —
    /// `"write failed: …"`, carrying the `target_ref_from_view` suggestion —
    /// when `target` resolves to no element, when a `<select>` offers no
    /// matching option, and when a radio is asked for a falsy value, which
    /// HTML gives no way to reach.
    ///
    /// With `include_snapshot`, a failure to take that snapshot also fails
    /// this call, even though the value was already written.
    pub async fn write(
        &mut self,
        target: &str,
        value: &str,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self.session_id()?;
        let url_before = self.current_url().await;

        interaction::fill_smart(
            &self.manager.client,
            &session_id,
            &self.ref_map,
            target,
            value,
            &self.iframe_sessions,
        )
        .await
        .map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Browser,
                format!("write failed: {e}"),
                crate::i18n::suggestion_key("target_ref_from_view", None),
            )
        })?;

        self.drain_events();
        let mut data = json!({
            "written": target,
            "value_len": value.len(),
            "fill_mode": "smart",
        });
        let url_after = self.current_url().await;
        stamp_navigation(&mut data, &url_before, &url_after);
        self.attach_snapshot_if(include_snapshot, data).await
    }

    /// Click at absolute page coordinates (requires experimental vision flag at CLI).
    ///
    /// # Errors
    ///
    /// Fails when the session has no active page, and with
    /// [`ErrorKind::Browser`] —
    /// `"click-at failed: …"`, carrying the `vision_coordinates` suggestion —
    /// when an input event is refused.
    ///
    /// Coordinates that hit nothing, or lie outside the viewport, are **not**
    /// an error: no element is resolved on this path, so the events are
    /// dispatched and the page decides. A dialog opened by the click is
    /// reported as `dialog_opened: true`.
    pub async fn click_at(
        &mut self,
        x: f64,
        y: f64,
        dblclick: bool,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self.session_id()?;
        let result = interaction::click_at(&self.manager.client, &session_id, x, y, dblclick)
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Browser,
                    format!("click-at failed: {e}"),
                    crate::i18n::suggestion_key("vision_coordinates", None),
                )
            })?;
        self.drain_events();
        if result.dialog_opened {
            self.mark_active_page_dialog_open();
        }
        let data = json!({
            "clicked_at": { "x": x, "y": y },
            "dblclick": dblclick,
            "dialog_opened": result.dialog_opened,
        });
        self.attach_snapshot_if(include_snapshot, data).await
    }

    /// Press one named key, such as `Enter` or `Tab`.
    ///
    /// # Errors
    ///
    /// Fails when the session has no active page, and with
    /// [`ErrorKind::Browser`] —
    /// `"keys failed: …"`, carrying the `cdp_key_name` suggestion — when the
    /// key event is refused.
    ///
    /// An unknown key NAME is not rejected: it is mapped to a best-effort
    /// key/code pair and dispatched, so a typo reads as a key the page ignored
    /// rather than as an error.
    ///
    /// The key lands on whatever the page has focused. Use
    /// [`keys_ex`](Self::keys_ex) to name the element first.
    pub async fn keys(&mut self, key: &str, include_snapshot: bool) -> Result<Value, CliError> {
        self.keys_ex(key, None, include_snapshot).await
    }

    /// Press one named key, optionally focusing `target` first.
    ///
    /// # Why the target is optional and not required
    ///
    /// Without one the key goes to the ambient focus, which is what the CLI's
    /// own `keys` subcommand has always done and what a script relying on a
    /// preceding click still needs. With one, the destination stops being
    /// ambient: the element is focused in the same step, so the keystroke and
    /// the field it belongs to cannot be separated by an intervening step that
    /// moves focus.
    ///
    /// # Errors
    ///
    /// Fails when the session has no active page, with
    /// [`ErrorKind::Browser`] — `"keys focus failed: …"`, carrying the
    /// `target_ref_from_view` suggestion — when `target` resolves to no
    /// element, and with `"keys failed: …"` when the key event is refused.
    ///
    /// An element that cannot take focus is NOT an error: the DOM `focus()`
    /// call is a no-op and the key is dispatched anyway, so a target that
    /// silently refuses focus reads as a key the page ignored.
    pub async fn keys_ex(
        &mut self,
        key: &str,
        target: Option<&str>,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        let session_id = self.session_id()?;
        let url_before = self.current_url().await;

        if let Some(t) = target {
            interaction::focus(
                &self.manager.client,
                &session_id,
                &self.ref_map,
                t,
                &self.iframe_sessions,
            )
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Browser,
                    format!("keys focus failed: {e}"),
                    crate::i18n::suggestion_key("target_ref_from_view", None),
                )
            })?;
        }

        interaction::press_key(&self.manager.client, &session_id, key)
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Browser,
                    format!("keys failed: {e}"),
                    crate::i18n::suggestion_key("cdp_key_name", None),
                )
            })?;

        self.drain_events();
        let mut data = json!({ "key": key, "target": target });
        let url_after = self.current_url().await;
        stamp_navigation(&mut data, &url_before, &url_after);
        stamp_typing_timing(&mut data);
        self.attach_snapshot_if(include_snapshot, data).await
    }

    /// Type text character by character, firing real key events.
    ///
    /// Slower than [`write`](Self::write) and that is the point: a page with
    /// autocomplete, masking or per-keystroke validation only sees this route.
    ///
    /// # Errors
    ///
    /// Fails when the session has no active page, and with
    /// [`ErrorKind::Browser`] on
    /// `"type failed: …"` when `target` resolves to no element, on
    /// `"type (focus-only) failed: …"` when the key events are refused, and on
    /// `"type --submit key failed: …"` when the trailing `submit` key is
    /// refused — the text is already typed by then.
    ///
    /// [`ErrorKind::Usage`] is declared for a
    /// call with neither `target` nor `focus_only`, but is unreachable: a
    /// `None` target already takes the focus-only branch. The CLI enforces
    /// that pairing at argv instead.
    ///
    /// In focus-only mode the `clear` keystrokes are best-effort, and nothing
    /// verifies that anything holds focus, so text can be typed into a page
    /// that discards it.
    pub async fn type_text(
        &mut self,
        target: Option<&str>,
        text: &str,
        clear: bool,
        submit: Option<&str>,
        focus_only: bool,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        let session_id = self.session_id()?;
        let url_before = self.current_url().await;

        let typed_target = if focus_only || target.is_none() {
            // tool-ref type_text: type into currently focused element
            if clear {
                // Select-all then type (best-effort clear of focused field)
                let _ =
                    interaction::press_key(&self.manager.client, &session_id, "Control+a").await;
                let _ =
                    interaction::press_key(&self.manager.client, &session_id, "Backspace").await;
            }
            interaction::type_text_into_active_context(
                &self.manager.client,
                &session_id,
                text,
                None,
            )
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Browser,
                    format!("type (focus-only) failed: {e}"),
                    crate::i18n::suggestion_key("target_ref_from_view", None),
                )
            })?;
            target.unwrap_or("(focused)").to_string()
        } else {
            let t = match target {
                Some(s) => s,
                None => {
                    return Err(CliError::new(
                        ErrorKind::Usage,
                        "type requires --target or --focus-only",
                    ));
                }
            };
            interaction::type_text(
                &self.manager.client,
                &session_id,
                &self.ref_map,
                t,
                text,
                clear,
                None,
                &self.iframe_sessions,
            )
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Browser,
                    format!("type failed: {e}"),
                    crate::i18n::suggestion_key("target_ref_from_view", None),
                )
            })?;
            t.to_string()
        };

        if let Some(key) = submit {
            interaction::press_key(&self.manager.client, &session_id, key)
                .await
                .map_err(|e| {
                    CliError::with_suggestion(
                        ErrorKind::Browser,
                        format!("type --submit key failed: {e}"),
                        crate::i18n::suggestion_key("cdp_key_name", None),
                    )
                })?;
        }

        self.drain_events();
        let mut data = json!({
            "typed": typed_target,
            "text_len": text.len(),
            "cleared": clear,
            "submit": submit,
            "focus_only": focus_only || target.is_none(),
        });
        let url_after = self.current_url().await;
        stamp_navigation(&mut data, &url_before, &url_after);
        stamp_typing_timing(&mut data);
        self.attach_snapshot_if(include_snapshot, data).await
    }

    /// The active page's URL, or an empty string when it cannot be read.
    ///
    /// A failure is folded into `""` on purpose: this is reported beside a
    /// result, never as one, and turning an input step into an error because
    /// its bookkeeping could not read a URL would be a worse trade.
    async fn current_url(&mut self) -> String {
        self.manager.get_url().await.unwrap_or_default()
    }
}

/// Record what the page URL did across an input action.
///
/// # Why an input step reports a URL at all
///
/// Every action here answers `ok: true` for "the events were dispatched", and
/// that is not the question the caller asked. A click on a dead submit button,
/// a key sent to a field nothing listens to, text typed into a page that
/// discards it — all three succeed by that definition. Measured 2026-08-31: a
/// `press` step carrying a `key` field returned `ok: true` with the target
/// echoed back and changed nothing on the page, and no field in the envelope
/// disagreed. `navigated` is the cheapest field that can disagree.
///
/// # What it does not prove
///
/// `navigated: false` is NOT proof that nothing happened: a same-page form
/// submit, a fetch, or a navigation still in flight when the URL is read all
/// leave it false. It is a positive signal only. The names are fixed English
/// because they are machine-readable envelope fields, not prose.
fn stamp_navigation(data: &mut Value, before: &str, after: &str) {
    let Some(obj) = data.as_object_mut() else {
        return;
    };
    obj.insert("url_before".into(), json!(before));
    obj.insert("url_after".into(), json!(after));
    obj.insert("navigated".into(), json!(before != after));
}

/// Report the per-character timing this process actually applied.
///
/// # Why the envelope needs it
///
/// The dispersion of typing delays is configurable through nine XDG keys, and
/// until now nothing in the answer said which values took effect. A caller
/// tuning them against a detector could not tell a key that changed the rhythm
/// from a key that was ignored, because both produce the same envelope.
///
/// # Why it is read back rather than read from config
///
/// [`Kinematics::timing_metrics`] reports the RESOLVED values, so
/// `--input-profile direct` publishes zero dispersion because `direct` sleeps
/// zero. Reading the config keys here instead would announce a dispersion the
/// profile suppresses — a pair that cannot both be true, which is a louder
/// signal than the one it would be trying to describe.
///
/// # Why only the typing verbs call it
///
/// `press` dispatches a mouse click and applies none of these values. Stamping
/// them there would publish a number that describes nothing the command did.
fn stamp_typing_timing(data: &mut Value) {
    let Some(obj) = data.as_object_mut() else {
        return;
    };
    let m = crate::native::interaction::active_kinematics().timing_metrics();
    obj.insert(
        "timing".into(),
        json!({
            "mean_ms": m.mean_ms,
            "stddev_ms": m.stddev_ms,
            "distribution": m.distribution,
        }),
    );
}
