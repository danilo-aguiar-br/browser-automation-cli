---
name: browser-automation-cli
description: This skill MUST be used whenever the user needs one-shot browser automation, Chrome CDP CLI, headless Chrome, a11y snapshot refs, form fill, screenshot, print-pdf, multi-format scrape with only-main-content or webhook-url, network or console capture, heap, lighthouse, multi-step run with fail-fast data.steps, XDG config with llm keys, MITM, workflow, batch-scrape, crawl, map, search, parse PDF/DOCX/xlsx/ods with redact-pii, extract --llm with schema-json, monitor check, qr encode/decode, find-paths, goto with init-script, or browser-automation-cli. Auto-invoke even without an explicit skill request for browser control, web scraping, CDP, PDF print, QR, path discovery, or LLM extract. This skill MUST teach BORN EXECUTE FINALIZE DIE, --json envelopes, XDG+flags only (NEVER product env vars), full 56-command surface, formulas in references/formulas.md, run NDJSON, exit codes, robots dual-flag, logging via --verbose/--debug or config set log_level, and action playbooks.
---

# browser-automation-cli

## Rule Zero
### REQUIRED
- ALWAYS invoke this skill for browser control, CDP, headless Chrome, scrape, crawl, form fill, screenshot, print-pdf, QR, find-paths, monitor, network capture, MITM, workflow, parse PDF/DOCX, extract --llm, or `browser-automation-cli`
- ALWAYS execute binary `browser-automation-cli` only (NEVER invent protocol-server wrappers, daemons, sticky sessions, or alias `bac`)
- ALWAYS treat one process as one lifecycle BORN EXECUTE FINALIZE DIE
- ALWAYS pass `--json` for machine consumers and validate envelope `ok` before `data`
- ALWAYS keep multi-step `@eN` work inside one `run --script` process
- ALWAYS run `schema --cmd <name> --json` before inventing unknown argv
- ALWAYS load executable formulas from `references/formulas.md` for full command argv
### FORBIDDEN
- NEVER invent product environment variables for settings or logging
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
- ALWAYS use system Chrome/Chromium discovered by the CLI (or XDG `chrome_path`)
- ALWAYS configure product defaults only via CLI flags or XDG `config set` / `config.toml`
- ALWAYS install with exactly `cargo install --path . --locked` or `cargo install browser-automation-cli --locked`
### FORBIDDEN
- NEVER keep a long-lived browser daemon between processes
- NEVER expect npm packaging or `.env` runtime product config
- NEVER invent product environment variables for logging (ALWAYS use `--verbose`/`--debug`/`-q` or `config set log_level`)
### Correct Pattern
```bash
cargo install browser-automation-cli --locked
browser-automation-cli doctor --offline --quick --json
```

## When to Invoke
### REQUIRED
- ALWAYS auto-invoke for browser automation, headless Chrome, CDP, a11y refs, form fill, screenshot, print-pdf, scrape, crawl, map, search, parse, extract --llm, monitor, qr, find-paths, network/console capture, heap, lighthouse, MITM, workflow, batch-scrape, multi-step run, or the binary name
- ALWAYS auto-invoke even when the user does not name this skill
- ALWAYS use HTTP scrape/crawl/map/search/parse when Chrome is unnecessary
### FORBIDDEN
- NEVER refuse browser tasks by claiming only GUIs or foreign protocol servers can do them
- NEVER invent cloud scrape SaaS or remote workflow servers for this product
### Correct Pattern
```bash
browser-automation-cli --json scrape https://example.com --format markdown --engine http
browser-automation-cli --timeout 90 --json run --script /tmp/steps.jsonl
```

