---
name: browser-automation-cli
description: This skill MUST be used when operating browser-automation-cli for Chrome CDP automation, local scraping, local media and page diagnostics. MUST activate for navigate, click, type, form submit, fill-form, storage export and import, accessibility snapshots with @eN refs, screenshots, PDF, LLM extract, multi-format scrape with rawHtml, batch-scrape, crawl, map, search, parse PDF DOCX XLSX ODS, monitor, QR, sheet-write, sg-scan, sg-rewrite, find-paths, console, network, loopback MITM, traffic capture with HAR, REST and GraphQL endpoint discovery, emulate, perf, lighthouse, screencast, heap, extensions, webmcp, workflow, multi-step run, record of replayable interactions, image info convert resize exif download, video info convert trim thumbnail manifest, audio info convert trim download. Delivers argv formulas, eight payload-reduction flags, JSON envelope, exit codes, 217 XDG keys with no environment variables, robots and residual-zero on disk.
---

# browser-automation-cli

## Zero Rule
### REQUIRED
- MUST ALWAYS invoke full binary `browser-automation-cli`
- MUST pass `--json` on EVERY programmatic call
- MUST parse ONLY stdout; pass `-q` or `--quiet` to silence stderr in pipelines
- MUST check exit code BEFORE trusting stdout
- MUST require `.ok == true` before `.data`; parse with `jaq`, NEVER `jq`
### FORBIDDEN
- NEVER invent alias `bac` or shortened binary names
- NEVER invent product environment variables or use `.env` for runtime config
- NEVER mask exit codes with `|| true`; NEVER parse stderr as JSON

## Mandatory Discovery
### REQUIRED
- MUST resolve live surface by discovery, NEVER memorized counts
- MUST run `--json commands`, `--json schema <cmd>` or `schema --cmd <cmd>`, `--json config list-keys`, `--json config path`
- MUST run `<cmd> --help` when schema is insufficient; `doctor --offline --quick` when host looks wrong
- MUST consult `references/formulas.md` for exhaustive argv
### FORBIDDEN
- NEVER invent flags absent from schema/help; NEVER invent PRD wishlist flags; NEVER invent XDG paths

## Identity and Lifecycle
### REQUIRED
- MUST treat every process as BORN → EXECUTE → FINALIZE → DIE; Chrome lives only inside that process
- MUST keep multi-step work needing surviving `@eN` refs inside ONE `run --script`
- MUST use system Chrome via discovery or `config set chrome_path`
- MUST map DevTools→CLI - click→`press`, fill→`write`, take_screenshot→`grab`, take_snapshot→`view`, type_text→`type`, press_key→`keys`, navigate_page→`goto`|`back`|`forward`|`reload`, evaluate_script→`eval`, list_network_requests→`net list`, list_console_messages→`console list`
- MUST treat `exec` as single-step only
### FORBIDDEN
- NEVER reuse `@eN` across processes; NEVER assume daemon/sticky/remote session/telemetry; NEVER call DevTools names as subcommands

## Payload Reduction (all 71 commands)
### REQUIRED
- MUST reduce with the binary's own flags, NEVER by piping stdout through `jaq`
- MUST use `--fields PATHS` to project dotted paths (CSV)
- MUST use `--filter-rows EXPR` with `key=value`, `key!=value` or `key~substring` (repeatable, ANDed)
- MUST use `--limit-rows N`, `--sort-rows PATH`, `--dedupe-by PATH`, `--count-only` on list payloads
- MUST use `--truncate-content CHARS` and `--max-output-bytes BYTES` to cap size
- MUST read `agent_ops.truncated` — it is the only signal separating a short payload from a cut one
- MUST treat a zero-match filter as an empty list with `ok: true`, never as an error
- MUST narrow with `--fields <key>` first when the error says data holds more than one list
- MUST know a missing field never matches, including under `!=`
- Measured: `doctor --offline --quick` is 26_277 bytes; `--fields residual.ghost_marker_processes` is 80
- MUST treat a reduction flag as NECESSARY but NOT SUFFICIENT for `agent_ops`, which is omitted when the flags produced nothing to report
- Measured: `--fields commands commands` returns only `data`, `ok`, `schema_version`, while adding `--limit-rows 3` adds `agent_ops` with `total`, `matched`, `truncated`
- MUST pass ONE single CSV to `--fields`; the flag is NOT repeatable
- Measured: `--fields residual --fields checks` returns `ok:false`, `error.kind` usage, exit 2
- MUST root `--fields` paths at `data`; write `residual`, NEVER `data.residual`
- Measured: `--fields data.residual` returns empty `data` with exit 0 — a SILENT wrong answer
- MUST read `agent_ops.unresolved_paths` to catch every path that resolved to nothing
- Measured: `--count-only commands` alone exits 2 with data holds more than one list
- MUST know the eight GLOBAL flags are `--fields`, `--filter-rows`, `--limit-rows`, `--sort-rows`, `--dedupe-by`, `--count-only`, `--truncate-content`, `--max-output-bytes`
- MUST treat `--select`, `--filter`, `--limit` and `--sort` as LOCAL per-command flags
- Measured: `image info --help` and `scrape --help` expose the local `--select` beside the global `--fields`, `--filter-rows` and `--limit-rows`
### FORBIDDEN
- NEVER pipe through `jaq`/`jq` to shrink a payload — that work belongs in the binary
- NEVER confuse the local `--select` family with the eight global reduction flags
- NEVER assume `agent_ops` exists just because you passed a reduction flag
### Correct Pattern
- MUST execute `browser-automation-cli --json --fields checks --filter-rows 'id=residual_disk' doctor --offline --quick`
- MUST execute `browser-automation-cli --json --fields checks --count-only doctor --offline --quick`


