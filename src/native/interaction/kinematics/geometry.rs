// SPDX-License-Identifier: MIT OR Apache-2.0
//! WHERE the pointer goes. The WHEN stays in [`super`].
//!
//! # The seam
//!
//! Every method in [`super`] answers with a duration: a gap, a dwell, a delay,
//! a pause. Every method here answers with a POSITION, and it is the only side
//! whose correctness is geometric — a curve that must land exactly on target, a
//! contact cloud whose SHAPE has to be radial rather than square, a step count
//! that scales with distance.
//!
//! That difference decides how the two are tested and how they fail. A timing
//! defect shows up as a distribution that is too narrow; a geometry defect
//! shows up as a path that is straight, an endpoint that drifts, or a cloud
//! with corners — none of which a timing assertion can see. Measured
//! 2026-08-31: 60 samples were enough to read a square out of the jitter cloud
//! while every timing test stayed green.

use super::{InputProfile, Kinematics};

/// Fraction of the travel distance used to bow the Bezier control points.
const CURVE_BOW: f64 = 0.18;

impl Kinematics {
    /// Offset a target point so repeated clicks never share one pixel.
    ///
    /// # Why radial and not two independent axes
    ///
    /// Calling [`super::Jitter::signed`] once per axis spreads the endpoint
    /// uniformly over a SQUARE of side `2r`, which inverts both properties of a
    /// hand: a human aims at the centre and lands near it, densely, with the
    /// far points rare. A square support makes the four corners — the points
    /// FARTHEST from the centre — exactly as likely as the centre itself.
    ///
    /// Measured 2026-08-31 over 60 clicks on one target, before this change:
    /// `max_x` and `max_y` were both `3.0` and the largest radius was `4.24`,
    /// which is `3 * sqrt(2)`. A maximum radius equal to the per-axis maximum
    /// times root two IS the diagonal of the square, and no radial distribution
    /// produces that relationship. The shape was readable off 60 samples.
    ///
    /// Sampling a Gaussian radius and a uniform angle instead gives the shape
    /// the third moment is looking for, and reuses [`super::Jitter::normal01`]
    /// rather than adding a second noise source with its own character.
    ///
    /// The radius is folded with `abs` rather than rejected-and-redrawn: a
    /// half-normal is the correct radial profile here, and redrawing would make
    /// the number of generator calls depend on the values drawn, which breaks
    /// the byte-for-byte reproducibility that `--input-seed` promises.
    pub fn jitter_target(&mut self, x: f64, y: f64) -> (f64, f64) {
        if self.profile == InputProfile::Direct || self.target_jitter_px <= 0.0 {
            return (x, y);
        }
        // `r` is the configured ceiling, so treat it as roughly three sigma:
        // clamping at `r` then trims a tail that is already thin instead of
        // amputating a fat one, which would pile probability onto the rim.
        let sigma = self.target_jitter_px / 3.0;
        let radius = (self.jitter.normal01() * sigma)
            .abs()
            .min(self.target_jitter_px);
        let angle = self.jitter.unit() * std::f64::consts::TAU;
        (x + radius * angle.cos(), y + radius * angle.sin())
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

    /// Sample count for a travel distance, capped by a budget drawn per gesture.
    ///
    /// The budget is SAMPLED and not fixed: a constant ceiling makes every
    /// trajectory past the cap carry exactly the same number of points, which is
    /// a countable signature even when no two points coincide.
    fn steps_for(&mut self, distance: f64) -> usize {
        let budget = self
            .sample(self.move_steps as u64, self.move_steps_stddev)
            .max(2) as usize;
        // One sample per ~24 px keeps short hops cheap; the budget is the
        // ceiling, never the floor, so a 4 px nudge does not pay for 24 round
        // trips.
        let scaled = (distance / 24.0).ceil() as usize;
        scaled.clamp(2, budget)
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

    /// The endpoint cloud must be RADIAL, not a square.
    ///
    /// The previous implementation drew each axis independently, spreading the
    /// contact point uniformly over a square. That inverts a hand twice: it
    /// makes the far corners as likely as the centre, and it gives the cloud a
    /// straight edge no aim produces. `target_jitter_moves_the_contact_point`
    /// above cannot catch it, because a square is every bit as good as a disc
    /// at making two clicks differ — it is a first-moment test, and shape is a
    /// third-moment property.
    ///
    /// The discriminator is the MEAN RADIUS as a fraction of the ceiling. For a
    /// uniform square of half-width `r` it is about `0.54 * r`; for the folded
    /// normal used here, with sigma of `r / 3`, it is about `0.27 * r`. The gap
    /// is wide enough that a threshold between them cannot be reached by luck
    /// at this sample size, and a regression to per-axis sampling fails it.
    #[test]
    fn target_jitter_is_radial_and_not_a_square() {
        let mut k = human(29);
        let ceiling = k.target_jitter_px;
        assert!(ceiling > 0.0, "the human profile must jitter at all");

        let samples = 4000;
        let mut total_radius = 0.0;
        let mut worst_radius: f64 = 0.0;
        for _ in 0..samples {
            let (x, y) = k.jitter_target(0.0, 0.0);
            let radius = x.hypot(y);
            total_radius += radius;
            worst_radius = worst_radius.max(radius);
        }
        let mean_radius = total_radius / f64::from(samples);

        assert!(
            worst_radius <= ceiling + 1e-9,
            "no sample may escape the configured ceiling: {worst_radius} > {ceiling}"
        );
        assert!(
            mean_radius < 0.40 * ceiling,
            "mean radius {mean_radius} is too far out for a radial draw; a uniform \
             square would sit near {} and that is the shape this test exists to reject",
            0.54 * ceiling
        );
        assert!(
            mean_radius > 0.10 * ceiling,
            "mean radius {mean_radius} collapsed toward the centre, which is variance \
             approaching zero and reads as machine outright"
        );
    }
}
