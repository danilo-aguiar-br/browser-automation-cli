---
name: browser-automation-cli
description: This skill MUST be used when the agent automates browsers with browser-automation-cli including Chrome CDP headless a11y refs form fill pick select-option grab print-pdf (NEVER blank - ALWAYS --url or navigate first) multi-format scrape network console heap lighthouse multi-step run NDJSON or JSON array fail-fast data.steps --json-steps wait multi-selector or url dialog if-present view allow-empty page new isolated-context MITM capture-url XDG config 16 keys Redis plain URL workflow batch-scrape crawl map search parse extract llm schema monitor qr find-paths sheet-write sg-scan sg-rewrite extension outside run clap --json usage errors. Auto-invoke for browser control scraping CDP PDF QR paths sheets structural scan LLM extract console assert or one-shot CLI even without naming this skill. This skill MUST teach BORN EXECUTE FINALIZE DIE --json envelopes XDG plus flags only (NEVER product env vars) full 61-command formulas in references/formulas.md exit codes robots dual-flag and action playbooks.
---

# browser-automation-cli

## Rule Zero
### REQUIRED
- ALWAYS invoke this skill for browser control, CDP, headless Chrome, scrape, crawl, form fill, pick, select-option, grab, print-pdf, QR, find-paths, sheet-write, sg-scan, sg-rewrite, monitor, network/console capture, heap, lighthouse, MITM, workflow, parse, extract --llm, XDG config/cache, or binary `browser-automation-cli`
- ALWAYS execute binary name exactly `browser-automation-cli` (NEVER invent alias `bac`, protocol-server wrappers, daemons, or sticky sessions)
- ALWAYS treat one process as one lifecycle BORN EXECUTE FINALIZE DIE
- ALWAYS pass `--json` for machine consumers and validate envelope `ok` before `data`
- ALWAYS keep multi-step `@eN` work inside one `run --script` process
- ALWAYS discover argv with `schema <cmd> --json` or `schema --cmd <cmd> --json` before inventing unknown flags
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
- NEVER invent product environment variables for logging (ALWAYS use `--verbose`/`--debug`/`-q` or `config set log_level` / `log_to_file`)
### Correct Pattern
```bash
cargo install browser-automation-cli --locked
browser-automation-cli doctor --offline --quick --json
```

## When to Invoke
### REQUIRED
- ALWAYS auto-invoke for browser automation, headless Chrome, CDP, a11y refs, form fill, pick/select-option, grab, print-pdf, scrape, crawl, map, search, parse, extract --llm, monitor, qr, find-paths, sheet-write, sg-scan, sg-rewrite, network/console capture, heap, lighthouse, redis cache XDG, MITM, workflow, batch-scrape, multi-step run, or the binary name
- ALWAYS auto-invoke even when the user does not name this skill
- ALWAYS use HTTP scrape/crawl/map/search/parse and local-only `qr`/`find-paths`/`sheet-write`/`sg-scan`/`sg-rewrite` when Chrome is unnecessary
### FORBIDDEN
- NEVER refuse browser tasks by claiming only GUIs or foreign protocol servers can do them
- NEVER invent cloud scrape SaaS or remote workflow servers for this product
### Correct Pattern
```bash
browser-automation-cli --json scrape https://example.com --format markdown,links --engine http
browser-automation-cli --timeout 90 --json run --script /tmp/steps.jsonl
browser-automation-cli --json sheet-write /tmp/rows.csv -o /tmp/out.xlsx
```

## Identity and Architecture
### REQUIRED
- ALWAYS treat the binary name as exactly `browser-automation-cli`
- ALWAYS treat one process as BORN, EXECUTE, FINALIZE, DIE
- ALWAYS keep multi-step browser work inside `run --script` when `@eN` refs MUST survive
- ALWAYS pass `--json` for every programmatic consumer
- ALWAYS configure product defaults only via flags or XDG `config set` / `config.toml`
- ALWAYS treat the live surface as exactly **61** top-level inventory names (`commands --json`)
### FORBIDDEN
- NEVER invent alias `bac`, sticky sessions, npm packaging, or product environment variables for settings
- NEVER reuse `@eN` refs across process launches
- NEVER assume only the DevTools-parity tools exist
### Correct Pattern
```bash
browser-automation-cli -q --timeout 60 --json goto https://example.com
browser-automation-cli --timeout 90 --json run --script /tmp/steps.jsonl
browser-automation-cli commands --json
```

