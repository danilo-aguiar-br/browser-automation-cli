//! Permanent gate proving the `wait` settling conditions actually WAIT (GAP-032).
//!
//! # Why this file exists
//!
//! `network_idle_ms`, `dom_stable_ms` and `min_count` are already present in the
//! parser, in `schema wait` and in the published JSON schema. A gate that reads
//! any of those surfaces would be green today and would stay green if the poll
//! loop in `src/browser/session/wait_emulate/wait.rs` returned on the first
//! iteration. Field presence is not behaviour.
//!
//! The property that has to hold is temporal: a page that only satisfies the
//! condition after N milliseconds must make `wait` return AFTER N, not at once.
//!
//! # Why the clock is read INSIDE the page
//!
//! Asserting `elapsed >= N` on the PROCESS would be the blind version of this
//! gate. Launching Chrome already costs several hundred milliseconds, so that
//! bound is met even by a `wait` that returns immediately.
//!
//! The first version of this file solved that by running each script twice and
//! asserting the DELTA, on the theory that startup is common to both runs and
//! cancels out. That holds for a CONSTANT cost and fails for noise that varies
//! WITHIN the measured window: when host load changes between the control run
//! and the test run, it enters the delta instead of leaving it. The gate was
//! intermittent for exactly that reason — roughly one failure in five even in
//! isolation, always on the two timing cases.
//!
//! Each case now brackets the `wait` step with two `eval` steps reading
//! `Date.now()` in the page. That span contains the wait and two CDP round
//! trips, and nothing else: no process startup, no browser launch, no teardown.
//!
//! The bounds are LOWER bounds, and that is what makes the measurement immune to
//! load rather than merely resistant to it. Scheduler delay can only make the
//! observed span LONGER, so a loaded host can never push it below the bound. The
//! failure it must catch — a `wait` that does not wait — moves the span the
//! other way, down to the round-trip floor.
//!
//! # A lower bound is only immune when its ORIGIN cannot drift
//!
//! That paragraph was stated too broadly and a measurement refuted it.
//!
//! `min_count_holds_the_wait_open_until_the_late_cards_arrive` bracketed the
//! wait with two clock steps and asserted the span was at least 1200 ms against
//! a 1500 ms content delay. Measured 2026-09-04 in a full-suite run, it
//! reported 1139 ms and failed. Nothing was wrong with the wait: the fixture's
//! timer starts during `goto`, so the FIRST clock reading races a timer that is
//! already running. A loaded host moves that reading later, which shrinks the
//! span — the bound is a lower bound on a difference whose START moves toward
//! the end.
//!
//! The rule that actually holds: a lower bound survives load when BOTH ends
//! come from the same clock AND the origin is anchored to the same event as the
//! delay being measured. That case now anchors on `window.__scheduledAt`, set by
//! the fixture at the instant it schedules its timer, so the cards cannot exist
//! before `origin + delay` and load can only add.
//!
//! Before adding a timing bound here, name the origin and ask what moves it.
//!
//! # The two deadline cases use UPPER bounds, and the PRODUCT measures them
//!
//! `an_unreachable_condition_times_out_instead_of_succeeding_blank` and
//! `wait_timeout_ms_is_honoured_as_the_public_deadline` assert `elapsed < 8000`
//! on PROCESS time. Both violate the doctrine stated just above, and the reason
//! is structural rather than sloppy: they guard a deadline FALLING BACK to the
//! built-in ten-second default, and "did not take too long" is an upper bound by
//! nature. The argument that makes lower bounds immune runs backwards here —
//! scheduler delay only ADDS time, so load pushes these cases toward failure.
//!
//! Three test-only rewrites were tried on 2026-09-04 and all three failed:
//!
//! 1. Absolute process time, the original. Inside the full suite, where every
//!    test binary launches a Chrome at once, a run whose deadline the product
//!    HONOURED took 10 799 ms and the case went red.
//! 2. Paired budgets, 2000 against 6000, on the theory that startup cancels in
//!    the difference. It does not: startup varied between 2.5 s and 10 s across
//!    two runs of the SAME script, so a 6500 ms budget finished in 8429 ms while
//!    a 2500 ms one took 12 174 ms. This is the very failure the section above
//!    already records for the FIRST version of this file.
//! 3. Page-clock start to process end, using `Date.now()` before the wait. This
//!    does drop startup, the largest term, but teardown stays inside the span
//!    and reached 12 282 ms on a host at 2 percent idle CPU.
//!
//! The product was cleared by direct measurement while chasing this: `ms:2000`
//! and `wait_timeout_ms:2000` both finish an unreachable wait in ~3.3 s, so the
//! knob is honoured and there is no GAP-053 regression to find.
//!
//! None of the three could work, and the reason is that the quantity itself was
//! never available: the run envelope reported no per-step duration, so nothing
//! in `data.steps[i]` said how long a step took and every deadline assertion had
//! to infer one from a clock the test does not control.
//!
//! It reports one now, and something better besides.
//!
//! `src/commands/run/engine.rs` times the step future and nothing else, so
//! `duration_ms` on each step excludes startup and teardown by construction.
//! That alone was not enough: measured 2026-09-04 in-suite, a step reported
//! 10 711 ms where an isolated run of the same script reported 2778 ms, and the
//! built-in default measures 10 269 to 10 541 ms. Whether that reading was a
//! substituted default or a starved poll loop is not decidable from a DURATION,
//! however cleanly the duration is measured, because both stories predict the
//! same number.
//!
//! So the timeout message now NAMES the deadline it applied — `deadline of
//! 2500ms` against `deadline of 10000ms` — and both cases compare that value to
//! the one they asked for. Comparing two NUMBERS retires the upper bound
//! entirely: a dropped key changes the number, and no amount of host load can.
//! What remains of the timing is a LOWER bound, which the doctrine above already
//! covers.
//!
//! The value is worth more to callers than to this file. An agent reading
//! `ok: false, kind: timeout` learned that A deadline passed and never which
//! one, so it could not tell a budget it asked for from a default that silently
//! replaced it.
//!
//! That is the same question this gate asks, and it now has the same answer.
//!
//! # Where the thresholds come from
//!
//! Three samples each, taken at load average 7.6 / 17.2 / 17.6, in milliseconds.
//! The floor is a wait whose condition is ALREADY satisfied, so it is what the
//! span collapses to when no waiting happens.
//!
//! | case | s1 | s2 | s3 | spread | bound |
//! |---|---|---|---|---|---|
//! | `min_count`, 1500 ms of delay | 1481 | 1476 | 1479 | 5 | 1200 |
//! | `network_idle_ms` 1500 | 1569 | 1568 | 1568 | 1 | 1200 |
//! | `dom_stable_ms` 600 over 2000 ms of churn | 2602 | 2600 | 2597 | 5 | 2000 |
//! | floor: condition already satisfied | 52 | 52 | 61 | 9 | — |
//!
//! A spread of single-digit milliseconds under a load average of seventeen is
//! the whole argument for reading the clock in the page: the process-level delta
//! it replaces carried 131 ms of noise on a QUIET machine.
//!
//! # The five committed cases
//!
//! | case | what it proves |
//! |---|---|
//! | positive: late cards | `min_count` holds until the Nth node exists |
//! | positive: quiet window | the `network_idle_ms` window is served in full |
//! | positive: mutating document | `dom_stable_ms` outlasts the mutations |
//! | negative: unreachable count | an unmet condition TIMES OUT, never succeeds blank |
//! | declared exclusion | the conditions are OR-ed, not AND-ed |
//!
//! The negative case is what separates a gate from theatre. Without it, a `wait`
//! that returned immediately in every situation would still satisfy every
//! positive control, because returning early can only make the deltas smaller —
//! and it is the unmet condition that would then silently succeed.
//!
//! # What this file does NOT cover
//!
//! - It does not check that `wait` conditions appear in `schema wait` or in
//!   `docs/schemas/wait.schema.json`; that is surface, and surface is exactly
//!   what this gate refuses to accept as evidence.
//! - It does not cover the `state`, `navigation` or `url` conditions, which
//!   predate this gap.
//! - It does not cover the one-shot `wait` subcommand, only the `run` step.
//!
//! # GAP-053: `wait_timeout_ms` is asserted
//!
//! The public deadline name (`wait_timeout_ms` / schema / skill formulas) is
//! accepted by the `run` step parser first (see `nav_steps/wait.rs`). The case
//! `wait_timeout_ms_is_honoured_as_the_public_deadline` pins that the documented
//! key is not silently dropped in favour of the ten-second built-in default.
//!
//! # Skip policy
//!
//! No binary, no fixture or no Chrome means SKIP LOUDLY. A silent green here
//! would rebuild the blind spot this gate exists to remove.

