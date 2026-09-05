//! Permanent gate for the input event TRACE (gaps.md G11, step 7 / "B7").
//!
//! # Why this file exists
//!
//! Every other input test in this repository asserts on an EFFECT: the document
//! scrolled, the field holds the text, the click landed. Those assertions have
//! a blind spot that is not theoretical -- it shipped.
//!
//! `scroll` used to move `scrollTop` from JavaScript and `type` used to fill a
//! field through `Input.insertText` alone. Both produced the correct effect and
//! neither delivered a single `wheel` or `keydown` to the page, so lazy-load,
//! infinite scroll, autocomplete and incremental validation stayed dead while
//! the suite stayed green. The gap was found by reading source, not by a test:
//! clippy, the full suite and ten gates all passed over it.
//!
//! gaps.md names the missing instrument in as many words -- "enquanto B7 não
//! existir, qualquer alegação sobre cinemática de input é não verificada por
//! construção". This file is B7. It asserts on WHAT THE PAGE RECEIVED.
//!
//! # Why the `direct` profile is asserted too
//!
//! A gate that only ever ran the `human` profile would also pass while
//! measuring nothing in particular. `--input-profile direct` is a real negative
//! control here, and the discrimination comes from the product's own contract:
//! `direct` keeps the scripted `scrollBy` path, which emits no `wheel` at all,
//! and the bare `insertText`, which emits no key events. So the same page and
//! the same steps must produce a trace under `human` and no trace under
//! `direct`. If a future change makes `human` stop synthesizing events, the
//! first two tests fail; if it makes `direct` start, the third fails.
//!
//! # Skip policy
//!
//! No Chrome, no binary or no fixture means SKIP LOUDLY. A silent green here
//! would rebuild exactly the blind spot this gate removes.

mod common;
use common::{binary, missing_binary, root};

const GATE: &str = "input_trace_gate";

fn fixture_url() -> Option<String> {
    let p = root().join("scripts/fixtures/input_trace/instrumented.html");
    p.exists().then(|| format!("file://{}", p.display()))
}

/// True when the host cannot run the gate. Prints why; never silently passes.
fn cannot_run() -> bool {
    if missing_binary(GATE) {
        return true;
    }
    if fixture_url().is_none() {
        common::skip_with_reason(
            "input_trace_gate",
            "fixture scripts/fixtures/input_trace/instrumented.html absent.",
        );
        return true;
    }
    false
}

/// Run one NDJSON script and return the parsed envelope.
///
/// `profile` is passed as a global flag so the whole script runs under it.
fn run_script(profile: &str, lines: &[String]) -> Option<serde_json::Value> {
    let bin = binary()?;
    // Each invocation needs its own directory: these tests are threads of ONE
    // binary and a pid-keyed path is shared, so they would overwrite each
    // other's script.
    // A `TempDir` and not a pid+counter path: the counter only ever resolved
    // COLLISION between the threads of this one binary, never cleanup, so an
    // assertion that panicked left the directory behind for good. The guard is
    // bound to a NAMED variable on purpose — `let _ = ...` drops it on the spot
    // and deletes the script before the child process can read it.
    let scratch = tempfile::Builder::new()
        .prefix("bac-input-trace-gate-")
        .tempdir()
        .ok()?;
    let dir = scratch.path();
    let script = dir.join("steps.jsonl");
    std::fs::write(&script, lines.join("\n")).ok()?;

    let out = common::isolated_cmd(&bin)
        .args([
            "-q",
            "--timeout",
            "180",
            "--json",
            "--input-profile",
            profile,
            // Seeded so jitter is reproducible: the assertions below are about
            // the SHAPE of the trace, and a gate that flakes on randomness
            // teaches people to rerun it until it is green.
            "--input-seed",
            "424242",
            "run",
            "--script",
        ])
        .arg(&script)
        .output()
        .ok()?;

    serde_json::from_slice(&out.stdout).ok()
}

