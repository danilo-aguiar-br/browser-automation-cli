// SPDX-License-Identifier: MIT OR Apache-2.0
//! `doctor --fingerprint`: compare the identity signals this process emits.
//!
//! Host `doctor` never looked at the page. The 0.1.9 audit showed that
//! `navigator.webdriver` deletion and a Windows UA next to a Linux
//! `navigator.platform` would have been caught by a trivial self-check. This
//! command is that check. It always scores the planned identity (no Chrome
//! required) and optionally live-probes when a browser can launch.

use serde_json::json;

use crate::browser_policy::{self, StealthProfile};
use crate::constants::{
    FINGERPRINT_MEASUREMENT_NOTE, FINGERPRINT_MEASUREMENT_SCOPE, FINGERPRINT_UNMEASURED_OS,
    STEALTH_PROFILE_TOKENS, STEALTH_SEED_DOES_NOT_VARY, STEALTH_SEED_FIELDS,
};
use crate::envelope::print_success_json;
use crate::native::stealth::{
    assess_signals, planned_stealth_signals, planned_vs_live, signals_from_live, CoherenceMismatch,
    FingerprintSignals, Identity,
};

/// Run the fingerprint-coherence diagnosis and return a process exit code.
pub fn run_fingerprint(json: bool, live: bool) -> i32 {
    let token = browser_policy::stealth_profile_token();
    let profile = token.resolved();
    let stealth = browser_policy::stealth_enabled();
    // Under stealth the crate's version IS the projected identity, so there is
    // nothing to probe. `--no-stealth` claims to describe the real browser and
    // therefore has to ask it.
    let (mut planned, version_source) = if stealth {
        (planned_stealth_signals(profile), None)
    } else {
        let (signals, source) = unpatched_chrome_signals();
        (signals, Some(source))
    };
    // `planned_stealth_signals` fills the screen from the Xvfb constant, which
    // is the VIEWPORT. Publishing that as the screen claims a browser with no
    // chrome, and the page — which grows the screen by the panel — then
    // contradicts it. Measured 2026-09-01 before this line existed:
    // `planned screen 1920x1080 != live 1920x1233`, exit 1.
    //
    // `resolve_screen` is the single source for the pair, so the plan is
    // reconciled through it rather than through a second copy of the arithmetic.
    let (screen_width, screen_height) =
        crate::native::stealth::resolve_screen(planned.inner_width, planned.inner_height);
    planned.screen_width = screen_width;
    planned.screen_height = screen_height;
    let static_mismatches = assess_signals(&planned, stealth);
    let (live_probe, live_mismatches) = if live {
        let mut probe = probe_live();
        let extra = probe
            .get("result")
            .and_then(|r| r.get("result").or(Some(r)))
            .and_then(signals_from_live)
            .map(|s| {
                let mut extra = assess_signals(&s, stealth);
                extra.extend(planned_vs_live(&planned, &s));
                extra
            })
            .unwrap_or_default();
        probe["mismatches"] = json!(extra
            .iter()
            .map(|m| json!({ "id": m.id, "message": m.message }))
            .collect::<Vec<_>>());
        (probe, extra)
    } else {
        (
            json!({ "attempted": false, "reason": "quick or offline" }),
            Vec::new(),
        )
    };

    let mut mismatches = static_mismatches;
    for m in live_mismatches {
        if !mismatches
            .iter()
            .any(|e| e.id == m.id && e.message == m.message)
        {
            mismatches.push(m);
        }
    }
    // The patch was WANTED but the CDP call to install it failed. Nothing else
    // in this envelope can tell that story: `stealth` reports the request, the
    // planned signals report the intent, and the live probe reports whatever
    // the unpatched page happened to say. Without this line the run looks
    // healthy while every automation marker is visible.
    if stealth && browser_policy::stealth_installed() == Some(false) {
        mismatches.push(CoherenceMismatch {
            id: "stealth_not_installed",
            message: "stealth was requested but the patch script failed to install; \
                      the reported identity is the unpatched browser"
                .to_string(),
        });
    }

    let ok = mismatches.is_empty();
    let sources = crate::native::stealth::signal_sources(stealth);
    // The scope constant names the recorte that has a live corpus. Comparing it
    // to the host we are actually on is a one-line honesty check that costs
    // nothing and closes the macOS/Windows blind spot.
    let scope_matches_host = FINGERPRINT_MEASUREMENT_SCOPE.starts_with(std::env::consts::OS);
    let data = json!({
        "schema_version": 1,
        "ok": ok,
        "stealth": stealth,
        "stealth_profile": token.as_str(),
        "stealth_profile_resolved": profile.as_str(),
        "stealth_profile_source": browser_policy::stealth_profile_source().as_str(),
        "profile_contradicts_host": crate::native::stealth::profile_contradicts_host(),
        "stealth_profiles": STEALTH_PROFILE_TOKENS,
        "stealth_seed_fields": STEALTH_SEED_FIELDS,
        "stealth_seed_does_not_vary": STEALTH_SEED_DOES_NOT_VARY,
        "measured_os": std::env::consts::OS,
        "measurement_scope": FINGERPRINT_MEASUREMENT_SCOPE,
        "unmeasured_os": FINGERPRINT_UNMEASURED_OS,
        "measurement_note": FINGERPRINT_MEASUREMENT_NOTE,
        // The argv this launch handed Chrome, or `null` when no launch has run.
        //
        // Published because an audit could not conduct the experiment that
        // decides whether a launch switch hurts fidelity — remove one, repeat
        // the measurement — the switches being visible only in `ps`. Reading
        // another process's command line is not a contract; this field is.
        //
        // `null` means "no launch yet", which is deliberately different from an
        // empty array meaning "launched with no flags".
        "launch_args": crate::native::cdp::chrome::launch_args()
            .map(<[String]>::to_vec),
        // Derived, never spelled. These three were literal `"host"` until 0.1.9,
        // and the GPU one was false on the default path: the envelope claimed a
        // host GPU next to `stealth: true` while the WebGL patch was rewriting
        // vendor and renderer from a table in the crate.
        "gpu_source": sources.gpu.as_str(),
        "fonts_source": sources.fonts.as_str(),
        "probe_page": crate::constants::ABOUT_BLANK,
        "fonts_method": "document.fonts.size",
        "fonts_note": "FontFaceSet size (CSS-loaded faces only; probe_page is about:blank so 0 is expected; not OS font enumeration)",
        "audio_source": sources.audio.as_str(),
        // `stealth` says what was ASKED for; this says what HAPPENED. `null`
        // when no browser launched in this process, so a `--quick` run does not
        // claim an installation it never attempted.
        "stealth_installed": browser_policy::stealth_installed(),
        // `--stealth-seed` in force. The envelope published what a seed varies
        // long before it published whether one exists.
        "stealth_seed_active": browser_policy::stealth_seed().is_some(),
        // `measured_os` is derived and `measurement_scope` is a constant, so on
        // a host outside the measured recorte they contradict each other. Say so
        // rather than leaving the reader to notice.
        "measurement_scope_matches_host": scope_matches_host,
        "identity": {
            "user_agent": planned.user_agent,
            "navigator_platform": planned.navigator_platform,
            "user_agent_data_platform": planned.ua_data_platform,
            "webdriver_present": planned.webdriver_in_navigator,
            "webdriver_value": planned.webdriver_value,
        },
        // Where the planned major came from. `null` under stealth, where the
        // crate table is the intended source rather than a fallback.
        "planned_version_source": version_source.map(PlannedVersionSource::as_str),
        "planned": planned_json(&planned),
        "mismatches": mismatches.iter().map(|m| json!({
            "id": m.id,
            "message": m.message,
        })).collect::<Vec<_>>(),
        "live": live_probe,
    });

    if json {
        match print_success_json(data) {
            Ok(()) => {}
            Err(e) if e.kind() == crate::error::ErrorKind::BrokenPipe => return 141,
            Err(e) => return crate::commands::common::emit_err(&e, true),
        }
    } else {
        let line = if ok {
            format!(
                "[pass] fingerprint — profile={} webdriver=present+{} platform={}",
                token.as_str(),
                planned
                    .webdriver_value
                    .map(|v| if v { "true" } else { "false" })
                    .unwrap_or("null"),
                planned.navigator_platform
            )
        } else {
            format!("[fail] fingerprint — {} mismatch(es)", mismatches.len())
        };
        match crate::output::writeln_stdout(line) {
            Ok(()) => {}
            Err(e) if e.kind() == crate::error::ErrorKind::BrokenPipe => return 141,
            Err(e) => return crate::commands::common::emit_err(&e, false),
        }
        let _ = crate::output::flush_stdout();
    }
    if ok {
        0
    } else {
        1
    }
}

