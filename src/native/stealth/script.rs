// SPDX-License-Identifier: MIT OR Apache-2.0
//! Product-owned patches that the emulation crate does not cover.
//!
//! # Why `navigator.webdriver` stays present
//!
//! A real Chrome always defines `webdriver` on `Navigator.prototype`. The
//! value is the boolean `false` when the session is not automated. Deleting
//! the property makes `'webdriver' in navigator` return `false` and
//! `String(navigator.webdriver)` return `undefined` — an anomaly no genuine
//! Chrome produces. A detector that tests *presence* therefore scores a
//! deleted property as more automated than the unpatched `false`.
//!
//! The patch defines the getter as `false` and leaves the property in place.

/// Patches applied before the crate's emulation payload.
///
/// `navigator_platform` is interpolated so a foreign `--stealth-profile`
/// cannot leave `navigator.platform` on the host OS while the User-Agent
/// claims another one.
pub fn product_patches(navigator_platform: &str) -> String {
    let platform = js_string_literal(navigator_platform);
    format!(
        r#"(()=>{{try{{
const proto = Object.getPrototypeOf(navigator);
if (proto) {{
  Object.defineProperty(proto, 'webdriver', {{
    get: function() {{ return false; }},
    configurable: true,
    enumerable: true
  }});
}}
try {{ delete navigator.webdriver; }} catch (_) {{}}
}}catch(_){{}}
try{{
const c = window.chrome || {{}};
if (!c.runtime) {{
  Object.defineProperty(c, 'runtime', {{
    value: {{ id: undefined, connect: function connect(){{}}, sendMessage: function sendMessage(){{}} }},
    enumerable: true, configurable: true
  }});
}}
Object.defineProperty(window, 'chrome', {{ value: c, writable: true, enumerable: true, configurable: true }});
}}catch(_){{}}
try{{
Object.defineProperty(navigator, 'platform', {{ get: function() {{ return {platform}; }}, configurable: true }});
}}catch(_){{}}
}})();"#
    )
}

/// Replace the Linux crate bug that recurses through wrapped `getImageData`.
///
/// The crate saves `getImageData` then, on Linux only, calls
/// `t.getImageData(...)` from `noisify` *after* wrapping the prototype.
/// Mac and Windows already use `getImageData.apply(t,[...])`.
#[must_use]
pub fn sanitize_crate_canvas(emulated: &str) -> String {
    emulated.replace(
        "a=t.getImageData(0,0,r,n)",
        "a=getImageData.apply(t,[0,0,r,n])",
    )
}

/// Restore native canvas methods after the crate payload.
///
/// `Fingerprint::NativeGPU` already chose the host GPU. A broken noise wrap
/// is worse than an honest hash: `toDataURL` must return a PNG, not throw.
#[must_use]
pub fn canvas_restore_native() -> String {
    r#"(function(){try{
var d=document.documentElement||document.head||document.body;
if(!d)return;
var f=document.createElement('iframe');
f.setAttribute('style','display:none');
d.appendChild(f);
var w=f.contentWindow;
if(!w){f.remove();return;}
var nT=w.HTMLCanvasElement.prototype.toDataURL;
var nB=w.HTMLCanvasElement.prototype.toBlob;
var nG=w.CanvasRenderingContext2D.prototype.getImageData;
f.remove();
Object.defineProperty(HTMLCanvasElement.prototype,'toDataURL',{value:function(){return nT.apply(this,arguments);},configurable:true});
Object.defineProperty(HTMLCanvasElement.prototype,'toBlob',{value:function(){return nB.apply(this,arguments);},configurable:true});
Object.defineProperty(CanvasRenderingContext2D.prototype,'getImageData',{value:function(){return nG.apply(this,arguments);},configurable:true});
}catch(_){}})();"#
        .to_string()
}

/// Re-apply `navigator.platform` after the crate payload, which may overwrite it.
pub fn platform_patch(navigator_platform: &str) -> String {
    let platform = js_string_literal(navigator_platform);
    format!(
        r#"(()=>{{try{{Object.defineProperty(navigator,'platform',{{get:function(){{return {platform};}},configurable:true}});}}catch(_){{}}}})();"#
    )
}

fn js_string_literal(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('\'', "\\'");
    format!("'{escaped}'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webdriver_stays_present_and_false() {
        let patches = product_patches("Win32");
        assert!(
            patches.contains("return false"),
            "webdriver getter must return false"
        );
        assert!(
            !patches.contains("delete proto.webdriver"),
            "deleting the property is the tell the patch exists to avoid"
        );
        assert!(patches.contains("webdriver"));
        assert!(patches.contains("'webdriver'"));
    }

    #[test]
    fn platform_is_the_identity_spelling() {
        let patches = product_patches("Win32");
        assert!(patches.contains("'Win32'"));
        let after = platform_patch("MacIntel");
        assert!(after.contains("'MacIntel'"));
    }

    #[test]
    fn every_patch_is_individually_guarded() {
        let patches = product_patches("Linux x86_64");
        assert!(patches.matches("try{").count() >= 3);
    }

    /// Window geometry moved to `chrome_geometry`, which owns the whole set.
    ///
    /// The old conditional here answered `outerHeight = innerHeight` whenever
    /// headless reported zero, which is a browser with no title bar. Leaving it
    /// in place would fight the new patch for the same property.
    #[test]
    fn product_patches_no_longer_own_the_window_geometry() {
        let patches = product_patches("Linux x86_64");
        assert!(!patches.contains("outerHeight"));
        assert!(!patches.contains("outerWidth"));
    }

    #[test]
    fn platform_literal_escapes_quotes() {
        assert_eq!(js_string_literal("O'Reilly"), "'O\\'Reilly'");
    }

    #[test]
    fn sanitize_rewrites_the_linux_recursive_getimagedata() {
        let buggy = "a=t.getImageData(0,0,r,n)";
        let fixed = sanitize_crate_canvas(buggy);
        assert!(!fixed.contains("t.getImageData(0,0"));
        assert!(fixed.contains("getImageData.apply(t,[0,0,r,n])"));
    }

    #[test]
    fn canvas_restore_mentions_native_methods() {
        let js = canvas_restore_native();
        assert!(js.contains("toDataURL"));
        assert!(js.contains("getImageData"));
        assert!(js.contains("iframe"));
    }
}