/// The steps every case shares: load the page, act, then read the trace.
///
/// `eval` comes last on purpose. It invalidates `@eN` refs, and reading the
/// trace before the gestures would measure an empty array.
fn trace_script(url: &str) -> Vec<String> {
    vec![
        format!(r#"{{"cmd":"goto","url":"{url}"}}"#),
        r##"{"cmd":"wait","selector":"#btn","wait_timeout_ms":8000}"##.into(),
        r#"{"cmd":"scroll","delta_y":400}"#.into(),
        r##"{"cmd":"type","target":"#field","text":"abcde"}"##.into(),
        r##"{"cmd":"press","target":"#btn"}"##.into(),
        r#"{"cmd":"eval","expression":"JSON.stringify(window.__trace)"}"#.into(),
    ]
}

/// Pull the recorded events out of the last `eval` step.
fn trace_of(env: &serde_json::Value) -> Vec<serde_json::Value> {
    let steps = env
        .pointer("/data/steps")
        .and_then(|s| s.as_array())
        .cloned()
        .unwrap_or_default();
    let raw = steps
        .iter()
        .rev()
        .find(|s| s.get("cmd").and_then(|c| c.as_str()) == Some("eval"))
        .and_then(|s| s.get("data"))
        .and_then(|d| d.get("result").or_else(|| d.get("value")))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let text = match raw {
        serde_json::Value::String(s) => s,
        other => other.to_string(),
    };
    serde_json::from_str::<Vec<serde_json::Value>>(&text).unwrap_or_default()
}

fn kinds<'a>(trace: &'a [serde_json::Value], kind: &str) -> Vec<&'a serde_json::Value> {
    trace
        .iter()
        .filter(|e| e.get("k").and_then(|k| k.as_str()) == Some(kind))
        .collect()
}

/// Under `human`, the page must actually RECEIVE the gestures.
///
/// This is the assertion whose absence let `scroll` and `type` ship without
/// emitting anything. Each count is the product's own documented contract, not
/// a guess: one wheel tick per `input_scroll_tick_px`, one keydown/keyup pair
/// per character.
#[test]
fn human_profile_delivers_wheel_and_key_events_to_the_page() {
    if cannot_run() {
        return;
    }
    let url = fixture_url().expect("fixture");
    let env = run_script("human", &trace_script(&url)).expect("run envelope");
    let trace = trace_of(&env);
    assert!(
        !trace.is_empty(),
        "the instrumented page recorded nothing at all -- either no gesture was \
         dispatched or the fixture stopped listening. envelope={env}"
    );

    let wheel = kinds(&trace, "wheel");
    assert!(
        wheel.len() >= 3,
        "a 400px scroll must reach the page as real wheel ticks; got {} \
         (this is the exact regression B7 exists to catch: the document can \
         scroll from JavaScript while the page receives zero wheel events, so \
         lazy-load and infinite-scroll stay dead). trace={trace:?}",
        wheel.len()
    );

    let down = kinds(&trace, "keydown");
    let up = kinds(&trace, "keyup");
    assert_eq!(
        (down.len(), up.len()),
        (5, 5),
        "typing 5 characters must produce 5 keydown and 5 keyup; got {} and {} \
         (bare Input.insertText fills the field and fires neither, which leaves \
         autocomplete, input masks and incremental validation dead). \
         trace={trace:?}",
        down.len(),
        up.len()
    );

    let moves = kinds(&trace, "mousemove");
    let downs = kinds(&trace, "mousedown");
    assert!(
        !downs.is_empty(),
        "press must reach the page as a mousedown. trace={trace:?}"
    );
    let first_down = trace
        .iter()
        .position(|e| e.get("k").and_then(|k| k.as_str()) == Some("mousedown"))
        .unwrap_or(trace.len());
    let approach: Vec<_> = trace[..first_down]
        .iter()
        .filter(|e| e.get("k").and_then(|k| k.as_str()) == Some("mousemove"))
        .filter_map(|e| Some((e.get("x")?.as_i64()?, e.get("y")?.as_i64()?)))
        .collect();
    let distinct: std::collections::HashSet<_> = approach.iter().collect();
    assert!(
        distinct.len() >= 2,
        "press must approach the target along a trajectory: at least 2 distinct \
         mousemove coordinates before mousedown, got {} distinct out of {} moves \
         ({} total mousemove in the trace). A single jump to the target is the \
         signature CDP-driven input leaves behind. trace={trace:?}",
        distinct.len(),
        approach.len(),
        moves.len()
    );
}

