---
name: browser-automation-cli
description: This skill MUST be used when operating browser-automation-cli for Chrome CDP automation, local scraping, and page diagnostics. MUST activate for navigate, click, type, form submit, fill-form, storage export/import, accessibility snapshots with @eN refs, screenshots png jpeg webp, PDF, LLM extract, multi-format scrape, batch-scrape, crawl, map, parse PDF DOCX XLSX ODS, monitor, QR, sheet-write, sg-scan, sg-rewrite, find-paths, console, network, loopback MITM, emulate, perf, lighthouse binary_source, screencast, heap, extensions, webmcp, workflow, multi-step run with wait_timeout_ms and scrape format, dialogs with dialog_settled and multi-tab. Delivers argv formulas, JSON envelope, exit codes, XDG config via CLI flags only, robots and residual-zero rules.
---

# browser-automation-cli

## Zero Rule
### REQUIRED
- MUST ALWAYS invoke full binary `browser-automation-cli`
- MUST pass `--json` on EVERY programmatic call
- MUST parse ONLY stdout; pass `-q` to silence stderr in pipelines
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
- MUST pass `--mitm` for interception; combine with `--mitm-har|--mitm-hosts|--mitm-ca-dir|--mitm-ws|--mitm-max-body-bytes|--mitm-no-media-bodies|--mitm-redact-secrets` only when required
### FORBIDDEN
- NEVER expect capture to survive process end; NEVER enable category/experimental gates by default; NEVER omit `--json` in agent pipelines

## XDG Config
### REQUIRED
- MUST configure ONLY via CLI flags and `config init|path|show|get|set|list-keys`
- MUST discover keys with `config list-keys --json` before set; resolve paths with `config path --json`
- MUST treat CLI flags as overrides of stored values
- MUST set secrets `encryption_key`, `openrouter_api_key`; binaries `chrome_path`, `lighthouse_path`, `ffmpeg_path`
- MUST set `cache_backend` sqlite|memory|redis; Redis only plain `cache_redis_url redis://...`
- MUST set `dialog_settle_ms` for dialog settle budget; logging via `config set log_level` or `--verbose`/`--debug`
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
- MUST discover `pick`/`select-option` via commands/schema; invoke via run/exec
### FORBIDDEN
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
- MUST branch exits - 0 ok, 2 usage, 65 data, 66 no-input, 69 unavailable, 70 software, 74 io, 78 config, 124 timeout, 130 cancel, 141 broken-pipe
- MUST retry only transient host/launch failures
### FORBIDDEN
- NEVER retry usage without fixing argv; NEVER treat human prose as contract

## Multi-step run Scripts
### REQUIRED
- MUST use `run --script <file>` (NDJSON lines or JSON array); every step has `cmd`
- MUST set `--timeout` for whole script; serialize grab/print-pdf with `path`; print-pdf needs `url` or prior goto
- MUST serialize wait with `selector` CSV or `selectors` array; public key `wait_timeout_ms`
- MUST serialize scrape with `url` + `format`|`formats` (text MUST NOT dump huge html)
- MUST serialize submit with `target`; dialog with `if_present` when may be absent; scroll with `dy`/`dx`
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

## Agent-First Laws
### REQUIRED
- MUST read `.data.dialog_settled` after real `dialog accept|dismiss`; when true DO NOT artificial-wait before next page observation
- MUST key multi-tab dialogs by `session_id`; tab switch under open dialog is best-effort domain enable
- MUST set settle via `config set dialog_settle_ms` only (XDG)
- MUST use `wait_timeout_ms` on run wait; `format`|`formats` on run scrape; grab only png|jpeg|webp
- MUST treat lighthouse `binary_source` real|mock honestly; native select reports `via: native_select` after input+change
- MUST use `submit` for real form submit+nav/request wait; storage ALWAYS requires `--path`; MUST pass `--url` only when the origin must load in the same process; export mode 0600; keep storage OUT of run
- MUST discover surface with commands/schema; NEVER invent flags
### FORBIDDEN
- NEVER invent product env for dialog settle, logging, or robots bypass

## Residual-Zero and Robots
### REQUIRED
- MUST treat residual-zero disk as success for every browser one-shot
- MUST validate `doctor --offline --quick --json` residual fields and `residual_disk` check
- MUST require zero `cli_marker_dirs`, `chromium_tmp_singleton_orphans`, `live_cli_marker_processes`
- MUST respect robots by default; bypass ONLY with BOTH `--ignore-robots` and `--i-accept-robots-risk`
### FORBIDDEN
- NEVER declare residual-zero without reading residual fields; NEVER mass-delete host temps; NEVER kill user/Flatpak Chrome
- NEVER bypass robots with one flag; NEVER invent robots bypass env

## Full Command Inventory
### REQUIRED
- MUST recognize all 65 - doctor, commands, schema, version, locale, goto, view, press, click-at, write, keys, type, wait, hover, drag, submit, fill-form, select-option, pick, upload, back, forward, reload, eval, grab, print-pdf, monitor, run, exec, extract, text, scroll, cookie, storage, attr, assert, console, net, page, dialog, scrape, batch-scrape, crawl, map, search, parse, qr, find-paths, sg-scan, sg-rewrite, sheet-write, mitm, workflow, config, emulate, resize, perf, lighthouse, screencast, heap, extension, devtools3p, webmcp, completions, man
- MUST re-discover live inventory with `commands --json`

## Execution Playbooks
### REQUIRED
- MUST execute formulas literally; validate envelope after each call; see `references/formulas.md` for full surface
### FORBIDDEN
- NEVER adapt by assumption without `schema <cmd> --json`

