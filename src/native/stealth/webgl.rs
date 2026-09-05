// SPDX-License-Identifier: MIT OR Apache-2.0
//! The WebGL mask, and the scopes it has to reach to be worth anything.
//!
//! Split out of `script.rs` because the patch stopped being one wrap of one
//! prototype: covering worker scopes needs a second script, built in the page
//! and shipped through a constructor shim, and the two halves only make sense
//! read together.
//!
//! # Why a window-only wrap was never enough
//!
//! `Page.addScriptToEvaluateOnNewDocument` reaches documents. A `Worker` has
//! its OWN global scope, its own `WebGLRenderingContext`, and no document —
//! nothing injected into `window` exists there. The only door into it is the
//! script the worker itself runs, which means the mask has to be carried in
//! by whoever constructs the worker.
//!
//! # What the leak actually was
//!
//! Measured 2026-09-04, 10 launches against a local fixture that reads the
//! renderer through six paths, with `--stealth-profile chrome-linux` on a
//! macOS host:
//!
//! - `canvas`, `canvas` WebGL2, main-thread `OffscreenCanvas`,
//!   `WEBGL_debug_renderer_info` via `getExtension`, and a pristine
//!   `about:blank` iframe realm: 0 of 10 leaked. Those paths were already
//!   covered, and the iframe one is covered by CDP, not by this file.
//! - A worker built from a `blob:` URL: 10 of 10 reported the REAL host GPU,
//!   `ANGLE (Apple, ANGLE Metal Renderer: Apple M1 Max, Unspecified Version)`,
//!   while the page claimed Linux. Same for a `{type:"module"}` worker.
//! - A classic same-origin worker: 8 of 10 disagreed with the window. It gets
//!   the crate's own worker patch, so it reports the crate CONSTANT — the raw
//!   `NVIDIA A100-PCIE-40GB/PCIe/SSE2` / `llvmpipe` spelling this module
//!   exists to rewrite — instead of the string the window reports. Twice out
//!   of ten it reported `ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device
//!   (Subzero) (0x0000C0DE)), SwiftShader driver)`: the raw rasteriser, which
//!   is the exact value the defect report recorded as an intermittent
//!   window-side leak. It was never intermittent and never window-side.
//! - `navigator.platform` inside every worker read `MacIntel`, 10 of 10, next
//!   to a window claiming `Linux x86_64`.
//!
//! The crate's own `unified_worker_override` explains the shape: it wraps
//! `Worker`, then bails out for `{type:"module"}`, for any scheme that is not
//! `http:`/`https:`, and for cross-origin. `blob:` is a scheme, so a bundler's
//! worker — the common case in 2026 — walks straight past it.
//!
//! # Why the pair is computed once and carried, not recomputed per scope
//!
//! Vendor, renderer and adapter are cross-checked together, so two scopes
//! answering two different strings is worse than one honest answer. A worker
//! re-running the fallback logic on its OWN raw values would land on a
//! different row of the table whenever its raw value differs from the
//! window's. So the window resolves the pair once and the literal travels
//! into the worker.

/// The mask, for both the window and every worker scope it can reach.
///
/// # Why macOS no longer returns an empty patch
///
/// The RESPELLING is still platform-gated: Chrome on macOS reports through
/// Metal and this product has not measured that spelling, so rewriting it
/// would trade a known incoherence for an unknown one. Propagating the
/// window's own answer into worker scopes is a different job and is correct
/// on every platform — it removes a contradiction rather than inventing a
/// value — so it runs unconditionally.
///
/// # What was measured about the format, 16 launches of 0.1.9
///
/// 13 of 16 carried NO `ANGLE (` prefix while `navigator.platform` said
/// `Linux x86_64`. Chrome on Linux ALWAYS reports through ANGLE, so the
/// native GL driver spelling is a stronger signal than the value it hides.
/// 2 exposed a software rasteriser, 2 drew a datacenter accelerator. The
/// crate's `GpuProfile` is an ATOMIC row — vendor, renderer, canvas format
/// and `hardware_concurrency` are drawn together because detectors
/// cross-check "strong GPU, few cores" — so the model is PRESERVED and only
/// the format is rewritten. Substitution happens only where the drawn model
/// is already impossible on its own, and the replacement is chosen from
/// `hardwareConcurrency` so the pair stays correlated rather than random.
#[must_use]
pub fn coherence_patch(navigator_platform: &str) -> String {
    let backend = if navigator_platform.contains("Win") {
        "'Direct3D11 vs_5_0 ps_5_0, D3D11'"
    } else if navigator_platform.contains("Linux") {
        "'OpenGL 4.6'"
    } else {
        // Falsy in JS: `fix` becomes the identity and the window wrap is
        // skipped, while the worker propagation below still runs.
        "null"
    };
    PATCH.replace("__BK__", backend)
}