## Global Flags
### REQUIRED
- ALWAYS pass `--json` for machine-readable envelopes
- ALWAYS pass `--json-steps` when a multi-step `run` MUST stream one NDJSON object per step (`step`, `cmd`, `ok`, `result`) on stdout
- ALWAYS pass `-q`/`--quiet` when stderr prose MUST NOT pollute agent transcripts
- ALWAYS pass `--verbose` or `--debug` for product logging detail (or set XDG `log_level` / `log_to_file`)
- ALWAYS pass `--timeout <seconds>` for wall-clock process budget when work can hang
- ALWAYS pass `--step-timeout <seconds>` for per-step budgets inside every multi-step `run`
- ALWAYS pass `--headed` only for interactive debug
- ALWAYS pass `--capture-console` before any same-process `console` or `assert console*` command that MUST see messages
- ALWAYS pass `--capture-network` before any same-process `net` command that MUST see requests
- ALWAYS pass category gates before gated tools `--category-memory`, `--category-extensions`, `--category-third-party`, `--category-webmcp`
- ALWAYS pass experimental gates before gated tools `--experimental-vision` for `click-at`, `--experimental-screencast` for `screencast`
### FORBIDDEN
- NEVER assume capture flags persist across process launches
- NEVER enable category/experimental surfaces silently in agent defaults
- NEVER invent product environment variables for settings or logging
### Correct Pattern
```bash
browser-automation-cli --json --json-steps --timeout 90 --capture-network run --script steps.jsonl
browser-automation-cli --category-memory heap summary --path snap.heapsnapshot --json
browser-automation-cli --experimental-vision click-at --x 10 --y 20 --json
browser-automation-cli --debug --json doctor --offline --quick
```

## Config XDG
### REQUIRED
- ALWAYS treat product settings as flags plus XDG config only
- ALWAYS use `config init|path|show|get|set|list-keys`
- ALWAYS use only these 16 keys: `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `log_to_file`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`, `cache_backend`, `cache_redis_url`
- ALWAYS discover keys with `config list-keys --json`
- ALWAYS set encryption with `config set encryption_key <secret>`
- ALWAYS set LLM extract credentials with `config set openrouter_api_key <key>` and ALWAYS set `llm_base_url` / `llm_model` when the LLM endpoint or model MUST differ from defaults
- ALWAYS set product log default with `config set log_level <error|warn|info|debug|trace>`
- ALWAYS set rotated file logs with `config set log_to_file true|false`
- ALWAYS set color with `config set color true|false` and Chrome path with `config set chrome_path <path>`
- ALWAYS set `cache_backend` to exactly `sqlite|memory|redis`
- ALWAYS set `cache_redis_url` to plain `redis://...` only when backend is redis
- ALWAYS resolve config/data/cache/state paths via `config path --json`
### FORBIDDEN
- NEVER invent product env vars for settings/encryption/LLM keys/logging/cache
- NEVER use `.env` as runtime product config
- NEVER log `encryption_key` or `openrouter_api_key` values
- NEVER invent config keys outside the 16 supported keys
- NEVER set `cache_redis_url` to `rediss://` (TLS is fail-closed; plain `redis://` only)
### Correct Pattern
```bash
browser-automation-cli config init --json
browser-automation-cli config path --json
browser-automation-cli config list-keys --json
browser-automation-cli config set timeout 90 --json
browser-automation-cli config set log_level info --json
browser-automation-cli config set log_to_file false --json
browser-automation-cli config set chrome_path /usr/bin/google-chrome --json
browser-automation-cli config set cache_backend sqlite --json
browser-automation-cli config set openrouter_api_key "replace-me" --json
browser-automation-cli config show --json
browser-automation-cli config get timeout --json
```

