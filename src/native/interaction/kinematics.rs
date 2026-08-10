// SPDX-License-Identifier: MIT OR Apache-2.0
//! Pointer and keyboard kinematics: geometry and timing, with no CDP in sight.
//!
//! **Pure by design.** Every function here maps numbers to numbers, so the whole
//! module is unit-testable without a browser. The dispatch modules
//! ([`super::pointer`], [`super::scroll`], [`super::keyboard`]) own the wire; this
//! module owns *where* and *when*.
//!
//! # Why it exists
//!
//! The interpolation loop lived in `drag_html5` alone, where it was written to
//! fix one bug: a single mouse hop skipped Chrome's drag threshold. The comment
//! left behind recorded the mechanism -- the renderer coalesces back-to-back
//! moves into one hop -- but the fix stayed scoped to `drag`, so `press`,
//! `hover`, `scroll`, and `type` kept dispatching input no hand could produce.
//!
//! # Profiles
//!
//! [`InputProfile::Direct`] reproduces the pre-0.1.8 dispatch byte for byte, for
//! callers that want speed or exact determinism. [`InputProfile::Human`] is the
//! default and costs real time: a click grows from three CDP calls to roughly
//! `input_move_steps + 3`.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::xdg::policy::{key, policy_u64, policy_usize};

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
    /// Used for per-character and per-move delays: a constant interval is itself
    /// a signature, because human timing is never uniform.
    pub fn vary_ms(&mut self, base: u64, spread: f64) -> u64 {
        if base == 0 {
            return 0;
        }
        let scaled = base as f64 * (1.0 + self.signed(spread));
        scaled.max(1.0).round() as u64
    }
}

/// Fraction of a delay that varies run to run.
const DELAY_SPREAD: f64 = 0.35;

/// Fraction of the travel distance used to bow the Bezier control points.
const CURVE_BOW: f64 = 0.18;

/// Tunables resolved once per process from XDG, then reused.
///
/// Read once because every getter hits the config resolver; a per-event read
/// would put file I/O inside the interaction loop.
#[derive(Debug, Clone)]
pub struct Kinematics {
    profile: InputProfile,
    jitter: Jitter,
    move_steps: usize,
    move_gap_ms: u64,
    click_dwell_ms: u64,
    key_dwell_ms: u64,
    type_delay_ms: u64,
    scroll_tick_px: f64,
    max_scroll_ticks: usize,
    target_jitter_px: f64,
}

impl Kinematics {
    /// Resolve tunables from XDG for `profile`, seeding jitter from `seed`.
    #[must_use]
    pub fn new(profile: InputProfile, seed: Option<u64>) -> Self {
        Self {
            profile,
            jitter: seed.map_or_else(Jitter::from_entropy, Jitter::from_seed),
            move_steps: policy_usize(key::INPUT_MOVE_STEPS).max(1),
            move_gap_ms: policy_u64(key::INPUT_MOVE_GAP_MS),
            click_dwell_ms: policy_u64(key::INPUT_CLICK_DWELL_MS),
            key_dwell_ms: policy_u64(key::INPUT_KEY_DWELL_MS),
            type_delay_ms: policy_u64(key::INPUT_TYPE_DELAY_MS),
            scroll_tick_px: policy_u64(key::INPUT_SCROLL_TICK_PX).max(1) as f64,
            max_scroll_ticks: policy_usize(key::INPUT_SCROLL_MAX_TICKS).max(1),
            target_jitter_px: policy_u64(key::INPUT_TARGET_JITTER_PX) as f64,
        }
    }

    /// Override the move budget with a route-specific pair of knobs.
    ///
    /// Used by the HTML5 drag route, whose `drag_move_steps` / `drag_move_gap_ms`
    /// predate the generalized `input_move_*` keys and are still documented and
    /// still settable. Sharing ONE trajectory generator must not turn an
    /// operator's existing configuration into a silent no-op: a knob that reads
    /// fine and changes nothing is worse than a knob that was removed, because
    /// nothing tells the operator it stopped working.
    ///
    /// The generator, the profile and the jitter stay shared; only the budget is
    /// route-local, which is what those two knobs always meant.
    #[must_use]
    pub fn with_move_budget(mut self, steps: usize, gap_ms: u64) -> Self {
        self.move_steps = steps.max(1);
        self.move_gap_ms = gap_ms;
        self
    }

