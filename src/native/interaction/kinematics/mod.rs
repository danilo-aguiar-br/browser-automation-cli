// SPDX-License-Identifier: MIT OR Apache-2.0
//! Pointer and keyboard kinematics: geometry and timing, with no CDP in sight.
//!
//! **Pure by design.** Every function here maps numbers to numbers, so the whole
//! module is unit-testable without a browser. The dispatch modules
//! ([`super::pointer`], [`mod@super::scroll`], [`super::keyboard`]) own the wire; this
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
//!
//! # Module map: profile | jitter | geometry | this file
//!
//! `profile` owns the process-level knobs and the timing shape; `jitter` owns
//! the seedable noise source; `geometry` owns WHERE the pointer goes; this file
//! owns WHEN it moves, which is where all three meet.

mod geometry;
mod jitter;
mod profile;
mod qwerty;

pub use jitter::Jitter;
pub use profile::{
    active, active_profile, set_input_profile, set_input_seed, InputProfile, TimingDistribution,
    TimingMetrics,
};

use profile::configured_distribution;

// Only the constants this module still names DIRECTLY. Every per-knob default
// now arrives through `policy_u64(key::…)`, which reads the operator override
// and falls back to the same constant, so importing those here as well would
// be a second path to the same number and an invitation to read the wrong one.
// The four that remain are used to RESCALE a route-local budget, which is a
// ratio between two defaults and not a value the operator sets.
use crate::constants::{
    INPUT_MOVE_GAP_MS, INPUT_MOVE_GAP_STDDEV_MS, INPUT_MOVE_STEPS, INPUT_MOVE_STEPS_STDDEV,
    WORD_BOUNDARY_PAUSE_CHARS,
};
use crate::xdg::policy::{key, policy_u64, policy_usize};

/// Tunables resolved once per process from XDG, then reused.
///
/// Read once because every getter hits the config resolver; a per-event read
/// would put file I/O inside the interaction loop.
#[derive(Debug, Clone)]
pub struct Kinematics {
    profile: InputProfile,
    jitter: Jitter,
    distribution: TimingDistribution,
    move_steps: usize,
    move_steps_stddev: u64,
    move_gap_ms: u64,
    move_gap_stddev_ms: u64,
    click_dwell_ms: u64,
    click_dwell_stddev_ms: u64,
    key_dwell_ms: u64,
    key_dwell_stddev_ms: u64,
    type_delay_ms: u64,
    type_delay_stddev_ms: u64,
    scroll_tick_px: u64,
    scroll_tick_stddev_px: u64,
    max_scroll_ticks: usize,
    target_jitter_px: f64,
    word_pause_permille: u64,
    word_pause_ms: u64,
    typo_permille: u64,
}

impl Kinematics {
    /// Resolve tunables from XDG for `profile`, seeding jitter from `seed`.
    #[must_use]
    pub fn new(profile: InputProfile, seed: Option<u64>) -> Self {
        Self {
            profile,
            jitter: seed.map_or_else(Jitter::from_entropy, Jitter::from_seed),
            distribution: configured_distribution(),
            move_steps: policy_usize(key::INPUT_MOVE_STEPS).max(1),
            move_steps_stddev: policy_u64(key::INPUT_MOVE_STEPS_STDDEV),
            move_gap_ms: policy_u64(key::INPUT_MOVE_GAP_MS),
            move_gap_stddev_ms: policy_u64(key::INPUT_MOVE_GAP_STDDEV_MS),
            click_dwell_ms: policy_u64(key::INPUT_CLICK_DWELL_MS),
            click_dwell_stddev_ms: policy_u64(key::INPUT_CLICK_DWELL_STDDEV_MS),
            key_dwell_ms: policy_u64(key::INPUT_KEY_DWELL_MS),
            key_dwell_stddev_ms: policy_u64(key::INPUT_KEY_DWELL_STDDEV_MS),
            type_delay_ms: policy_u64(key::INPUT_TYPE_DELAY_MS),
            type_delay_stddev_ms: policy_u64(key::INPUT_TYPE_DELAY_STDDEV_MS),
            scroll_tick_px: policy_u64(key::INPUT_SCROLL_TICK_PX).max(1),
            scroll_tick_stddev_px: policy_u64(key::INPUT_SCROLL_TICK_STDDEV_PX),
            max_scroll_ticks: policy_usize(key::INPUT_SCROLL_MAX_TICKS).max(1),
            target_jitter_px: policy_u64(key::INPUT_TARGET_JITTER_PX) as f64,
            word_pause_permille: policy_u64(key::INPUT_WORD_PAUSE_PERMILLE),
            word_pause_ms: policy_u64(key::INPUT_WORD_PAUSE_MS),
            typo_permille: policy_u64(key::INPUT_TYPO_PERMILLE),
        }
    }

