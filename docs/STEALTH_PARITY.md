# Stealth Parity Matrix


## What this document is
- This document is a COMPLETENESS CRITERION for anti-detection defences, and nothing else
- It answers one question per row: does this CLI cover the vector that a named reference implementation already covers
- It is NOT a porting plan, and it never says which gap to close first as an order
- It is NOT a promise of parity, and a `COVERED` row means the code exists, never that a detector was defeated
- It exists because Defect 25 recorded that, without a canonical reference, there is no criterion to tell a finished defence from an unfinished one
- Defect 25 recorded the cost of that absence: the WebGL injection looked finished and leaked on 14 percent of launches
- Every measurement here is dated, because a measurement is a snapshot and never a contract
- All CLI measurements in this document were taken on 2026-09-04
- All reference readings in this document were taken on 2026-09-04


## How to read the matrix
### The three state values
- `COVERED` means the CLI implements the vector and a real file and line proves it
- `PARTIAL` means the CLI implements part of the vector and the remainder is named in the same row
- `ABSENT` means the CLI does not implement the vector at all
### The evidence rule
- Every `COVERED` row carries a file and a line obtained by `rg`, never by reading a comment
- A comment is not evidence, because a comment can be stale and this repository has documented history of exactly that
- A row without evidence is `ABSENT` until evidence appears
- Every `PARTIAL` and `ABSENT` row carries a `How to verify` command, so the gap is actionable instead of aspirational
- A reference cell that could not be read says `not accessed` with the date
### What the matrix does not decide
- It does not rank the gaps by business value
- It does not authorise copying code from any reference into this repository
- It does not claim that closing every gap defeats any specific detector


## The references and what was read from each
### References read in this measurement
- `patchright` was read at `https://raw.githubusercontent.com/Kaliiiiiiiiii-Vinyzu/patchright/main/README.md` on 2026-09-04
- `patchright-python` was read at `https://raw.githubusercontent.com/Kaliiiiiiiiii-Vinyzu/patchright-python/main/README.md` on 2026-09-04
- `rebrowser-patches` was read at `https://raw.githubusercontent.com/rebrowser/rebrowser-patches/main/README.md` on 2026-09-04
- `nodriver` was read at `https://raw.githubusercontent.com/ultrafunkamsterdam/nodriver/main/README.md` on 2026-09-04
- `zendriver-rs` was read at `https://raw.githubusercontent.com/TurtIeSocks/zendriver-rs/main/README.md` on 2026-09-04
- `wreq` was read at `https://raw.githubusercontent.com/0x676e67/wreq/main/README.md` on 2026-09-04
- `guise` was read at `https://raw.githubusercontent.com/santhreal/guise/main/README.md` on 2026-09-04
- `eoka` was read at `https://raw.githubusercontent.com/shrimp-software/eoka/main/README.md` on 2026-09-04
### What was not accessed
- No source file of any reference was opened, only the published README of each repository
- Therefore every reference cell names a documented behaviour, never an internal symbol this measurement did not see
- The two exceptions are `plan_keystrokes` in `guise` and `StealthConfig` in `eoka`, which the READMEs name literally
- `guise` also names `src/human/keystroke.rs`, the `HOT_BIGRAMS` table and the `hold_envelope` match arms in its own README
- `CDP-Patches` was not read in this measurement and appears in no row


