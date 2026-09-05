// SPDX-License-Identifier: MIT OR Apache-2.0
//! Screen metrics that travel with the viewport so they cannot drift.
//!
//! Headless Chrome reports `screen.width`/`screen.height` as 800×600 unless
//! `Emulation.setDeviceMetricsOverride` also sets `screenWidth`/`screenHeight`.
//! A 1920×1080 viewport on an 800×600 screen is a fingerprint no desktop
//! produces. This module is the single place that pair is computed.

use serde_json::{json, Value};
use std::sync::Mutex;

/// Where the explicit screen override came from (ETD: never ambient).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenSource {
    /// `--screen` on argv.
    Argv,
    /// `screen` field on a `run` / `exec` step.
    Step,
    /// XDG `screen` applied at launch.
    Xdg,
    /// No explicit override; screen mirrors the viewport.
    Derived,
    /// An explicit override existed and the FLOOR was larger, so the number in
    /// the envelope is the floor and not what the caller asked for.
    ///
    /// Measured 2026-09-04: `emulate --screen 2560x1440` over a 1920x1080
    /// viewport answered `{"height":1444}` next to `screen_source: "step"`,
    /// because `chrome_geometry` draws the panel per identity and that draw
    /// occasionally lands above the requested height. The number was right and
    /// the provenance was wrong, which is worse than either alone: a caller
    /// reading `step` has no way to learn its value did not survive.
    Floor,
}

impl ScreenSource {
    /// Every variant, in envelope order.
    ///
    /// The token test iterates this and matches EXHAUSTIVELY, so a variant
    /// added without a frozen token fails to COMPILE. The previous test was a
    /// hand-written list of four asserts, and `Floor` shipped past it in
    /// 2026-09-04 without anyone noticing the fifth line was missing.
    pub const ALL: [Self; 5] = [
        Self::Argv,
        Self::Step,
        Self::Xdg,
        Self::Derived,
        Self::Floor,
    ];

    /// Stable envelope token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Argv => "argv",
            Self::Step => "step",
            Self::Xdg => "xdg",
            Self::Derived => "derived",
            Self::Floor => "floor",
        }
    }
}

/// Optional explicit screen override plus its source.
///
/// A `Mutex` rather than `OnceLock`: `run --script` can emit more than one
/// emulate/resize step, and the second one must be able to change the pair.
static SCREEN_STATE: Mutex<(Option<(i32, i32)>, ScreenSource)> =
    Mutex::new((None, ScreenSource::Derived));

/// Publish an explicit screen size for this process.
///
/// Later [`resolve_screen`] uses this instead of mirroring the viewport.
/// Passing `None` means "derive from the viewport".
pub fn set_screen_override(size: Option<(i32, i32)>, source: ScreenSource) {
    // `lock_recover` rather than `if let Ok`: swallowing a poisoned lock here
    // means the override is silently NOT published, and the process then
    // reports a screen it never applied. Recovering keeps the write and the
    // report in agreement, which is the whole point of publishing the source.
    let mut guard = crate::sync_util::lock_recover(&SCREEN_STATE);
    *guard = (
        size,
        if size.is_some() {
            source
        } else {
            ScreenSource::Derived
        },
    );
}

/// Current explicit override, if any.
#[must_use]
pub fn current_screen_override() -> Option<(i32, i32)> {
    crate::sync_util::lock_recover(&SCREEN_STATE).0
}

/// Where the current override came from (`derived` when none).
///
/// Reads through `lock_recover` for the same reason as the setter: a poisoned
/// lock answering `derived` would report a provenance the process never chose.
#[must_use]
pub fn current_screen_source() -> ScreenSource {
    crate::sync_util::lock_recover(&SCREEN_STATE).1
}

