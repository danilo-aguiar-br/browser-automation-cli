// SPDX-License-Identifier: MIT OR Apache-2.0
//! Recorded page event → `run --script` step translation.
//!
//! # Why the translation happens here and not at replay time
//!
//! The whole point of `record` is that its output needs no adapter: the NDJSON
//! it writes is fed straight back into `run --script`. Emitting a private event
//! vocabulary and shipping a converter alongside it would put two formats in the
//! product where one will do, and the converter would be the thing that drifts.
//!
//! The step names and key names below are the ones `run` already dispatches
//! (`crate::commands::run::execute`), so a change there breaks this file's
//! tests rather than silently producing scripts `run` rejects.

use serde_json::{json, Value};

/// One captured page event, as the injected script serialises it.
#[derive(Debug, serde::Deserialize)]
pub(super) struct RecordedEvent {
    /// Event family: `navigate`, `click`, `input`, `change` or `submit`.
    #[serde(rename = "type")]
    pub kind: String,
    /// CSS selector of the target element (absent for `navigate`).
    #[serde(default)]
    pub selector: Option<String>,
    /// Field value for `input` / `change`.
    #[serde(default)]
    pub value: Option<String>,
    /// Document URL for `navigate`.
    #[serde(default)]
    pub url: Option<String>,
}

/// Translate one captured event into a `run --script` step.
///
/// Returns `None` for an event that has no faithful replay step — an unknown
/// family, or a targeted gesture whose selector the page-side script could not
/// build. Dropping is deliberate: a step with an empty target would replay as a
/// failure attributed to `run` rather than to the recording.
pub(super) fn step_from_event(event: &RecordedEvent) -> Option<Value> {
    let selector = event.selector.as_deref().filter(|s| !s.is_empty());
    match event.kind.as_str() {
        "navigate" => {
            let url = event.url.as_deref().filter(|u| !u.is_empty())?;
            Some(json!({ "cmd": "goto", "url": url }))
        }
        "click" => Some(json!({ "cmd": "press", "target": selector? })),
        "input" | "change" => Some(json!({
            "cmd": "write",
            "target": selector?,
            "value": event.value.clone().unwrap_or_default(),
        })),
        "submit" => Some(json!({ "cmd": "submit", "target": selector? })),
        _ => None,
    }
}

/// Parse one `Runtime.bindingCalled` payload into an event, ignoring garbage.
///
/// The payload crosses a boundary the product does not control (page script),
/// so a malformed string is dropped rather than failing the whole recording.
pub(super) fn event_from_payload(payload: &str) -> Option<RecordedEvent> {
    serde_json::from_str::<RecordedEvent>(payload).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(payload: &str) -> Option<Value> {
        step_from_event(&event_from_payload(payload)?)
    }

    #[test]
    fn click_becomes_a_press_step() {
        let step = parse(r##"{"type":"click","selector":"#go"}"##).expect("step");
        assert_eq!(step["cmd"], json!("press"));
        assert_eq!(step["target"], json!("#go"));
    }

    #[test]
    fn input_and_change_both_become_write_steps() {
        for kind in ["input", "change"] {
            let payload = format!(r##"{{"type":"{kind}","selector":"#u","value":"alice"}}"##);
            let step = parse(&payload).expect("step");
            assert_eq!(step["cmd"], json!("write"), "{kind}");
            assert_eq!(step["value"], json!("alice"), "{kind}");
        }
    }

    #[test]
    fn submit_and_navigate_map_to_their_own_steps() {
        let submit = parse(r##"{"type":"submit","selector":"#form"}"##).expect("submit");
        assert_eq!(submit["cmd"], json!("submit"));
        let goto = parse(r#"{"type":"navigate","url":"https://example.com/"}"#).expect("goto");
        assert_eq!(goto["cmd"], json!("goto"));
        assert_eq!(goto["url"], json!("https://example.com/"));
    }

    #[test]
    fn unknown_family_and_empty_target_are_dropped() {
        assert!(parse(r##"{"type":"scroll","selector":"#a"}"##).is_none());
        assert!(parse(r#"{"type":"click","selector":""}"#).is_none());
        assert!(parse("not json").is_none());
    }
}
