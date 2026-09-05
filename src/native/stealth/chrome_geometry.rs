// SPDX-License-Identifier: MIT OR Apache-2.0
//! Window geometry that a real desktop Chrome could actually produce.
//!
//! # The defect this closes
//!
//! Measured 2026-09-01 against the shipped binary: with stealth ON a page read
//! `innerWidth 1920, innerHeight 1080, outerWidth 1920, outerHeight 1080` —
//! a browser with ZERO chrome. `screen` mirrored the viewport through
//! `resolve_screen`, so `screen.availHeight` equalled `screen.height` as well,
//! and `navigator.maxTouchPoints` answered `1` in every mode.
//!
//! Each of those numbers passes on its own. The SET is impossible: no desktop
//! has a browser without a title bar, a screen without a panel, and exactly one
//! touch point at the same time. The pair is a louder signal than any single
//! value it hides, which is why this module fixes the whole set rather than the
//! loudest member.
//!
//! # What was measured, and where the rest comes from
//!
//! Control run on this host, 2026-09-01, a real Chromium in a floating window:
//! `inner 1479x1578`, `outer 1521x1708` — 42 px horizontal and 130 px vertical.
//! The same Chromium maximised: 0 px horizontal and 86 px vertical. The
//! difference between the two is the window-manager decoration (42 x 44); the
//! 86 px common to both is the tab strip plus toolbar.
//!
//! Panel height, measured the same day with `xprop -root _NET_WORKAREA` against
//! `xrandr` on this host: screen `6144x3456`, work area `6144x3376` at y=80.
//! So the GNOME panel eats 80 px vertically and 0 px horizontally.
//!
//! Those are the only geometry numbers this product has measured. A third
//! anchor pair — a bookmarks bar, a Windows title bar — is NOT invented here,
//! because a fabricated anchor would reintroduce exactly the made-up number
//! this module exists to remove.
//!
//! # Why a distribution and not a constant
//!
//! Pinning `outerHeight = innerHeight + 130` would make every instance of this
//! product report the identical chrome height. Zero variance is a STRONGER
//! signal than a wrong mean: a wrong mean looks like an unusual human, zero
//! variance looks like no human at all. So the two measured anchors are drawn
//! per identity, and the unmeasured axes (panel height, the free space around a
//! floating window) are sampled inside a band centred on the measurement.
//!
//! The band is UNIFORM and its edges were not measured. That is stated rather
//! than dressed up: the point of the band is that the value is not a constant,
//! not that its extremes are known. A log-normal shape was rejected on purpose —
//! the long right tail belongs to human INTER-EVENT TIME, and a window
//! decoration has no tail to reproduce.
//!
//! # Why the draw is seeded by the identity
//!
//! Within one window the chrome height does not change between page loads, so
//! the value must be constant inside an identity and only vary across
//! identities. [`for_process`] draws once per process and caches, seeded by
//! `--stealth-seed` plus the profile when a seed exists, so two one-shot runs
//! that share a cookie and an address also share a window. Without a seed the
//! draw is per-process, which is the same lifetime the rest of the identity
//! already has.
//!
//! # One source for the screen/viewport pair
//!
//! Measured 2026-09-01: `doctor --fingerprint` reported
//! `planned screen 1920x1080 != live 1920x1233`, exit 1. The plan published a
//! screen equal to the viewport while this patch grew it in the page. Two
//! numbers for one fact is the same defect class as the geometry itself.
//!
//! [`screen_for_viewport`] is now the ONLY place the pair is computed.
//! `screen::resolve_screen` uses it as the floor it hands to
//! `Emulation.setDeviceMetricsOverride`, and the injected script derives
//! `availWidth`/`availHeight` from the screen the page actually reports rather
//! than from the viewport. So the page cannot answer a screen the plan did not
//! publish: the patch subtracts the panel the plan already added.
//!
//! An explicit `--screen` still WINS, and it is still raised when it is too
//! small — the floor is now "window plus panel" rather than "viewport", which
//! is the same rule applied to a window that finally has chrome.
//!
//! # What the finished patch measures
//!
//! 300 drawn identities executed through node's `vm` against a stub
//! `window`/`Screen`/`Navigator` with a 1920x1080 viewport, 2026-09-01:
//!
//! - `screen.height >= availHeight >= outerHeight > innerHeight` held 300/300,
//!   and so did the width chain and `availTop <= screenY`.
//! - `outerHeight - innerHeight`: 26 distinct values, mean 110.29, sd 22.41,
//!   range 80..136. The bimodality is the two anchors, not noise.
//! - `screen.height - availHeight`: 33 distinct values, mean 78.68, sd 9.00.
//! - `navigator.maxTouchPoints`: 0 in all 300.
//!
//! The number that matters is the standard deviation, not the mean: a constant
//! would have reported sd 0 and passed every other check in this file.
//!
//! # What `--no-stealth` still reports, and why that is left alone
//!
//! This patch reaches the page ONLY through `script_for_process`, which returns
//! `None` when `browser_policy::stealth_enabled()` is false. Under
//! `--no-stealth` no script is injected at all, so `navigator.maxTouchPoints`
//! keeps answering `1` and the window keeps reporting no chrome.
//!
//! That is a DECISION, not an oversight. `--no-stealth` is the caller asking
//! for the browser as it is, and a mode named "no stealth" that silently
//! rewrote `maxTouchPoints` would be lying about the one thing it promises.
//! Closing it there would mean patching the launch path instead, which is a
//! different contract and a different file.

