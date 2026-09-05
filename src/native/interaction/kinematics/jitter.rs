// SPDX-License-Identifier: MIT OR Apache-2.0
//! The seedable noise source and the truncation that keeps its draws on the wire.

use crate::constants::{
    TIMING_MIN_DISPERSION_RATIO, TIMING_SAMPLE_CEILING_RATIO, TIMING_SAMPLE_FLOOR_RATIO,
};

use super::TimingDistribution;

/// Small xorshift64* generator.
///
/// Deliberately not a crypto RNG and deliberately not a new dependency: the only
/// requirement is a cheap, seedable, reproducible jitter source. Seeding it from
/// `--input-seed` makes a `human` run byte-reproducible, which is what lets the
/// event-trace tests assert on jitter without becoming flaky.
#[derive(Debug, Clone)]
pub struct Jitter {
    state: u64,
}

impl Jitter {
    /// Seed explicitly, for reproducible runs.
    #[must_use]
    pub fn from_seed(seed: u64) -> Self {
        // A zero state is a fixed point of xorshift; nudge it off zero.
        Self {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    /// Seed from the OS, for ordinary runs.
    #[must_use]
    pub fn from_entropy() -> Self {
        let mut buf = [0_u8; 8];
        // A failed OS draw must not abort input; a fixed fallback seed only
        // costs reproducibility across runs, which is not a safety property.
        let seed = match getrandom::getrandom(&mut buf) {
            Ok(()) => u64::from_le_bytes(buf),
            Err(_) => 0x2545_F491_4F6C_DD1D,
        };
        Self::from_seed(seed)
    }

    /// Next raw value.
    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform value in `[0.0, 1.0)`.
    pub fn unit(&mut self) -> f64 {
        // 53 bits is the exact mantissa width of f64, so every draw is representable.
        (self.next_u64() >> 11) as f64 / (1_u64 << 53) as f64
    }

    /// Uniform value in `[-radius, radius]`.
    pub fn signed(&mut self, radius: f64) -> f64 {
        (self.unit() * 2.0 - 1.0) * radius
    }

    /// Scale `base` by a factor in `[1 - spread, 1 + spread]`, never below 1.
    ///
    /// The [`TimingDistribution::Uniform`] arm of [`Self::sample_ms`]. Kept, and
    /// no longer the default: symmetric noise reproduces the WIDTH of human
    /// timing and none of its SHAPE, and the third moment reads the difference
    /// off directly.
    pub fn vary_ms(&mut self, base: u64, spread: f64) -> u64 {
        if base == 0 {
            return 0;
        }
        let scaled = base as f64 * (1.0 + self.signed(spread));
        scaled.max(1.0).round() as u64
    }

    /// One draw from the standard normal, by Box-Muller.
    ///
    /// Box-Muller rather than a sum of uniforms: the tail is the entire point of
    /// this module, and the tail is exactly what the central-limit shortcut gets
    /// wrong. Both uniforms are consumed on every call and the transform's
    /// second output is discarded rather than cached, so the number of draws per
    /// sample is fixed and a seeded run stays reproducible.
    ///
    /// # Why not `rand_distr`, decided 2026-08-31
    ///
    /// The project rule is to prefer a published crate over a hand-rolled
    /// implementation, so this needs a stated reason rather than silence.
    /// `rand_distr` 0.6.0 exists and does expose `Normal` and `LogNormal`
    /// (verified on docs.rs), so availability is not the argument.
    ///
    /// The argument is the property stated above: `--input-seed` promises that
    /// one seed reproduces one run, and that holds only while the number of
    /// draws taken from the generator is a function of the CALL SEQUENCE and
    /// never of the VALUES drawn. Box-Muller consumes exactly two uniforms per
    /// call, unconditionally. Any sampler that rejects and redraws — the usual
    /// shape for a fast normal — consumes a count that depends on what it drew,
    /// so two runs on the same seed desynchronise at the first rejection and
    /// every later draw differs.
    ///
    /// That is a property of THIS caller, not a defect in the crate. Adopting
    /// it would mean either pinning an internal algorithm the crate does not
    /// promise to keep, or giving up the reproducibility guarantee.
    pub fn normal01(&mut self) -> f64 {
        // `unit()` is half-open on [0, 1), so `u1` can be exactly zero and
        // `ln(0)` is negative infinity. Nudging onto the smallest positive f64
        // bounds the draw without REJECTING it: a rejection loop would consume
        // an unpredictable number of values and break reproducibility.
        let u1 = self.unit().max(f64::MIN_POSITIVE);
        let u2 = self.unit();
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }

    /// One log-normal draw with arithmetic mean `mean_ms` and standard deviation
    /// `stddev_ms`, truncated onto the configured floor and ceiling of the mean.
    ///
    /// The parameters are the ARITHMETIC moments and not the log-space `mu` and
    /// `sigma`, because the moment an operator can measure in the browser is the
    /// one they should be able to set.
    pub fn lognormal_ms(&mut self, mean_ms: u64, stddev_ms: u64) -> u64 {
        if mean_ms == 0 {
            return 0;
        }
        // A DECLARED floor, not an accidental one. `stddev_ms == 0` is variance
        // zero, and variance zero is a STRONGER signal than a wrong mean: a
        // wrong mean reads as an unusual human, no variance reads as no human.
        // Today zero is unreachable because every default is non-zero and
        // `policy_u64` filters `n > 0` -- but that is protection by
        // circumstance, and the first refactor that changes either would open a
        // silent hole. Clamping here makes the guarantee local to the sampler
        // that needs it, so it survives whatever the callers become.
        let stddev_ms = stddev_ms.max(minimum_stddev_ms(mean_ms));
        let mean = mean_ms as f64;
        let cv = stddev_ms as f64 / mean;
        let sigma_sq = (1.0 + cv * cv).ln();
        let mu = mean.ln() - sigma_sq / 2.0;
        truncate_ms((mu + sigma_sq.sqrt() * self.normal01()).exp(), mean)
    }

    /// One normal draw with the given moments, truncated like
    /// [`Self::lognormal_ms`].
    pub fn normal_ms(&mut self, mean_ms: u64, stddev_ms: u64) -> u64 {
        if mean_ms == 0 {
            return 0;
        }
        // Same declared floor as [`Self::lognormal_ms`]; see the comment there.
        let stddev_ms = stddev_ms.max(minimum_stddev_ms(mean_ms));
        let mean = mean_ms as f64;
        truncate_ms(stddev_ms as f64 * self.normal01() + mean, mean)
    }

    /// One draw of `mean_ms` under `distribution`, dispersed by `stddev_ms`.
    pub fn sample_ms(
        &mut self,
        distribution: TimingDistribution,
        mean_ms: u64,
        stddev_ms: u64,
    ) -> u64 {
        match distribution {
            TimingDistribution::LogNormal => self.lognormal_ms(mean_ms, stddev_ms),
            TimingDistribution::Normal => self.normal_ms(mean_ms, stddev_ms),
            TimingDistribution::Uniform => {
                if mean_ms == 0 {
                    return 0;
                }
                // A uniform of standard deviation `s` has half-width
                // `s * sqrt(3)`, so one knob means one dispersion under every
                // shape and switching shape does not silently rescale it.
                let spread = stddev_ms as f64 * 3.0_f64.sqrt() / mean_ms as f64;
                self.vary_ms(mean_ms, spread)
            }
        }
    }
}

/// Clamp a raw sample onto the configured floor and ceiling of its mean.
///
/// A log-normal has unbounded support, so an unclamped draw lands anywhere from
/// a millisecond to a minute. Rounding is to the nearest millisecond and to
/// nothing coarser: quantizing onto a step would rebuild the exact grid the
/// dispersion exists to break, and a millisecond is the wire's own resolution
/// rather than one this module chose.
/// Smallest dispersion a delay of `mean_ms` is allowed to carry.
///
/// Enforces [`TIMING_MIN_DISPERSION_RATIO`] so no configuration and no future
/// caller can request a delay with variance zero, which is the one shape that
/// says "machine" outright. Never below 1 ms for a non-zero mean, because the
/// wire cannot express less and rounding would silently restore the constant.
fn minimum_stddev_ms(mean_ms: u64) -> u64 {
    if mean_ms == 0 {
        return 0;
    }
    ((mean_ms as f64 * TIMING_MIN_DISPERSION_RATIO).round() as u64).max(1)
}

fn truncate_ms(sample: f64, mean: f64) -> u64 {
    let floor = (mean * TIMING_SAMPLE_FLOOR_RATIO).max(1.0);
    sample
        .clamp(floor, mean * TIMING_SAMPLE_CEILING_RATIO)
        .round() as u64
}