    /// The active profile.
    #[must_use]
    pub fn profile(&self) -> InputProfile {
        self.profile
    }

    /// Pause between synthesized positions, varied.
    pub fn move_gap_ms(&mut self) -> u64 {
        self.jitter.vary_ms(self.move_gap_ms, DELAY_SPREAD)
    }

    /// Hold time between press and release, varied.
    pub fn click_dwell_ms(&mut self) -> u64 {
        if self.profile == InputProfile::Direct {
            return 0;
        }
        self.jitter.vary_ms(self.click_dwell_ms, DELAY_SPREAD)
    }

    /// Hold time between `keyDown` and `keyUp`, varied.
    pub fn key_dwell_ms(&mut self) -> u64 {
        if self.profile == InputProfile::Direct {
            return 0;
        }
        self.jitter.vary_ms(self.key_dwell_ms, DELAY_SPREAD)
    }

    /// Delay before the next character, varied.
    ///
    /// `explicit` wins when the caller passed `--delay-ms`, so an operator who
    /// asks for a rhythm gets exactly that rhythm, jittered.
    pub fn type_delay_ms(&mut self, explicit: Option<u64>) -> u64 {
        match (self.profile, explicit) {
            (_, Some(ms)) => self.jitter.vary_ms(ms, DELAY_SPREAD),
            (InputProfile::Direct, None) => 0,
            (InputProfile::Human, None) => self.jitter.vary_ms(self.type_delay_ms, DELAY_SPREAD),
        }
    }

    /// Offset a target point so repeated clicks never share one pixel.
    pub fn jitter_target(&mut self, x: f64, y: f64) -> (f64, f64) {
        if self.profile == InputProfile::Direct || self.target_jitter_px <= 0.0 {
            return (x, y);
        }
        let r = self.target_jitter_px;
        (x + self.jitter.signed(r), y + self.jitter.signed(r))
    }

    /// Intermediate positions from `from` to `to`, excluding `from`, including `to`.
    ///
    /// `Direct` yields the destination alone, which is the pre-0.1.8 behaviour.
    /// `Human` yields a cubic Bezier with bowed control points and an ease-in-out
    /// velocity profile, so the path is neither straight nor uniformly paced --
    /// a straight line at constant speed is itself a synthetic signature.
    ///
    /// Step count scales with distance in the spirit of Fitts's law: a short hop
    /// gets few samples, a long sweep gets the full budget.
    pub fn path(&mut self, from: (f64, f64), to: (f64, f64)) -> Vec<(f64, f64)> {
        if self.profile == InputProfile::Direct {
            return vec![to];
        }
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        let distance = dx.hypot(dy);
        if distance < f64::EPSILON {
            return vec![to];
        }

        let steps = self.steps_for(distance);
        // Bow the control points perpendicular to travel: a hand overshoots the
        // axis it is not correcting, it does not ride the straight line.
        let (nx, ny) = (-dy / distance, dx / distance);
        let bow = distance * CURVE_BOW;
        let c1 = (
            from.0 + dx * 0.3 + nx * self.jitter.signed(bow),
            from.1 + dy * 0.3 + ny * self.jitter.signed(bow),
        );
        let c2 = (
            from.0 + dx * 0.7 + nx * self.jitter.signed(bow),
            from.1 + dy * 0.7 + ny * self.jitter.signed(bow),
        );

        (1..=steps)
            .map(|step| {
                let t = ease_in_out(step as f64 / steps as f64);
                cubic_bezier(from, c1, c2, to, t)
            })
            .collect()
    }

    /// Sample count for a travel distance, capped by the configured budget.
    fn steps_for(&self, distance: f64) -> usize {
        // One sample per ~24 px keeps short hops cheap; the knob is the ceiling,
        // never the floor, so a 4 px nudge does not pay for 24 round trips.
        let scaled = (distance / 24.0).ceil() as usize;
        scaled.clamp(2, self.move_steps)
    }