## Global Flags
### REQUIRED
- MUST accept global flags before or after subcommand
- MUST pass `--json`; pass `--json-steps` for one NDJSON object per `run` step
- MUST pass `--timeout <secs>` whole-process; `--step-timeout <secs>` per `run` step
- MUST pass `--max-concurrency <N>` to bound batch/crawl/CDP fan-out
- MUST pass `--artifacts-dir`, `--correlation-id`, `--plain`, `--headed` only when those controls are required
- MUST pass `--capture-console` in SAME process as `console`/console asserts; `--capture-network` with `net`
- MUST pass `--lang en` or `--lang pt-BR`; `--verbose` or `--debug` for tracing (or `config set log_level`)
- MUST pass category gates only when required - `--category-memory` (heap), `--category-extensions` (extension), `--category-third-party` (devtools3p), `--category-webmcp` (webmcp)
- MUST pass `--experimental-vision` for `click-at`; `--experimental-screencast` for `screencast`
- MUST pass `--mitm` for interception; combine with `--mitm-har|--mitm-hosts|--mitm-ca-dir|--mitm-ws|--mitm-max-body-bytes|--mitm-no-media-bodies|--mitm-redact-secrets|--mitm-no-redact-secrets` only when required
- MUST know MITM secret redaction is ON by default, so `--mitm-redact-secrets` restates it and changes nothing
- MUST pass `--mitm-no-redact-secrets` to unmask for THIS run, and `mitm redact --secrets false` to write the persistent policy that stops masking
- MUST know that asking to mask and to unmask in the same run resolves to MASKING, because the safe reading of a contradiction about secrets is to mask
- MUST know the default is ON because a capture lands on disk for an agent to read, so forgetting the flag costs a missing header while the opposite default would cost a leaked session cookie
- MUST pass `--dump-on-failure` with `--artifacts-dir` and with `--capture-console` or `--capture-network` to write console and network evidence on failure
- MUST keep those capture flags in the SAME process, because capture dies with the process
- MUST know `--allow-outside-roots` permits local reads and artifact writes OUTSIDE the allowed roots, and is explicit risk acceptance to pass only with declared intent
- MUST treat the XDG key `allowed_roots` as the normal surface for widening those roots
- MUST know stealth is ON by default and masks the automation markers a real Chrome never exposes
- MUST pass `--no-stealth` to turn the anti-detection patches off for this run
- MUST pass `--stealth-profile auto|chrome-linux|chrome-win|chrome-mac` and PREFER `auto`, which follows the host and is almost always right
- MUST pass `--stealth-seed <SEED>` to pin one identity across processes (`hardwareConcurrency`, `deviceMemory`, GPU, `history.length`, Chrome build — not UA/platform/screen)
- MUST run `browser-automation-cli --json doctor --fingerprint` to audit identity coherence
- MUST list profiles with `--stealth-profile list` or `commands --json`
- MUST know that without a seed a 50-URL crawl over 50 one-shot processes presents 50 distinct machines
- MUST pass `--proxy <URL>` (`http`, `https`, `socks5`) as the egress proxy for BOTH Chrome and the HTTP engine
- MUST pass `--proxy-bypass <HOSTS>` for hosts that skip the proxy, in Chrome bypass-list syntax
- MUST pass `--min-delay-ms <MS>` to raise the same-origin courtesy floor for this invocation only
- MUST know the effective wait is the MAXIMUM of the flag, XDG `scrape_min_delay_ms` and `Crawl-delay`
- MUST store proxy credentials with `config set proxy_username` and `config set proxy_password` in XDG, NEVER in argv, because the process table shows argv
- MUST know part of the anti-detection surface has NO flag at all and is reachable ONLY through XDG
- MUST know the `http2_*` family drives the HTTP/2 fingerprint of the `--engine http` transport and is XDG-only
- MUST tune that fingerprint with `config set http2_enabled`, `config set http2_adaptive_window`, `config set http2_initial_stream_window_size`, `config set http2_initial_connection_window_size`, `config set http2_max_header_list_size` and `config set http2_max_frame_size`
- MUST know a mismatched HTTP/2 fingerprint identifies the client as automated even when the headers look real
- MUST execute `config set stealth false` as the persistent equivalent of `--no-stealth`
- MUST execute `config set stealth_profile <PROFILE>` and `config set stealth_seed <SEED>` to persist what those flags do per process
- MUST discover the live surface with `config list-keys --json` instead of trusting any static list
- MUST pass `--input-profile human|direct`; `human` is the default
- MUST know `human` interpolates pointer trajectories, dwells between press and release and paces typing
- MUST pass `--input-seed <SEED>` so a `human` run reproduces exactly; without it the jitter comes from the OS and two runs differ
- MUST pass `--warmup` to visit the origin root before the target URL so the session already carries cookies and a referrer chain
- MUST pass `--warmup-url <URL>` to warm that URL instead of the target origin root
- MUST pass `--browser-mode auto|headless|headed` as the canonical window mode for THIS run; `--headless` and `--headed` are shorthands for two of its values, all three beat the XDG `browser_mode`, and `config set browser_mode <MODE>` writes the persistent default the flag overrides
- MUST pass `--no-xvfb` only in headed mode on Linux, to skip the private virtual display and use the current one
- MUST pass `--expect <EXPR>` with `key=value`, `key!=value` or `key~substring` to assert the emitted payload (repeatable, ANDed)
- MUST pass `--expect-exit-code` to exit 65 when any `--expect` is unmet, instead of only reporting it
- MUST know `--expect-exit-code` is off by default because changing an exit code on data content would silently break callers
### FORBIDDEN
- NEVER expect capture to survive process end; NEVER enable category/experimental gates by default; NEVER omit `--json` in agent pipelines
- NEVER pass proxy credentials in argv; NEVER claim a foreign platform in `--stealth-profile` when the host says otherwise