use std::time::Instant;

mod common;
use common::{binary, chrome_not_ready, missing_binary, root};

const GATE: &str = "wait_conditions_gate";

fn fixture_url(name: &str, query: &str) -> Option<String> {
    let p = root().join("scripts/fixtures/wait_conditions").join(name);
    p.exists()
        .then(|| format!("file://{}?{}", p.display(), query))
}

/// Reads the page clock. Bracketing a step with two of these measures that step
/// and two CDP round trips, and nothing else.
const CLOCK: &str = r#"{"cmd":"eval","expression":"Date.now()"}"#;

/// Run a script through `run`, returning the envelope and the process wall time.
///
/// The wall time is reported for diagnostics only. No assertion depends on it:
/// it carries browser startup, which is the cost that made the process-level
/// measurement unusable.
///
/// Each invocation gets its own directory: the cases run as threads of ONE test
/// binary, so a path keyed only by pid would be shared and the scripts would
/// overwrite each other.
fn run_timed(lines: &[String]) -> Option<(serde_json::Value, u128)> {
    let bin = binary()?;
    // A `TempDir` and not a pid+counter path: the counter only ever resolved
    // COLLISION between the threads of this one binary, never cleanup, so an
    // assertion that panicked left the directory behind for good. The guard is
    // bound to a NAMED variable on purpose — `let _ = ...` drops it on the spot
    // and deletes the script before the child process can read it.
    let scratch = tempfile::Builder::new()
        .prefix("bac-wait-cond-gate-")
        .tempdir()
        .ok()?;
    let dir = scratch.path();
    let script = dir.join("steps.jsonl");
    std::fs::write(&script, lines.join("\n")).ok()?;

    let started = Instant::now();
    let out = common::isolated_cmd(&bin)
        .args(["-q", "--timeout", "120", "--json", "run", "--script"])
        .arg(&script)
        .output()
        .ok()?;
    let elapsed = started.elapsed().as_millis();

    Some((serde_json::from_slice(&out.stdout).ok()?, elapsed))
}