fn planned_json(s: &FingerprintSignals) -> serde_json::Value {
    json!({
        "webdriver_in_navigator": s.webdriver_in_navigator,
        "webdriver_in_prototype": s.webdriver_in_prototype,
        "webdriver_value": s.webdriver_value,
        "user_agent": s.user_agent,
        "navigator_platform": s.navigator_platform,
        "ua_data_platform": s.ua_data_platform,
        "screen_width": s.screen_width,
        "screen_height": s.screen_height,
        "inner_width": s.inner_width,
        "inner_height": s.inner_height,
    })
}

/// Where the major version in the `--no-stealth` plan came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlannedVersionSource {
    /// Read from the Chrome/Chromium binary this host would actually launch.
    ChromeBinary,
    /// The dependency's table, because the binary could not be probed.
    CrateTable,
}

impl PlannedVersionSource {
    const fn as_str(self) -> &'static str {
        match self {
            Self::ChromeBinary => "chrome_binary",
            Self::CrateTable => "crate_table",
        }
    }
}

/// Chrome with stealth off: host platform, HeadlessChrome UA when headless,
/// webdriver present. This product rejects `--enable-automation`. Chrome
/// 151/152 then reports boolean `false` (MDN still lists `--headless` /
/// `--remote-debugging-port 0` as `true` — a version divergence).
///
/// # Why this asks the binary instead of the crate
///
/// `Identity::for_profile` takes its major from
/// `spider_fingerprint::spoof_user_agent::get_default_version`, which is the
/// right source for a SPOOF: under stealth the crate's version IS the identity
/// being projected. It is the wrong source here. `--no-stealth` means "describe
/// the browser without patches", and the crate table describes a build the host
/// may not have. Measured: the plan announced major 152 while the host ran
/// Chromium 151.0.7922.137, so the mode meant to report reality reported an
/// invented version — and the plan-vs-live comparison then had a permanent
/// false divergence baked into it.
///
/// The probe is paid only on this path, and a failure falls back to the crate
/// table rather than guessing. The envelope publishes which one answered.
fn unpatched_chrome_signals() -> (FingerprintSignals, PlannedVersionSource) {
    let host = StealthProfile::Auto.resolved();
    let id = Identity::for_profile(host);
    let (base_ua, source) = match probe_host_chrome_major() {
        Some(major) => (
            id.user_agent_with_major(&major),
            PlannedVersionSource::ChromeBinary,
        ),
        None => (id.user_agent.clone(), PlannedVersionSource::CrateTable),
    };
    let headless = crate::browser_policy::mode().launches_headless();
    let user_agent = if headless {
        base_ua.replace("Chrome/", "HeadlessChrome/")
    } else {
        base_ua
    };
    let signals = FingerprintSignals {
        webdriver_in_navigator: true,
        webdriver_in_prototype: true,
        webdriver_value: Some(false),
        user_agent,
        navigator_platform: id.navigator_platform.to_string(),
        // An UNPATCHED Chromium exposes no `navigator.userAgentData` on the
        // probe page: the value the stealth path plans here is created BY the
        // patch. Planning it with stealth off promised a signal the page never
        // emits, which is an assertion this mode has no business making.
        ua_data_platform: None,
        screen_width: crate::constants::DEFAULT_XVFB_WIDTH as i32,
        screen_height: crate::constants::DEFAULT_XVFB_HEIGHT as i32,
        inner_width: crate::constants::DEFAULT_XVFB_WIDTH as i32,
        inner_height: crate::constants::DEFAULT_XVFB_HEIGHT as i32,
    };
    (signals, source)
}

