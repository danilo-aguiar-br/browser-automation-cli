---
name: browser-automation-cli
description: Triggers when the user asks for one-shot browser automation, Chrome CDP agent CLI, accessibility snapshot refs, fill form, screenshot, scrape page, network or console capture, heap snapshot, lighthouse, multi-step run scripts, headless chrome, or browser-automation-cli. This skill MUST be used to invoke the Rust CLI browser-automation-cli for non-interactive, JSON-enveloped browser work with NASCE EXECUTA FINALIZE MORRE lifecycle. Covers global flags, command inventory, run NDJSON scripts, exit codes, envelopes, category gates, experimental vision and screencast, robots policy, and environment variables.
---

# browser-automation-cli


## Identity and Architecture
### REQUIRED
- Treat the binary name as always `browser-automation-cli`
- Treat one process as one Chrome lifecycle: NASCE, EXECUTA, FINALIZE, MORRE
- Use system Chrome or Chromium discovered by the CLI
- Keep multi-step browser work inside `run --script` when `@eN` refs must survive
- Prefer `--json` for every programmatic consumer
- Install with `cargo install --path . --locked` while `publish = false`
- After crates.io release use `cargo install browser-automation-cli --locked`
- MSRV is Rust 1.88.0

### FORBIDDEN
- Do not invent a short alias such as `bac`
- Do not keep a daemon or sticky browser session between processes
- Do not expect npm packaging for this product
- Do not reuse `@eN` refs across separate process launches
- Do not enable remote telemetry

### Correct Pattern
```bash
browser-automation-cli doctor --offline --quick --json
browser-automation-cli -q --timeout 60 --json goto https://example.com
```


## Global Flags
### REQUIRED
- Pass `--json` for machine-readable envelopes
- Pass `-q` or `--quiet` when stderr prose would pollute agent transcripts
- Pass `--timeout <seconds>` for wall-clock process budget
- Pass `--step-timeout <seconds>` for per-step budgets inside `run`
- Pass `--headed` only for interactive debug
- Pass `--capture-console` when later `console` commands must see messages
- Pass `--capture-network` when later `net` commands must see requests
- Pass category gates only when deep tools are required
- Pass experimental gates only when those surfaces are intentionally needed

### FORBIDDEN
- Do not assume capture flags persist across separate process launches
- Do not enable category or experimental surfaces silently in agent defaults

### Correct Pattern
```bash
browser-automation-cli --json --timeout 90 --capture-network run --script steps.jsonl
browser-automation-cli --category-memory heap summary --path snap.heapsnapshot --json
browser-automation-cli --experimental-vision click-at --x 10 --y 20 --json
```


## Discovery Commands
### REQUIRED
- Use `doctor` to diagnose Chrome readiness
- Use `commands --json` for the live command inventory and DevTools tool map
- Use `schema --cmd <name> --json` before inventing argv for a command
- Use `version --json` to pin binary identity
- Use `completions <shell>` for human shell setup

### Correct Pattern
```bash
browser-automation-cli commands --json
browser-automation-cli schema --cmd goto --json
browser-automation-cli version --json
```


## Navigation and Pages
### REQUIRED
- Use `goto <url>` to navigate
- Use `back`, `forward`, and `reload` for history control
- Use `page list|new|select|close` for multi-tab management
- Use `scrape <url>` when only body text is needed
- Use `wait` for ms, text, selector, or load state

### Correct Pattern
```bash
browser-automation-cli --json goto https://example.com
browser-automation-cli --json page list
browser-automation-cli --json wait --text Example --ms 1000
```


## Snapshot and Input
### REQUIRED
- Use `view` for accessibility snapshots with `@eN` refs
- Use `press` for clicks, not a fictional `click` product name
- Use `write` for smart fill, not a fictional `fill` product name
- Use `type` for keystroke typing with optional `--target` or `--focus-only`
- Use `keys` for single key presses
- Use `hover`, `drag`, `fill-form`, and `upload` for the matching tool-ref actions
- Use `click-at` only with `--experimental-vision`

### FORBIDDEN
- Do not invent product aliases `snapshot`, `click`, `fill`, or `screenshot`
- Do not call `click-at` without the experimental vision gate