#### A. Diagnostics
- `browser-automation-cli --json doctor --offline --quick` · `version` · `locale` · `commands` · `schema run` · `config list-keys` · `man --out /tmp/browser-automation-cli.1` · `completions bash`

#### B. Navigate and inspect
- `browser-automation-cli --timeout 60 --json goto https://example.com --init-script 'window.__ready=true' --handle-before-unload accept --navigation-timeout-ms 15000`
- `browser-automation-cli --json view --detailed` · `text @e1` · `attr @e1 href` · `eval 'document.title'` · `reload --ignore-cache` · `back` · `forward`

#### C. Interact
- `browser-automation-cli --json press @e1 --include-snapshot` · `write @e2 "text"` · `type "hello" --target @e2 --clear --submit Enter`
- `browser-automation-cli --json submit "#user" --timeout-ms 8000` · `keys Enter` · `hover @e1` · `drag --from @e1 --to @e2` · `upload @e4 /tmp/file.txt`
- `browser-automation-cli --json scroll --delta-y 400` · `fill-form --fields-json '[{"target":"@e3","value":"x"}]'`
- `browser-automation-cli --json exec pick --target @e1 --option Anomaly` · `exec select-option --target @e2 --option High`
- `browser-automation-cli --experimental-vision --json click-at --x 10 --y 20`

#### D. Artifacts
- `browser-automation-cli --json grab --path /tmp/p.png --format png --full-page`
- `browser-automation-cli --timeout 60 --json print-pdf --path /tmp/p.pdf --url https://example.com`
- `browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/b.baseline --write-baseline --engine http`
- `browser-automation-cli --json qr encode --text "https://example.com" --format png --path /tmp/qr.png` · `qr decode --path /tmp/qr.png`

#### E. Scrape and extract
- `browser-automation-cli --json scrape https://example.com --format markdown,links,metadata --engine http --only-main-content`
- `browser-automation-cli --json scrape https://example.com --format summary --format product --format branding --engine browser`
- `browser-automation-cli --timeout 120 --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2 --engine browser`
- `browser-automation-cli --json crawl https://example.com --limit 20 --max-depth 2 --format text` · `map https://example.com --limit 50` · `search "example domain" --limit 10`
- `browser-automation-cli --json parse /tmp/doc.pdf` · `parse /tmp/sheet.ods --redact-pii`
- `browser-automation-cli --timeout 120 --json extract --llm --question "What is the title?" --schema-json /tmp/s.json https://example.com`

#### F. Console and network
- `browser-automation-cli --capture-console --json console list` · `console dump --path /tmp/console.json` · `assert console-empty` · `assert console-no-match --pattern TypeError`
- `browser-automation-cli --capture-network --json net list` · `net get 0`

#### G. Tabs, cookies, storage, dialogs
- `browser-automation-cli --json page new --isolated-context session-a --url https://example.com` · `page list` · `page select 0 --bring-to-front`
- `browser-automation-cli --json cookie set --cookies-json '[{"name":"a","value":"b","url":"https://example.com"}]'` · `cookie list`
- `browser-automation-cli --json storage export --path /tmp/auth.json --url https://example.com` · `storage import --path /tmp/auth.json --url https://example.com`
- `browser-automation-cli --json dialog accept --if-present` then read `.data.dialog_settled`

#### H. MITM
- `browser-automation-cli --json mitm init-ca` · `mitm capture-url https://example.com --har /tmp/c.har` · `mitm block --host example.com --path /ads` · `mitm allow --host example.com` · `mitm ws list` · `mitm apis` · `mitm graphql` · `mitm har --out /tmp/c2.har`

#### I. Perf and memory
- `browser-automation-cli --json emulate --user-agent "Mozilla/5.0" --viewport "390x844x3,mobile,touch" --network-conditions "Slow 3G"` · `resize --width 1280 --height 720`
- `browser-automation-cli --json perf start` · `perf stop --path /tmp/trace.json`
- `browser-automation-cli --timeout 180 --json lighthouse https://example.com --out-dir /tmp/lh --device desktop` then read `data.binary_source`
- `browser-automation-cli --category-memory --json heap take --path /tmp/s.heapsnapshot` · `heap summary --path /tmp/s.heapsnapshot` · `heap retainers --path /tmp/s.heapsnapshot --node 42`
- `browser-automation-cli --experimental-screencast --json screencast start --path /tmp/cast`

#### J. Local tools
- `browser-automation-cli --json find-paths --glob '**/*.rs' .` · `sheet-write /tmp/rows.csv -o /tmp/out.xlsx --sheet Data` · `sg-scan . --limit 100` · `sg-rewrite .` then `--apply` only after dry-run review

#### K. Extensions and third-party
- `browser-automation-cli --category-extensions --json extension list` · `extension install /tmp/ext` · `extension reload <ext-id>`
- `browser-automation-cli --category-third-party --json devtools3p list` · `--category-webmcp --json webmcp list`

#### L. Workflow and multi-step
- `browser-automation-cli --json workflow run --manifest /tmp/wf.json --journal /tmp/wf.journal` · `workflow resume --manifest /tmp/wf.json` · `workflow status --name demo`
- `browser-automation-cli --timeout 90 --json --json-steps --capture-console run --script /tmp/steps.jsonl` · `exec goto https://example.com`

## Absolute Prohibitions
### FORBIDDEN
- NEVER invent alias `bac` or product environment variables
- NEVER invent missing flags or forbidden third-party product brands
- NEVER treat mock lighthouse as real LHR validation; NEVER avif on grab
- NEVER put mitm/storage/extension install|uninstall inside run; NEVER treat exec as multi-step
- NEVER reuse `@eN` after DIE; NEVER bypass robots without both risk flags; NEVER skip residual-zero after browser one-shots