## Contract Rules
### REQUIRED
- ALWAYS use `doctor` for Chrome/XDG health; `commands --json` for inventory; `schema <cmd>` or `schema --cmd <cmd>` before inventing argv; `version --json` to pin identity
- ALWAYS treat `run --script` as accepting NDJSON (one object per line) OR a top-level JSON array of step objects
- ALWAYS on `run` fail-fast error envelopes inspect partial `data.steps` when present
- ALWAYS use `--json-steps` when per-step stdout streaming is REQUIRED
- ALWAYS use `find-paths` with `--glob` for shell-style filters (and/or regex pattern, `--extension`, `--type`, `--limit`, `--max-depth`, `--hidden`, `--no-ignore`)
- ALWAYS use `sheet-write <input> -o|--out <out.xlsx>` (CSV or JSON array-of-objects); pass `--sheet` when a non-default sheet name is REQUIRED
- ALWAYS use `sg-scan [PATHS]...` for structural lint; `sg-rewrite [PATHS]...` dry-run default; pass `--apply` only when write is REQUIRED
- ALWAYS resolve lighthouse binary as flag `--lighthouse-path` → XDG `lighthouse_path` → PATH; ALWAYS inspect envelope `binary_source` as `real|mock`
- ALWAYS use `grab --path <file>` (NEVER bare positional); `type` with positional TEXT plus `--target` OR `--focus-only`
- ALWAYS pass fill-form as command `--json '[{"target":"@eN","value":"x"}]'` plus global `--json`; upload requires target+path
- ALWAYS use `pick` / `select-option` for custom select / badge / popover / `role=option` (via `run` steps or `exec`; fields `target` + `option`)
- ALWAYS use `click-at` only with `--experimental-vision`; `screencast` only with `--experimental-screencast`
- ALWAYS use `console` only after same-process `--capture-console`; `net` only after same-process `--capture-network`
- ALWAYS expect `console dump` to write a valid JSON array (`[]` when empty — NEVER empty 0-byte files)
- ALWAYS compose `emulate` with `--user-agent`/`--viewport`/`--network-conditions` (NEVER `--device`)
- ALWAYS gate `heap` with `--category-memory`; `extension` with `--category-extensions`; `devtools3p` with `--category-third-party`; `webmcp` with `--category-webmcp`
- ALWAYS run `extension install|uninstall|list|reload|trigger` as top-level commands with `--category-extensions` (intentionally OUTSIDE `run --script`)
- ALWAYS bind MITM to `127.0.0.1` only; treat CA/captures as sensitive host material
- ALWAYS prefer `mitm capture-url <URL>` for one-shot proxy + Chrome + navigate + capture
- ALWAYS use workflow JSON manifests only; resume skips successful journal steps under XDG state
- ALWAYS treat `exec` as single-step inline only (NOT multi-step engine)
- ALWAYS use scrape formats `text|markdown|html|raw-html|links|metadata|screenshot|summary|product|branding` (CSV or repeatable `--format` / alias `--formats`) and engines `http|browser`; HTTP engine MUST NOT launch Chrome
- ALWAYS use `batch-scrape` / `crawl` with `--engine browser` when JS-rendered pages are REQUIRED
- ALWAYS treat scrape `--webhook-url` as one-shot operator POST of result data (NOT product telemetry)
- ALWAYS use scrape `--only-main-content` when main-content extraction is REQUIRED
- ALWAYS use `goto` with `--init-script`, `--handle-before-unload accept|dismiss`, and `--navigation-timeout-ms` when the task REQUIRES them
- ALWAYS use `reload --ignore-cache` when a cache-bypass reload is REQUIRED (NEVER invent `goto --ignore-cache`)
- ALWAYS use `page new --isolated-context` for an isolated browser context (flag alone uses the default isolated name; pass a name for a named context); in `run` steps set `"isolated_context":"my-ctx"` or `true`
- ALWAYS use `print-pdf --path` for PDF artifacts; ALWAYS navigate first OR pass `--url` in the same one-shot; ALWAYS use `print-pdf` steps inside `run` AFTER `goto`; blank `about:blank` without navigated content is REFUSED (expected fail-closed)
- ALWAYS use `view --allow-empty` only when blank `about:blank` snapshots are intentional; otherwise empty page fails closed
- ALWAYS use `dialog accept|dismiss --if-present` when a dialog may or may not be showing
- ALWAYS use `monitor check` with `--url` and `--baseline`
- ALWAYS use `qr encode --text` / `qr decode --path`
- ALWAYS use `parse` for html/md/txt/pdf/docx/xlsx/ods; pass `--redact-pii` when masking PII is REQUIRED
- ALWAYS set XDG `openrouter_api_key` before `extract --llm`; ALWAYS pass `--question`; ALWAYS pass `--schema-json` when structured LLM extract is REQUIRED
- ALWAYS expect `attr` to fall back to DOM properties when the HTML attribute is null
- ALWAYS use `wait` multi `--text` as OR (any match resolves); NEVER as AND
- ALWAYS use `wait` multi-selector as OR (`selector` comma-list, `selectors` array, or repeatable values); report `matched_selector` when present
- ALWAYS use `wait` with `url` / `url_contains` / `navigation` in `run` scripts when post-navigation stability is REQUIRED
- ALWAYS use `assert console-empty` / `assert console-no-match --pattern` (or NDJSON `kind=console_empty` / `console_no_match`) after same-process `--capture-console`
- ALWAYS use `scroll --delta-y`/`--delta-x` (NDJSON MUST use `dy` or `delta_y`); `assert url … --contains` (NDJSON MUST use `url_contains` when contains is REQUIRED)
- ALWAYS expect clap usage failures to emit JSON error envelopes when `--json` is already on argv (`ok:false`, `error.kind=usage`, exit 2)
- ALWAYS copy full argv from `references/formulas.md` when building one-shot commands
### FORBIDDEN
- NEVER invent aliases `snapshot`, `click`, `fill`, `screenshot`, or `bac`
- NEVER expect page state or `@eN` to survive FINALIZE DIE into a new process
- NEVER invent cloud scrape SaaS or remote sticky workflow servers
- NEVER replace browser `run --script` multi-step `@eN` work with workflow
- NEVER set Redis cache with `rediss://`
- NEVER print a blank page with `print-pdf` (ALWAYS navigate first or pass `--url`)
- NEVER put `extension install|uninstall` inside `run --script` (ALWAYS top-level with `--category-extensions`)
- NEVER invent `goto --ignore-cache` (reload owns `--ignore-cache`)
### Correct Pattern
```bash
browser-automation-cli schema goto --json
browser-automation-cli schema --cmd run --json
browser-automation-cli --json grab --path /tmp/page.png --full-page
browser-automation-cli --json assert url https://example.com --contains
browser-automation-cli --json find-paths --glob '**/*.rs' .
```

