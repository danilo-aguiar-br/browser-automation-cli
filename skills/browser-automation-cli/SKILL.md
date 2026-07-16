---
name: browser-automation-cli
description: One-shot browser automation CLI for AI agents via Chrome CDP. Use for navigate, a11y view with @eN refs, input, network/console capture, heap, perf, extensions, multi-step run scripts in a single process. Install with cargo. Triggers include browser automation, fill form, screenshot, scrape page, headless chrome, CDP agent CLI, heap snapshot, lighthouse.
---

# browser-automation-cli


## Install
```bash
cargo install --path . --locked
# or after crates.io: cargo install browser-automation-cli --locked
```


## Model (one-shot)
- One process = one Chrome lifecycle (NASCE EXECUTA FINALIZE MORRE)
- Multi-step browser work only with `run --script` in the same process
- Page refs `@eN` live only inside that process
- PROIBIDO: session daemon between commands
- PROIBIDO: npm install for this product
- Binary name is always `browser-automation-cli` (never `bac`)


## Global flags
```bash
browser-automation-cli --json <cmd> ...
browser-automation-cli -q --timeout 120 <cmd> ...
browser-automation-cli -v --step-timeout 30 run --script steps.jsonl
browser-automation-cli --headed goto https://example.com --json
browser-automation-cli --capture-console --capture-network run --script s.jsonl --json
browser-automation-cli --category-memory heap summary --path snap.heapsnapshot --json
browser-automation-cli --category-extensions extension list --json
browser-automation-cli --experimental-vision click-at --x 10 --y 20 --json
browser-automation-cli --experimental-screencast screencast start --json
```


## Essential commands
```bash
browser-automation-cli doctor --offline --quick --json
browser-automation-cli commands --json
browser-automation-cli schema --cmd goto --json
browser-automation-cli goto https://example.com --json
browser-automation-cli view --json
browser-automation-cli press @e1 --include-snapshot --json
browser-automation-cli write @e2 "text" --json
browser-automation-cli text @e2 --json
browser-automation-cli scroll --delta-y 400 --json
browser-automation-cli grab out.png --full-page --json
browser-automation-cli cookie list --json
browser-automation-cli run --script steps.jsonl --json
```


## DevTools tool map (tool-ref → CLI)
- click → `press` (`--dblclick`, `--include-snapshot`)
- fill → `write`
- fill_form → `fill-form --json '[...]'`
- type_text → `type TEXT [--target] [--submit] [--focus-only]`
- press_key → `keys`
- hover/drag/upload/dialog → same names
- click_at → `click-at --x --y` (needs `--experimental-vision`)
- navigate_page → `goto` | `back` | `forward` | `reload`
- list/new/select/close pages → `page list|new|select|close` (`--page-id` alias; `page new --isolated-context` tries Browser.createBrowserContext and documents limitation if unsupported)
- wait_for → `wait --text ... --selector ... --state ...`
- take_snapshot → `view`
- take_screenshot → `grab`
- evaluate_script → `eval`
- list/get console → `console list|get` (needs `--capture-console`)
- list/get network → `net list|get` (needs `--capture-network`)
- emulate / resize_page → `emulate` / `resize`
- performance_* → `perf start|stop|insight`
- lighthouse_audit → `lighthouse`
- screencast_* → `screencast start|stop` (experimental flag)
- heap * → `heap take|close|compare|summary|details|...` (deep ops: `--category-memory`)
- extension * → `extension ...` (`--category-extensions`)
- third-party / webmcp → `devtools3p` / `webmcp` with category flags


## Multi-step recipe
```bash
cat > /tmp/demo.browser-automation.jsonl <<'EOF'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":500}
{"cmd":"view"}
{"cmd":"grab","path":"shot.png"}
EOF
browser-automation-cli run --script /tmp/demo.browser-automation.jsonl --json --timeout 60
```


## Envelope
- Success stdout: `{"schema_version":1,"ok":true,"data":...}`
- Error stdout (with `--json`): `{"schema_version":1,"ok":false,"error":{...}}`
- stderr: local diagnostics only (tracing); no remote telemetry


## PRD pillars not yet fully shipped
- Firecrawl local (batch-scrape, crawl, map, search formats)
- MITM one-shot (hudsucker HAR)
- Workflow journal (petgraph + rusqlite)
- See `docs_prd/parity_devtools_matrix.md` pillar table


## Canonical names
- `view` (not snapshot)
- `press` (not click)
- `write` (not fill)
- `grab` (not screenshot)
- Binary: `browser-automation-cli`