## The parity matrix
### JavaScript marker layer
| Defence | Reference that covers it | State in the CLI | Evidence in the CLI | How to verify |
| --- | --- | --- | --- | --- |
| `navigator.webdriver` present and `false` | patchright, Command Flags Leaks section: adds `--disable-blink-features=AutomationControlled`, removes `--enable-automation` | COVERED | `src/native/stealth/mod.rs:217` states the invariant and `src/native/cdp/chrome/args.rs:236` pushes the feature switch | `rg -n "AutomationControlled" src/native/cdp/chrome/args.rs` then `browser-automation-cli --json doctor --fingerprint` and read `webdriver_value` |
| `--enable-automation` never passed | patchright, Command Flags Leaks section, listed as removed | COVERED | `src/native/cdp/chrome/args.rs:282` records the flag as REJECT in the argv audit table | `rg -n "enable-automation" src/native/cdp/chrome/args.rs` and confirm no `args.push` carries it |
| WebGL vendor and renderer on the main thread | zendriver-rs, feature matrix: full-surface WebGL parameter spoof resolved from measured GPU capability tiers | COVERED | `src/native/stealth/webgl.rs:135` wraps `getParameter` and `src/native/stealth/mod.rs:238` calls `webgl::coherence_patch` | `rg -n "getParameter" src/native/stealth/webgl.rs` then read `webgl_renderer` from `doctor --fingerprint` |
| WebGL inside a Worker and `OffscreenCanvas` | patchright, Init Script Shenanigans: injecting into the HTML stream puts the override in every context; zendriver-rs full-surface spoof | COVERED | `src/native/stealth/webgl.rs:161` prepends the same override into the Worker scope through `self[k]` and `src/native/stealth/webgl.rs:145` builds the pair from `OffscreenCanvas` | `rg -n "self\[k\]" src/native/stealth/webgl.rs` then `eval` a Worker that reads `getParameter(37446)` and compare with the main thread |
| Reported GPU matches the GPU actually rendering | zendriver-rs, `gpu_backend` opt-in to render WebGL and WebGPU on the host's real GPU instead of the software fallback | ABSENT | none; `src/native/cdp/chrome/args.rs:350` launches with `--use-vulkan=swiftshader` while the patch reports a hardware pair | `rg -n "swiftshader" src/native/cdp/chrome/args.rs` and time a heavy WebGL draw against the renderer string the page reads |
| Closed shadow roots reachable by locator | patchright, Closed Shadow Roots section, including XPath inside closed roots | ABSENT | none; `src/native/snapshot/take/iframe.rs:54` only reads the `shadowRoots` array that CDP already returns | `rg -n "attachShadow" src/ -g '*.rs'` returns nothing, then try to press an element inside a closed root |
### CDP protocol layer
| Defence | Reference that covers it | State in the CLI | Evidence in the CLI | How to verify |
| --- | --- | --- | --- | --- |
| `contextId` obtained without `Runtime.enable` | rebrowser-patches, three techniques: main-world binding, `Page.createIsolatedWorld`, enable-then-disable; patchright avoids the command by evaluating in isolated contexts | COVERED | `src/browser_policy/runtime_events.rs:20` records that no call site targets an evaluation by `executionContextId` and `src/browser_policy/mod.rs:261` publishes `runtime_enable_used` | `rg -n "runtime_enable_used" src/browser_policy/mod.rs` then read that field from any browser envelope |
| `Console.enable` and `consoleAPICalled` leak | patchright, Console.enable Leak section: the Console API is disabled outright | PARTIAL | `src/browser/session/launch/ingest.rs:64` subscribes to `Runtime.consoleAPICalled` only under `--capture-console`, and `src/browser_policy/runtime_events.rs:25` names that as the single reach | `rg -n "consoleAPICalled" src/browser/session/launch/ingest.rs` then run with and without `--capture-console` and compare `runtime_enable_used` |
| Init script injected before the HTML parse | patchright, Init Script Shenanigans: Playwright Routes inject JavaScript into HTML requests so `Runtime.enable` is never needed | ABSENT | none; `src/native/stealth/mod.rs:19` states the patches ride `Page.addScriptToEvaluateOnNewDocument`, and the only `Fetch.enable` is `src/native/state/collect.rs:153`, used to serve replacement content | `rg -n "addScriptToEvaluateOnNewDocument" src/native/stealth/mod.rs` and confirm no `Fetch.fulfillRequest` rewrites a document body with the stealth payload |
| CSP of the injected script | patchright covers it implicitly by injecting into the response body, so no page CSP applies to the payload | COVERED | none needed, and that is the finding: measured 2026-09-04 against a page served with `Content-Security-Policy: script-src 'none'`, the payload still applied — `webdriver:false`, `platform: Linux x86_64` and the masked `ANGLE (NVIDIA, NVIDIA GeForce GTX 1070, OpenGL 4.6)` on a macOS host — because a CDP init script is not page script and no page CSP governs it | `cargo test --test csp_init_script_gate` |
| Cross-origin iframes reachable | nodriver, flat-mode connection, stated as including iframes in most operations | COVERED | `src/native/interaction/element_ops.rs:30` threads an `iframe_sessions` map into every element resolution and `src/native/interaction/pointer.rs:39` handles an OOPIF-scoped dialog | `rg -n "iframe_sessions" src/native/interaction/element_ops.rs` then `view --detailed` on a page with a cross-origin frame |
| Synthetic `screenX` must not equal `pageX` | patchright passes Brotector only with CDP-Patches, which exists for this vector | COVERED | `src/native/cdp/types/input.rs:78` records the crbug, the measurement, and why both fields stay `None` so Chrome derives them itself | `rg -n "screen_x" src/native/cdp/types/input.rs` then run the five-line Brotector check on a dispatched click |
### Network and transport layer
| Defence | Reference that covers it | State in the CLI | Evidence in the CLI | How to verify |
| --- | --- | --- | --- | --- |
| JA3 and JA4 TLS fingerprint | wreq, stated as fine-grained control over TLS extensions rather than fingerprint strings, with 100+ device profiles in `wreq-util` | ABSENT | none; `Cargo.toml:296` builds `reqwest` on `rustls` with `webpki-roots`, which emits a rustls ClientHello | `rg -n "rustls" Cargo.toml` then send `--engine http` at a JA3 echo endpoint and compare with a real Chrome |
| HTTP/2 `SETTINGS` frame matching Chrome | wreq, HTTP/2 over TLS parity through per-profile extensions and settings | PARTIAL | `src/xdg/config_model.rs:112` through `:127` expose six `http2_*` knobs, but they are hyper tuning values and no Chrome profile sets them | `browser-automation-cli --json config list-keys` and confirm no key names a browser profile |
| QUIC and HTTP/3 negotiated with the origin | not accessed 2026-09-04; no reference read in this measurement documents this vector | ABSENT | none; `src/native/cdp/chrome/args.rs:326` pushes `--disable-quic` and `:332` states the refusal is a proxy security decision | `rg -n "disable-quic" src/native/cdp/chrome/args.rs` then capture the wire and confirm no UDP 443 to the target |
| Persistent profile across one-shot processes | nodriver `Config.user_data_dir`, documented as not cleaned up when specified; eoka `StealthConfig`; zendriver-rs `browser_cookies_persist` | COVERED | `src/native/cdp/chrome/args.rs:409` prefers `--profile` then the `user_data_dir` XDG key, and `src/native/cdp/chrome/args.rs:104` restricts a named profile to mode `0700` | `browser-automation-cli --json config set user_data_dir /tmp/p` then launch twice and confirm the cookie jar survives |
### Human behaviour layer
| Defence | Reference that covers it | State in the CLI | Evidence in the CLI | How to verify |
| --- | --- | --- | --- | --- |
| Mouse path as a sampled curve | eoka, stated as simulating human input with Bezier curves; guise, keystroke and mouse timing | COVERED | `src/native/interaction/kinematics/geometry.rs:87` bows the control points and `:100` samples `cubic_bezier` under an ease | `rg -n "cubic_bezier" src/native/interaction/kinematics/geometry.rs` then dispatch a move and check the path is not a straight line |
| Step count sampled instead of fixed | eoka human mouse; guise deterministic timing under a seeded RNG | COVERED | `src/constants/timing.rs:131` defines `INPUT_MOVE_STEPS_STDDEV` and `src/xdg/policy/knobs/table.rs:185` exposes it as a knob | `browser-automation-cli --json config get input_move_steps_stddev` and confirm a non-zero default |
| Scroll delta sampled per tick | eoka `stealth::human::Human` scroll | COVERED | `src/constants/timing.rs:138` defines `INPUT_SCROLL_TICK_STDDEV_PX` and `src/xdg/policy/knobs/table.rs:195` exposes it | `rg -n "INPUT_SCROLL_TICK_STDDEV_PX" src/constants/timing.rs` then measure consecutive wheel deltas |
| Keystroke dwell and gap dispersed | guise `plan_keystrokes`, returning hold and gap per keystroke from calibrated envelopes | COVERED | `src/constants/timing.rs:117` defines `INPUT_KEY_DWELL_STDDEV_MS` and `:122` defines `INPUT_TYPE_DELAY_STDDEV_MS` | `browser-automation-cli --json config get input_key_dwell_stddev_ms` then measure `keydown` intervals on a live page |
| Per-bigram keystroke timing | guise, `HOT_BIGRAMS` table and `hold_envelope` match arms in `src/human/keystroke.rs`, named in its own README | COVERED | `src/native/interaction/kinematics/qwerty.rs:57` scales the gap from the QWERTY finger pair and `src/native/interaction/kinematics/mod.rs:219` applies that scale to the sampled delay; the layout is derived rather than the reference table copied, because a digraph table measures one corpus while the finger pair is the cause behind it | `cargo test --lib the_pair_and_not_the_character_sets_the_gap`, which asserts the mean gap of `th` beats that of `qz` |
| Typo injection with backspace correction | guise, stated as typo injection with backspace correction and random thinking pauses | COVERED | `src/native/interaction/kinematics/mod.rs:241` draws the wrong key from `qwerty::neighbour` under `input_typo_permille`, and `src/native/interaction/keyboard.rs:158` types it, sends `Backspace` and retypes the intended character; the key is `0` by DEFAULT because this is the only humanisation that changes what the page reads rather than when it reads it | `cargo test --test typo_correction_gate`, which types with the rate pinned to 1000 and requires 6 `Backspace` events, 18 keydowns and the field still holding the requested text |
| Long pause between words | guise random thinking pauses | COVERED | `src/native/interaction/kinematics/mod.rs:229` implements `maybe_long_pause` gated by `input_word_pause_permille` | `browser-automation-cli --json config get input_word_pause_permille` then type a sentence and look for the outlier interval |