## Inventory
### REQUIRED
- ALWAYS treat this exact **61-name** surface as MANDATORY inventory
- ALWAYS load at least one executable line per name from `references/formulas.md`

doctor commands schema version goto view press click-at write keys type wait hover drag fill-form select-option pick upload back forward reload eval grab print-pdf monitor run exec extract text scroll cookie attr assert console net page dialog scrape batch-scrape crawl map search parse qr find-paths sg-scan sg-rewrite sheet-write mitm workflow config emulate resize perf lighthouse screencast heap extension devtools3p webmcp completions

### FORBIDDEN
- NEVER invent alias names outside this inventory
- NEVER omit PRD-only commands when they are the correct tool
### Correct Pattern
```bash
browser-automation-cli commands --json
browser-automation-cli schema print-pdf --json
browser-automation-cli schema pick --json
browser-automation-cli schema sheet-write --json
```

## Action Playbooks
### REQUIRED
- ALWAYS execute these formulas as-is unless `schema <cmd>` forces a flag change
- ALWAYS keep `@eN` multi-step work inside one `run --script` process
- ALWAYS validate envelope `ok` after every invocation
- ALWAYS use `references/formulas.md` for the remaining surface
### FORBIDDEN
- NEVER invent `bac`, product env vars, bare `grab` paths, `emulate --device`, `rediss://`, or non-JSON workflow manifests
### Correct Pattern

#### A. doctor / version / commands / schema positional
```bash
browser-automation-cli doctor --offline --quick --json
browser-automation-cli version --json
browser-automation-cli commands --json
browser-automation-cli schema goto --json
browser-automation-cli schema run --json
browser-automation-cli schema --cmd sheet-write --json
```

#### B. config all 16 keys + list-keys
```bash
browser-automation-cli config init --json
browser-automation-cli config path --json
browser-automation-cli config list-keys --json
browser-automation-cli config set lang en --json
browser-automation-cli config set timeout 90 --json
browser-automation-cli config set artifacts_dir /tmp/browser-automation-cli-artifacts --json
browser-automation-cli config set ignore_robots false --json
browser-automation-cli config set namespace demo --json
browser-automation-cli config set encryption_key "replace-me" --json
browser-automation-cli config set color true --json
browser-automation-cli config set log_level info --json
browser-automation-cli config set log_to_file false --json
browser-automation-cli config set chrome_path /usr/bin/google-chrome --json
browser-automation-cli config set lighthouse_path /usr/bin/lighthouse --json
browser-automation-cli config set openrouter_api_key "replace-me" --json
browser-automation-cli config set llm_base_url "https://openrouter.ai/api/v1" --json
browser-automation-cli config set llm_model "openai/gpt-4o-mini" --json
browser-automation-cli config set cache_backend sqlite --json
browser-automation-cli config set cache_redis_url "redis://127.0.0.1:6379" --json
browser-automation-cli config show --json
browser-automation-cli config get timeout --json
```