    /// Timing this instance will actually apply to a typing gesture.
    ///
    /// Reads back the RESOLVED values rather than the configured ones, so the
    /// envelope cannot claim a dispersion the profile suppressed: `direct`
    /// reports zero because `direct` sleeps zero.
    #[must_use]
    pub fn timing_metrics(&self) -> TimingMetrics {
        if self.profile == InputProfile::Direct {
            return TimingMetrics {
                mean_ms: 0,
                stddev_ms: 0,
                distribution: TimingDistribution::Uniform.as_str(),
            };
        }
        TimingMetrics {
            mean_ms: self.type_delay_ms,
            stddev_ms: self.type_delay_stddev_ms,
            distribution: self.distribution.as_str(),
        }
    }

    /// One draw of `mean` dispersed by `stddev` under the active distribution.
    fn sample(&mut self, mean: u64, stddev: u64) -> u64 {
        self.jitter.sample_ms(self.distribution, mean, stddev)
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
        // The DISPERSION travels with the mean it disperses. The drag budget is
        // a quarter of the input one, so carrying the input stddev across would
        // put the coefficient of variation near 1 and make most drag gestures
        // draw their own truncation floor. Both divisors are non-zero compile
        // time constants.
        self.move_steps_stddev = steps as u64 * INPUT_MOVE_STEPS_STDDEV / INPUT_MOVE_STEPS;
        self.move_gap_stddev_ms = gap_ms * INPUT_MOVE_GAP_STDDEV_MS / INPUT_MOVE_GAP_MS;
        self.move_steps = steps.max(1);
        self.move_gap_ms = gap_ms;
        self
    }

    /// The active profile.
    #[must_use]
    pub fn profile(&self) -> InputProfile {
        self.profile
    }

    /// Pause between synthesized positions, sampled.
    pub fn move_gap_ms(&mut self) -> u64 {
        self.sample(self.move_gap_ms, self.move_gap_stddev_ms)
    }

    /// Hold time between press and release, sampled.
    pub fn click_dwell_ms(&mut self) -> u64 {
        if self.profile == InputProfile::Direct {
            return 0;
        }
        self.sample(self.click_dwell_ms, self.click_dwell_stddev_ms)
    }

    /// Hold time between `keyDown` and `keyUp`, sampled.
    pub fn key_dwell_ms(&mut self) -> u64 {
        if self.profile == InputProfile::Direct {
            return 0;
        }
        self.sample(self.key_dwell_ms, self.key_dwell_stddev_ms)
    }

    /// Delay before the next character, sampled.
    ///
    /// `explicit` wins when the caller passed `--delay-ms`, so an operator who
    /// asks for a rhythm gets exactly that rhythm, dispersed. The dispersion of
    /// an explicit mean is scaled from the default's coefficient of variation:
    /// an operator who halves the mean and keeps the old absolute stddev would
    /// otherwise get a distribution wide enough to reach the floor constantly.
    pub fn type_delay_ms(&mut self, explicit: Option<u64>) -> u64 {
        match (self.profile, explicit) {
            (_, Some(ms)) => {
                let stddev = if self.type_delay_ms == 0 {
                    0
                } else {
                    ms * self.type_delay_stddev_ms / self.type_delay_ms
                };
                self.sample(ms, stddev)
            }
            (InputProfile::Direct, None) => 0,
            (InputProfile::Human, None) => {
                self.sample(self.type_delay_ms, self.type_delay_stddev_ms)
            }
        }
    }

    /// Delay before the next character, plus the pause `ch` may have earned.
    ///
    /// Human typing is not one distribution but two superposed: a fast
    /// within-word rhythm and an occasional stop to think. The stop is what puts
    /// the long right tail on the interval distribution, and widening the fast
    /// rhythm alone reproduces the WIDTH of human timing without its SHAPE.
    pub fn type_delay_after(&mut self, ch: char, next: Option<char>, explicit: Option<u64>) -> u64 {
        let base = self.type_delay_ms(explicit);
        // Scaled by the PAIR, because a gap belongs to two characters and not
        // to one. `next` is `None` at the end of the text, where there is no
        // pair and therefore nothing to scale.
        let paced = match next {
            Some(n) if self.profile == InputProfile::Human => {
                base.saturating_mul(qwerty::gap_permille(ch, n)) / 1000
            }
            _ => base,
        };
        paced + self.maybe_long_pause(ch)
    }

