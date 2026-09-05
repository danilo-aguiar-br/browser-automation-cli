// SPDX-License-Identifier: MIT OR Apache-2.0
//! Explicit Target Designation: publish WHAT a side-effecting verb acted on.
//!
//! # Why this module exists
//!
//! A verb with a side effect must receive its target in argv, never inherit one
//! from ambient state, and its arity must never carry semantics. That rule is
//! unenforceable from the outside unless the envelope says which target the
//! process actually resolved and where that value came from.
//!
//! Measured 2026-08-18: exactly ONE command in the whole product published those
//! two fields (`config unset`). Every other side-effecting verb returned an
//! envelope from which an ambient-target violation was indistinguishable from an
//! explicit one — `page close` with no `--index`, `cookie clear` with no scope,
//! `keys` typing into whatever held focus, `sg-rewrite --apply` defaulting its
//! path list to `.`.
//!
//! Publishing is not cosmetic. It is what makes the rule testable: a gate can
//! assert that a verb refused, or that it acted on an argv-sourced target, only
//! if the envelope carries the evidence.

use serde_json::Value;

/// Where the target a verb acted on came from.
///
/// The variants are ordered from most explicit to least. Anything below
/// [`Self::Argv`] is a claim the caller should be able to audit, which is the
/// whole point of emitting it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetSource {
    /// Named directly on the command line, as a positional or a flag value.
    Argv,
    /// Read from a `run` / `exec` step field rather than from process argv.
    Step,
    /// Resolved from an XDG configuration key.
    Xdg,
    /// Derived from process state the caller did not name — the shape the ETD
    /// rule exists to make visible. A verb that reports this is telling the
    /// caller it guessed.
    Ambient,
}

impl TargetSource {
    /// Stable envelope token. These strings are machine contract, not prose, and
    /// are therefore never localised.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Argv => "argv",
            Self::Step => "step",
            Self::Xdg => "xdg",
            Self::Ambient => "ambient",
        }
    }
}

/// Attach `target_resolved` and `target_source` to a verb's data object.
///
/// A non-object `data` is returned untouched: the two fields describe a record,
/// and wrapping a bare array or scalar to carry them would change the shape the
/// command documents, which is a worse outcome than the missing annotation.
#[must_use]
pub fn with_target(mut data: Value, resolved: &str, source: TargetSource) -> Value {
    if let Some(obj) = data.as_object_mut() {
        obj.insert("target_resolved".into(), Value::from(resolved));
        obj.insert("target_source".into(), Value::from(source.as_str()));
    }
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tokens_are_stable() {
        assert_eq!(TargetSource::Argv.as_str(), "argv");
        assert_eq!(TargetSource::Step.as_str(), "step");
        assert_eq!(TargetSource::Xdg.as_str(), "xdg");
        assert_eq!(TargetSource::Ambient.as_str(), "ambient");
    }

    #[test]
    fn an_object_gains_both_fields() {
        let out = with_target(json!({"ok": true}), "#user", TargetSource::Argv);
        assert_eq!(out["target_resolved"], "#user");
        assert_eq!(out["target_source"], "argv");
        assert_eq!(out["ok"], true);
    }

    #[test]
    fn a_non_object_is_returned_unchanged_rather_than_reshaped() {
        let out = with_target(json!([1, 2, 3]), "x", TargetSource::Argv);
        assert_eq!(out, json!([1, 2, 3]));
    }

    #[test]
    fn ambient_is_expressible_because_hiding_it_would_defeat_the_gate() {
        let out = with_target(json!({}), "active tab", TargetSource::Ambient);
        assert_eq!(out["target_source"], "ambient");
    }
}