#### C. goto beforeunload accept|dismiss / reload --ignore-cache
```bash
browser-automation-cli --timeout 60 --json goto https://example.com \
  --init-script 'window.__ready=true' \
  --handle-before-unload accept \
  --navigation-timeout-ms 15000
browser-automation-cli --timeout 60 --json goto https://example.com --handle-before-unload dismiss
browser-automation-cli --json reload --ignore-cache
# FORBIDDEN: goto --ignore-cache (reload owns --ignore-cache)
```

#### D. page new --isolated-context
```bash
browser-automation-cli --json page new --isolated-context
browser-automation-cli --json page new --isolated-context my-ctx --url https://example.com
cat > /tmp/isolated.browser-automation.jsonl <<'JSONL'
{"cmd":"page","action":"new","isolated_context":"my-ctx","url":"https://example.com"}
{"cmd":"page","action":"new","isolated_context":true}
{"cmd":"wait","ms":300}
{"cmd":"view"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/isolated.browser-automation.jsonl
```

#### E. wait multi-selector OR + url / url_contains / navigation
```bash
cat > /tmp/wait.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","selector":"h1, main, #content","wait_timeout_ms":10000}
{"cmd":"wait","selectors":["#app",".ready"],"wait_timeout_ms":10000}
{"cmd":"wait","url_contains":"example.com","wait_timeout_ms":10000}
{"cmd":"wait","navigation":true,"wait_timeout_ms":10000}
{"cmd":"wait","text":["Example","Demo"],"ms":500}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/wait.browser-automation.jsonl
browser-automation-cli --json wait --selector "h1" --state load --ms 500
browser-automation-cli --json wait --text Example --text Demo --ms 1000
```

#### F. run NDJSON / array / --json-steps / print-pdf step (AFTER goto)
```bash
cat > /tmp/form.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":500}
{"cmd":"view"}
{"cmd":"write","target":"@e1","value":"hello"}
{"cmd":"press","target":"@e2"}
{"cmd":"print-pdf","path":"/tmp/form.pdf"}
{"cmd":"grab","path":"/tmp/form.png"}
JSONL
browser-automation-cli --timeout 90 --json run --script /tmp/form.browser-automation.jsonl
browser-automation-cli --timeout 90 --json --json-steps run --script /tmp/form.browser-automation.jsonl

cat > /tmp/demo.array.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"wait","ms":500},
  {"cmd":"view"},
  {"cmd":"scroll","dy":400},
  {"cmd":"assert","kind":"url","url_contains":"example.com"},
  {"cmd":"print-pdf","path":"/tmp/array-run.pdf"},
  {"cmd":"grab","path":"/tmp/array-run.png"}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/demo.array.json
```

#### G. select-option / pick (custom select / badge / popover / role=option)
```bash
cat > /tmp/pick.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":400}
{"cmd":"view"}
{"cmd":"pick","target":"@e1","option":"Anomalia"}
{"cmd":"select-option","target":"@e2","option":"Alta"}
JSONL
browser-automation-cli --timeout 90 --json run --script /tmp/pick.browser-automation.jsonl
browser-automation-cli --json exec pick --target @e1 --option Anomalia
```

#### H. assert console_empty / console_no_match
```bash
cat > /tmp/assert-console.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":300}
{"cmd":"assert","kind":"console_empty"}
{"cmd":"assert","kind":"console_no_match","pattern":"TypeError"}
JSONL
browser-automation-cli --capture-console --timeout 60 --json run --script /tmp/assert-console.browser-automation.jsonl
browser-automation-cli --capture-console --json assert console-empty
browser-automation-cli --capture-console --json assert console-no-match --pattern TypeError
```

#### I. dialog --if-present / view --allow-empty / console dump []
```bash
browser-automation-cli --json dialog accept --if-present
browser-automation-cli --json dialog dismiss --if-present
browser-automation-cli --json view --allow-empty
cat > /tmp/console-dump.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"console","action":"clear"}
{"cmd":"console","action":"dump","path":"/tmp/console.json"}
JSONL
browser-automation-cli --capture-console --timeout 60 --json run --script /tmp/console-dump.browser-automation.jsonl
# /tmp/console.json MUST be valid JSON array (at least "[]")
```