## Identity and Architecture
### REQUIRED
- ALWAYS treat the binary name as exactly `browser-automation-cli`
- ALWAYS treat one process as BORN, EXECUTE, FINALIZE, DIE
- ALWAYS keep multi-step browser work inside `run --script` when `@eN` refs MUST survive
- ALWAYS pass `--json` for every programmatic consumer
- ALWAYS configure product defaults only via flags or XDG `config set` / `config.toml`
- ALWAYS treat the live surface as 56 top-level command names
### FORBIDDEN
- NEVER invent alias `bac`, sticky sessions, npm packaging, or product environment variables for settings
- NEVER reuse `@eN` refs across process launches
- NEVER assume only the 52 parity tools exist
### Correct Pattern
```bash
browser-automation-cli -q --timeout 60 --json goto https://example.com
browser-automation-cli --timeout 90 --json run --script /tmp/steps.jsonl
```

## Global Flags
### REQUIRED
- ALWAYS pass `--json` for machine-readable envelopes
- ALWAYS pass `-q`/`--quiet` when stderr prose MUST NOT pollute agent transcripts
- ALWAYS pass `--verbose` or `--debug` for product logging detail (or set XDG `log_level`)
- ALWAYS pass `--timeout <seconds>` for wall-clock process budget when work can hang
- ALWAYS pass `--step-timeout <seconds>` for per-step budgets inside every multi-step `run`
- ALWAYS pass `--headed` only for interactive debug
- ALWAYS pass `--capture-console` before any same-process `console` command that MUST see messages
- ALWAYS pass `--capture-network` before any same-process `net` command that MUST see requests
- ALWAYS pass category gates before gated tools `--category-memory`, `--category-extensions`, `--category-third-party`, `--category-webmcp`
- ALWAYS pass experimental gates before gated tools `--experimental-vision` for `click-at`, `--experimental-screencast` for `screencast`
### FORBIDDEN
- NEVER assume capture flags persist across process launches
- NEVER enable category/experimental surfaces silently in agent defaults
- NEVER invent product environment variables for settings or logging
### Correct Pattern
```bash
browser-automation-cli --json --timeout 90 --capture-network run --script steps.jsonl
browser-automation-cli --category-memory heap summary --path snap.heapsnapshot --json
browser-automation-cli --experimental-vision click-at --x 10 --y 20 --json
browser-automation-cli --debug --json doctor --offline --quick
```

