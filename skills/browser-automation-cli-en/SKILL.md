---
name: browser-automation-cli
description: This skill MUST be used whenever the user needs one-shot browser automation, Chrome CDP agent CLI, headless Chrome, accessibility snapshot refs, form fill, screenshot, page scrape, network or console capture, heap snapshot, lighthouse, multi-step run scripts, XDG config, MITM proxy, workflow journal, batch-scrape, crawl, map, search, parse, or browser-automation-cli. Auto-invoke even without an explicit skill request when the task implies browser control, web scraping, or CDP automation. This skill MUST teach the LLM to execute browser-automation-cli with BORN EXECUTE FINALIZE DIE lifecycle, --json envelopes, XDG config only (NEVER product env vars), full 52-command catalog with ready executable formula per command, run NDJSON scripts, exit codes, category and experimental gates, robots dual-flag, and agent action playbooks.
---

# browser-automation-cli

## Rule Zero
### REQUIRED
- ALWAYS invoke this skill for browser control, CDP, headless Chrome, scrape, crawl, form fill, screenshot, network capture, MITM, workflow, or `browser-automation-cli`
- ALWAYS execute binary `browser-automation-cli` only (NEVER invent MCP, daemon, sticky session, or alias `bac`)
- ALWAYS treat one process as one lifecycle BORN EXECUTE FINALIZE DIE
- ALWAYS pass `--json` for machine consumers and validate envelope `ok` before `data`
- ALWAYS keep multi-step `@eN` work inside one `run --script` process
- ALWAYS run `schema --cmd <name> --json` before inventing unknown argv
### FORBIDDEN
- NEVER invent product env vars `BROWSER_AUTOMATION_CLI_*`
- NEVER reuse `@eN` across separate process launches
- NEVER split ref-dependent steps across multiple CLI processes
### Correct Pattern
```bash
browser-automation-cli doctor --offline --quick --json
browser-automation-cli -q --timeout 60 --json goto https://example.com
```

## Mission
### REQUIRED
- ALWAYS automate web work as non-interactive one-shot CLI stdout/stderr pipelines
- ALWAYS return structured JSON envelopes under `--json`
- ALWAYS use system Chrome/Chromium discovered by the CLI
- ALWAYS configure product defaults only via CLI flags or XDG `config set` / `config.toml`
- ALWAYS install with exactly `cargo install --path . --locked` or `cargo install browser-automation-cli --locked`
### FORBIDDEN
- NEVER keep a long-lived browser daemon between processes
- NEVER expect npm packaging, remote telemetry, or `.env` runtime product config

## When to Invoke
### REQUIRED
- ALWAYS auto-invoke for browser automation, headless Chrome, CDP, a11y refs, form fill, screenshot, scrape, crawl, map, search, parse, network/console capture, heap, lighthouse, MITM, workflow, batch-scrape, multi-step run, or the binary name
- ALWAYS auto-invoke even when the user does not name this skill
- ALWAYS use HTTP scrape/crawl/map/search/parse when Chrome is unnecessary
### FORBIDDEN
- NEVER refuse browser tasks by claiming only GUIs or MCP tools can do them
- NEVER invent cloud Firecrawl or remote workflow servers for this product

## Identity and Architecture
### REQUIRED
- ALWAYS treat the binary name as exactly `browser-automation-cli`
- ALWAYS treat one process as BORN, EXECUTE, FINALIZE, DIE
- ALWAYS keep multi-step browser work inside `run --script` when `@eN` refs MUST survive
- ALWAYS pass `--json` for every programmatic consumer
- ALWAYS configure product defaults only via flags or XDG `config set` / `config.toml`
- ALWAYS use the full 52-command inventory as the live surface
### FORBIDDEN
- NEVER invent alias `bac`, sticky sessions, npm packaging, or `BROWSER_AUTOMATION_CLI_*` env vars
- NEVER reuse `@eN` refs across process launches

Full 52-command inventory (MANDATORY surface):