## XDG Config
### REQUIRED
- MUST configure ONLY via CLI flags and `config init|path|show|get|set|unset|list-keys`
- MUST discover keys with `config list-keys --json` before set; resolve paths with `config path --json`
- MUST treat CLI flags as overrides of stored values
- MUST know `config unset` is the inverse of `set`, while `config set <key> ""` is NOT
- MUST set secrets `encryption_key` and `openrouter_api_key`
- MUST set binaries `chrome_path`, `lighthouse_path`, `ffmpeg_path`
- MUST set `cache_backend` sqlite|memory|redis; Redis only plain `cache_redis_url redis://...`
- MUST set `dialog_settle_ms` for dialog settle budget; logging via `config set log_level` or `--verbose`/`--debug`
- MUST read `references/xdg-keys.md` for every XDG key with its default and description before setting any key not named here
### FORBIDDEN
- NEVER invent product env for any key; NEVER log secrets/cookies; NEVER use `rediss://`; NEVER set redis backend without URL

## Argv Contract and Surface
### REQUIRED
- MUST pass `grab --path <file>` and `grab --format png|jpeg|webp` only; pass `--quality`/`--element` only when required
- MUST pass `print-pdf --path <file>` ALWAYS; one-shot MUST also pass `--url` (blank page refused)
- MUST pass `view --detailed` for full a11y tree (argv is `--detailed`, NOT `--verbose`); run JSON accepts `verbose` or `detailed`
- MUST pass `view --allow-empty` only when blank snapshot is intentional
- MUST pass `type <TEXT>` with `--target` OR `--focus-only`
- MUST pass `fill-form --fields-json '[{"target":"@eN","value":"x"}]'`; `cookie set --cookies-json '[...]'` (NEVER payload via `--json`)
- MUST pass `submit <TARGET>` (form or field owning form); `--timeout-ms` only when non-default wait required
- MUST pass `storage export|import --path <FILE>` ALWAYS; `--url` when origin must load in-process
- MUST pass `mitm block --host`; `mitm allow --host` (host required); `mitm ws list|get`
- MUST pass `reload --ignore-cache` only on reload (NEVER goto); `goto --handle-before-unload accept|dismiss` explicit
- MUST pass `sheet-write <in> -o <out.xlsx>`; `emulate` via UA/viewport/network flags (NEVER `--device`)
- MUST pass `assert url <v> --contains` for substring; `workflow run --manifest <json>`; `--journal` only when explicit path required
- MUST pass `eval --file-path <FILE>` to write the result to a file and `--service-worker-id` to target a service worker
- MUST discover `pick`/`select-option` via commands/schema; invoke via run/exec
- MUST read the key `html` after `scrape --format html` and `rawHtml` after `--format rawHtml`
- Measured with `--engine http`: they are DISTINCT keys carrying DISTINCT payloads
### FORBIDDEN
- NEVER treat `rawHtml` as an alias of `html`; reading the wrong key returns nothing
- NEVER bare positional path on grab/print-pdf; NEVER one-shot print-pdf without `--url`; NEVER avif
- NEVER put `mitm`, `storage`, or `extension install|uninstall` inside `run`
- NEVER use `view --verbose`