/// Screen size that must accompany a viewport of `width`×`height`.
///
/// An explicit override wins, but it is never allowed to be smaller than the
/// FLOOR: a window larger than the screen is the same contradiction this module
/// exists to prevent.
///
/// The floor used to be the viewport itself, which forced `screen == inner` and
/// therefore a browser with no chrome — the impossible pair that
/// `chrome_geometry` exists to close. It is now the window plus its panel, from
/// `chrome_geometry::screen_for_viewport`, which is the SINGLE place the pair is
/// computed. The injected script reverses the same arithmetic from the screen
/// the page reports, so the plan published here and the value the page answers
/// cannot drift apart.
///
/// With stealth off the floor collapses back to the viewport, because no script
/// runs then and growing the screen alone would create the mismatch.
#[must_use]
pub fn resolve_screen(width: i32, height: i32) -> (i32, i32) {
    let (floor_w, floor_h) = super::chrome_geometry::screen_for_viewport(width, height);
    match current_screen_override() {
        Some((sw, sh)) => (sw.max(floor_w), sh.max(floor_h)),
        None => (floor_w, floor_h),
    }
}

/// Provenance of the pair [`resolve_screen`] would return for this viewport.
///
/// Not [`current_screen_source`], which answers where the REQUEST came from and
/// keeps answering it even when the request lost. This answers where the NUMBER
/// came from, which is the only thing an envelope reader can act on.
///
/// Must be called with the same `width`/`height` as the matching
/// [`resolve_screen`], because the floor is a function of the viewport.
#[must_use]
pub fn resolved_screen_source(width: i32, height: i32) -> ScreenSource {
    let requested = current_screen_override();
    let Some((sw, sh)) = requested else {
        return current_screen_source();
    };
    let (floor_w, floor_h) = super::chrome_geometry::screen_for_viewport(width, height);
    if sw < floor_w || sh < floor_h {
        ScreenSource::Floor
    } else {
        current_screen_source()
    }
}

/// Parse `WxH` into a screen pair, rejecting zeroes and non-integers.
///
/// # Errors
///
/// Fails with `"screen missing height"` when there is no `x` separator, with
/// `"screen expected WxH"` when a third component follows, with
/// `"screen width must be integer"` / `"screen height must be integer"` for a
/// non-numeric component, and with
/// `"screen width and height must be positive"` for a zero or negative
/// dimension — a screen smaller than any viewport is the contradiction this
/// module exists to prevent.
pub fn parse_screen_spec(raw: &str) -> Result<(i32, i32), String> {
    let mut parts = raw.split('x').map(str::trim);
    let width: i32 = parts
        .next()
        .ok_or_else(|| "screen empty; expected WxH".to_string())?
        .parse()
        .map_err(|_| "screen width must be integer".to_string())?;
    let height: i32 = parts
        .next()
        .ok_or_else(|| "screen missing height".to_string())?
        .parse()
        .map_err(|_| "screen height must be integer".to_string())?;
    if parts.next().is_some() {
        return Err("screen expected WxH".to_string());
    }
    if width <= 0 || height <= 0 {
        return Err("screen width and height must be positive".to_string());
    }
    Ok((width, height))
}