## Config XDG
### REQUIRED
- ALWAYS treat product settings as flags plus XDG config only
- ALWAYS use `config path|init|show|set|get`
- ALWAYS use only these 13 keys: `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`
- ALWAYS set encryption with `config set encryption_key <secret>`
- ALWAYS set LLM extract credentials with `config set openrouter_api_key <key>` and ALWAYS set `llm_base_url` / `llm_model` when the LLM endpoint or model MUST differ from defaults
- ALWAYS set product log default with `config set log_level <error|warn|info|debug|trace>`
- ALWAYS set color with `config set color true|false` and Chrome path with `config set chrome_path <path>`
- ALWAYS resolve config/data/cache/state paths via `config path --json`
### FORBIDDEN
- NEVER invent product env vars for settings/encryption/LLM keys/logging
- NEVER use `.env` as runtime product config
- NEVER log `encryption_key` or `openrouter_api_key` values
- NEVER invent config keys outside the 13 supported keys
### Correct Pattern
```bash
browser-automation-cli config init --json
browser-automation-cli config path --json
browser-automation-cli config set timeout 90 --json
browser-automation-cli config set log_level info --json
browser-automation-cli config set chrome_path /usr/bin/google-chrome --json
browser-automation-cli config set openrouter_api_key "replace-me" --json
browser-automation-cli config show --json
browser-automation-cli config get timeout --json
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
- ALWAYS use scrape formats `text|markdown|html|raw-html|links|metadata|screenshot|summary|product|branding` and engines `http|browser`; HTTP engine MUST NOT launch Chrome
- ALWAYS treat scrape `--webhook-url` as one-shot operator POST of result data (NOT product telemetry)
- ALWAYS use scrape `--only-main-content` when main-content extraction is REQUIRED
- ALWAYS use `goto` with `--init-script`, `--handle-before-unload`, and `--navigation-timeout-ms` when the task REQUIRES them
- ALWAYS use `print-pdf --path` for PDF artifacts; ALWAYS pass `--url` when the page MUST be navigated in the same one-shot
- ALWAYS use `monitor check` with `--url` and `--baseline`
- ALWAYS use `qr encode --text` / `qr decode --path`; `find-paths` is filesystem only (no Chrome)
- ALWAYS use `find-paths` with `--extension`, `--type`, `--limit`, `--max-depth`, `--hidden`, `--no-ignore` as the task REQUIRES
- ALWAYS use `parse` for html/md/txt/pdf/docx/xlsx/ods; pass `--redact-pii` when masking PII is REQUIRED
- ALWAYS set XDG `openrouter_api_key` before `extract --llm`; ALWAYS pass `--question`; ALWAYS pass `--schema-json` when structured LLM extract is REQUIRED
- ALWAYS expect `attr` to fall back to DOM properties when the HTML attribute is null
- ALWAYS use `scroll --delta-y`/`--delta-x` (NDJSON MUST use `dy` or `delta_y`); `assert url … --contains` (NDJSON MUST use `url_contains` when contains is REQUIRED)
- ALWAYS on `run` fail-fast error envelopes inspect partial `data.steps` when present
- ALWAYS copy full argv from `references/formulas.md` when building one-shot commands
### FORBIDDEN
- NEVER invent aliases `snapshot`, `click`, `fill`, `screenshot`, or `bac`
- NEVER expect page state or `@eN` to survive FINALIZE DIE into a new process
- NEVER invent cloud scrape SaaS or remote sticky workflow servers
- NEVER replace browser `run --script` multi-step `@eN` work with workflow
### Correct Pattern
```bash
browser-automation-cli schema --cmd goto --json
browser-automation-cli --json grab --path /tmp/page.png --full-page
browser-automation-cli --json assert url https://example.com --contains
```

## Inventory
### REQUIRED
- ALWAYS treat this exact 56-name surface as MANDATORY inventory
- ALWAYS load at least one executable line per name from `references/formulas.md`

doctor commands schema version goto view press click-at write keys type wait hover drag fill-form upload back forward reload eval grab print-pdf monitor run exec extract text scroll cookie attr assert console net page dialog scrape batch-scrape crawl map search parse qr find-paths mitm workflow config emulate resize perf lighthouse screencast heap extension devtools3p webmcp completions

### FORBIDDEN
- NEVER invent alias names outside this inventory
- NEVER omit PRD-only commands when they are the correct tool
### Correct Pattern
```bash
browser-automation-cli commands --json
browser-automation-cli schema --cmd print-pdf --json
```

## Action Playbooks
### REQUIRED
- ALWAYS execute these formulas as-is unless `schema --cmd` forces a flag change
- ALWAYS keep `@eN` multi-step work inside one `run --script` process
- ALWAYS validate envelope `ok` after every invocation
- ALWAYS use `references/formulas.md` for the remaining surface
### FORBIDDEN
- NEVER invent `bac`, product env vars, bare `grab` paths, `emulate --device`, or non-JSON workflow manifests
### Correct Pattern

#### A. Goto with init-script, beforeunload, navigation timeout
```bash
browser-automation-cli --timeout 60 --json goto https://example.com \
  --init-script 'window.__ready=true' \
  --handle-before-unload \
  --navigation-timeout-ms 15000
