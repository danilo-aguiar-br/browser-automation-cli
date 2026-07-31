// SPDX-License-Identifier: MIT OR Apache-2.0
//! Durable role+name locators (GAP-034 pillar 2).
//!
//! # Why `@eN` is not enough
//!
//! A snapshot ref is an **ordinal minted by this process**: `@e7` means "the
//! seventh node this snapshot numbered". It dies with the process and it moves
//! when the page changes, so it cannot be written down, stored in a script, or
//! handed to the next invocation. That is what makes the exported storage state
//! only half a solution: the caller restores the session and still has no way
//! to point at anything.
//!
//! # What a durable locator is
//!
//! The accessible **role** and **name** are properties of the element itself,
//! not of a numbering pass, so they survive a new process. The wire form is:
//!
//! ```text
//! role=button[name="Sign in"]
//! role=textbox[name="Email"][2]     // second match, when the pair repeats
//! ```
//!
//! # Ambiguity is explicit, never silent
//!
//! When a role+name pair matches more than one node the locator carries a
//! 1-based `[n]`. Emitting a bare locator for an ambiguous pair would resolve
//! to a different element after any page change, which is the exact failure
//! `@eN` already has.

mod assign;
mod parse;

#[cfg(test)]
mod tests;

pub use assign::assign_locators;

use parse::{read_bracket, unquote};

use serde::{Deserialize, Serialize};

/// A role+name locator that survives across processes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableLocator {
    /// Accessible role (`button`, `textbox`, `link`, …).
    pub role: String,
    /// Accessible name as exposed to assistive technology.
    pub name: String,
    /// 1-based disambiguator; `None` when the pair is unique in the snapshot.
    pub nth: Option<usize>,
}

impl DurableLocator {
    /// Build a locator.
    ///
    /// `nth` is the caller's decision, not a heuristic here: `[1]` is noise for
    /// a unique pair but REQUIRED for the first of a repeated pair, and only
    /// the caller knows which. [`assign_locators`] makes that call per snapshot.
    pub fn new(role: impl Into<String>, name: impl Into<String>, nth: Option<usize>) -> Self {
        Self {
            role: role.into(),
            name: name.into(),
            nth,
        }
    }

    /// Wire form: `role=button[name="Sign in"]` with optional `[n]`.
    pub fn to_wire(&self) -> String {
        let escaped = self.name.replace('\\', "\\\\").replace('"', "\\\"");
        match self.nth {
            Some(n) => format!("role={}[name=\"{}\"][{}]", self.role, escaped, n),
            None => format!("role={}[name=\"{}\"]", self.role, escaped),
        }
    }

    /// True when `input` looks like a durable locator rather than CSS or `@eN`.
    pub fn looks_like(input: &str) -> bool {
        input.trim_start().starts_with("role=")
    }

    /// Parse the wire form; `None` when `input` is not a durable locator.
    pub fn parse(input: &str) -> Option<Self> {
        let rest = input.trim().strip_prefix("role=")?;
        let (role, rest) = match rest.find('[') {
            Some(i) => (&rest[..i], &rest[i..]),
            // `role=button` with no name is still a valid, broader locator,
            // but `role=` alone addresses nothing.
            None => {
                let bare = rest.trim();
                if bare.is_empty() {
                    return None;
                }
                return Some(Self::new(bare, "", None));
            }
        };
        let role = role.trim();
        if role.is_empty() {
            return None;
        }

        let mut name = String::new();
        let mut nth = None;
        let mut cursor = rest;
        while let Some(open) = cursor.find('[') {
            let body_start = open + 1;
            let (body, next) = read_bracket(&cursor[body_start..])?;
            if let Some(raw) = body.strip_prefix("name=") {
                name = unquote(raw);
            } else if let Ok(n) = body.trim().parse::<usize>() {
                nth = Some(n);
            } else {
                return None;
            }
            cursor = next;
        }
        if !cursor.trim().is_empty() {
            return None;
        }
        Some(Self::new(role, name, nth))
    }

    /// Index into a candidate list (0-based); `nth` is 1-based on the wire.
    pub fn candidate_index(&self) -> usize {
        self.nth.map(|n| n.saturating_sub(1)).unwrap_or(0)
    }

    /// True when this locator addresses `(role, name)`.
    pub fn matches(&self, role: &str, name: &str) -> bool {
        if !self.role.eq_ignore_ascii_case(role) {
            return false;
        }
        // An empty name in the locator means "any name with this role".
        self.name.is_empty() || self.name == name
    }
}