### Correct Pattern
```bash
browser-automation-cli --json view
browser-automation-cli --json press @e1 --include-snapshot
browser-automation-cli --json write @e2 "text"
browser-automation-cli --json fill-form --json '[{"target":"@e3","value":"x"}]'
```


## Observation and Artifacts
### REQUIRED
- Use `grab <path>` for screenshots
- Use `extract`, `text`, `attr`, `scroll`, and `assert` for content checks
- Use `cookie list|set|clear` for jar helpers
- Use `dialog` to accept or dismiss dialogs

### Correct Pattern
```bash
browser-automation-cli --json grab /tmp/page.png --full-page
browser-automation-cli --json text @e2
browser-automation-cli --json cookie list
```


## Capture, Eval, and DevTools Depth
### REQUIRED
- Use `console list|get` only after `--capture-console` on the same process
- Use `net list|get` only after `--capture-network` on the same process
- Use `eval` for JavaScript evaluation
- Use `emulate` and `resize` for device and viewport control
- Use `perf start|stop|insight` for performance work
- Use `lighthouse` when the external binary is available
- Use `screencast` only with `--experimental-screencast`
- Use deep `heap` analysis only with `--category-memory`
- Use `extension` only with `--category-extensions`
- Use `devtools3p` only with `--category-third-party`
- Use `webmcp` only with `--category-webmcp`

### Correct Pattern
```bash
browser-automation-cli --capture-console --json console list
browser-automation-cli --json eval 'document.title'
browser-automation-cli --category-memory --json heap summary --path snap.heapsnapshot
```


## Multi-step Run Scripts
### REQUIRED
- Use `run --script <path>` for NDJSON steps in one process
- Put one JSON object per line with a `cmd` field
- Keep shared page state and `@eN` refs inside that single process
- Set `--timeout` large enough for the full script

### FORBIDDEN
- Do not split ref-dependent steps across multiple CLI processes
- Do not treat `exec` as a full multi-step engine

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


## JSON Envelope
### REQUIRED
- Expect success stdout as `{"schema_version":1,"ok":true,"data":...}`
- Expect error stdout under `--json` as `{"schema_version":1,"ok":false,"error":{...}}`
- Validate `ok` before reading `data`
- Keep stderr for diagnostics and tracing only

### Correct Pattern
```bash
out=$(browser-automation-cli -q --json version)
echo "$out" | jaq -e '.ok == true'
echo "$out" | jaq -r '.data.version'
```


## Exit Codes and Retry
### REQUIRED
- Branch on exit code before trusting stdout
- Treat `0` as success
- Treat `2` as usage and fix argv
- Treat `65` as data error
- Treat `66` as no input
- Treat `69` as unavailable and repair Chrome install
- Treat `70` as software, browser, or protocol failure
- Treat `74` as I/O failure
- Treat `78` as config failure
- Treat `124` as timeout and raise budget or shorten work
- Treat `130` as cancellation
- Treat `141` as broken pipe
- Retry only transient host or browser launch failures with backoff

### FORBIDDEN
- Do not retry pure usage failures without changing argv
- Do not mask exit codes with `|| true` in agent pipelines


## Environment Variables
### REQUIRED
- Honour `BROWSER_AUTOMATION_CLI_JSON`, `QUIET`, `VERBOSE`, `DEBUG`, `TIMEOUT`, and `STEP_TIMEOUT`
- Honour capture, category, experimental, headed, artifacts, lang, robots, encryption, namespace, and color variables documented in README
- Keep `BROWSER_AUTOMATION_CLI_ENCRYPTION_KEY` out of durable logs

### FORBIDDEN
- Do not log encryption keys or cookie values


## Robots Policy
### REQUIRED
- Respect robots defaults
- When bypassing, satisfy the dual-flag policy with `--ignore-robots` and `--i-accept-robots-risk`

### FORBIDDEN
- Do not bypass robots with a single casual flag in agent automation


## Canonical Names
### REQUIRED
- Use `view` not snapshot
- Use `press` not click
- Use `write` not fill
- Use `grab` not screenshot
- Use binary `browser-automation-cli` only