#### J. scrape multi-format / batch-scrape --engine browser
```bash
browser-automation-cli --json scrape https://example.com --format markdown,links,metadata --engine http --only-main-content
browser-automation-cli --json scrape https://example.com --format text --format html --engine browser
browser-automation-cli --json scrape https://example.com --format text --engine http --webhook-url https://127.0.0.1:9000/hook
printf '%s\n' https://example.com https://example.org > /tmp/urls.txt
browser-automation-cli --timeout 120 --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2 --engine browser
browser-automation-cli --json batch-scrape --urls-file /tmp/urls.txt --format markdown --engine http
browser-automation-cli --json crawl https://example.com --limit 20 --max-depth 2 --format text --engine browser
```

#### K. mitm capture-url
```bash
browser-automation-cli --json mitm init-ca
browser-automation-cli --timeout 60 --json mitm capture-url https://example.com --seconds 30 --har /tmp/capture.har
browser-automation-cli --json mitm status
browser-automation-cli --json mitm list --limit 50
browser-automation-cli --json mitm har --out /tmp/capture2.har
```

#### L. extract LLM via XDG
```bash
browser-automation-cli config set openrouter_api_key "replace-me" --json
browser-automation-cli config set llm_base_url "https://openrouter.ai/api/v1" --json
browser-automation-cli config set llm_model "openai/gpt-4o-mini" --json
printf '%s\n' '{"type":"object","properties":{"title":{"type":"string"}},"required":["title"]}' > /tmp/extract.schema.json
browser-automation-cli --timeout 120 --json extract --llm --question "What is the main title?" --schema-json /tmp/extract.schema.json https://example.com
```

#### M. print-pdf one-shot (ALWAYS --url or prior navigate) / monitor / qr / find-paths
```bash
# REQUIRED: pass --url for one-shot navigate, OR print only after navigated content exists
browser-automation-cli --timeout 60 --json print-pdf --path /tmp/page.pdf --url https://example.com
# FORBIDDEN blank print (expected refuse without navigated content or --url):
# browser-automation-cli --json print-pdf --path /tmp/blank.pdf
browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/example.baseline --write-baseline --engine http
browser-automation-cli --json qr encode --text "https://example.com" --format png --path /tmp/qr.png
browser-automation-cli --json qr decode --path /tmp/qr.png
browser-automation-cli --json find-paths --glob '**/*.rs' .
```

#### N. sheet-write / sg-scan / sg-rewrite / parse
```bash
printf '%s\n' 'name,value' 'a,1' 'b,2' > /tmp/rows.csv
browser-automation-cli --json sheet-write /tmp/rows.csv -o /tmp/out.xlsx --sheet Data
browser-automation-cli --json sg-scan . --limit 100
browser-automation-cli --json sg-rewrite . --apply
browser-automation-cli --json parse /tmp/doc.pdf
browser-automation-cli --json parse /tmp/doc.docx --redact-pii
```

#### O. extension install|uninstall OUTSIDE run
```bash
browser-automation-cli --category-extensions --json extension list
browser-automation-cli --category-extensions --json extension install --path /tmp/ext
browser-automation-cli --category-extensions --json extension uninstall --id <ext-id>
# FORBIDDEN: extension install|uninstall inside run --script (ALWAYS top-level)
```

#### P. clap --json usage error envelope
```bash
set +e
out=$(browser-automation-cli --json not-a-real-cmd 2>/dev/null)
code=$?
set -e
echo "$out" | jaq -e '.ok == false'
echo "$out" | jaq -e '.error.kind == "usage"'
echo "exit=$code"
# exit MUST be 2; envelope MUST be JSON when --json already on argv
```