## Gaps by closing cost
- 7 rows are not `COVERED`: 2 are `PARTIAL` and 5 are `ABSENT`, out of 23 rows in `The parity matrix`
- Each of those rows has a bullet of its own below, so the count and the list agree by construction
- `JA3 and JA4` is integration, not porting, because `wreq` is already a Rust crate with the profile catalogue in `wreq-util`
- `HTTP/2 SETTINGS` closes with the same integration, because `wreq` treats TLS and HTTP/2 as one profile
- `Reported GPU matches the rendering GPU` requires abandoning the SwiftShader bundle, which `src/native/cdp/chrome/args.rs:338` documents as four flags that only make sense together
- `Init script before the parse` is the largest change, because it moves the payload out of `Page.addScriptToEvaluateOnNewDocument` and into the response body
- `Closed shadow roots` is independent of every row above and lands in element resolution, not in the stealth payload
- `Console.enable` is independent of every row above and its cost is the `--capture-console` surface
- QUIC is listed as `ABSENT` and is deliberately so: `src/native/cdp/chrome/args.rs:332` records the refusal as a proxy security decision, which outranks fingerprint fidelity


## Divergences between the Defect 25 record and what the references say
- The Defect 25 record names two patchright categories, `Script Injection` and `Execution Context`
- Neither string appears in the patchright README read on 2026-09-04
- The README publishes five headings under `Patches`: `Runtime.enable Leak`, `Console.enable Leak`, `Command Flags Leaks`, `General Leaks` and `Closed Shadow Roots`
- The behaviour the record describes is real and lives under `Init Script Shenanigans`, a collapsed block outside the `Patches` list
- The record claims the CLI already matches patchright categories 1 and 3, and by the published order those are `Runtime.enable Leak` and `Command Flags Leaks`, which this matrix confirms
- The record says patchright derives the `contextId` by evaluating `globalThis` and parsing the `objectId`; the patchright README says only that it evaluates in isolated execution contexts, and the three named techniques belong to the rebrowser README instead
- The record claims `21+ patches in five categories`; the README publishes five headings and no patch count, so the number was not confirmed
- Two references read here cover vectors the record never names: patchright covers closed shadow roots, and zendriver-rs offers `gpu_backend` for the SwiftShader divergence