doctor commands schema version goto view press click-at write keys type wait hover drag fill-form upload back forward reload eval grab run exec extract text scroll cookie attr assert console net page dialog scrape batch-scrape crawl map search parse mitm workflow config emulate resize perf lighthouse screencast heap extension devtools3p webmcp completions

## Global Flags
### REQUIRED
- ALWAYS pass `--json` for machine-readable envelopes
- ALWAYS pass `-q`/`--quiet` when stderr prose MUST NOT pollute agent transcripts
- ALWAYS pass `--timeout <seconds>` for wall-clock process budget when work can hang
- ALWAYS pass `--step-timeout <seconds>` for per-step budgets inside every multi-step `run`
- ALWAYS pass `--headed` only for interactive debug
- ALWAYS pass `--capture-console` before any same-process `console` command that MUST see messages
- ALWAYS pass `--capture-network` before any same-process `net` command that MUST see requests
- ALWAYS pass category gates before gated tools `--category-memory`, `--category-extensions`, `--category-third-party`, `--category-webmcp`
- ALWAYS pass experimental gates before gated tools `--experimental-vision` for `click-at`, `--experimental-screencast` for `screencast`
- ALWAYS use OS vars only outside product config `RUST_LOG`, `NO_COLOR`
### FORBIDDEN
- NEVER assume capture flags persist across process launches
- NEVER enable category/experimental surfaces silently in agent defaults
- NEVER invent `BROWSER_AUTOMATION_CLI_*` env vars
### Correct Pattern
```bash
browser-automation-cli --json --timeout 90 --capture-network run --script steps.jsonl
browser-automation-cli --category-memory heap summary --path snap.heapsnapshot --json
browser-automation-cli --experimental-vision click-at --x 10 --y 20 --json
RUST_LOG=debug browser-automation-cli --debug --json doctor --offline --quick
```

## Config XDG
### REQUIRED
- ALWAYS treat product settings as flags plus XDG config only
- ALWAYS use `config init|path|show|get|set`
- ALWAYS use only keys `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`
- ALWAYS set encryption with `config set encryption_key <secret>`
- ALWAYS expect Linux layout under `$XDG_CONFIG_HOME/browser-automation-cli` (and matching data/cache/state)
### FORBIDDEN
- NEVER invent product env vars for settings/encryption
- NEVER use `.env` as runtime product config
- NEVER log `encryption_key` values
- NEVER invent config keys outside the seven supported keys
### Correct Pattern
```bash
browser-automation-cli config init --json
browser-automation-cli config path --json
browser-automation-cli config set timeout 90 --json
browser-automation-cli config set encryption_key "replace-me" --json
browser-automation-cli config show --json
```

## Full Command Catalog (all 52)
### REQUIRED
- ALWAYS execute formulas below as-is unless `schema --cmd` forces a flag change
- ALWAYS pass global `--json` for machine consumers on every programmatic invocation
- ALWAYS cover every name in the 52-command inventory at least once with an executable line
### FORBIDDEN
- NEVER invent argv outside this catalog without `schema --cmd`
- NEVER invent alias `bac` or product env vars
### Correct Pattern

#### Meta / discovery
```bash
browser-automation-cli doctor --offline --quick --json
browser-automation-cli commands --json
browser-automation-cli schema --cmd goto --json
browser-automation-cli version --json
browser-automation-cli completions bash
```

#### Navigation
```bash
browser-automation-cli --json goto https://example.com
browser-automation-cli --json back
browser-automation-cli --json forward
browser-automation-cli --json reload --ignore-cache
browser-automation-cli --json page info
browser-automation-cli --json page list
browser-automation-cli --json page new --url https://example.com
browser-automation-cli --json page select 0 --bring-to-front
browser-automation-cli --json page close --index 0
browser-automation-cli --json wait --ms 500
browser-automation-cli --json wait --text Example --text Demo --ms 1000
browser-automation-cli --json wait --selector "h1" --state load
```

