// SPDX-License-Identifier: MIT OR Apache-2.0
//! Where a resolved policy value came from.
//!
//! # Why this is shared instead of private to one policy
//!
//! This enum was born inside `stealth.rs` as `ProfileSource`, describing where
//! the active `--stealth-profile` token came from. Nothing about it is specific
//! to stealth: EVERY value this module publishes is resolved by the same three
//! steps the module doc names — flag, then XDG, then the compiled default.
//!
//! Keeping it private to one policy is what let an asymmetry grow. Measured
//! 2026-08-31: `stealth_profile` could say where it came from and `browser_mode`
//! could not. So an operator could prove which stealth profile was chosen and
//! could NOT prove whether the browser ran headless — the one decision here with
//! a security consequence. The difference was never technical. The type simply
//! lived in the wrong file, and a type in the wrong file is a capability the
//! neighbouring code cannot reach.
//!
//! `ProfileSource` stays as an alias, so every existing path keeps resolving.

/// Which of the three precedence steps produced a policy value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicySource {
    /// The compiled default, because nothing overrode it.
    Default,
    /// The XDG config file.
    Xdg,
    /// An explicit flag on argv.
    Flag,
}

impl PolicySource {
    /// Stable token for envelopes.
    ///
    /// These three strings are a wire contract: an agent branches on them to
    /// tell "headless because I asked" from "headless by luck of the default".
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Xdg => "xdg",
            Self::Flag => "flag",
        }
    }

    /// Encode for the process-global atomics this module publishes through.
    pub(super) fn code(self) -> u8 {
        match self {
            Self::Default => 0,
            Self::Xdg => 1,
            Self::Flag => 2,
        }
    }

    /// Decode, treating any unknown code as [`Self::Default`].
    pub(super) fn from_code(code: u8) -> Self {
        match code {
            1 => Self::Xdg,
            2 => Self::Flag,
            _ => Self::Default,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PolicySource;

    #[test]
    fn source_codes_round_trip() {
        for s in [PolicySource::Default, PolicySource::Xdg, PolicySource::Flag] {
            assert_eq!(PolicySource::from_code(s.code()), s);
        }
    }

    /// An unknown code must not invent a provenance nobody recorded.
    #[test]
    fn unknown_code_reads_as_default_not_flag() {
        assert_eq!(PolicySource::from_code(200), PolicySource::Default);
    }

    #[test]
    fn tokens_are_the_published_spelling() {
        assert_eq!(PolicySource::Default.as_str(), "default");
        assert_eq!(PolicySource::Xdg.as_str(), "xdg");
        assert_eq!(PolicySource::Flag.as_str(), "flag");
    }
}
