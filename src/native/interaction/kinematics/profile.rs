// SPDX-License-Identifier: MIT OR Apache-2.0
//! Process-level input profile, timing shape, and the knobs resolved from them.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;

use crate::constants::DEFAULT_INPUT_TIMING_DISTRIBUTION;

use super::Kinematics;
// Named only by the doc comments below. Importing it keeps the intra-doc links
// resolving against the same path readers see in the rendered page.
#[allow(unused_imports)]
use super::Jitter;

/// Process-level `--input-profile direct` override.
///
/// # Concurrency
///
/// `Relaxed` because this is a standalone preference flag: no other memory is
/// published through it, and it is written once during CLI dispatch before any
/// interaction runs.
static DIRECT_PROFILE: AtomicBool = AtomicBool::new(false);

/// Process-level `--input-seed`, with `0` meaning "unset" (seed from the OS).
///
/// A caller that genuinely wants seed `0` still gets a deterministic run:
/// [`Jitter::from_seed`] maps `0` onto a fixed non-zero state.
static INPUT_SEED: AtomicU64 = AtomicU64::new(0);

/// Publish the resolved `--input-profile`. Called once from CLI dispatch.
pub fn set_input_profile(profile: InputProfile) {
    DIRECT_PROFILE.store(profile == InputProfile::Direct, Ordering::Relaxed);
}

/// Publish the resolved `--input-seed`. Called once from CLI dispatch.
pub fn set_input_seed(seed: Option<u64>) {
    INPUT_SEED.store(seed.unwrap_or(0), Ordering::Relaxed);
}

/// The profile in force for this process.
#[must_use]
pub fn active_profile() -> InputProfile {
    if DIRECT_PROFILE.load(Ordering::Relaxed) {
        InputProfile::Direct
    } else {
        InputProfile::Human
    }
}

/// Kinematics for this process, resolved from the published profile and seed.
///
/// Each call builds a fresh instance so the jitter stream is owned by the caller
/// rather than shared across concurrent interactions, which would make a seeded
/// run non-reproducible under any fan-out.
#[must_use]
pub fn active() -> Kinematics {
    let seed = INPUT_SEED.load(Ordering::Relaxed);
    Kinematics::new(active_profile(), (seed != 0).then_some(seed))
}

/// How input events are shaped before dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputProfile {
    /// Interpolated trajectories, dwell times, and per-character rhythm.
    #[default]
    Human,
    /// One event per logical action, exactly as released before 0.1.8.
    Direct,
}

impl InputProfile {
    /// Parse the `--input-profile` token.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "human" => Some(Self::Human),
            "direct" => Some(Self::Direct),
            _ => None,
        }
    }

    /// Stable wire string for envelopes and schemas.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Direct => "direct",
        }
    }

    /// Whether this profile synthesizes intermediate events.
    #[must_use]
    pub fn is_human(self) -> bool {
        self == Self::Human
    }
}

/// Shape of the dispersion applied around a delay's mean.
///
/// # Why the shape is a knob and the default is log-normal
///
/// A detector measures the SECOND moment because the first is trivial to
/// imitate, and it reads the THIRD to tell a scaled constant from a hand.
/// Measured 2026-08-31 on the final browser event, 20 characters under `human`:
/// n=19, mean 141.26 ms, stddev 20.38 ms, **skewness 0.036**. Human inter-key
/// intervals are asymmetric with a long right tail and a skewness between 1 and
/// 3. Zero skewness does not read as an unusual typist; it reads as no typist.
///
/// [`Self::Normal`] and [`Self::Uniform`] stay reachable so a caller replaying an
/// older trace can get the old shape back, not because either models a hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimingDistribution {
    /// Asymmetric with a long right tail. The default, and the only human shape.
    #[default]
    LogNormal,
    /// Symmetric, bell-shaped. Right width, wrong skew.
    Normal,
    /// Symmetric, flat. The pre-0.1.9 behaviour of [`Jitter::vary_ms`].
    Uniform,
}

impl TimingDistribution {
    /// Parse the `input_timing_distribution` token.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "lognormal" => Some(Self::LogNormal),
            "normal" => Some(Self::Normal),
            "uniform" => Some(Self::Uniform),
            _ => None,
        }
    }

    /// Stable wire string for envelopes and schemas.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LogNormal => "lognormal",
            Self::Normal => "normal",
            Self::Uniform => "uniform",
        }
    }
}

/// Timing a gesture actually applied, for the envelope to publish.
///
/// Declaring a distribution the execution does not run is its own signal: a pair
/// that cannot both be true denounces more than the value it was hiding. These
/// are the numbers that were USED, read back off the resolved kinematics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimingMetrics {
    /// Mean of the per-character delay, in milliseconds.
    pub mean_ms: u64,
    /// Standard deviation asked of that delay, in milliseconds.
    pub stddev_ms: u64,
    /// Shape the samples were drawn from.
    pub distribution: &'static str,
}

/// The shape configured for this process, or the default when unset or bad.
///
/// # Why a `OnceLock` and not a plain read
///
/// `input_timing_distribution` is a STRING key, so it is not in the promoted
/// `policy_knobs!` table and there is no cached `policy_u64` for it -- reading
/// it means `load_config`, which touches the disk. [`Kinematics::new`] runs
/// once per GESTURE, so an uncached read would put file I/O inside the
/// interaction loop, which is the exact cost that struct's own doc comment says
/// it exists to avoid. Resolved once per process instead.
///
/// Falls back rather than failing: `config set` already REFUSES an unparsable
/// token, so a bad value can only come from a hand-edited file, and a one-shot
/// process that declines to move a mouse over a typo in one knob is worse than
/// one that moves it with the documented default.
pub(super) fn configured_distribution() -> TimingDistribution {
    static RESOLVED: OnceLock<TimingDistribution> = OnceLock::new();
    *RESOLVED.get_or_init(|| {
        crate::xdg::load_config()
            .ok()
            .and_then(|cfg| cfg.input_timing_distribution)
            .as_deref()
            .and_then(TimingDistribution::parse)
            .unwrap_or_else(|| {
                TimingDistribution::parse(DEFAULT_INPUT_TIMING_DISTRIBUTION).unwrap_or_default()
            })
    })
}