#### Snapshot / input
```bash
browser-automation-cli --json view
browser-automation-cli --json press @e1 --include-snapshot
browser-automation-cli --experimental-vision --json click-at --x 10 --y 20
browser-automation-cli --json write @e2 "hello"
browser-automation-cli --json keys Enter
browser-automation-cli --json type "hello" --target @e2 --clear --submit Enter
browser-automation-cli --json type "world" --focus-only
browser-automation-cli --json hover @e1
browser-automation-cli --json drag --from @e1 --to @e2
browser-automation-cli --json fill-form --json '[{"target":"@e3","value":"x"}]'
browser-automation-cli --json upload @e4 /tmp/file.txt
```

#### Observation
```bash
browser-automation-cli --json grab --path /tmp/page.png --full-page
browser-automation-cli --json extract @e1
browser-automation-cli --json extract @e1 --attr href
browser-automation-cli --json text @e2
browser-automation-cli --json scroll --delta-y 400
browser-automation-cli --json attr @e1 href
browser-automation-cli --json assert url https://example.com --contains
browser-automation-cli --json assert text "Example"
browser-automation-cli --capture-console --json assert console --level error
browser-automation-cli --json cookie list
browser-automation-cli --json cookie set --json '[{"name":"a","value":"b","url":"https://example.com"}]'
browser-automation-cli --json cookie clear
browser-automation-cli --json dialog accept
browser-automation-cli --json dialog dismiss
browser-automation-cli --json eval 'document.title'
```

#### Capture
```bash
browser-automation-cli --capture-console --json console list
browser-automation-cli --capture-console --json console get 0
browser-automation-cli --capture-console --json console clear
browser-automation-cli --capture-console --json console dump --path /tmp/console.json
browser-automation-cli --capture-network --json net list
browser-automation-cli --capture-network --json net get 0
```

#### Scrape local
```bash
browser-automation-cli --json scrape https://example.com --format markdown --engine http
browser-automation-cli --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2
browser-automation-cli --json crawl https://example.com --limit 20 --max-depth 2 --format text
browser-automation-cli --json map https://example.com --limit 50 --max-depth 2
browser-automation-cli --json search "example domain" --limit 10
browser-automation-cli --json parse /tmp/page.html
```

#### Emulate / perf / depth
```bash
browser-automation-cli --json emulate --user-agent "Mozilla/5.0" --viewport "390x844x3,mobile,touch" --network-conditions "Slow 3G"
browser-automation-cli --json resize --width 1280 --height 720
browser-automation-cli --json perf start
browser-automation-cli --json perf stop --path /tmp/trace.json
browser-automation-cli --json perf insight --name DocumentLatency
browser-automation-cli --json lighthouse https://example.com
browser-automation-cli --experimental-screencast --json screencast start --path /tmp/cast
browser-automation-cli --experimental-screencast --json screencast stop
browser-automation-cli --category-memory --json heap take --path /tmp/snap.heapsnapshot
browser-automation-cli --category-memory --json heap summary --path /tmp/snap.heapsnapshot
browser-automation-cli --category-memory --json heap compare --base /tmp/a.heapsnapshot --current /tmp/b.heapsnapshot
browser-automation-cli --category-extensions --json extension list
browser-automation-cli --category-extensions --json extension install --path /tmp/ext
browser-automation-cli --category-third-party --json devtools3p list
browser-automation-cli --category-third-party --json devtools3p exec SomeTool --params '{}'
browser-automation-cli --category-webmcp --json webmcp list
browser-automation-cli --category-webmcp --json webmcp exec SomeTool --input '{}'
```