## JSON Envelope and Exit Codes
### REQUIRED
- MUST expect success `schema_version`+`ok` true+`data`; failure `ok` false+`error`
- MUST expect invalid argv with `--json` as `error.kind`=`usage` exit 2
- MUST read partial `data.steps` on run failure; `matched_selector` on multi-selector wait
- MUST read lighthouse `data.binary_source` real|mock; NEVER treat mock as LHR parser validation
- MUST read `.data.dialog_settled` after real dialog accept|dismiss; when true DO NOT insert artificial wait
- MUST read the browser witness on every browser envelope: `browser_mode_requested`, `browser_mode_effective`, `browser_mode_source`, `display_backend` and `runtime_enable_used`
- MUST read `browser_mode_requested` as the mode argv or XDG asked for, before resolution
- MUST read `browser_mode_effective` as `headless` or `headed`, which is what the launch actually did
- MUST read `browser_mode_source` as `default`, `xdg` or `flag`, the precedence step that decided the mode
- MUST treat `browser_mode_source` `default` as headless by luck and NEVER as a proven requirement
- MUST read `display_backend` as `headless`, `xvfb` or `host`; only `host` can paint on the operator screen
- MUST read `runtime_enable_used` as the boolean stating whether this launch enabled the CDP Runtime domain
- MUST expect `runtime_enable_used` true the moment `--capture-console` is passed
- MUST read `serp_endpoint` on every `search` envelope as `known` or `unknown`
- MUST treat `serp_endpoint` `unknown` as a `search_base_url` that does not understand the dimension parameters
- MUST treat a `search` that found no organic result as a DECLARED failure with `ok` false and `error.kind` `data`, and NEVER as success carrying an empty list
- MUST read `serp_endpoint` and `search_base_url` under `data` on the FAILURE envelope too, because that pair is what separates an unknown endpoint from an empty web
- MUST branch exits - 0 ok, 2 usage, 65 data, 66 no-input, 69 unavailable, 70 software, 74 io, 78 config, 124 timeout, 130 cancel, 141 broken-pipe
- MUST retry only transient host/launch failures
### FORBIDDEN
- NEVER retry usage without fixing argv; NEVER treat human prose as contract