/// Chrome major version of the binary this host would launch, if it answers.
fn probe_host_chrome_major() -> Option<String> {
    let path = crate::native::cdp::chrome::find_chrome()?;
    let line = crate::platform::probe_binary_version(&path)?;
    crate::native::stealth::chrome_major_from_version_line(&line)
}

fn probe_live() -> serde_json::Value {
    match crate::browser::block_on_browser(async {
        let mut session = crate::browser::OneShotSession::launch_headless().await?;
        let _ = session
            .goto(
                crate::constants::ABOUT_BLANK,
                crate::robots::RobotsPolicy::Ignore,
            )
            .await?;
        let expr = r#"(function(){
          var uad = navigator.userAgentData;
          var gl = null, vendor = null, renderer = null;
          try {
            var c = document.createElement('canvas');
            gl = c.getContext('webgl') || c.getContext('experimental-webgl');
            if (gl) {
              var ext = gl.getExtension('WEBGL_debug_renderer_info');
              if (ext) {
                vendor = gl.getParameter(ext.UNMASKED_VENDOR_WEBGL);
                renderer = gl.getParameter(ext.UNMASKED_RENDERER_WEBGL);
              }
            }
          } catch (e) {}
          var fontCount = 0;
          try { fontCount = (document.fonts && document.fonts.size) ? document.fonts.size : 0; } catch (e) {}
          var audioRate = null;
          try {
            var ac = new (window.AudioContext || window.webkitAudioContext)();
            audioRate = ac.sampleRate;
            if (ac.close) ac.close();
          } catch (e) {}
          var canvasOk = false, canvasErr = null;
          try {
            var cv = document.createElement('canvas');
            cv.width = 16; cv.height = 16;
            var url = cv.toDataURL('image/png');
            canvasOk = typeof url === 'string' && url.indexOf('data:image/') === 0;
          } catch (e) { canvasErr = String(e); }
          return {
            webdriver_in_navigator: ('webdriver' in navigator),
            webdriver_in_prototype: ('webdriver' in Navigator.prototype),
            webdriver_value: (typeof navigator.webdriver === 'boolean') ? navigator.webdriver : null,
            user_agent: String(navigator.userAgent),
            navigator_platform: String(navigator.platform),
            ua_data_platform: uad && uad.platform ? String(uad.platform) : null,
            screen_width: screen.width,
            screen_height: screen.height,
            inner_width: window.innerWidth,
            inner_height: window.innerHeight,
            webgl_vendor: vendor,
            webgl_renderer: renderer,
            font_count: fontCount,
            audio_sample_rate: audioRate,
            canvas_todataurl_ok: canvasOk,
            canvas_todataurl_error: canvasErr
          };
        })()"#;
        let value = session.eval(expr, None, None, None).await?;
        let _ = session.shutdown().await;
        Ok::<_, crate::error::CliError>(value)
    }) {
        Ok(v) => json!({ "attempted": true, "ok": true, "result": v }),
        Err(e) => json!({
            "attempted": true,
            "ok": false,
            "error": e.message(),
        }),
    }
}

/// Emit the four valid `--stealth-profile` tokens (no browser launch).
pub fn emit_stealth_profiles(json: bool) -> i32 {
    let data = json!({
        "stealth_profiles": STEALTH_PROFILE_TOKENS,
        "default": crate::constants::DEFAULT_STEALTH_PROFILE,
    });
    if json {
        match print_success_json(data) {
            Ok(()) => 0,
            Err(e) if e.kind() == crate::error::ErrorKind::BrokenPipe => 141,
            Err(e) => crate::commands::common::emit_err(&e, true),
        }
    } else {
        for token in STEALTH_PROFILE_TOKENS {
            if crate::output::writeln_stdout(*token).is_err() {
                return 141;
            }
        }
        let _ = crate::output::flush_stdout();
        0
    }
}