#### MITM / workflow / config / run
```bash
browser-automation-cli --json mitm init-ca
browser-automation-cli --json mitm start --seconds 30
browser-automation-cli --json mitm status
browser-automation-cli --json mitm list --limit 50
browser-automation-cli --json mitm get 0
browser-automation-cli --json mitm har --out /tmp/capture.har
browser-automation-cli --json mitm export --out /tmp/capture.json
browser-automation-cli --json mitm domains
browser-automation-cli --json mitm apis
browser-automation-cli --json workflow run --manifest /tmp/wf.json
browser-automation-cli --json workflow resume --manifest /tmp/wf.json
browser-automation-cli --json workflow status --name demo
browser-automation-cli config init --json
browser-automation-cli config path --json
browser-automation-cli config show --json
browser-automation-cli config get timeout --json
browser-automation-cli config set timeout 90 --json
browser-automation-cli config set encryption_key "replace-me" --json
browser-automation-cli --timeout 60 --json run --script /tmp/steps.jsonl
browser-automation-cli --json exec goto https://example.com
```

## Contract Rules
### REQUIRED
- ALWAYS use `doctor` for Chrome/XDG health; `commands --json` for inventory; `schema --cmd` before inventing argv; `version --json` to pin identity
- ALWAYS use `wait` multi `--text` as OR (any match resolves); NEVER as AND
- ALWAYS use `grab --path <file>` (NEVER bare positional); `type` with positional TEXT plus `--target` OR `--focus-only`
- ALWAYS pass fill-form as command `--json '[{"target":"@eN","value":"x"}]'` plus global `--json`; upload requires target+path
- ALWAYS use `click-at` only with `--experimental-vision`; `screencast` only with `--experimental-screencast`
- ALWAYS use `console` only after same-process `--capture-console`; `net` only after same-process `--capture-network`
- ALWAYS compose `emulate` with `--user-agent`/`--viewport`/`--network-conditions` (NEVER `--device`)
- ALWAYS gate `heap` with `--category-memory`; `extension` with `--category-extensions`; `devtools3p` with `--category-third-party`; `webmcp` with `--category-webmcp`
- ALWAYS bind MITM to `127.0.0.1` only; treat CA/captures as sensitive host material
- ALWAYS use workflow JSON manifests only; resume skips successful journal steps under XDG state
- ALWAYS treat `exec` as single-step inline only (NOT multi-step engine)
- ALWAYS use scrape formats `text|markdown|html|links|metadata` and engines `http|browser`; HTTP engine MUST NOT launch Chrome
### FORBIDDEN
- NEVER invent aliases `snapshot`, `click`, `fill`, `screenshot`, or `bac`
- NEVER expect page state or `@eN` to survive FINALIZE DIE into a new process
- NEVER invent cloud Firecrawl or remote sticky workflow servers
- NEVER replace browser `run --script` multi-step `@eN` work with workflow

## Multi-step Run Scripts
### REQUIRED
- ALWAYS use `run --script <path>` for NDJSON steps in one process
- ALWAYS put one JSON object per line with field `cmd`
- ALWAYS keep shared page state and `@eN` refs inside that single process
- ALWAYS set `--timeout` large enough for the full script
- ALWAYS encode grab as `{"cmd":"grab","path":"/tmp/example.png"}` inside NDJSON
### FORBIDDEN
- NEVER split ref-dependent steps across processes
- NEVER treat `exec` as multi-step engine
- NEVER expect `@eN` to survive process DIE
### Correct Pattern
```bash
cat > /tmp/demo.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":500}
{"cmd":"view"}
{"cmd":"grab","path":"/tmp/example.png"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/demo.browser-automation.jsonl
```

## Workflow Manifest
### REQUIRED
- ALWAYS use `workflow run --manifest <path>` with JSON path (example `/tmp/wf.json`)
- ALWAYS use `workflow resume --manifest <path>`; `workflow status`; pass `--journal` when non-default journal path is REQUIRED
### FORBIDDEN
- NEVER use non-JSON workflow manifests
### Correct Pattern
```bash
cat > /tmp/wf.json <<'JSON'
{"name":"demo","steps":[{"id":"ping","cmd":"echo","args":{"message":"start"}}]}
JSON
browser-automation-cli --json workflow run --manifest /tmp/wf.json
browser-automation-cli --json workflow resume --manifest /tmp/wf.json
browser-automation-cli --json workflow status --name demo
```