#### Q. fail-fast data.steps
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
echo "exit=$code"
```

#### R. lighthouse + redis XDG
```bash
browser-automation-cli --timeout 180 --json lighthouse https://example.com | jaq '.data.binary_source // .'
browser-automation-cli config set cache_backend redis --json
browser-automation-cli config set cache_redis_url "redis://127.0.0.1:6379" --json
# FORBIDDEN: rediss://
```

#### S. workflow JSON
```bash
cat > /tmp/wf.json <<'JSON'
{"name":"demo","steps":[{"id":"ping","cmd":"echo","args":{"message":"start"}}]}
JSON
browser-automation-cli --json workflow run --manifest /tmp/wf.json
browser-automation-cli --json workflow resume --manifest /tmp/wf.json
browser-automation-cli --json workflow status --name demo
```

## Multi-step Run Scripts
### REQUIRED
- ALWAYS use `run --script <path>` for multi-step work in one process
- ALWAYS accept script body as NDJSON (one JSON object per line with field `cmd`) OR a top-level JSON array of step objects
- ALWAYS keep shared page state and `@eN` refs inside that single process
- ALWAYS set `--timeout` large enough for the full script
- ALWAYS pass `--json-steps` when agents MUST stream per-step results
- ALWAYS encode grab as `{"cmd":"grab","path":"/tmp/example.png"}` inside steps
- ALWAYS encode print-pdf as `{"cmd":"print-pdf","path":"/tmp/example.pdf"}` AFTER a `goto` step (or pass `"url"` when one-shot navigate is REQUIRED)
- ALWAYS encode isolated page new as `{"cmd":"page","action":"new","isolated_context":"my-ctx"}` or `"isolated_context":true`
- ALWAYS encode pick as `{"cmd":"pick","target":"@eN","option":"..."}` or `{"cmd":"select-option",...}`
- ALWAYS encode scroll dy as `{"cmd":"scroll","dy":400}` or `"delta_y":400`
- ALWAYS encode url assert as `{"cmd":"assert","kind":"url","url_contains":"example.com"}` when using contains
- ALWAYS encode console asserts as `{"cmd":"assert","kind":"console_empty"}` or `{"cmd":"assert","kind":"console_no_match","pattern":"..."}`
- ALWAYS on fail-fast errors parse partial `data.steps` from the error envelope when present
### FORBIDDEN
- NEVER split ref-dependent steps across processes
- NEVER treat `exec` as multi-step engine
- NEVER expect `@eN` to survive process DIE
- NEVER print-pdf on blank unnavigated pages inside `run` (ALWAYS `goto` first)
### Correct Pattern
```bash
cat > /tmp/demo.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com","init_script":"window.__x=1","handle_before_unload":"accept","navigation_timeout_ms":15000}
{"cmd":"wait","selector":"h1, main","wait_timeout_ms":10000}
{"cmd":"wait","url_contains":"example.com"}
{"cmd":"page","action":"new","isolated_context":"my-ctx","url":"https://example.com"}
{"cmd":"view"}
{"cmd":"scroll","dy":400}
{"cmd":"assert","kind":"url","url_contains":"example.com"}
{"cmd":"print-pdf","path":"/tmp/example.pdf"}
{"cmd":"grab","path":"/tmp/example.png"}
JSONL
browser-automation-cli --timeout 60 --json --json-steps run --script /tmp/demo.browser-automation.jsonl
```

## Workflow Manifest
### REQUIRED
- ALWAYS use `workflow run --manifest <path>` with JSON path
- ALWAYS use `workflow resume --manifest <path>`; `workflow status`; pass `--journal` when non-default journal path is REQUIRED
### FORBIDDEN
- NEVER use non-JSON workflow manifests
- NEVER replace browser `@eN` multi-step `run --script` with workflow
### Correct Pattern
```bash
cat > /tmp/wf.json <<'JSON'
{"name":"demo","steps":[{"id":"ping","cmd":"echo","args":{"message":"start"}}]}
JSON
browser-automation-cli --json workflow run --manifest /tmp/wf.json
```

## JSON Envelope
### REQUIRED
- ALWAYS expect success `{"schema_version":1,"ok":true,"data":...}`
- ALWAYS expect error under `--json` `{"schema_version":1,"ok":false,"error":{...}}`
- ALWAYS validate `ok` before reading `data`
- ALWAYS on `run` fail-fast error envelopes inspect partial `data.steps` when present
- ALWAYS on `--json-steps` consume one NDJSON object per completed step
- ALWAYS on lighthouse success inspect `data.binary_source` as `real|mock`
- ALWAYS expect clap usage failures (bad argv / unknown subcommand) to emit JSON envelopes when `--json` is already on argv (`error.kind=usage`, exit 2)
- ALWAYS keep stderr for diagnostics/tracing only
### FORBIDDEN
- NEVER treat human prose stdout under `--json` as the primary contract
- NEVER ignore `ok:false` with non-zero exit
- NEVER assume usage failures are prose-only when `--json` is on argv
### Correct Pattern
```bash
out=$(browser-automation-cli -q --json version)
echo "$out" | jaq -e '.ok == true'
set +e; bad=$(browser-automation-cli --json not-a-real-cmd 2>/dev/null); set -e
echo "$bad" | jaq -e '.ok == false and .error.kind == "usage"'
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
- ALWAYS keep the DevTools parity map as the interaction core, AND use extra PRD surface (`print-pdf`, `pick`/`select-option`, `monitor`, `qr`, `find-paths`, `sheet-write`, `sg-scan`, `sg-rewrite`, parse/extract/scrape family) when the task needs them
- ALWAYS use flags/XDG for product settings; product logging MUST use `--verbose`/`--debug`/`-q` or `config set log_level` / `log_to_file`
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
- NEVER set `cache_redis_url` to `rediss://` (ALWAYS plain `redis://` only)
- NEVER print blank pages with `print-pdf` (ALWAYS `--url` or prior navigate)
- NEVER put `extension install|uninstall` inside `run` (ALWAYS top-level `--category-extensions`)
- NEVER invent `goto --ignore-cache`
### Correct Pattern
```bash
browser-automation-cli --json grab --path /tmp/x.png
browser-automation-cli --json workflow run --manifest /tmp/wf.json
browser-automation-cli --timeout 60 --json run --script /tmp/steps.jsonl
browser-automation-cli config set cache_redis_url "redis://127.0.0.1:6379" --json
browser-automation-cli --json page new --isolated-context my-ctx --url https://example.com
browser-automation-cli --timeout 60 --json print-pdf --path /tmp/page.pdf --url https://example.com
```