## Multi-step run Scripts
### REQUIRED
- MUST use `run --script <file>` (NDJSON lines or JSON array); every step has `cmd`
- MUST use `run --script -` to read NDJSON steps from stdin, one step per line, against one live session
- MUST treat stdin mode as still one-shot: one BORN, one DIE, EOF triggers FINALIZE
- MUST expect stdin mode to validate each line on arrival and report `validation: "per-line"`
- MUST prefer stdin over shell process substitution: `run --script <(printf ...)` is rejected by the file jail
- MUST set `--timeout` for whole script; serialize grab/print-pdf with `path`; print-pdf needs `url` or prior goto
- MUST serialize wait with `selector` CSV or `selectors` array; public key `wait_timeout_ms`
- MUST serialize scrape with `url` + `format`|`formats` (text MUST NOT dump huge html)
- MUST serialize submit with `target`; dialog with `if_present` when may be absent; scroll with `dy`/`dx`
- MUST serialize a blank view with `allow_empty`, and a detailed view in run with `verbose` or `detailed`
- MUST serialize assert `kind` in url|text|console|console_empty|console_no_match; isolated tab via `isolated_context`
- MUST keep OUT of run - meta, config, mitm, storage, workflow, crawl, map, batch-scrape, search, parse, qr, find-paths, sg-scan, sg-rewrite, sheet-write, monitor, extension install/uninstall, nested run/exec
### FORBIDDEN
- NEVER split `@eN` steps across processes; NEVER ignore partial `data.steps`
### Critical step one-liners
- `{"cmd":"goto","url":"https://example.com","handle_before_unload":"accept","navigation_timeout_ms":15000}`
- `{"cmd":"wait","selector":"h1, main, #content","wait_timeout_ms":10000}`
- `{"cmd":"view","verbose":true}` · `{"cmd":"write","target":"@e1","value":"hello"}`
- `{"cmd":"submit","target":"#user","timeout_ms":8000}` · `{"cmd":"scrape","url":"https://example.com","format":"text"}`
- `{"cmd":"pick","target":"@e1","option":"Anomaly"}` · `{"cmd":"select-option","target":"@e2","option":"High"}`
- `{"cmd":"dialog","action":"accept","if_present":true}`
- `{"cmd":"grab","path":"/tmp/p.png","format":"png"}` · `{"cmd":"print-pdf","path":"/tmp/p.pdf","url":"https://example.com"}`
- MUST confirm every key against `schema <cmd> --json` before adapting any step

## Agent-First Laws
### REQUIRED
- MUST key multi-tab dialogs by `session_id`; tab switch under open dialog is best-effort domain enable
- MUST expect native `select-option`/`pick` to dispatch input then change and report `via: native_select`
- MUST use `submit` for real form submit plus nav/request wait; `storage export` writes mode 0600 and stays OUT of run
- MUST discover surface with commands/schema; NEVER invent flags
### FORBIDDEN
- NEVER invent product env for dialog settle, logging, or robots bypass

## Residual-Zero and Robots
### REQUIRED
- MUST treat residual-zero disk as success for every browser one-shot that is the only concurrent invocation, validated by `doctor --offline --quick --json`
- MUST require `residual_disk` not `fail`; zero `orphan_marker_dirs`; zero `ghost_marker_processes`
- After DIE alone MUST expect zero `cli_marker_dirs` and zero `chromium_tmp_singleton_orphans` (`residual_disk` `pass`)
- MUST treat `sibling_live_processes > 0` as healthy concurrency (`warn`, never fail)
- MUST NOT require zero `live_cli_marker_processes` (legacy Chrome-child process count; prefer `sibling_live_processes`)
- MUST treat `config set user_data_dir <PATH>` as the explicit decision to GIVE UP residual-zero, because the profile becomes the operator's
- MUST know the key ships ABSENT, and that absence is what buys the throwaway profile a one-shot leaves nothing behind
- MUST know the sweep judges ONLY `browser-automation-cli-chrome-*` marker dirs under the scanned roots, so an operator profile is never counted and never collected
- MUST restore the default with `browser-automation-cli config unset user_data_dir`; `config set user_data_dir ""` also clears the opt-in, because whitespace-only reads as absent for THIS key
- MUST know that directory is created 0700 on Unix because it holds cookies and tokens
- MUST respect robots by default; bypass ONLY with BOTH `--ignore-robots` and `--i-accept-robots-risk`
### FORBIDDEN
- NEVER declare residual-zero without reading residual fields; NEVER mass-delete host temps; NEVER kill user/Flatpak Chrome
- NEVER fail a host solely because `live_cli_marker_processes > 0` while orphans/ghosts are zero
- NEVER bypass robots with one flag; NEVER invent robots bypass env