/// CDP params for `Emulation.setDeviceMetricsOverride` with screen attached.
#[must_use]
pub fn device_metrics_override(
    width: i32,
    height: i32,
    device_scale_factor: f64,
    mobile: bool,
) -> Value {
    let (screen_width, screen_height) = resolve_screen(width, height);
    json!({
        "width": width,
        "height": height,
        "deviceScaleFactor": device_scale_factor,
        "mobile": mobile,
        "screenWidth": screen_width,
        "screenHeight": screen_height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serialises every case that touches the PROCESS-WIDE screen override.
    ///
    /// `set_screen_override` writes a static, and `cargo test` runs the cases of
    /// one binary as parallel THREADS of one process, so two cases touching it
    /// are racing on the same memory rather than on separate copies.
    ///
    /// Measured 2026-09-04: at the default thread count
    /// `a_roomy_override_still_wins` failed one run in three, reading
    /// (2084, 1467) where it had asked for (5120, 2880). That pair is the FLOOR
    /// its sibling computes, so the sibling`s `set_screen_override(None, ..)`
    /// cleanup had landed between this case`s setup and its read. Serially the
    /// same six cases pass every time.
    ///
    /// A plain `Mutex` and not a test-only dependency: the ordering needed is
    /// "one at a time", and `lock_recover` already provides it across this tree
    /// while keeping one panicking case from freezing the rest.
    static SCREEN_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Without an override the screen is the FLOOR, never the headless default.
    ///
    /// This used to assert `screen == viewport`, which is the impossible pair
    /// `chrome_geometry` closes: a screen the size of the viewport requires a
    /// browser with no chrome. The property that survives is that the screen
    /// comes from the single source and never shrinks below the viewport, and
    /// that is asserted against that source rather than against a literal, so
    /// the test does not depend on whether stealth happens to be on.
    #[test]
    fn without_override_the_screen_comes_from_the_single_source() {
        let _guard = crate::sync_util::lock_recover(&SCREEN_LOCK);
        let (floor_w, floor_h) = super::super::chrome_geometry::screen_for_viewport(1920, 1080);
        let params = device_metrics_override(1920, 1080, 1.0, false);
        assert_eq!(params["width"], 1920);
        assert_eq!(params["height"], 1080);
        assert_eq!(params["screenWidth"], floor_w);
        assert_eq!(params["screenHeight"], floor_h);
        assert!(floor_w >= 1920, "screen narrower than the viewport");
        assert!(floor_h >= 1080, "screen shorter than the viewport");
        assert_ne!(params["screenWidth"], 800);
        assert_ne!(params["screenHeight"], 600);
    }

    /// An override smaller than the window is RAISED, never honoured as-is.
    ///
    /// A screen the caller asked for that cannot hold the window plus its panel
    /// is the same contradiction the module exists to prevent, so the floor wins
    /// on that axis while a roomy override still wins outright.
    #[test]
    fn an_impossible_override_is_raised_to_the_floor() {
        let _guard = crate::sync_util::lock_recover(&SCREEN_LOCK);
        let (floor_w, floor_h) = super::super::chrome_geometry::screen_for_viewport(1920, 1080);
        set_screen_override(Some((640, 480)), ScreenSource::Argv);
        let raised = resolve_screen(1920, 1080);
        set_screen_override(None, ScreenSource::Derived);
        assert_eq!(raised, (floor_w, floor_h));
        assert!(raised.0 >= 1920 && raised.1 >= 1080);
    }

    #[test]
    fn a_roomy_override_still_wins() {
        let _guard = crate::sync_util::lock_recover(&SCREEN_LOCK);
        set_screen_override(Some((5120, 2880)), ScreenSource::Argv);
        let resolved = resolve_screen(1920, 1080);
        set_screen_override(None, ScreenSource::Derived);
        assert_eq!(resolved, (5120, 2880));
    }

    #[test]
    fn parse_screen_spec_accepts_desktop_pair() {
        assert_eq!(parse_screen_spec("1920x1080").unwrap(), (1920, 1080));
    }

    #[test]
    fn parse_screen_spec_rejects_the_headless_default_shape_when_empty() {
        assert!(parse_screen_spec("").is_err());
        assert!(parse_screen_spec("0x0").is_err());
        assert!(parse_screen_spec("1920").is_err());
    }

    #[test]
    fn screen_source_tokens_are_stable() {
        for source in ScreenSource::ALL {
            // Exhaustive on purpose: a variant added to the enum stops this
            // file from compiling until its token is written down here.
            let expected = match source {
                ScreenSource::Argv => "argv",
                ScreenSource::Step => "step",
                ScreenSource::Xdg => "xdg",
                ScreenSource::Derived => "derived",
                ScreenSource::Floor => "floor",
            };
            assert_eq!(source.as_str(), expected);
        }
    }

    #[test]
    fn every_variant_is_listed_in_all() {
        let mut seen: Vec<&str> = ScreenSource::ALL.iter().map(|s| s.as_str()).collect();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(
            seen.len(),
            ScreenSource::ALL.len(),
            "ALL repeats a variant, so the token test skips one"
        );
    }
}
