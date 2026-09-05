// SPDX-License-Identifier: MIT OR Apache-2.0
//! Where each fingerprint signal in the `doctor --fingerprint` envelope came
//! from: the host, or the anti-detection patch.
//!
//! # Why this type exists
//!
//! `doctor --fingerprint` used to spell three provenance fields as literals:
//! `"gpu_source": "host"`, `"fonts_source": "host"`, `"audio_source": "host"`.
//! Two of them happened to be true. The GPU one was false on the default path.
//!
//! Measured 2026-08-17 on this host. Under stealth the probe reported vendor
//! `Google Inc. (NVIDIA Corporation)` with renderer `NVIDIA GeForce GTX 1070`;
//! with `--no-stealth` the same probe reported `Google Inc. (Google)` with an
//! ANGLE SwiftShader renderer; the machine itself has an RTX 3060. So neither
//! the reported GPU nor its claimed driver came from the host, while the field
//! next to it said `host` and the field above it said `stealth: true`.
//!
//! The literal was not careless. It was the downstream consequence of a comment
//! in [`super::build_script`] asserting that `Fingerprint::NativeGPU` reports
//! the real adapter. It does not: the WebGL patch rides on the TIER, and
//! `Tier::BasicWithConsole` installs `unified_worker_override`, which rewrites
//! `getParameter(37445/37446)` on the main window.
//!
//! A label that is right by accident is debt. `audio_source` and `fonts_source`
//! still answer `host` today, but they answer it by ASKING rather than by
//! assertion, so the day an audio patch lands the label follows without anyone
//! remembering to edit a string.

/// Whether a fingerprint signal reflects the host or the anti-detection patch.
///
/// Deliberately not `Serialize`. The envelope token is produced by
/// [`SignalSource::as_str`] at the point of emission, exactly like
/// [`super::ScreenSource`], so renaming a variant can never rename a field the
/// agents parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalSource {
    /// The value is whatever the browser and the machine actually produce.
    Host,
    /// The value is synthesised by the stealth patch and does not describe this
    /// machine.
    Stealth,
}

impl SignalSource {
    /// Stable envelope token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Host => "host",
            Self::Stealth => "stealth",
        }
    }

    /// Resolve a signal's provenance for THIS process.
    ///
    /// `stealth_active` is the process-wide switch published by
    /// [`crate::browser_policy::stealth_enabled`]. `patched` says whether the
    /// installed patch touches this particular signal at all; a signal the
    /// patch never rewrites stays `Host` even with stealth on.
    #[must_use]
    pub const fn resolve(stealth_active: bool, patched: bool) -> Self {
        if stealth_active && patched {
            Self::Stealth
        } else {
            Self::Host
        }
    }
}

/// Provenance of every signal the fingerprint envelope labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalSources {
    /// WebGL vendor and renderer.
    pub gpu: SignalSource,
    /// `AudioContext.sampleRate`.
    pub audio: SignalSource,
    /// `document.fonts.size`.
    pub fonts: SignalSource,
}

/// Resolve provenance for the whole signal set.
///
/// The `patched` answers come from [`crate::constants::STEALTH_CONTROLLED_SIGNALS`],
/// which is the single list of what the installed patch actually rewrites.
///
/// NOT derived from `STEALTH_SEED_FIELDS`. That constant answers a different
/// question — what varies WITH THE SEED — and conflating the two would create a
/// second honesty bug: `userAgent` sits in `STEALTH_SEED_DOES_NOT_VARY` and is
/// spoofed all the same.
#[must_use]
pub fn signal_sources(stealth_active: bool) -> SignalSources {
    use crate::constants::{
        stealth_controls, SIGNAL_AUDIO_SAMPLE_RATE, SIGNAL_FONT_COUNT, SIGNAL_GPU,
    };
    SignalSources {
        gpu: SignalSource::resolve(stealth_active, stealth_controls(SIGNAL_GPU)),
        audio: SignalSource::resolve(stealth_active, stealth_controls(SIGNAL_AUDIO_SAMPLE_RATE)),
        fonts: SignalSource::resolve(stealth_active, stealth_controls(SIGNAL_FONT_COUNT)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tokens are a wire contract. Renaming a variant must not rename them.
    #[test]
    fn signal_source_tokens_are_stable() {
        assert_eq!(SignalSource::Host.as_str(), "host");
        assert_eq!(SignalSource::Stealth.as_str(), "stealth");
    }

    #[test]
    fn an_unpatched_signal_stays_host_even_under_stealth() {
        assert_eq!(SignalSource::resolve(true, false), SignalSource::Host);
        assert_eq!(SignalSource::resolve(false, false), SignalSource::Host);
    }

    #[test]
    fn a_patched_signal_is_stealth_only_while_stealth_is_on() {
        assert_eq!(SignalSource::resolve(true, true), SignalSource::Stealth);
        assert_eq!(SignalSource::resolve(false, true), SignalSource::Host);
    }

    /// The regression this whole module exists to prevent: GPU labelled `host`
    /// while the patch that rewrites it is installed.
    #[test]
    fn gpu_is_never_labelled_host_while_stealth_is_active() {
        assert_eq!(signal_sources(true).gpu, SignalSource::Stealth);
        assert_eq!(signal_sources(false).gpu, SignalSource::Host);
    }

    /// Audio and fonts are measured as genuinely host-backed. If a future patch
    /// starts rewriting them, this test fails and forces the table to be told.
    #[test]
    fn audio_and_fonts_are_host_backed_in_both_modes() {
        for active in [true, false] {
            let s = signal_sources(active);
            assert_eq!(s.audio, SignalSource::Host);
            assert_eq!(s.fonts, SignalSource::Host);
        }
    }
}