## Full Command Inventory
### REQUIRED
- MUST recognize all 71 - doctor, commands, schema, version, locale, goto, view, press, click-at, write, keys, type, wait, hover, drag, submit, fill-form, select-option, pick, upload, back, forward, reload, eval, grab, print-pdf, monitor, run, exec, extract, text, scroll, cookie, storage, attr, assert, console, net, page, dialog, scrape, batch-scrape, crawl, map, search, parse, qr, record, image, video, audio, find-paths, sg-scan, sg-rewrite, sheet-write, sitemap, feed, mitm, workflow, config, emulate, resize, perf, lighthouse, screencast, heap, extension, devtools3p, webmcp, completions, man
- MUST use local image pipeline for download/convert/resize/EXIF (no Chrome): `image info|convert|resize|download|exif`
- MUST keep agent-native media stdout for image, video and audio: paths, hashes, dims, codecs, duration and flags only; NEVER pixel base64 unless `grab --include-base64`, NEVER raw media frames, NEVER PCM
- MUST project with `image info --select format,width,height,sha256` to save tokens
- MUST use local video pipeline (no Chrome): `video info|download|convert|to-mp3|trim|thumbnail|manifest` with optional OS ffmpeg/ffprobe (XDG `ffmpeg_path`)
- MUST use `video manifest` to summarize an HLS/DASH manifest without downloading media
- MUST project with `video info --select container,duration_secs,streams,sha256` and convert `--select path_out,auto_reencoded,video_codec`
- MUST use local audio pipeline (no Chrome): `audio info|download|convert|trim` with optional OS ffmpeg/ffprobe (XDG `ffmpeg_path`)
- MUST project with `audio info --select format,codec,duration,bytes,sha256` and convert `--select path_out,lossy_transcode,suggestion`
- MUST NOT shell out to ffmpeg manually when `video convert` or `audio convert` can remux/re-encode (smart copy / auto re-encode); prefer `upload` for CDP file upload
- MUST set audio caps via XDG only: `audio_max_input_bytes` `audio_download_max_bytes` `audio_default_format` `audio_default_bitrate`
- MUST treat local webp encode as lossless (`quality_applied` false); jpeg honours quality
- MUST treat `--keep-exif` as intent-only (re-encode cannot re-attach EXIF; `keep_exif_honored` false)
- MUST configure image limits only via XDG `config set` (`image_*`) — never product env vars
- MUST read image text natively as an agent; the CLI ships no text-recognition action and no external C binary
- MUST treat EXIF as the only metadata surface (no IPTC/XMP); `image exif --select tags` aliases to `exif`
- MUST reject AVIF/HEIC encode; SVG has no resvg — use `--allow-non-image` only for intentional raw bytes
- MUST NOT confuse `image download` (single image URL) with a whole-site tree download
- MUST re-discover live inventory with `commands --json`


## How To Scrape
### Mandatory Sequence
- MUST choose the engine BEFORE writing any argv and start with `--engine http`, which launches no browser
- MUST switch to `--engine browser` only when the page needs JavaScript
- MUST name every wanted format in ONE `--format` CSV or repeated flags
- MUST read the `html` key after `--format html` and the `rawHtml` key after `--format rawHtml`
- MUST add `--only-main-content` to trim boilerplate before parsing
- MUST execute `browser-automation-cli --json scrape https://example.com --format markdown,links --engine http --only-main-content`
- MUST shrink the envelope with the eight global reduction flags
### Scaling Up
- MUST use `batch-scrape --urls-file` with `--concurrency` for a known URL list
- MUST use `crawl` with `--limit` and `--max-depth` when links must be followed
- MUST use `map` when only the URL inventory is wanted
- NEVER call `crawl` when `map` already answers the question
### Measured Traps
- MUST respect robots by default on every scrape family command and bypass ONLY with BOTH `--ignore-robots` and `--i-accept-robots-risk`
- NEVER bypass robots with a single flag