use sha2::{Digest, Sha256};
use std::sync::OnceLock;

/// One window's geometry, in CSS pixels, all offsets relative to the viewport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChromeGeometry {
    /// `outerWidth - innerWidth`: window border, 0 when maximised.
    pub chrome_width: i32,
    /// `outerHeight - innerHeight`: title bar plus tab strip plus toolbar.
    pub chrome_height: i32,
    /// `availWidth - outerWidth`: free desktop beside a floating window.
    pub free_width: i32,
    /// `availHeight - outerHeight`: free desktop below a floating window.
    pub free_height: i32,
    /// `screen.height - availHeight`: the desktop panel or task bar.
    pub panel_height: i32,
    /// `screenX` inside the free space.
    pub offset_x: i32,
    /// `screenY` below the panel, inside the free space.
    pub offset_y: i32,
}

/// The two anchors measured on this host, 2026-09-01: (horizontal, vertical).
///
/// Index 0 is the maximised window, index 1 the floating one. The list is short
/// because it holds only what was measured; see the module docs.
const ANCHORS: [(i32, i32); 2] = [(0, 86), (42, 130)];

/// Draw the geometry for one identity.
///
/// Deterministic in `seed`, which is the whole point: a page that reads
/// `outerHeight` on load and again after a click must see the same number.
#[must_use]
pub(crate) fn draw(seed: &str) -> ChromeGeometry {
    let digest = Sha256::digest(seed.as_bytes());
    let byte = |i: usize| i32::from(digest[i]);
    let pair = |i: usize| i32::from(u16::from_be_bytes([digest[i], digest[i + 1]]));

    let floating = byte(0) % 2 == 1;
    let (anchor_w, anchor_h) = ANCHORS[usize::from(floating)];

    // +/- 6 px around the anchor: theme and density move the toolbar by a few
    // pixels on real machines, and a hard anchor would put every identity on
    // the same row. The width of the band was NOT measured.
    let chrome_height = anchor_h + byte(1) % 13 - 6;
    let chrome_width = if floating { anchor_w + byte(2) % 5 } else { 0 };

    // 80 px measured here; the band is +/-16 for the same reason as above.
    let panel_height = 64 + byte(3) % 33;

    // A maximised window fills the work area exactly, so both gaps are zero and
    // the window sits at the top-left corner below the panel. Reporting free
    // space for a window that has none would be the impossible pair again.
    let (free_width, free_height) = if floating {
        (8 + pair(4) % 313, 8 + pair(6) % 253)
    } else {
        (0, 0)
    };
    let offset_x = if free_width > 0 {
        pair(8) % (free_width + 1)
    } else {
        0
    };
    let offset_y = if free_height > 0 {
        pair(10) % (free_height + 1)
    } else {
        0
    };

    ChromeGeometry {
        chrome_width,
        chrome_height,
        free_width,
        free_height,
        panel_height,
        offset_x,
        offset_y,
    }
}

/// The geometry in force for this process, or `None` when stealth is off.
///
/// Drawn once and cached, for the reason in the module docs: a page that reads
/// `outerHeight` on load and again after a click must see the same number.
///
/// With `--stealth-seed` the draw is a function of the seed and the profile, so
/// N one-shot runs sharing a cookie also share a window. Without a seed there is
/// nothing stable to hash — `spider_fingerprint` redraws the whole identity per
/// process anyway — so the process id and the clock stand in, which pins the
/// value for this process and nothing longer. That is deliberately NOT a
/// fallback to a constant: a constant would report the same window from every
/// installation of this product, which is the zero-variance tell the module
/// exists to remove.
#[must_use]
fn for_process() -> Option<ChromeGeometry> {
    static GEOMETRY: OnceLock<Option<ChromeGeometry>> = OnceLock::new();
    *GEOMETRY.get_or_init(|| {
        if !crate::browser_policy::stealth_enabled() {
            return None;
        }
        let profile = crate::browser_policy::stealth_profile();
        let seed = match crate::browser_policy::stealth_seed() {
            Some(seed) => format!("{seed}\u{0}{}", profile.as_str()),
            None => format!(
                "{}\u{0}{}\u{0}{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_nanos()),
                profile.as_str()
            ),
        };
        Some(draw(&seed))
    })
}