    /// The wrong key a hand hits instead of `ch`, or `None`.
    ///
    /// Governed by the XDG key `input_typo_permille`, which is ZERO by default:
    /// unlike every other knob here, this one changes what the page reads
    /// mid-word, so it is the caller's decision and never the default.
    ///
    /// The caller types the returned character, sends `Backspace`, then types
    /// `ch`, so the field ends up holding exactly the requested text.
    pub fn maybe_typo(&mut self, ch: char) -> Option<char> {
        if self.profile == InputProfile::Direct || self.typo_permille == 0 {
            return None;
        }
        if self.jitter.unit() * 1000.0 >= self.typo_permille as f64 {
            return None;
        }
        // A second draw picks the side, so the hand is as likely to overshoot
        // left as right instead of always slipping the same way.
        qwerty::neighbour(ch, self.jitter.unit() < 0.5)
    }

    /// A long pause after `ch`, or zero.
    ///
    /// Only at a word or sentence boundary, and only as often as the XDG key
    /// `input_word_pause_permille` says, in chances per thousand. Pausing at
    /// every boundary would be as mechanical as never pausing.
    ///
    /// The key is the authority, not the constant: the compiled default lives
    /// in [`crate::constants::INPUT_WORD_PAUSE_PERMILLE`], but an operator
    /// override is what this actually reads.
    pub fn maybe_long_pause(&mut self, ch: char) -> u64 {
        if self.profile == InputProfile::Direct
            || self.word_pause_permille == 0
            || self.word_pause_ms == 0
            || !WORD_BOUNDARY_PAUSE_CHARS.contains(&ch)
        {
            return 0;
        }
        if self.jitter.unit() * 1000.0 >= self.word_pause_permille as f64 {
            return 0;
        }
        // Half the mean, so the pause itself is a distribution and not a second
        // constant hiding behind a coin flip.
        let stddev = self.word_pause_ms / 2;
        self.sample(self.word_pause_ms, stddev)
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
        //
        // The notch size is SAMPLED per gesture: a constant one makes the same
        // delta always decompose into the same tick sequence, so the split
        // itself is a fingerprint even though each tick carries a fresh delay.
        let tick_px = self
            .sample(self.scroll_tick_px, self.scroll_tick_stddev_px)
            .max(1) as f64;
        let ticks = ((span / tick_px).ceil().max(1.0) as usize).min(self.max_scroll_ticks);
        (1..=ticks)
            .map(|i| {
                let done = (i - 1) as f64 / ticks as f64;
                let next = i as f64 / ticks as f64;
                (delta_x * (next - done), delta_y * (next - done))
            })
            .collect()
    }
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
    fn the_pair_and_not_the_character_sets_the_gap() {
        // Means over many draws, not single draws: the gap is SAMPLED, so one
        // pair of draws can invert by noise while the distributions do not.
        // Same seed on both sides, so the two runs consume the same jitter
        // stream and the only difference left is the bigram factor.
        let mean = |a: char, b: char| -> u64 {
            let mut k = human(7);
            (0..400).map(|_| k.type_delay_after(a, Some(b), None)).sum()
        };
        let alternating = mean('t', 'h');
        let same_finger = mean('q', 'z');
        assert!(
            alternating < same_finger,
            "`th` alternates hands and must beat `qz`, which repeats one finger: \
             {alternating} vs {same_finger}"
        );
    }

    #[test]
    fn the_last_character_has_no_pair_to_be_scaled_by() {
        // `None` is the end of the text. Scaling it would mean inventing a
        // successor, and the sampled gap must come through untouched.
        let mut paired = human(11);
        let mut unpaired = human(11);
        assert_eq!(
            unpaired.type_delay_after('q', None, Some(200)),
            paired.type_delay_ms(Some(200)),
            "with no next character the gap must equal the plain sample"
        );
    }

    #[test]
    fn typing_is_never_wrong_unless_the_operator_asks() {
        // `input_typo_permille` defaults to zero, so the default path must not
        // emit a character the caller never wrote. Four hundred draws, because
        // a rate this small hides behind a handful.
        let mut k = human(3);
        assert!(
            (0..400).all(|_| k.maybe_typo('a').is_none()),
            "the default profile injected a typo"
        );
        let mut direct = Kinematics::new(InputProfile::Direct, Some(3));
        assert!(
            direct.maybe_typo('a').is_none(),
            "`direct` must stay literal"
        );
    }

    #[test]
    fn an_injected_typo_is_always_a_key_the_hand_could_reach() {
        // Exercises the mechanism without the knob: whatever rate is set, the
        // wrong character must be a neighbour and never the intended one.
        for ch in "qwertyuiopasdfghjklzxcvbnm".chars() {
            for left in [true, false] {
                let wrong = qwerty::neighbour(ch, left).expect("letter has a neighbour");
                assert_ne!(wrong, ch);
            }
        }
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