/// The last `eval` result in the envelope, as milliseconds.
///
/// Used by spans anchored in the PAGE rather than bracketed by two harness
/// clock steps: the page computes the difference itself, so only one reading
/// crosses the wire and there is no second endpoint to drift.
fn last_eval_ms(env: &serde_json::Value) -> i64 {
    env.pointer("/data/steps")
        .and_then(|s| s.as_array())
        .and_then(|steps| {
            steps
                .iter()
                .filter(|s| s.get("cmd").and_then(|c| c.as_str()) == Some("eval"))
                .filter_map(|s| s.pointer("/data/result").and_then(serde_json::Value::as_i64))
                .next_back()
        })
        .expect("the anchored eval must return a number; a missing reading is a broken measurement, not a slow wait")
}

/// Milliseconds between the two `CLOCK` readings, measured in the page.
///
/// Panics rather than defaulting when the readings are missing: a silent zero
/// here would turn a broken measurement into an assertion failure blamed on the
/// wait, which is the wrong diagnosis to hand whoever reads the output.
fn page_span_ms(env: &serde_json::Value) -> i64 {
    let clocks: Vec<i64> = env
        .pointer("/data/steps")
        .and_then(|s| s.as_array())
        .map(|steps| {
            steps
                .iter()
                .filter(|s| s.get("cmd").and_then(|c| c.as_str()) == Some("eval"))
                .filter_map(|s| s.pointer("/data/result").and_then(|v| v.as_i64()))
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(
        clocks.len(),
        2,
        "expected exactly two page clock readings around the wait, got {clocks:?}. \
         The measurement itself is broken; do not read this as a wait defect."
    );
    clocks[1] - clocks[0]
}

/// The product`s own `duration_ms` for the step that FAILED.
///
/// # Why the product measures this and the test cannot
///
/// A deadline case asks "did the step take too long", which is an UPPER bound,
/// and the doctrine at the head of this file explains why only LOWER bounds
/// survive host load: scheduler delay adds time, so it pushes an upper bound
/// toward failure and a lower bound away from it.
///
/// Three test-only measurements were tried on 2026-09-04 and all three failed.
/// Process wall time carries a browser launch that varied between 2.5 s and
/// 10 s. Paired budgets do not cancel that variance — the file header already
/// records the same refutation for the first version of this gate. A page-clock
/// reading before the wait drops startup but keeps teardown, and reached
/// 12 282 ms on a host at 2 percent idle CPU.
///
/// The step now reports its own duration, measured in `src/commands/run/engine.rs`
/// around the step future and nothing else. Startup, teardown and every other
/// process cost fall outside it by construction, which leaves only scheduler
/// delay INSIDE the wait — bounded, and orders of magnitude below the several
/// seconds of slack these cases carry.
fn failed_step_duration_ms(env: &serde_json::Value) -> u64 {
    let d = env
        .pointer("/data/steps")
        .and_then(|s| s.as_array())
        .and_then(|steps| {
            steps
                .iter()
                .find(|s| s.get("ok").and_then(|o| o.as_bool()) == Some(false))
        })
        .and_then(|s| s.get("duration_ms"))
        .and_then(serde_json::Value::as_u64);
    d.unwrap_or_else(|| {
        panic!(
            "no failing step carries `duration_ms` in {env}. The measurement itself is broken; \
             do not read this as a wait defect."
        )
    })
}

/// The `waited` entries reported by the single `wait` step of a script.
fn waited_kinds(env: &serde_json::Value) -> Vec<String> {
    env.pointer("/data/steps")
        .and_then(|s| s.as_array())
        .map(|steps| {
            steps
                .iter()
                .filter(|s| s.get("cmd").and_then(|c| c.as_str()) == Some("wait"))
                .filter_map(|s| {
                    s.pointer("/data/waited")
                        .and_then(|w| w.as_array())
                        .cloned()
                })
                .flatten()
                .filter_map(|w| w.get("kind").and_then(|k| k.as_str()).map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// True when the host cannot run the gate. Prints why; never silently passes.
fn cannot_run(fixture: &str) -> bool {
    if missing_binary(GATE) {
        return true;
    }
    if fixture_url(fixture, "").is_none() {
        common::skip_with_reason(
            "wait_conditions_gate",
            &format!("fixture scripts/fixtures/wait_conditions/{fixture} absent."),
        );
        return true;
    }
    if chrome_not_ready(GATE, &binary().expect("binary")) {
        return true;
    }
    false
}

/// POSITIVE CONTROL: the wrapper paints at once and the cards arrive late, so
/// `min_count` must hold the wait open until the third node exists.
///
/// A `min_count` that was parsed and then ignored would satisfy the selector on
/// its first matching node and return at the round-trip floor. The page-measured
/// span is the evidence.
#[test]
fn min_count_holds_the_wait_open_until_the_late_cards_arrive() {
    if cannot_run("late_content.html") {
        return;
    }
    let script = |delay: u32| {
        let url = fixture_url("late_content.html", &format!("delay={delay}&cards=3"))
            .expect("fixture url");
        vec![
            format!(r#"{{"cmd":"goto","url":"{url}"}}"#),
            CLOCK.to_string(),
            r##"{"cmd":"wait","selector":"#list li","min_count":3,"ms":9000}"##.to_string(),
            CLOCK.to_string(),
        ]
    };

    // Anchored in the PAGE, at the instant the fixture scheduled its timer, and
    // NOT at a clock the harness reads after `goto`.
    //
    // Measured 2026-09-04 under a full-suite run: the harness-anchored version
    // of this case reported 1139 ms against a 1500 ms delay and failed its own
    // 1200 ms floor. Nothing was wrong with the wait. The pre-wait clock reading
    // races the timer that `goto` already started, so a loaded host moves the
    // START later and the span SHRINKS — a lower bound whose origin drifts is
    // not immune to load, it only looks it. See the file header.
    let anchored = vec![
        format!(
            r#"{{"cmd":"goto","url":"{}"}}"#,
            fixture_url("late_content.html", "delay=1500&cards=3").expect("fixture url")
        ),
        r##"{"cmd":"wait","selector":"#list li","min_count":3,"ms":9000}"##.to_string(),
        r#"{"cmd":"eval","expression":"Date.now() - window.__scheduledAt"}"#.to_string(),
    ];
    let (delayed_env, delayed_wall) = run_timed(&anchored).expect("delayed run");
    assert_eq!(
        delayed_env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "the delayed page must satisfy the condition once the cards land: {delayed_env}"
    );
    assert!(
        waited_kinds(&delayed_env).contains(&"selector".to_string()),
        "the wait must report the selector condition, got {:?}",
        waited_kinds(&delayed_env)
    );

    let waited = last_eval_ms(&delayed_env);
    assert!(
        waited >= 1_490,
        "a 1500 ms content delay must hold the wait open until the cards land; \
         measured {waited}ms from the page's own scheduling instant (process \
         wall time was {delayed_wall}ms, which this assertion deliberately \
         ignores). Both ends of this span come from the page clock and share an \
         origin, so load can only make it LONGER: a value below the delay means \
         the wait returned before the cards existed, which is the early \
         evaluation GAP-032 was opened for."
    );

    // Instrument control: the SAME script against a page that is already
    // complete must collapse to the floor. Without it, a `wait` that always
    // slept 1500 ms would satisfy the assertion above.
    //
    // This is the one load-sensitive bound in the file, because it is an UPPER
    // bound. The floor measured 52, 52 and 61 ms at load average 17, so 700
    // leaves better than tenfold headroom.
    let (immediate_env, _) = run_timed(&script(0)).expect("immediate run");
    let floor = page_span_ms(&immediate_env);
    assert!(
        floor < 700,
        "an already-satisfied condition must return at once; the page measured \
         {floor}ms. A wait that always sleeps would pass the lower bound above \
         and fail here."
    );
}

/// POSITIVE CONTROL: the quiet window is served in full.
///
/// Network counting is unconditional — `--capture-network` governs only the
/// request log, see `src/browser/session/launch.rs`. With no request at all the
/// page counts as quiet since the wait began, so the window itself is the whole
/// cost and the span must contain it.
///
/// The count is already session-local: `network_is_quiet` reads `net_inflight`
/// and `net_last_activity`, which belong to this session and cannot see a
/// neighbouring browser. What used to make this case intermittent was not WHAT
/// it counted but WHERE the clock was read.
#[test]
fn the_network_quiet_window_is_served_in_full() {
    if cannot_run("late_content.html") {
        return;
    }
    let script = |idle: u32| {
        let url = fixture_url("late_content.html", "delay=0&cards=3").expect("fixture url");
        vec![
            format!(r#"{{"cmd":"goto","url":"{url}"}}"#),
            CLOCK.to_string(),
            format!(r#"{{"cmd":"wait","network_idle_ms":{idle},"ms":9000}}"#),
            CLOCK.to_string(),
        ]
    };

    let (long_env, long_wall) = run_timed(&script(1500)).expect("long window run");
    assert_eq!(
        long_env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "the quiet window must be satisfiable on a static page: {long_env}"
    );
    assert!(
        waited_kinds(&long_env).contains(&"network_idle".to_string()),
        "the run must report the network_idle condition, got {:?}",
        waited_kinds(&long_env)
    );

    let waited = page_span_ms(&long_env);
    assert!(
        waited >= 1_200,
        "a 1500 ms quiet window must be served in full; the page measured \
         {waited}ms (process wall time was {long_wall}ms, deliberately ignored). \
         A span near the round-trip floor means the window is accepted and then \
         not served."
    );

    // Instrument control: a short window must NOT cost the long one's time. This
    // is what proves the window VALUE is honoured rather than some fixed sleep.
    let (short_env, _) = run_timed(&script(50)).expect("short window run");
    let short = page_span_ms(&short_env);
    assert!(
        short < 700,
        "a 50 ms window must return promptly; the page measured {short}ms. \
         If a short window costs as much as a long one, the value is ignored and \
         the assertion above is satisfied by a constant."
    );
}

/// POSITIVE CONTROL: DOM stability outlasts the mutations.
///
/// The fixture appends a node every 100 ms for a fixed span. The stability
/// window can only close after the LAST mutation, so 2000 ms of churn plus a
/// 600 ms window cannot be served in less than the churn.
#[test]
fn dom_stability_outlasts_a_mutating_document() {
    if cannot_run("mutating.html") {
        return;
    }
    let script = |churn: u32| {
        let url =
            fixture_url("mutating.html", &format!("churn={churn}&every=100")).expect("fixture url");
        vec![
            format!(r#"{{"cmd":"goto","url":"{url}"}}"#),
            CLOCK.to_string(),
            r#"{"cmd":"wait","dom_stable_ms":600,"ms":15000}"#.to_string(),
            CLOCK.to_string(),
        ]
    };

    let (churning_env, churning_wall) = run_timed(&script(2000)).expect("churning run");
    assert_eq!(
        churning_env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "the churning document must eventually settle: {churning_env}"
    );
    assert!(
        waited_kinds(&churning_env).contains(&"dom_stable".to_string()),
        "the run must report the dom_stable condition, got {:?}",
        waited_kinds(&churning_env)
    );

    let waited = page_span_ms(&churning_env);
    assert!(
        waited >= 2_000,
        "2000 ms of mutation plus a 600 ms window cannot be served in less than \
         2000 ms; the page measured {waited}ms (process wall time was \
         {churning_wall}ms, deliberately ignored). A short span means the \
         fingerprint is compared once and never again."
    );

    // Instrument control: a document that never churns must settle after roughly
    // the window alone, well below the churning case. This is what shows the
    // span above tracks the MUTATIONS rather than a constant.
    let (settled_env, _) = run_timed(&script(0)).expect("settled run");
    let settled = page_span_ms(&settled_env);
    assert!(
        settled < 1_500,
        "a document that never mutates must settle after about the 600 ms window; \
         the page measured {settled}ms. If it costs as much as the churning case, \
         the wait is not tracking mutations at all."
    );
}

/// NEGATIVE: a condition that can never be satisfied must TIME OUT.
///
/// This is the case the positive controls cannot provide. Returning early only
/// shrinks a delta, so a `wait` that never waited would still pass every timing
/// assertion above — and would show up here, and only here, as a blank success.
///
/// It also pins the deadline itself: with `ms: 2500` the failure has to arrive
/// well before the built-in ten-second fallback.
#[test]
fn an_unreachable_condition_times_out_instead_of_succeeding_blank() {
    if cannot_run("late_content.html") {
        return;
    }
    let url = fixture_url("late_content.html", "delay=0&cards=3").expect("fixture url");
    let (env, _wall) = run_timed(&[
        format!(r#"{{"cmd":"goto","url":"{url}"}}"#),
        r##"{"cmd":"wait","selector":"#list li","min_count":9,"ms":2500}"##.to_string(),
    ])
    .expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(false),
        "a page with three cards must NOT satisfy min_count 9: {env}"
    );
    assert_eq!(
        env.pointer("/error/kind").and_then(|v| v.as_str()),
        Some("timeout"),
        "an unmet wait condition is a timeout, not another kind: {env}"
    );

    // The message has to name the condition. "not met" alone forces the caller
    // to guess which of the OR-ed conditions was pending.
    let message = env
        .pointer("/error/message")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        message.contains("#list li") && message.contains("9"),
        "the timeout must name the unmet condition and its threshold; got {message}"
    );

    // The message NAMES the deadline that was applied, so this is a comparison
    // of two numbers and not of two clocks; see the header section on upper
    // bounds. `deadline of 10000ms` is what a dropped key looks like.
    assert!(
        message.contains("deadline of 2500ms"),
        "the deadline given as `ms` must be honoured; the step reported {message}, \
         and `deadline of 10000ms` would mean it fell back to the built-in default"
    );
    // A LOWER bound, which host load can only push away from failure.
    let waited = failed_step_duration_ms(&env);
    assert!(
        waited > 1_200,
        "a genuine timeout for a 2500 ms budget should not finish in {waited}ms"
    );
}

/// DECLARED EXCLUSION: the conditions are OR-ed, not AND-ed.
///
/// Passing a selector that is already satisfied together with a five-second
/// quiet window returns at once, on the selector. That is the documented algebra
/// in `OneShotSession::wait_for_conditions`, and it is worth pinning because it
/// reads like a bug from the outside: the quiet window was requested and not
/// served.
///
/// Callers that need AND semantics chain two `wait` steps.
#[test]
fn the_conditions_are_or_ed_and_the_first_satisfied_one_returns() {
    if cannot_run("late_content.html") {
        return;
    }
    let url = fixture_url("late_content.html", "delay=0&cards=3").expect("fixture url");
    let (env, _wall) = run_timed(&[
        format!(r#"{{"cmd":"goto","url":"{url}"}}"#),
        CLOCK.to_string(),
        r##"{"cmd":"wait","selector":"#list li","min_count":3,"network_idle_ms":5000,"ms":12000}"##
            .to_string(),
        CLOCK.to_string(),
    ])
    .expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "the already-satisfied selector must return: {env}"
    );
    let kinds = waited_kinds(&env);
    assert!(
        kinds.contains(&"selector".to_string()),
        "the selector is the condition that was met first, got {kinds:?}"
    );
    assert!(
        !kinds.contains(&"network_idle".to_string()),
        "an OR set returns on the FIRST satisfied condition; reporting both means \
         the five-second window was also served, got {kinds:?}"
    );
    let waited = page_span_ms(&env);
    assert!(
        waited < 700,
        "OR semantics must not pay for the unsatisfied five-second quiet window; \
         the page measured {waited}ms. If this approaches the window, the algebra \
         became AND."
    );
}

/// GAP-053: the public key `wait_timeout_ms` must set the deadline (not fall
/// through to the built-in ~10 s default when only that name is present).
#[test]
fn wait_timeout_ms_is_honoured_as_the_public_deadline() {
    if cannot_run("late_content.html") {
        return;
    }
    let url = fixture_url("late_content.html", "delay=0&cards=3").expect("fixture url");
    let (env, _wall) = run_timed(&[
        format!(r#"{{"cmd":"goto","url":"{url}"}}"#),
        r##"{"cmd":"wait","selector":"#list li","min_count":9,"wait_timeout_ms":2000}"##
            .to_string(),
    ])
    .expect("run envelope");
    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(false),
        "unreachable min_count must fail: {env}"
    );
    assert_eq!(
        env.pointer("/error/kind").and_then(|v| v.as_str()),
        Some("timeout"),
        "must be a timeout: {env}"
    );

    // Two NUMBERS, never two clocks: the message names the deadline applied.
    let message = env
        .pointer("/data/steps/1/error/message")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        message.contains("deadline of 2000ms"),
        "wait_timeout_ms:2000 must be honoured; the step reported {message}, and \
         `deadline of 10000ms` is exactly the GAP-053 regression this case exists \
         to catch"
    );
    // A LOWER bound, which host load can only push away from failure.
    let waited = failed_step_duration_ms(&env);
    assert!(
        waited > 1_200,
        "a genuine timeout for a 2000 ms budget should not finish in {waited}ms"
    );
}

/// ENVIRONMENT GUARD: this one never skips.
///
/// The other cases in this file return early when the host is not ready, and a
/// test that returns counts as a PASS. On a machine without Chrome that turns
/// the whole file green while it tested nothing, and the honest SKIP lines this
/// file writes to stderr are easy to lose in `cargo test` output.
///
/// A test that fails the ENVIRONMENT is not a test that fails the CODE. Keeping
/// them apart is what lets the behavioural cases skip gracefully for someone
/// developing without a browser, while an unusable host still turns exactly one
/// case RED in one place.
///
/// Every fixture this file uses is checked, so a single missing HTML file is
/// reported by name instead of silently disabling the cases that need it.
#[test]
fn the_host_can_actually_run_this_gate() {
    for fixture in ["late_content.html", "mutating.html"] {
        assert!(
            !cannot_run(fixture),
            "host cannot run this gate with fixture {fixture}: the other cases in              this file skipped, and a skip is NOT a pass. The SKIP line on stderr              names the missing precondition (binary, fixture, or Chrome)."
        );
    }
}