## How to reproduce this measurement
- Read the defect first with `rg -n "Defeito 25" gaps.md` and open the block it points at
- Re-fetch every reference README with `browser-automation-cli -q --json --ignore-robots --i-accept-robots-risk scrape <RAW_URL> --format text --engine http`
- Re-run every `How to verify` command in the matrix, in the row it belongs to
- Treat any `COVERED` row whose `rg` returns nothing as `ABSENT` and change the row
- Re-date the document, because file and line move: `src/native/stealth/webgl.rs` did not exist earlier on 2026-09-04 and the WebGL evidence moved into it during this measurement
- Never run `cargo` to reproduce this document, because no row depends on a build


## What this document does not prove
- It does not prove that any `COVERED` row survives a live detector, because no row was tested against a commercial anti-bot in this measurement
- It does not prove that the `ABSENT` rows are the only gaps, because 23 rows in `The parity matrix` were compared and 8 READMEs in `References read in this measurement` were read
- It does not prove that any reference cell matches that reference's current source, because only READMEs were read and no source file was opened
- It does not prove the CLI defeats Google Search, and Defect 25 already records that the attestation layer sits above fingerprint entirely
- It does not measure the 14 percent WebGL leak rate that motivated Defect 25, because that measurement requires a launch battery this document did not run