/// Consecutive intervals must not all be identical.
///
/// Perfectly uniform spacing is a signature on its own, and it is what a fixed
/// `sleep` between events produces. Asserting it here is what keeps a future
/// "simplification" from replacing the jitter with a constant.
#[test]
fn human_profile_spacing_is_not_perfectly_uniform() {
    if cannot_run() {
        return;
    }
    let url = fixture_url().expect("fixture");
    let env = run_script("human", &trace_script(&url)).expect("run envelope");
    let trace = trace_of(&env);
    let times: Vec<f64> = trace
        .iter()
        .filter_map(|e| e.get("t").and_then(serde_json::Value::as_f64))
        .collect();
    assert!(
        times.len() >= 4,
        "not enough events to judge spacing: {} (envelope={env})",
        times.len()
    );
    let gaps: Vec<f64> = times.windows(2).map(|w| w[1] - w[0]).collect();
    // Sub-millisecond resolution from performance.now(), so "identical" means
    // identical, not "close". Two distinct gaps is the whole claim.
    let distinct = gaps
        .iter()
        .map(|g| format!("{g:.3}"))
        .collect::<std::collections::HashSet<_>>();
    assert!(
        distinct.len() >= 2,
        "every inter-event gap was identical ({distinct:?}), which is a fixed \
         cadence and a fingerprint by itself. gaps={gaps:?}"
    );
}

/// `direct` is the declared escape hatch, and it must stay bit-for-bit cheap.
///
/// The product documents `direct` as keeping the scripted `scrollBy` and the
/// bare `insertText`. That makes this the negative control: the same page and
/// the same steps must reach the page WITHOUT wheel or key events. Without this
/// case the two tests above could pass on a build that synthesizes events
/// unconditionally, and `direct` would have silently stopped being an opt-out.
#[test]
fn direct_profile_emits_no_synthetic_wheel_or_key_events() {
    if cannot_run() {
        return;
    }
    let url = fixture_url().expect("fixture");
    let env = run_script("direct", &trace_script(&url)).expect("run envelope");
    let trace = trace_of(&env);

    let wheel = kinds(&trace, "wheel");
    assert!(
        wheel.is_empty(),
        "--input-profile direct must keep the scripted scrollBy path and emit no \
         wheel; got {}. trace={trace:?}",
        wheel.len()
    );
    let down = kinds(&trace, "keydown");
    assert!(
        down.is_empty(),
        "--input-profile direct must keep bare Input.insertText and emit no \
         keydown; got {}. trace={trace:?}",
        down.len()
    );
}

// ---------------------------------------------------------------------------
// Second moment: the SHAPE of the dispersion, not just its presence.
//
// The three tests above prove the page RECEIVES the gestures and that the gaps
// are not all identical. "Not all identical" is a weak claim, and the product
// passed it while producing symmetric noise: measured 2026-08-31 on the final
// browser event, 20 characters under `human`, the interval distribution came
// out at mean 141.26 ms, stddev 20.38 ms, **skewness 0.036**. A detector reads
// the second moment because the first is trivial to imitate, and the third to
// tell a scaled constant from a hand. Zero skewness is not an unusual typist.
//
// These cases need no browser and no fixture: `Jitter` is pure arithmetic, so
// the shape can be asserted directly instead of inferred from a trace.
// ---------------------------------------------------------------------------

use browser_automation_cli::constants::{
    DEFAULT_INPUT_TIMING_DISTRIBUTION, INPUT_TYPE_DELAY_MS, INPUT_TYPE_DELAY_STDDEV_MS,
    TIMING_MIN_DISPERSION_RATIO, TIMING_SAMPLE_CEILING_RATIO, TIMING_SAMPLE_FLOOR_RATIO,
};
// `Jitter` and not `Kinematics`: the sampler is pure arithmetic, while building
// a `Kinematics` resolves XDG and would make the assertion depend on whoever
// runs the suite. `TimingDistribution` is deliberately absent — the enum is not
// re-exported from `native::interaction`, so each shape is exercised through the
// `Jitter` method it dispatches to.
use browser_automation_cli::native::interaction::{Jitter, TimingDistribution};

/// Samples large enough that the third moment is a measurement, not a coin flip.
const SAMPLES: usize = 4_000;