## How To Monitor Network Traffic
### Mandatory Sequence
- MUST decide first whether the traffic lives in this process or another
- MUST pass `--capture-network` in the SAME process that runs `net list`, which sees nothing without it
- MUST NOT call `net list` as a top-level subcommand: it refuses with exit 2, because the capture buffer dies with the process that filled it
- MUST narrow with `--page-idx`, `--page-size`, `--resource-types`, `--include-preserved`
- MUST serialize the step `{"cmd":"net","action":"list","resource_types":"Document"}` inside `run`
### The Resource-Type Filter
- MUST pass `--resource-types` as ONE comma-separated list, matched EXACTLY and case-insensitively
- MUST draw every token from the CDP vocabulary — Document, Stylesheet, Image, Media, Font, Script, TextTrack, XHR, Fetch, Prefetch, EventSource, WebSocket, Manifest, SignedExchange, Ping, CSPViolationReport, Preflight, FedCM, Other
- MUST expect an unknown token to be REFUSED with exit 2 and `error.kind` usage, naming the offending token
- MUST know the refusal lands BEFORE any Chrome launch, so a typo costs a parse and never a browser
- MUST read `resourceType` on every captured record; a request whose type CDP omitted is stored as `Other` and NEVER without the key
- MUST treat an empty result as proof the page had no such resource, because a typo can no longer reach that branch
### Buffer Ceiling and Declared Truncation
- MUST read `dropped_oldest` in the `net` and `console` envelopes; it counts records discarded to hold the buffer under its cap
- MUST reconstruct what the page really produced as `total` plus `dropped_oldest`
- MUST move that cap ONLY with `config set event_tracker_max_entries <N>`; no flag exposes it
- MUST pass `--include-preserved` on `net get` and `console get`, not only on the `list` forms, so one index addresses the SAME record on both
### Crossing Processes With MITM
- MUST run `mitm capture-url <URL>` to write a capture file
- MUST read the written location from `data.capture_path`
- MUST feed that path back with `--capture-path <FILE>` on later calls
- MUST know `--capture-path` serves `mitm list|get|domains|apis|graphql|ws`
- MUST treat `--capture-path` as the ONLY bridge between one-shot processes
- MUST execute `browser-automation-cli --timeout 60 --json mitm capture-url https://example.com --har /tmp/c.har`
- MUST then execute `browser-automation-cli --json mitm domains --capture-path <FILE>`
### Measured Traps
- Measured on example.com: `capture_count` 37 across 9 distinct hosts
- Measured: `mitm domains --capture-path` returned accounts.google.com and play.google.com
- MUST treat those hosts as Chrome background noise, never as page traffic
- MUST narrow with `--hosts` at capture time to remove that noise
- MUST treat zero endpoints as an honest answer, NEVER as a failure


## How To Interact With APIs
### Mandatory Sequence
- MUST run `goto` on the target origin BEFORE calling its API, because `eval` executes in the PAGE origin context
- MUST wrap every `fetch` in try/catch and return the error message
- MUST pass `--typed` to read `data.value` plus `data.value_type`
- Measured: `eval '({a:1,b:"x"})' --typed` returns the object with `value_type` object
- MUST execute `browser-automation-cli --json goto https://example.com` then `eval '...' --typed`
- MUST expect a returned promise to be awaited automatically
- NEVER add an await key to an `eval` step; no such key exists
- MUST take a fresh `view` after any `eval` to obtain new refs
### Carrying Authentication
- MUST use `storage export --path <FILE>` to capture authenticated state
- MUST use `storage import --path <FILE>` to restore it in the next process
- MUST keep both commands OUT of `run`
### Measured Traps
- Measured A/B: `fetch` without a prior `goto` returns `Failed to fetch`
- Measured A/B: the same `fetch` after `goto` returns `ok:200`
- Measured: a rejected promise without try/catch returns null with exit 0
- MUST treat that null as a SILENT failure, never as an empty result
- MUST know an `eval` step emits `refs_invalidated` true and kills every `@eN`
- MUST confirm every step key against `schema <cmd> --json` before serializing


## Execution Playbooks
### REQUIRED
- MUST execute formulas literally; validate envelope after each call; see `references/formulas.md` for full surface
### FORBIDDEN
- NEVER adapt by assumption without `schema <cmd> --json`

### A. Diagnostics
- `browser-automation-cli --json doctor --offline --quick` · `version` · `locale` · `commands` · `schema run` · `config list-keys` · `config unset <key>` · `man --out /tmp/browser-automation-cli.1` · `completions bash`

### B. Navigate and inspect
- `browser-automation-cli --timeout 60 --json goto https://example.com --init-script 'window.__ready=true' --handle-before-unload accept --navigation-timeout-ms 15000`
- `browser-automation-cli --json view --detailed` · `text @e1` · `attr @e1 href` · `eval 'document.title'` · `reload --ignore-cache` · `back` · `forward`

### C. Interact
- `browser-automation-cli --json press @e1 --include-snapshot` · `write @e2 "text"` · `type "hello" --target @e2 --clear --submit Enter`
- `browser-automation-cli --json submit "#user" --timeout-ms 8000` · `keys Enter` · `hover @e1` · `drag --from @e1 --to @e2` · `upload @e4 /tmp/file.txt`
- `browser-automation-cli --json wait --selector "h1, main, #content" --wait-timeout-ms 10000` · `scroll --delta-y 400` · `fill-form --fields-json '[{"target":"@e3","value":"x"}]'`
- `browser-automation-cli --json exec pick --target @e1 --option Anomaly` · `exec select-option --target @e2 --option High`
- `browser-automation-cli --experimental-vision --json click-at --x 10 --y 20`