    /// Split a scroll delta into wheel ticks of at most `input_scroll_tick_px`.
    ///
    /// Both axes travel together in every tick: CDP rejects a `mouseWheel` that
    /// carries only one delta, so emitting `deltaY` alone silently does nothing.
    /// `Direct` returns a single tick carrying the whole delta.
    pub fn wheel_ticks(&mut self, delta_x: f64, delta_y: f64) -> Vec<(f64, f64)> {
        if delta_x == 0.0 && delta_y == 0.0 {
            return Vec::new();
        }
        if self.profile == InputProfile::Direct {
            return vec![(delta_x, delta_y)];
        }
        let span = delta_x.abs().max(delta_y.abs());
        // Capped, because every tick is a CDP round trip.
        //
        // Uncapped, the count is `span / scroll_tick_px`, so the COST of a
        // scroll grew linearly with the distance requested: a 100000 px delta
        // asked for 1000 round trips and ran the command out of time. The
        // caller then reads a timeout and blames the page.
        //
        // The split below is proportional, so a capped count hands each tick
        // more pixels and the total travel is unchanged. Only the granularity
        // of the gesture degrades, and only past the ceiling.
        let ticks =
            ((span / self.scroll_tick_px).ceil().max(1.0) as usize).min(self.max_scroll_ticks);
        (1..=ticks)
            .map(|i| {
                let done = (i - 1) as f64 / ticks as f64;
                let next = i as f64 / ticks as f64;
                (delta_x * (next - done), delta_y * (next - done))
            })
            .collect()
    }
}

/// Ease-in-out on `t` in `[0, 1]`: slow start, fast middle, slow arrival.
///
/// Constant velocity is the tell that a straight line alone does not give away:
/// a real pointer accelerates away from rest and decelerates onto the target.
fn ease_in_out(t: f64) -> f64 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