/// The screen a viewport of `width`x`height` must sit on.
///
/// The SINGLE source for the pair. `screen::resolve_screen` calls this for the
/// CDP override, and the injected script reverses it from the screen the page
/// reports, so the plan and the page cannot disagree by construction.
///
/// Returns the viewport unchanged when stealth is off, because no script runs
/// then and growing the screen would create the mismatch this closes.
#[must_use]
pub fn screen_for_viewport(width: i32, height: i32) -> (i32, i32) {
    match for_process() {
        Some(g) => (
            width + g.chrome_width + g.free_width,
            height + g.chrome_height + g.free_height + g.panel_height,
        ),
        None => (width, height),
    }
}

/// The patch that installs this geometry, plus the desktop touch-point count.
///
/// # Why the two live in one patch
///
/// They are the same claim. `maxTouchPoints: 1` is what headless Chrome reports
/// by default — `spider_fingerprint` says so in its own comment on
/// `spoof_touch_screen`, and `setup_defaults` leaves `touch_screen` off, so the
/// crate never emits the correction. A desktop that reports one touch point
/// while also reporting a window with no title bar is one incoherent device,
/// not two independent bugs.
///
/// `TouchEvent` and `ontouchstart` are deliberately left alone: desktop Chrome
/// defines both even with no touch screen, so removing them would trade a known
/// tell for a new one.
///
/// Every getter carries the native `toString` spelling, copied from the shape
/// the crate already uses, so a detector reading
/// `Object.getOwnPropertyDescriptor(...).get.toString()` sees what Chrome says.
#[must_use]
pub fn geometry_patch() -> String {
    for_process().map(patch_for).unwrap_or_default()
}