/// One shape of dispersion, named as it appears in the config token.
type Shape = (&'static str, fn(u64) -> Vec<u64>);

/// The three shapes `input_timing_distribution` selects between.
const SHAPES: &[Shape] = &[
    ("lognormal", draw_lognormal),
    ("normal", draw_normal),
    ("uniform", draw_uniform),
];

/// Mean, standard deviation and skewness of a delay sample.
fn moments(xs: &[u64]) -> (f64, f64, f64) {
    let n = xs.len() as f64;
    let mean = xs.iter().map(|&x| x as f64).sum::<f64>() / n;
    let m2 = xs.iter().map(|&x| (x as f64 - mean).powi(2)).sum::<f64>() / n;
    let m3 = xs.iter().map(|&x| (x as f64 - mean).powi(3)).sum::<f64>() / n;
    let sd = m2.sqrt();
    (mean, sd, m3 / sd.powi(3))
}

/// `SAMPLES` log-normal delays of the default typing rhythm.
fn draw_lognormal(seed: u64) -> Vec<u64> {
    let mut j = Jitter::from_seed(seed);
    (0..SAMPLES)
        .map(|_| j.lognormal_ms(INPUT_TYPE_DELAY_MS, INPUT_TYPE_DELAY_STDDEV_MS))
        .collect()
}

/// The same, drawn from a normal.
fn draw_normal(seed: u64) -> Vec<u64> {
    let mut j = Jitter::from_seed(seed);
    (0..SAMPLES)
        .map(|_| j.normal_ms(INPUT_TYPE_DELAY_MS, INPUT_TYPE_DELAY_STDDEV_MS))
        .collect()
}

/// The same, drawn from the pre-0.1.9 uniform.
///
/// A uniform of standard deviation `s` has half-width `s * sqrt(3)`, which is
/// how `sample_ms` converts the knob for this arm. Repeating the conversion here
/// is what makes this a fair control rather than a weaker one.
fn draw_uniform(seed: u64) -> Vec<u64> {
    let spread = INPUT_TYPE_DELAY_STDDEV_MS as f64 * 3.0_f64.sqrt() / INPUT_TYPE_DELAY_MS as f64;
    let mut j = Jitter::from_seed(seed);
    (0..SAMPLES)
        .map(|_| j.vary_ms(INPUT_TYPE_DELAY_MS, spread))
        .collect()
}

/// A seeded run must still replay exactly.
///
/// Reproducibility is the property the whole event-trace gate rests on, and the
/// log-normal sampler consumes TWO uniforms per draw where the old uniform arm
/// consumed one. A rejection loop instead of the `MIN_POSITIVE` guard inside
/// `normal01` would consume an unpredictable number and break exactly this.
#[test]
fn the_same_seed_replays_the_same_delay_sequence() {
    let a = draw_lognormal(424_242);
    let b = draw_lognormal(424_242);
    assert_eq!(a, b, "--input-seed must keep a human run reproducible");
}

/// Seeding must choose WHICH noise, never WHETHER there is noise.
///
/// The failure this catches is a "simplification" that makes the sampler
/// deterministic to make the gate stable. Variance zero is a stronger signal
/// than a wrong mean: a wrong mean reads as an unusual human, and no variance
/// reads as no human.
#[test]
fn different_seeds_produce_different_sequences() {
    let a = draw_lognormal(1);
    let b = draw_lognormal(2);
    assert_ne!(a, b, "the seed must not be the only source of dispersion");
    // The support is integer milliseconds inside the truncation window, so the
    // count of distinct values is capped by the WINDOW and not by `SAMPLES`.
    // Comparing against a fraction of `SAMPLES` would be a bound no correct
    // sampler can meet: measured, 4000 draws land on 231 of the ~357 values the
    // window admits. The claim worth asserting is that the draws spread across
    // that window rather than piling onto a handful of points.
    let distinct: std::collections::HashSet<u64> = a.iter().copied().collect();
    assert!(
        distinct.len() > 100,
        "only {} distinct values in {SAMPLES} draws; the sampler collapsed onto \
         a grid",
        distinct.len()
    );
}

/// The distribution must have a long RIGHT tail, not merely a width.
///
/// This is the assertion the product failed. `uniform` is asserted alongside as
/// the negative control: it satisfies every "the gaps differ" test in this file
/// and still has a skewness indistinguishable from zero, which is what makes it
/// the shape a detector recognises.
#[test]
fn the_lognormal_shape_is_skewed_right_and_the_uniform_one_is_not() {
    let (mean, sd, skew) = moments(&draw_lognormal(7));
    assert!(
        skew > 0.5,
        "lognormal skewness {skew:.3} (mean {mean:.2} ms, sd {sd:.2} ms); human \
         inter-key intervals sit between 1 and 3, and a symmetric spread is the \
         exact signature this sampler exists to remove"
    );

    let (_, _, flat) = moments(&draw_uniform(7));
    assert!(
        flat.abs() < 0.2,
        "uniform skewness {flat:.3} should be ~0; if this fails the control is \
         broken and the test above proves nothing"
    );
}

/// The knob an operator sets must be the moment they can measure.
///
/// `lognormal_ms` takes ARITHMETIC moments and solves the log-space parameters
/// from them, so a stddev that came out systematically low would mean the
/// solution is wrong, not that the draw was unlucky.
#[test]
fn the_sample_dispersion_matches_the_requested_stddev() {
    let asked = INPUT_TYPE_DELAY_STDDEV_MS as f64;
    for &(name, draw) in SHAPES {
        let (mean, sd, skew) = moments(&draw(31));
        let error = (sd - asked).abs() / asked;
        println!("{name}: mean {mean:.2} ms, sd {sd:.2} ms, skew {skew:.3}");
        assert!(
            error < 0.15,
            "{name}: sd {sd:.2} ms against {asked} asked ({:.1}% off), mean {mean:.2} ms",
            error * 100.0
        );
    }
}

/// A stddev of zero must NOT produce a constant.
///
/// This is the assertion that turns a comment into a guarantee. `stddev_ms == 0`
/// used to return the mean unchanged, which is variance zero -- the one shape
/// that says "machine" outright, and a stronger signal than any wrong mean.
/// Nothing reachable requests zero today: every default is non-zero and
/// `policy_u64` filters `n > 0`. That is protection by CIRCUMSTANCE, and this
/// case exists so the first refactor that changes either circumstance fails
/// here instead of shipping a constant delay in silence.
#[test]
fn a_zero_stddev_still_disperses_because_variance_zero_is_the_worst_signal() {
    let mut j = Jitter::from_seed(4242);
    let samples: Vec<u64> = (0..SAMPLES)
        .map(|_| j.lognormal_ms(INPUT_TYPE_DELAY_MS, 0))
        .collect();
    let (mean, sd, _) = moments(&samples);
    let floor = INPUT_TYPE_DELAY_MS as f64 * TIMING_MIN_DISPERSION_RATIO;
    println!("zero-stddev request: mean {mean:.2} ms, sd {sd:.2} ms, floor {floor:.2} ms");
    assert!(
        sd > 0.0,
        "a zero stddev collapsed the delay onto a constant; variance zero is \
         not a tight rhythm, it is no hand at all"
    );
    // Within 25% of the declared floor: the sampler must honour the FLOOR, not
    // merely avoid the constant, or a future one-millisecond fudge would pass.
    assert!(
        (sd - floor).abs() / floor < 0.25,
        "sd {sd:.2} ms against a declared floor of {floor:.2} ms; the clamp is \
         not the one TIMING_MIN_DISPERSION_RATIO names"
    );
}

/// Every shape token must survive the round trip through config.
///
/// `input_timing_distribution` is validated on WRITE by `config set`, so a
/// token this parser rejects would be a key an operator can never set, and a
/// token it accepts but cannot re-emit would break `config get`.
#[test]
fn every_distribution_token_round_trips() {
    for &(name, _) in SHAPES {
        let parsed = TimingDistribution::parse(name);
        assert!(
            parsed.is_some(),
            "{name} is a documented token but parses to None"
        );
        assert_eq!(
            parsed.map(TimingDistribution::as_str),
            Some(name),
            "{name} did not survive parse then as_str"
        );
    }
    assert_eq!(
        TimingDistribution::parse(DEFAULT_INPUT_TIMING_DISTRIBUTION),
        Some(TimingDistribution::default()),
        "the compiled default must be a token its own parser accepts, or every \
         unconfigured process falls back to something nobody wrote down"
    );
    assert_eq!(TimingDistribution::parse("gaussian"), None);
}

/// Truncation must hold on both sides.
///
/// A log-normal has unbounded support: unclamped, a draw lands at one
/// millisecond (a paste, not a typist) or stalls a one-shot process past its own
/// timeout. The bounds are named constants, and this reads them rather than
/// repeating the numbers.
#[test]
fn every_sample_lands_inside_the_named_truncation_bounds() {
    let mean = INPUT_TYPE_DELAY_MS as f64;
    let floor = (mean * TIMING_SAMPLE_FLOOR_RATIO).floor();
    let ceiling = (mean * TIMING_SAMPLE_CEILING_RATIO).ceil();
    for &(name, draw) in SHAPES {
        for seed in 1_u64..=4 {
            let samples = draw(seed);
            let (lo, hi) = (
                samples.iter().copied().min().unwrap_or_default(),
                samples.iter().copied().max().unwrap_or_default(),
            );
            println!("{name} seed {seed}: min {lo} ms, max {hi} ms");
            let out: Vec<u64> = samples
                .into_iter()
                .filter(|&v| (v as f64) < floor || (v as f64) > ceiling)
                .collect();
            assert!(
                out.is_empty(),
                "{name} seed {seed}: {} samples outside [{floor}, {ceiling}] ms, \
                 first few {:?}",
                out.len(),
                &out[..out.len().min(5)]
            );
        }
    }
}