/// Cubic Bezier point at `t`.
fn cubic_bezier(
    p0: (f64, f64),
    p1: (f64, f64),
    p2: (f64, f64),
    p3: (f64, f64),
    t: f64,
) -> (f64, f64) {
    let u = 1.0 - t;
    let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    (
        a * p0.0 + b * p1.0 + c * p2.0 + d * p3.0,
        a * p0.1 + b * p1.1 + c * p2.1 + d * p3.1,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn human(seed: u64) -> Kinematics {
        Kinematics::new(InputProfile::Human, Some(seed))
    }

    #[test]
    fn direct_profile_reproduces_single_hop() {
        let mut k = Kinematics::new(InputProfile::Direct, Some(1));
        assert_eq!(k.path((0.0, 0.0), (100.0, 100.0)), vec![(100.0, 100.0)]);
        assert_eq!(k.wheel_ticks(0.0, 400.0), vec![(0.0, 400.0)]);
        assert_eq!(k.click_dwell_ms(), 0);
        assert_eq!(k.key_dwell_ms(), 0);
        assert_eq!(k.type_delay_ms(None), 0);
        assert_eq!(k.jitter_target(10.0, 20.0), (10.0, 20.0));
    }

    #[test]
    fn human_path_lands_exactly_on_target() {
        let mut k = human(42);
        let path = k.path((0.0, 0.0), (300.0, 200.0));
        let last = *path.last().expect("non-empty path");
        assert!((last.0 - 300.0).abs() < 1e-9, "x drifted: {last:?}");
        assert!((last.1 - 200.0).abs() < 1e-9, "y drifted: {last:?}");
    }

    #[test]
    fn human_path_leaves_the_straight_line() {
        let mut k = human(7);
        let path = k.path((0.0, 0.0), (400.0, 0.0));
        // A straight horizontal drag would keep y at exactly zero throughout.
        assert!(
            path.iter().any(|(_, y)| y.abs() > 0.5),
            "path never bowed off the axis: {path:?}"
        );
    }

    #[test]
    fn human_path_is_not_uniformly_paced() {
        let mut k = human(11);
        let path = k.path((0.0, 0.0), (500.0, 0.0));
        let gaps: Vec<f64> = path.windows(2).map(|w| w[1].0 - w[0].0).collect();
        let first = gaps.first().copied().unwrap_or_default();
        assert!(
            gaps.iter().any(|g| (g - first).abs() > 1.0),
            "constant velocity is a synthetic signature: {gaps:?}"
        );
    }

    #[test]
    fn short_hop_costs_fewer_samples_than_long_sweep() {
        let mut k = human(3);
        let short = k.path((0.0, 0.0), (5.0, 0.0)).len();
        let long = k.path((0.0, 0.0), (900.0, 0.0)).len();
        assert!(short < long, "short={short} long={long}");
        assert!(short >= 2, "even a nudge needs an intermediate move");
    }

    #[test]
    fn wheel_ticks_split_and_conserve_the_delta() {
        let mut k = human(5);
        let ticks = k.wheel_ticks(0.0, 400.0);
        assert!(ticks.len() >= 3, "400px must not be one giant tick");
        let total: f64 = ticks.iter().map(|(_, dy)| dy).sum();
        assert!((total - 400.0).abs() < 1e-9, "delta lost: {total}");
    }

    #[test]
    fn wheel_ticks_carry_both_axes() {
        let mut k = human(5);
        // CDP drops a mouseWheel that supplies only one delta, so a zero on the
        // idle axis must still be present rather than omitted.
        let ticks = k.wheel_ticks(120.0, 240.0);
        let (sx, sy): (f64, f64) = ticks
            .iter()
            .fold((0.0, 0.0), |(ax, ay), (x, y)| (ax + x, ay + y));
        assert!((sx - 120.0).abs() < 1e-9 && (sy - 240.0).abs() < 1e-9);
    }

    #[test]
    fn a_huge_delta_does_not_buy_a_huge_number_of_round_trips() {
        // Every tick is one CDP round trip, so an uncapped split made the COST
        // of a scroll grow with the distance asked for: 100000 px was 1000 round
        // trips and ran the command out of time. The caller then reads a timeout
        // and blames the page for a cost the client chose.
        let mut k = human(5);
        let ticks = k.wheel_ticks(0.0, 100_000.0);
        assert!(
            ticks.len() <= crate::constants::INPUT_SCROLL_MAX_TICKS as usize,
            "{} ticks for one gesture; the ceiling is not holding",
            ticks.len()
        );
        // Capping the COUNT must not cost travel: the split is proportional, so
        // the ticks simply carry more each. A cap that silently scrolled less
        // would be a worse bug than the latency it fixes.
        let total: f64 = ticks.iter().map(|(_, dy)| dy).sum();
        assert!((total - 100_000.0).abs() < 1e-6, "delta lost: {total}");
    }

    #[test]
    fn zero_scroll_produces_no_ticks() {
        let mut k = human(5);
        assert!(k.wheel_ticks(0.0, 0.0).is_empty());
    }

    #[test]
    fn same_seed_reproduces_the_same_path() {
        let a = human(99).path((0.0, 0.0), (250.0, 130.0));
        let b = human(99).path((0.0, 0.0), (250.0, 130.0));
        assert_eq!(a, b, "--input-seed must make a human run reproducible");
    }

    #[test]
    fn different_seeds_diverge() {
        let a = human(1).path((0.0, 0.0), (250.0, 130.0));
        let b = human(2).path((0.0, 0.0), (250.0, 130.0));
        assert_ne!(a, b);
    }

    #[test]
    fn target_jitter_moves_the_contact_point() {
        let mut k = human(13);
        let a = k.jitter_target(100.0, 100.0);
        let b = k.jitter_target(100.0, 100.0);
        assert_ne!(a, b, "repeated clicks must not share one pixel");
    }

    #[test]
    fn explicit_type_delay_wins_over_the_profile() {
        let mut k = Kinematics::new(InputProfile::Direct, Some(4));
        // Direct suppresses the default rhythm but must still honour an operator
        // who asked for one explicitly.
        assert!(k.type_delay_ms(Some(50)) > 0);
    }

    #[test]
    fn profile_tokens_round_trip() {
        for p in [InputProfile::Human, InputProfile::Direct] {
            assert_eq!(InputProfile::parse(p.as_str()), Some(p));
        }
        assert_eq!(InputProfile::parse("bezier"), None);
    }

    #[test]
    fn jitter_stays_inside_its_radius() {
        let mut j = Jitter::from_seed(77);
        for _ in 0..1000 {
            let v = j.signed(3.0);
            assert!((-3.0..=3.0).contains(&v), "out of range: {v}");
        }
    }

    #[test]
    fn varied_delay_never_collapses_to_zero() {
        let mut j = Jitter::from_seed(21);
        for _ in 0..1000 {
            assert!(j.vary_ms(1, 0.9) >= 1, "a 1ms delay must stay observable");
        }
        assert_eq!(j.vary_ms(0, 0.5), 0, "zero stays zero");
    }
}