```

#### B. HTTP scrape main content + webhook
```bash
browser-automation-cli --json scrape https://example.com --format markdown --engine http --only-main-content
browser-automation-cli --json scrape https://example.com --format text --engine http --webhook-url https://127.0.0.1:9000/hook
```

#### C. Browser multi-format scrape
```bash
browser-automation-cli --timeout 60 --json scrape https://example.com --format markdown --engine browser
browser-automation-cli --timeout 60 --json scrape https://example.com --format links --engine browser
browser-automation-cli --timeout 60 --json scrape https://example.com --format screenshot --engine browser
```

#### D. Extract LLM with schema-json
```bash
browser-automation-cli config set openrouter_api_key "replace-me" --json
browser-automation-cli config set llm_base_url "https://openrouter.ai/api/v1" --json
browser-automation-cli config set llm_model "openai/gpt-4o-mini" --json
printf '%s\n' '{"type":"object","properties":{"title":{"type":"string"}},"required":["title"]}' > /tmp/extract.schema.json
browser-automation-cli --timeout 120 --json extract --llm --question "What is the main title?" --schema-json /tmp/extract.schema.json https://example.com
```

#### E. Form fill via run NDJSON
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

#### F. Run fail-fast inspect data.steps
```bash
cat > /tmp/failfast.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":200}
{"cmd":"view"}
{"cmd":"assert","kind":"url","url_contains":"this-must-not-match.invalid"}
{"cmd":"grab","path":"/tmp/never.png"}
JSONL
set +e
out=$(browser-automation-cli -q --timeout 60 --json run --script /tmp/failfast.browser-automation.jsonl 2>/dev/null)
code=$?
set -e
echo "$out" | jaq -e '.ok == false'
echo "$out" | jaq -e '(.data.steps | type) == "array"'
echo "$out" | jaq -r '.data.steps | length'
echo "exit=$code"
```

#### G. print-pdf
```bash
browser-automation-cli --timeout 60 --json print-pdf --path /tmp/page.pdf --url https://example.com
```

#### H. monitor check
```bash
browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/example.baseline --write-baseline --engine http
browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/example.baseline --engine http
```

#### I. QR encode/decode
```bash
browser-automation-cli --json qr encode --text "https://example.com" --format png --path /tmp/qr.png
browser-automation-cli --json qr decode --path /tmp/qr.png
```

#### J. find-paths full flags
```bash
browser-automation-cli --json find-paths '\.rs$' . --hidden --no-ignore --max-depth 4 --extension rs --type f --limit 100
```

#### K. parse PDF/DOCX/xlsx/ods
```bash
browser-automation-cli --json parse /tmp/doc.pdf
browser-automation-cli --json parse /tmp/doc.docx --redact-pii
browser-automation-cli --json parse /tmp/sheet.xlsx
browser-automation-cli --json parse /tmp/sheet.ods --redact-pii
```

#### L. scroll dy + attr property fallback + i18n lang
```bash
browser-automation-cli --json scroll --delta-y 400
browser-automation-cli --json attr @e1 href
browser-automation-cli --json attr @e1 value
browser-automation-cli config set lang en --json
browser-automation-cli --lang pt-BR --json version
```

#### M. MITM 127.0.0.1 + XDG config
```bash
browser-automation-cli --json mitm init-ca
browser-automation-cli --json mitm start --seconds 30
browser-automation-cli config init --json
browser-automation-cli config set artifacts_dir /tmp/browser-automation-cli-artifacts --json
browser-automation-cli config show --json
```

#### N. Network capture list (same process)
```bash
cat > /tmp/net.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":400}
{"cmd":"net","action":"list","resource_types":"Document,XHR"}
JSONL
browser-automation-cli --capture-network --timeout 60 --json run --script /tmp/net.browser-automation.jsonl
```

## Multi-step Run Scripts
### REQUIRED
- ALWAYS use `run --script <path>` for NDJSON steps in one process
- ALWAYS put one JSON object per line with field `cmd`
- ALWAYS keep shared page state and `@eN` refs inside that single process
- ALWAYS set `--timeout` large enough for the full script
- ALWAYS encode grab as `{"cmd":"grab","path":"/tmp/example.png"}` inside NDJSON
- ALWAYS encode scroll dy as `{"cmd":"scroll","dy":400}` or `"delta_y":400`
- ALWAYS encode url assert as `{"cmd":"assert","kind":"url","url_contains":"example.com"}` when using contains
- ALWAYS on fail-fast errors parse partial `data.steps` from the error envelope when present
### FORBIDDEN
- NEVER split ref-dependent steps across processes
- NEVER treat `exec` as multi-step engine
- NEVER expect `@eN` to survive process DIE
### Correct Pattern
```bash
cat > /tmp/demo.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com","init_script":"window.__x=1","handle_before_unload":true,"navigation_timeout_ms":15000}
{"cmd":"wait","ms":500}
{"cmd":"view"}
{"cmd":"scroll","dy":400}
{"cmd":"assert","kind":"url","url_contains":"example.com"}
{"cmd":"grab","path":"/tmp/example.png"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/demo.browser-automation.jsonl
```

## Workflow Manifest
### REQUIRED
- ALWAYS use `workflow run --manifest <path>` with JSON path
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

## JSON Envelope
### REQUIRED
- ALWAYS expect success `{"schema_version":1,"ok":true,"data":...}`
- ALWAYS expect error under `--json` `{"schema_version":1,"ok":false,"error":{...}}`
- ALWAYS validate `ok` before reading `data`
- ALWAYS on `run` fail-fast error envelopes inspect partial `data.steps` when present
- ALWAYS keep stderr for diagnostics/tracing only
### FORBIDDEN
- NEVER treat human prose stdout under `--json` as the primary contract
- NEVER ignore `ok:false` with non-zero exit
### Correct Pattern
```bash
out=$(browser-automation-cli -q --json version)
echo "$out" | jaq -e '.ok == true'
```

## Exit Codes
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

## Robots
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

## DevTools Map
### REQUIRED
- ALWAYS use binary `browser-automation-cli` only
- ALWAYS use `view` not snapshot; `press` not click; `write` not fill; `grab` not screenshot
- ALWAYS map DevTools tools exactly click→press, fill→write, take_screenshot→grab, take_snapshot→view, type_text→type, press_key→keys, fill_form→fill-form, upload_file→upload, click_at→click-at, navigate_page→goto|back|forward|reload, wait_for→wait, evaluate_script→eval, list_network_requests→net list, list_console_messages→console list
- ALWAYS keep the 52-tool DevTools parity map as the interaction core, AND use extra PRD surface commands (`print-pdf`, `monitor`, `qr`, `find-paths`, parse/extract/scrape family) when the task needs them
- ALWAYS use flags/XDG for product settings; product logging MUST use `--verbose`/`--debug`/`-q` or `config set log_level`
### FORBIDDEN
- NEVER invent product aliases that conflict with this map
- NEVER call DevTools names as CLI subcommands
- NEVER ignore PRD-only commands when they are the correct tool for the job
### Correct Pattern
```bash
browser-automation-cli --json view
browser-automation-cli --json press @e1
browser-automation-cli --json grab --path /tmp/x.png
```

## Absolute Prohibitions
### REQUIRED
- ALWAYS refuse illegal patterns and rewrite to the canonical CLI surface
### FORBIDDEN
- NEVER invent `bac` or product environment variables for settings or logging
- NEVER use `.env` as runtime product config
- NEVER pass bare positional path to `grab` (ALWAYS `--path`)
- NEVER invent `emulate --device`
- NEVER use non-JSON workflow manifests
- NEVER treat `exec` as multi-step engine (ALWAYS `run --script`)
- NEVER reuse `@eN` across processes
- NEVER enable category/experimental gates without intent
- NEVER expose MITM beyond `127.0.0.1`
- NEVER invent cloud scrape SaaS or remote sticky workflow servers
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
- ALWAYS confirm console/net capture only with same-process capture flags
- ALWAYS confirm `type` positional TEXT + `--target` OR `--focus-only`; fill-form command `--json` array + global `--json`
- ALWAYS confirm all 13 config keys; NEVER invent product env; logging via `--verbose`/`--debug`/`-q`/`log_level` only
- ALWAYS confirm exit codes 0,2,65,66,69,70,74,78,124,130,141; robots dual-flag; category/experimental gates; schema discovery
- ALWAYS confirm full 56-command inventory and `references/formulas.md` formulas
- ALWAYS confirm playbooks for goto flags, print-pdf, QR, find-paths, monitor, parse, extract LLM, scrape main-content/webhook, fail-fast `data.steps`, scroll dy, attr fallback, i18n lang
### FORBIDDEN
- NEVER ship agent glue that violates this checklist
### Correct Pattern
```bash
browser-automation-cli commands --json
browser-automation-cli schema --cmd run --json
browser-automation-cli doctor --offline --quick --json
```