/// The patch text for one drawn geometry.
///
/// Split from [`geometry_patch`] so the tests can exercise the JavaScript
/// without publishing process-global stealth policy first.
#[must_use]
fn patch_for(g: ChromeGeometry) -> String {
    format!(
        r#"(()=>{{try{{
var CW={cw},CH={ch},PN={pn},OX={ox},OY={oy};
var IW0=window.innerWidth||0,IH0=window.innerHeight||0;
var SW0=screen.width||IW0,SH0=screen.height||IH0;
function iw(){{return window.innerWidth||IW0;}}
function ih(){{return window.innerHeight||IH0;}}
function aw(){{return Math.max(SW0,iw()+CW);}}
function ah(){{return Math.max(SH0-PN,ih()+CH);}}
function sw(){{return aw();}}
function sh(){{return ah()+PN;}}
function ow(){{return Math.min(iw()+CW,aw());}}
function oh(){{return Math.min(ih()+CH,ah());}}
function sx(){{return Math.min(OX,aw()-ow());}}
function sy(){{return PN+Math.min(OY,ah()-oh());}}
function al(){{return 0;}}
function at(){{return PN;}}
function N(f,n){{try{{Object.defineProperty(f,'toString',{{value:function(){{return 'function get '+n+'() {{ [native code] }}';}},configurable:true}});}}catch(_){{}}return f;}}
function D(o,n,f){{try{{Object.defineProperty(o,n,{{get:N(f,n),configurable:true}});}}catch(_){{}}}}
D(window,'outerWidth',ow);D(window,'outerHeight',oh);
D(window,'screenX',sx);D(window,'screenY',sy);
D(window,'screenLeft',sx);D(window,'screenTop',sy);
var SP=(typeof Screen!=='undefined'&&Screen.prototype)?Screen.prototype:window.screen;
D(SP,'width',sw);D(SP,'height',sh);
D(SP,'availWidth',aw);D(SP,'availHeight',ah);
D(SP,'availLeft',al);D(SP,'availTop',at);
}}catch(_){{}}
try{{
var z=function(){{return 0;}};
Object.defineProperty(z,'toString',{{value:function(){{return 'function get maxTouchPoints() {{ [native code] }}';}},configurable:true}});
Object.defineProperty(Navigator.prototype,'maxTouchPoints',{{get:z,configurable:true}});
Object.defineProperty(Navigator.prototype,'msMaxTouchPoints',{{get:z,configurable:true}});
}}catch(_){{}}
}})();"#,
        cw = g.chrome_width,
        ch = g.chrome_height,
        pn = g.panel_height,
        ox = g.offset_x,
        oy = g.offset_y,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every draw must satisfy `screen >= avail >= outer >= inner`.
    ///
    /// Checked on the Rust side because the JS getters are built directly from
    /// these fields: a negative gap here is an impossible pair in the page.
    #[test]
    fn the_containment_order_holds_for_every_seed() {
        for i in 0..512 {
            let g = draw(&format!("seed-{i}"));
            assert!(g.chrome_height > 0, "outerHeight would equal innerHeight");
            assert!(g.chrome_width >= 0);
            assert!(g.free_width >= 0 && g.free_height >= 0);
            assert!(
                g.panel_height > 0,
                "availHeight would equal screen.height, which no desktop shows"
            );
            assert!(g.offset_x <= g.free_width);
            assert!(g.offset_y <= g.free_height);
        }
    }

    /// A maximised window has no free desktop around it and sits at the corner.
    #[test]
    fn a_window_without_a_border_reports_no_free_space() {
        for i in 0..512 {
            let g = draw(&format!("seed-{i}"));
            if g.chrome_width == 0 {
                assert_eq!(g.free_width, 0);
                assert_eq!(g.free_height, 0);
                assert_eq!(g.offset_x, 0);
                assert_eq!(g.offset_y, 0);
            }
        }
    }

    /// One identity, one window: the value cannot move between page loads.
    #[test]
    fn the_draw_is_stable_for_one_identity() {
        assert_eq!(draw("identity-a"), draw("identity-a"));
        assert_eq!(patch_for(draw("identity-a")), patch_for(draw("identity-a")));
    }

    /// Zero variance is the signal this module exists to remove.
    ///
    /// A constant would pass every other test in this file, so the second
    /// moment is asserted explicitly rather than assumed.
    #[test]
    fn the_geometry_disperses_across_identities() {
        let heights: std::collections::HashSet<i32> = (0..512)
            .map(|i| draw(&format!("seed-{i}")).chrome_height)
            .collect();
        assert!(
            heights.len() >= 20,
            "chrome height collapsed to {} distinct values; that is a grid",
            heights.len()
        );
        let panels: std::collections::HashSet<i32> = (0..512)
            .map(|i| draw(&format!("seed-{i}")).panel_height)
            .collect();
        assert!(panels.len() >= 20, "panel height collapsed to a constant");
        let both: std::collections::HashSet<(i32, i32)> = (0..512)
            .map(|i| {
                let g = draw(&format!("seed-{i}"));
                (g.chrome_width, g.chrome_height)
            })
            .collect();
        assert!(both.len() >= 10, "the chrome pair collapsed to a grid");
    }

    /// Both measured anchors must survive; drawing only one is a constant again.
    #[test]
    fn both_measured_anchors_are_reachable() {
        let maximised = (0..512)
            .map(|i| draw(&format!("seed-{i}")))
            .any(|g| g.chrome_width == 0);
        let floating = (0..512)
            .map(|i| draw(&format!("seed-{i}")))
            .any(|g| g.chrome_width >= 42);
        assert!(maximised, "the maximised anchor is never drawn");
        assert!(floating, "the floating anchor is never drawn");
    }

    #[test]
    fn the_patch_reports_a_desktop_touch_point_count() {
        let js = patch_for(draw("s"));
        assert!(js.contains("maxTouchPoints"));
        assert!(js.contains("msMaxTouchPoints"));
        assert!(js.contains("return 0"));
    }

    #[test]
    fn the_patch_covers_the_three_measured_incoherences() {
        let js = patch_for(draw("s"));
        for marker in [
            "outerHeight",
            "outerWidth",
            "availHeight",
            "availWidth",
            "availTop",
            "screenY",
            "maxTouchPoints",
        ] {
            assert!(js.contains(marker), "patch never mentions {marker}");
        }
    }

    /// One failing block must not take the others down, as elsewhere in stealth.
    #[test]
    fn every_block_is_individually_guarded() {
        let js = patch_for(draw("s"));
        assert!(js.matches("try{").count() >= 2);
        assert!(js.contains("catch(_){}"));
    }

    /// A detector reading the getter's source must see Chrome's spelling.
    ///
    /// Measured 2026-09-01 by running 300 drawn patches through node's `vm`
    /// against a stub `window`/`Screen`/`Navigator`:
    /// `Object.getOwnPropertyDescriptor(window,'outerHeight').get.toString()`
    /// returned `function get outerHeight() { [native code] }`.
    #[test]
    fn the_getters_wear_the_native_tostring() {
        let js = patch_for(draw("s"));
        assert!(js.contains("[native code]"));
    }
}
