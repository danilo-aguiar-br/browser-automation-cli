// SPDX-License-Identifier: MIT OR Apache-2.0
//! Interaction recording: page gestures in, `run --script` steps out.
//!
//! # Shape of the loop
//!
//! 1. `Runtime.enable` + `Runtime.addBinding` publish a function on the page.
//! 2. `Page.addScriptToEvaluateOnNewDocument` installs the capture listeners so
//!    they exist **before** the recorded document runs its own script.
//! 3. A fresh broadcast receiver is taken **before** navigating, so the very
//!    first `navigate` event is not lost to the race with `goto`.
//! 4. The loop drains `Runtime.bindingCalled` until either ceiling is reached.
//!
//! # Both ceilings are real
//!
//! `seconds` and `max_events` are independent stops and the first one to fire
//! ends the recording. A recorder with only a time budget hangs the caller for
//! its full window on a page that fires nothing; one with only an event budget
//! never returns on a page that fires nothing at all.

mod script;
mod steps;

use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tokio::sync::broadcast::error::RecvError;

use crate::constants::RECORD_BINDING_NAME;
use crate::error::{CliError, ErrorKind};
use crate::robots::RobotsPolicy;

use super::OneShotSession;

/// CDP event method carrying one call of the recorder binding.
const BINDING_CALLED: &str = "Runtime.bindingCalled";

impl OneShotSession {
    /// Navigate to `url` and record page gestures as replayable `run` steps.
    ///
    /// Returns `{ steps, events, truncated, seconds }` where `steps` is already
    /// in `run --script` shape, `events` is `steps.len()`, and `truncated` says
    /// the event ceiling — not the clock — is what ended the recording.
    ///
    /// # Errors
    ///
    /// Fails when the recorder cannot be armed (CDP `Runtime` / `Page` domain
    /// refused) or when the navigation to `url` fails.
    pub async fn record_interactions(
        &mut self,
        url: &str,
        robots: RobotsPolicy,
        seconds: u64,
        max_events: usize,
    ) -> Result<Value, CliError> {
        self.arm_recorder().await?;
        // Subscribe BEFORE the navigation: the capture script emits `navigate`
        // while `goto` is still in flight, and a receiver taken afterwards
        // starts at the current tail of the channel.
        let mut rx = self.manager.client.subscribe();
        self.goto(url, robots).await?;

        let started = Instant::now();
        let deadline = started + Duration::from_secs(seconds);
        let slice = Duration::from_millis(crate::xdg::resolve_event_pump_slice_ms());
        let mut recorded: Vec<Value> = Vec::new();
        let mut truncated = false;

        while Instant::now() < deadline {
            match tokio::time::timeout(slice, rx.recv()).await {
                Ok(Ok(event)) => {
                    if let Some(step) = step_for(&event) {
                        recorded.push(step);
                        if recorded.len() >= max_events {
                            truncated = true;
                            break;
                        }
                    }
                }
                // A lagged receiver dropped events it never saw; the recording
                // is incomplete but the remaining gestures are still worth it.
                Ok(Err(RecvError::Lagged(_))) => continue,
                Ok(Err(RecvError::Closed)) => break,
                // No event in this slice: keep waiting until the deadline.
                Err(_elapsed) => continue,
            }
        }

        Ok(json!({
            "events": recorded.len(),
            "truncated": truncated,
            "seconds": started.elapsed().as_secs(),
            "steps": recorded,
        }))
    }

    /// Publish the recorder binding and install the capture script.
    async fn arm_recorder(&mut self) -> Result<(), CliError> {
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        // `Runtime` is only enabled at launch under `--capture-console`, and
        // `Runtime.bindingCalled` does not arrive without it.
        self.manager
            .client
            .send_command_no_params("Runtime.enable", Some(&session_id))
            .await
            .map_err(|e| CliError::new(ErrorKind::Protocol, format!("Runtime.enable: {e}")))?;
        self.manager
            .client
            .send_command(
                "Runtime.addBinding",
                Some(json!({ "name": RECORD_BINDING_NAME })),
                Some(&session_id),
            )
            .await
            .map_err(|e| CliError::new(ErrorKind::Protocol, format!("Runtime.addBinding: {e}")))?;
        self.manager
            .client
            .attach_page_binding_forwarders()
            .await
            .map_err(|e| CliError::new(ErrorKind::Protocol, format!("binding listener: {e}")))?;
        self.manager
            .add_script_to_evaluate(&script::capture_script())
            .await
            .map_err(|e| {
                CliError::new(ErrorKind::Protocol, format!("record capture script: {e}"))
            })?;
        Ok(())
    }
}

/// Step for one CDP event, or `None` when the event is not ours.
fn step_for(event: &crate::native::cdp::types::CdpEvent) -> Option<Value> {
    if event.method != BINDING_CALLED {
        return None;
    }
    if event.params.get("name").and_then(Value::as_str) != Some(RECORD_BINDING_NAME) {
        return None;
    }
    let payload = event.params.get("payload").and_then(Value::as_str)?;
    steps::step_from_event(&steps::event_from_payload(payload)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::cdp::types::CdpEvent;

    fn event(method: &str, name: &str, payload: &str) -> CdpEvent {
        CdpEvent {
            method: method.to_string(),
            params: json!({ "name": name, "payload": payload }),
            session_id: None,
        }
    }

    #[test]
    fn only_our_binding_on_the_right_method_produces_a_step() {
        let good = event(
            BINDING_CALLED,
            RECORD_BINDING_NAME,
            r##"{"type":"click","selector":"#go"}"##,
        );
        assert_eq!(step_for(&good).expect("step")["cmd"], json!("press"));

        let other_method = event(
            "Runtime.consoleAPICalled",
            RECORD_BINDING_NAME,
            r##"{"type":"click","selector":"#go"}"##,
        );
        assert!(step_for(&other_method).is_none());

        let other_binding = event(
            BINDING_CALLED,
            "someOtherBinding",
            r##"{"type":"click","selector":"#go"}"##,
        );
        assert!(step_for(&other_binding).is_none());
    }
}