/// The patch source, with `__BK__` standing in for the ANGLE backend literal.
///
/// Written as a template rather than a `format!` because the body is dense
/// JavaScript: doubling every brace to satisfy the formatter would make the
/// only readable copy of this logic unreadable.
const PATCH: &str = r#"(function(){try{
var VE=37445,RE=37446,BK=__BK__;
var SOFT=/SwiftShader|llvmpipe|softpipe|Software Rasterizer|Microsoft Basic Render/i;
var DC=/(Tesla|A100|H100|V100|L40|MI[0-9][0-9]+|RTX A[0-9][0-9][0-9][0-9])|(^|[^A-Za-z0-9])(T4|L4|A10)([^A-Za-z0-9]|$)/i;
var FB=[["NVIDIA","NVIDIA GeForce GTX 1650"],["Intel","Mesa Intel(R) UHD Graphics 620 (KBL GT2)"],["AMD","AMD Radeon RX 6600"]];
var HC=0;try{HC=navigator.hardwareConcurrency|0;}catch(_){}
var PLAT="";try{PLAT=String(navigator.platform);}catch(_){}
function sv(v,r){var s=String(v||"")+" "+String(r||"");
if(/NVIDIA/i.test(s))return "NVIDIA";
if(/Intel/i.test(s))return "Intel";
if(/AMD|Radeon|ATI/i.test(s))return "AMD";
return "Intel";}
function clean(r){return String(r||"").replace(/\/PCIe\/SSE2/g,"").replace(/\s+$/,"");}
function unwrap(r){var t=String(r||"");
if(t.indexOf("ANGLE (")!==0||t.charAt(t.length-1)!==")")return t;
var p=t.slice(7,-1).split(", ");
if(p.length>1)p.shift();
if(p.length>1&&/^(OpenGL|OpenGL ES|Direct3D|Metal|Vulkan)\b/.test(p[p.length-1]))p.pop();
return p.join(", ");}
function fix(v,r){
if(!BK)return [String(v),String(r)];
var vendor=v,rend=unwrap(r);
if(SOFT.test(rend)||DC.test(rend)){
var i=(HC||8)%FB.length;vendor=FB[i][0];rend=FB[i][1];}
var s=sv(vendor,rend),d=clean(rend);
return ["Google Inc. ("+s+")","ANGLE ("+s+", "+d+", "+BK+")"];}
var MASK=null;try{MASK=new WeakMap();}catch(_){}
try{if(MASK){var FT=Function.prototype.toString;
var T=function toString(){
try{var n=MASK.get(this);if(n!==undefined)return "function "+n+"() { [native code] }";}catch(_){}
return FT.call(this);};
MASK.set(T,"toString");
Object.defineProperty(Function.prototype,"toString",{value:T,configurable:true,writable:true});}}catch(_){}
function nat(f,n){try{
if(MASK)MASK.set(f,n);
Object.defineProperty(f,"name",{value:n,configurable:true});
}catch(_){}return f;}
var PAIR=null;
function wrap(p){if(!p||!p.getParameter)return;var o=p.getParameter;
if(!PAIR){var d=Object.create(p);try{PAIR=fix(o.call(d,VE),o.call(d,RE));}catch(_){}}
if(!BK)return;
var g=nat(function getParameter(x){
if(x===VE||x===RE){var c=PAIR||fix(o.call(this,VE),o.call(this,RE));return x===VE?c[0]:c[1];}
return o.call(this,x);},"getParameter");
Object.defineProperty(p,"getParameter",{value:g,configurable:true,writable:true});}
if(typeof WebGLRenderingContext!=="undefined")wrap(WebGLRenderingContext.prototype);
if(typeof WebGL2RenderingContext!=="undefined")wrap(WebGL2RenderingContext.prototype);
function pairNow(){if(PAIR)return PAIR;
try{var cv=(typeof OffscreenCanvas!=="undefined")?new OffscreenCanvas(1,1):document.createElement("canvas");
var g=cv.getContext("webgl")||cv.getContext("webgl2");
if(g){g.getExtension("WEBGL_debug_renderer_info");PAIR=fix(g.getParameter(VE),g.getParameter(RE));}}catch(_){}
if(!PAIR){var i=(HC||8)%FB.length;PAIR=fix(FB[i][0],FB[i][1]);}
return PAIR;}
var PRE=null;
function pre(){if(PRE)return PRE;var c=pairNow();
PRE="try{var V="+JSON.stringify(c[0])+",R="+JSON.stringify(c[1])+";"
+"var M=null;try{M=new WeakMap()}catch(_){}"
+"try{if(M){var FT=Function.prototype.toString;"
+"var T=function toString(){try{var n=M.get(this);if(n!==undefined)return 'function '+n+'() { [native code] }'}catch(_){}return FT.call(this)};"
+"M.set(T,'toString');"
+"Object.defineProperty(Function.prototype,'toString',{value:T,configurable:true,writable:true})}}catch(_){}"
+"var N=function(f,n){try{if(M)M.set(f,n);"
+"Object.defineProperty(f,'name',{value:n,configurable:true});}catch(_){}return f};"
+"['WebGLRenderingContext','WebGL2RenderingContext'].forEach(function(k){"
+"var p=self[k]&&self[k].prototype;if(!p||!p.getParameter)return;var o=p.getParameter;"
+"var g=N(function getParameter(x){return x===37445?V:(x===37446?R:o.call(this,x))},'getParameter');"
+"Object.defineProperty(p,'getParameter',{value:g,configurable:true,writable:true});});"
+"try{Object.defineProperty(navigator,'hardwareConcurrency',{get:function(){return "+HC+"},configurable:true});}catch(_){}"
+"try{Object.defineProperty(navigator,'platform',{get:function(){return "+JSON.stringify(PLAT)+"},configurable:true});}catch(_){}"
+"}catch(_){}";
return PRE;}
function bu(src){return URL.createObjectURL(new Blob([src],{type:"text/javascript"}));}
function shim(u,mod){var U;
try{U=new URL(String(u),self.location.href);}catch(_){return null;}
if((U.protocol==="http:"||U.protocol==="https:")&&U.origin!==self.location.origin)return null;
var abs=U.toString();
if(mod)return bu("import "+JSON.stringify(bu("(function(){"+pre()+"})();"))+";\nimport "+JSON.stringify(abs)+";\n");
return bu("(function(){"+pre()+"})();\ntry{importScripts("+JSON.stringify(abs)+")}catch(_){}\n");}
function mk(C,nm){var W=function(u,o){var s=null;
try{s=shim(u,!!(o&&typeof o==="object"&&o.type==="module"));}catch(_){}
return s===null?new C(u,o):new C(s,o);};
try{W.prototype=C.prototype;}catch(_){}
return nat(W,nm);}
try{if(typeof Worker==="function")window.Worker=mk(Worker,"Worker");}catch(_){}
try{if(typeof SharedWorker==="function")window.SharedWorker=mk(SharedWorker,"SharedWorker");}catch(_){}
}catch(e){}})();"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_backend_placeholder_is_always_substituted() {
        for platform in ["Linux x86_64", "Win32", "MacIntel"] {
            let js = coherence_patch(platform);
            assert!(!js.contains("__BK__"), "{platform} kept the placeholder");
        }
    }

    #[test]
    fn linux_and_windows_spell_their_own_angle_backend() {
        assert!(coherence_patch("Linux x86_64").contains("'OpenGL 4.6'"));
        assert!(coherence_patch("Win32").contains("'Direct3D11 vs_5_0 ps_5_0, D3D11'"));
    }

    /// macOS keeps the crate's window value and still gets worker coverage.
    ///
    /// The old patch returned an EMPTY string here, which meant a mac profile
    /// shipped no worker propagation at all — measured 2026-09-04, every
    /// worker then read the host `navigator.platform` and the host GPU.
    #[test]
    fn mac_skips_the_respelling_but_keeps_the_worker_shim() {
        let js = coherence_patch("MacIntel");
        assert!(js.contains("BK=null"), "mac must not respell the renderer");
        assert!(!js.is_empty());
        assert!(js.contains("window.Worker=mk(Worker"));
        assert!(js.contains("SharedWorker"));
    }

    /// The scopes a window-only wrap cannot reach.
    #[test]
    fn the_patch_reaches_the_worker_scope_by_its_own_globals() {
        let js = coherence_patch("Linux x86_64");
        // `self[k]`, not `window[k]`: there is no `window` inside a worker.
        assert!(js.contains("self[k]&&self[k].prototype"));
        assert!(js.contains("importScripts"));
        // Module workers get two STATIC imports so the patch module is
        // evaluated before the real one; a dynamic import would run after.
        assert!(js.contains("import \"+JSON.stringify(bu("));
    }

    /// Both prototypes, in the window and in the worker.
    #[test]
    fn both_webgl_prototypes_are_covered_in_both_scopes() {
        let js = coherence_patch("Linux x86_64");
        // Window: one `typeof` guard plus one `wrap` call per prototype.
        assert!(js.contains(
            "typeof WebGLRenderingContext!==\"undefined\")wrap(WebGLRenderingContext.prototype)"
        ));
        assert!(js.contains(
            "typeof WebGL2RenderingContext!==\"undefined\")wrap(WebGL2RenderingContext.prototype)"
        ));
        // Worker: both names in the list the pre-script iterates.
        assert!(js.contains("['WebGLRenderingContext','WebGL2RenderingContext']"));
    }

    /// A wrapper whose `toString` shows source is a tell by itself.
    ///
    /// An own `toString` property on the wrapper does NOT close this:
    /// `Function.prototype.toString.call(fn)` reaches the prototype method
    /// directly and never consults the own property. Measured 2026-09-04:
    /// with the own-property mask in place, 10 of 10 launches still handed
    /// the wrapper's source to that call. The own property is also a tell in
    /// itself, since a native method has none — so the mask lives on
    /// `Function.prototype.toString`, keyed by a `WeakMap`, and the
    /// replacement reports itself as native through the same map.
    #[test]
    fn every_installed_function_reports_as_native() {
        let js = coherence_patch("Linux x86_64");
        assert!(js.contains("[native code]"));
        // Both scopes patch the prototype method, not the own property.
        assert_eq!(js.matches("Function.prototype.toString").count(), 2);
        assert_eq!(js.matches("new WeakMap()").count(), 2);
        assert!(
            !js.contains("Object.defineProperty(f,\"toString\""),
            "an own toString property is itself a marker a native method lacks"
        );
        // The mask is applied to the getParameter wrap AND to the Worker /
        // SharedWorker constructors, which the crate left announcing their
        // own source.
        assert!(js.contains("nat(W,nm)"));
        assert!(js.contains("\"getParameter\")"));
    }

    /// One pair for every scope, or the mask creates an impossible machine.
    #[test]
    fn the_worker_receives_the_window_pair_as_a_literal() {
        let js = coherence_patch("Linux x86_64");
        assert!(js.contains("var c=pairNow()"));
        assert!(js.contains("var V=\"+JSON.stringify(c[0])"));
        // hardwareConcurrency and platform travel with it: a worker that
        // answers the host values next to a masked window is the same
        // contradiction the renderer mismatch was.
        assert!(js.contains("hardwareConcurrency"));
        assert!(js.contains("'platform'"));
    }

    /// Cross-origin workers are left alone ON PURPOSE.
    ///
    /// `importScripts` of a cross-origin script from a blob worker needs CORS
    /// the site did not have to grant. Shimming it would replace a leak with
    /// a broken page, so the constructor is delegated untouched and the gap
    /// is stated rather than hidden.
    #[test]
    fn cross_origin_workers_are_delegated_untouched() {
        let js = coherence_patch("Linux x86_64");
        assert!(js.contains("U.origin!==self.location.origin)return null"));
    }
}