## Action Playbooks
### REQUIRED
- ALWAYS execute these formulas as-is unless schema discovery forces a flag change
- ALWAYS keep `@eN` multi-step work inside one `run --script` process
- ALWAYS validate envelope `ok` after every invocation
### FORBIDDEN
- NEVER invent `bac`, product env vars, bare `grab` paths, `emulate --device`, or non-JSON workflow manifests
### Correct Pattern
#### A. Doctor then version
```bash
browser-automation-cli doctor --offline --quick --json
browser-automation-cli version --json
```
#### B. HTTP scrape markdown
```bash
browser-automation-cli --json scrape https://example.com --format markdown --engine http
```
#### C. Form fill via run NDJSON
```bash
cat > /tmp/form.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":500}
{"cmd":"view"}
{"cmd":"write","target":"@e1","value":"hello"}
{"cmd":"press","target":"@e2"}
{"cmd":"grab","path":"/tmp/form.png"}
JSONL
browser-automation-cli --timeout 90 --json run --script /tmp/form.browser-automation.jsonl
```
#### D. Wait OR multi-text
```bash
browser-automation-cli --timeout 60 --json wait --text "Example Domain" --text "Example" --ms 5000
```
#### E. Network capture list
```bash
cat > /tmp/net.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":400}
{"cmd":"net","action":"list","resource_types":"Document,XHR"}
JSONL
browser-automation-cli --capture-network --timeout 60 --json run --script /tmp/net.browser-automation.jsonl
```
#### F. MITM init-ca + start
```bash
browser-automation-cli --json mitm init-ca
browser-automation-cli --json mitm start --seconds 30
```
#### G. Workflow JSON manifest
```bash
cat > /tmp/wf.json <<'JSON'
{"name":"demo","steps":[{"id":"ping","cmd":"echo","args":{"message":"start"}}]}
JSON
browser-automation-cli --json workflow run --manifest /tmp/wf.json
```
#### H. batch-scrape / crawl / map / search / parse
```bash
printf '%s\n' https://example.com https://example.org > /tmp/urls.txt
browser-automation-cli --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2
browser-automation-cli --json crawl https://example.com --limit 20 --max-depth 2 --format text
browser-automation-cli --json map https://example.com --limit 50 --max-depth 2
browser-automation-cli --json search "example domain" --limit 10
browser-automation-cli --json parse /tmp/page.html
```
#### I. Config XDG
```bash
browser-automation-cli config init --json
browser-automation-cli config set timeout 90 --json
browser-automation-cli config set encryption_key "replace-me" --json
browser-automation-cli config show --json
```
#### J. Schema discovery
```bash
browser-automation-cli schema --cmd scrape --json
browser-automation-cli schema --cmd fill-form --json
browser-automation-cli commands --json
```

## JSON Envelope
### REQUIRED
- ALWAYS expect success `{"schema_version":1,"ok":true,"data":...}`
- ALWAYS expect error under `--json` `{"schema_version":1,"ok":false,"error":{...}}`
- ALWAYS validate `ok` before reading `data`
- ALWAYS keep stderr for diagnostics/tracing only
### FORBIDDEN
- NEVER treat human prose stdout under `--json` as the primary contract
- NEVER ignore `ok:false` with non-zero exit
### Correct Pattern
```bash
out=$(browser-automation-cli -q --json version)
echo "$out" | jaq -e '.ok == true'
```

## Exit Codes and Retry
### REQUIRED
- ALWAYS branch on exit code before trusting stdout
- ALWAYS treat codes `0` success; `2` usage/fix argv; `65` data; `66` no input; `69` unavailable/repair Chrome; `70` software/browser/protocol; `74` I/O; `78` config; `124` timeout/raise budget; `130` cancel; `141` broken pipe
- ALWAYS re-run only transient host/browser launch failures with backoff
### FORBIDDEN
- NEVER re-run pure usage failures without changing argv
- NEVER mask exit codes with `|| true`
### Correct Pattern
```bash
set +e; browser-automation-cli -q --timeout 60 --json goto https://example.com; code=$?; set -e
case "$code" in 0) echo ok;; 2) echo fix_argv;; 69) echo repair_chrome;; 124) echo raise_timeout;; *) echo fail_$code;; esac
```