## Agent Validation Checklist
### REQUIRED
- ALWAYS confirm binary `browser-automation-cli` and lifecycle BORN EXECUTE FINALIZE DIE
- ALWAYS confirm `--json` envelope `ok` and multi-step `@eN` inside one `run --script`
- ALWAYS confirm `run --script` accepts NDJSON AND top-level JSON array
- ALWAYS confirm `--json-steps` streams per-step NDJSON when required
- ALWAYS confirm fail-fast partial `data.steps` on error envelopes
- ALWAYS confirm clap usage failures emit JSON when `--json` is already on argv
- ALWAYS confirm `grab --path`, JSON workflow manifest, no `emulate --device`, wait multi-text OR, wait multi-selector OR, wait url/url_contains/navigation
- ALWAYS confirm `pick`/`select-option`, `print-pdf` AFTER navigate or with `--url` (blank refuse), `dialog --if-present`, `view --allow-empty`, `console dump` → `[]`
- ALWAYS confirm `page new --isolated-context` and run field `isolated_context` string or true
- ALWAYS confirm `reload --ignore-cache` only (NEVER `goto --ignore-cache`)
- ALWAYS confirm `extension install|uninstall` top-level OUTSIDE run with `--category-extensions`
- ALWAYS confirm console/net capture only with same-process capture flags
- ALWAYS confirm `assert console-empty` / `console-no-match` and NDJSON `console_empty` / `console_no_match`
- ALWAYS confirm `type` positional TEXT + `--target` OR `--focus-only`; fill-form command `--json` array + global `--json`
- ALWAYS confirm all 16 config keys via `config list-keys`; NEVER invent product env; logging via `--verbose`/`--debug`/`-q`/`log_level`/`log_to_file` only
- ALWAYS confirm `cache_backend` sqlite|memory|redis and `cache_redis_url` redis:// only (rediss fail-closed)
- ALWAYS confirm lighthouse resolve order flag → XDG → PATH and envelope `binary_source` real|mock
- ALWAYS confirm scrape multi-format, batch-scrape/crawl `--engine browser`, mitm capture-url, schema positional
- ALWAYS confirm `find-paths --glob`, `sheet-write`, `sg-scan`, `sg-rewrite`
- ALWAYS confirm exit codes 0,2,65,66,69,70,74,78,124,130,141; robots dual-flag; category/experimental gates; schema discovery
- ALWAYS confirm full **61-command** inventory and `references/formulas.md` formulas
### FORBIDDEN
- NEVER ship agent glue that violates this checklist
### Correct Pattern
```bash
browser-automation-cli commands --json
browser-automation-cli schema run --json
browser-automation-cli config list-keys --json
browser-automation-cli doctor --offline --quick --json
browser-automation-cli --json page new --isolated-context
```