### D. Artifacts
- `browser-automation-cli --json grab --path /tmp/p.png --format png --full-page`
- `browser-automation-cli --timeout 60 --json print-pdf --path /tmp/p.pdf --url https://example.com`
- `browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/b.baseline --write-baseline --engine http`
- `browser-automation-cli --json qr encode --text "https://example.com" --format png --path /tmp/qr.png` · `qr decode --path /tmp/qr.png`

### E. Scrape and extract
- `browser-automation-cli --json scrape https://example.com --format markdown,links,metadata --engine http --only-main-content`
- `browser-automation-cli --json scrape https://example.com --format summary --format product --format branding --engine browser`
- `browser-automation-cli --timeout 120 --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2 --engine browser`
- `browser-automation-cli --json crawl https://example.com --limit 20 --max-depth 2 --format text` · `map https://example.com --limit 50` · `search "example domain" --limit 10`
- `browser-automation-cli --json parse /tmp/doc.pdf` · `parse /tmp/sheet.ods --redact-pii`
- `browser-automation-cli --timeout 120 --json extract --llm --question "What is the title?" --schema-json /tmp/s.json https://example.com`

### F. Console and network
- `browser-automation-cli --capture-console --json console dump --path /tmp/console.json` · `assert console-empty` · `assert console-no-match --pattern TypeError`
- `console list`, `console get`, `net list` and `net get` are `run --script` steps only; the top-level forms refuse with exit 2

### G. Tabs, cookies, storage, dialogs
- `browser-automation-cli --json page new --isolated-context session-a --url https://example.com` · `page list` · `page select 0 --bring-to-front`
- `browser-automation-cli --json cookie set --cookies-json '[{"name":"a","value":"b","url":"https://example.com"}]'` · `cookie list`
- `browser-automation-cli --json storage export --path /tmp/auth.json --url https://example.com` · `storage import --path /tmp/auth.json --url https://example.com`
- `browser-automation-cli --json dialog accept --if-present` then read `.data.dialog_settled`

### H. MITM
- `browser-automation-cli --json mitm init-ca` · `mitm capture-url https://example.com --har /tmp/c.har` · `mitm block --host example.com --path /ads` · `mitm allow --host example.com` · `mitm ws list` · `mitm apis` · `mitm graphql` · `mitm har --out /tmp/c2.har`

### I. Perf and memory
- `browser-automation-cli --json emulate --user-agent "Mozilla/5.0" --viewport "390x844x3,mobile,touch" --network-conditions "Slow 3G"` · `resize --width 1280 --height 720`
- `browser-automation-cli --json perf start` · `perf stop --path /tmp/trace.json`
- `browser-automation-cli --timeout 180 --json lighthouse https://example.com --out-dir /tmp/lh --device desktop` then read `data.binary_source`
- `browser-automation-cli --category-memory --json heap take --path /tmp/s.heapsnapshot` · `heap summary --path /tmp/s.heapsnapshot` · `heap retainers --path /tmp/s.heapsnapshot --node 42`
- `browser-automation-cli --experimental-screencast --json screencast start --path /tmp/cast`

### J. Local tools
- `browser-automation-cli --json find-paths --glob '**/*.rs' .` · `sheet-write /tmp/rows.csv -o /tmp/out.xlsx --sheet Data` · `sg-scan . --limit 100` · `sg-rewrite .` then `--apply` only after dry-run review

### K. Extensions and third-party
- `browser-automation-cli --category-extensions --json extension list` · `extension install /tmp/ext` · `extension reload <ext-id>`
- `browser-automation-cli --category-third-party --json devtools3p list` · `--category-webmcp --json webmcp list`

### L. Workflow and multi-step
- `browser-automation-cli --json workflow run --manifest /tmp/wf.json --journal /tmp/wf.journal` · `workflow resume --manifest /tmp/wf.json` · `workflow status --name demo`
- `browser-automation-cli --timeout 90 --json --json-steps --capture-console run --script /tmp/steps.jsonl` · `exec goto https://example.com`

### M. Record and replay
- `browser-automation-cli --json record --url https://example.com --path /tmp/rec.jsonl --seconds 30 --max-events 200`
- MUST pass `record --url` and `record --path`, both REQUIRED; `--seconds` defaults to 30 wall-clock, `--max-events` defaults to 200 steps, and the FIRST ceiling reached stops the recording
- MUST replay the recorded NDJSON directly with `run --script /tmp/rec.jsonl`

## Absolute Prohibitions
### FORBIDDEN
- NEVER invent an alias, a product environment variable, a missing flag or a third-party product brand
- NEVER skip residual-zero after a browser one-shot