## Robots Policy
### REQUIRED
- ALWAYS respect robots defaults
- ALWAYS bypass only with dual flags together `--ignore-robots` AND `--i-accept-robots-risk`
### FORBIDDEN
- NEVER bypass robots with a single flag
- NEVER invent robots bypass env vars
### Correct Pattern
```bash
browser-automation-cli --json scrape https://example.com --format text --engine http
browser-automation-cli --ignore-robots --i-accept-robots-risk --json scrape https://example.com --format text --engine http
```

## Canonical Names and DevTools Map
### REQUIRED
- ALWAYS use binary `browser-automation-cli` only
- ALWAYS use `view` not snapshot; `press` not click; `write` not fill; `grab` not screenshot
- ALWAYS map DevTools tools exactly click→press, fill→write, take_screenshot→grab, take_snapshot→view, type_text→type, press_key→keys, fill_form→fill-form, upload_file→upload, click_at→click-at, navigate_page→goto|back|forward|reload, wait_for→wait, evaluate_script→eval, list_network_requests→net list, list_console_messages→console list
- ALWAYS use flags/XDG for product settings; outside that use only OS vars `RUST_LOG` and `NO_COLOR`
### FORBIDDEN
- NEVER invent product aliases that conflict with this map
- NEVER call DevTools names as CLI subcommands

## Absolute Prohibitions
### REQUIRED
- ALWAYS refuse illegal patterns and rewrite to the canonical CLI surface
### FORBIDDEN
- NEVER invent `bac` or `BROWSER_AUTOMATION_CLI_*`
- NEVER use `.env` as runtime product config
- NEVER pass bare positional path to `grab` (ALWAYS `--path`)
- NEVER invent `emulate --device`
- NEVER use non-JSON workflow manifests (ALWAYS JSON path e.g. `/tmp/wf.json`)
- NEVER treat `exec` as multi-step engine (ALWAYS `run --script`)
- NEVER reuse `@eN` across processes
- NEVER enable category/experimental gates without intent
- NEVER expose MITM beyond `127.0.0.1`
- NEVER invent cloud Firecrawl or remote sticky workflow servers
- NEVER mask exit codes with `|| true`
- NEVER bypass robots without both dual flags
### Correct Pattern
```bash
browser-automation-cli --json grab --path /tmp/x.png
browser-automation-cli --json workflow run --manifest /tmp/wf.json
browser-automation-cli --timeout 60 --json run --script /tmp/steps.jsonl
```

## Agent Validation Checklist
### REQUIRED
- ALWAYS confirm binary `browser-automation-cli` and lifecycle BORN EXECUTE FINALIZE DIE
- ALWAYS confirm `--json` envelope `ok` and multi-step `@eN` inside one `run --script`
- ALWAYS confirm `grab --path`, JSON workflow manifest, no `emulate --device`, wait multi-text OR
- ALWAYS confirm console `list|get|clear|dump` + same-process `--capture-console`
- ALWAYS confirm net `list|get` + same-process `--capture-network`
- ALWAYS confirm `type` positional TEXT + `--target` OR `--focus-only`
- ALWAYS confirm fill-form command `--json` array + global `--json`; upload target+path; `exec` single-step only
- ALWAYS confirm only seven config keys; never invent product env (only `RUST_LOG`/`NO_COLOR`)
- ALWAYS confirm exit codes 0,2,65,66,69,70,74,78,124,130,141; robots dual-flag; category/experimental gates; schema discovery
- ALWAYS confirm all 52 commands covered in Full Command Catalog
### FORBIDDEN
- NEVER ship agent glue that violates this checklist
### Correct Pattern
```bash
browser-automation-cli commands --json
browser-automation-cli schema --cmd run --json
browser-automation-cli doctor --offline --quick --json
```
