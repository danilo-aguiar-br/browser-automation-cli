[English](COOKBOOK.md) | [Português Brasileiro](COOKBOOK.pt-BR.md)

# Cookbook — browser-automation-cli

> Practical recipes with copy-ready commands for one-shot browser work. Lifecycle: BORN EXECUTE FINALIZE DIE.


## Latency Note
- Chrome launch dominates cold start on browser-engine commands
- Prefer one `run` script over many separate launches when steps share state
- HTTP scrape, crawl, map, search, parse, qr, image, video, find-paths, sheet-write, sg-scan, and sg-rewrite avoid Chrome when you only need content or local IO
- Each process is BORN, EXECUTE, FINALIZE, DIE with no shared browser across invocations


## Default Values Reference
- Global timeout default is `0` meaning no process wall budget unless set by flag or XDG config
- Step timeout default is `0` meaning inherit global timeout
- Headless mode is default unless `--headed`
- JSON is off unless `--json`
- Product settings come from flags and `config` (XDG CLI) only
- Logging: `--verbose` / `--debug` / `-q` or XDG `log_level`
- Color: `config set color`; Chrome path: `config set chrome_path`
- Resolve paths with `config path --json`


## How To Init XDG Config
```bash
browser-automation-cli --json config init
browser-automation-cli --json config path
browser-automation-cli --json config show
browser-automation-cli --json config set timeout 60
browser-automation-cli --json config set lang en
browser-automation-cli --json config set namespace demo
browser-automation-cli --json config set artifacts_dir /tmp/browser-automation-cli-artifacts
browser-automation-cli --json config set ignore_robots false
browser-automation-cli --json config set encryption_key "replace-me-with-a-secret"
browser-automation-cli --json config set color true
browser-automation-cli --json config set log_level info
browser-automation-cli --json config set chrome_path /usr/bin/chromium
browser-automation-cli --json config set lighthouse_path ./scripts/mock-lighthouse.sh
browser-automation-cli --json config set dialog_settle_ms 2000
browser-automation-cli --json config list-keys
browser-automation-cli --json config get timeout
browser-automation-cli --json config get encryption_key
browser-automation-cli --json config get color
browser-automation-cli --json config get dialog_settle_ms
```
- `config init` creates XDG dirs and default `config.toml`
- Discover live keys with `config list-keys --json` (do not hard-code a fixed count; includes `dialog_settle_ms` and more)
- Flags always override file config for that invocation
- Product settings use only flags and `config path|init|show|set|get|unset|list-keys`


## How To Unset a Config Key
```bash
browser-automation-cli --json config set stealth_seed fleet-01
browser-automation-cli --json config get stealth_seed
browser-automation-cli --json config unset stealth_seed
browser-automation-cli --json config get stealth_seed
```
- `config unset <KEY>` restores one key to its built-in default and is the real inverse of `set`
- `config set <key> ""` is not an inverse: on a string key it writes an empty value the normal path never produces
- On a numeric key that same empty value is a parse error, not a reset
- Unsetting a key that is already absent succeeds, so a script never needs to know the previous state


## How To Configure XDG LLM Keys
```bash
browser-automation-cli --json config set openrouter_api_key YOUR_KEY
browser-automation-cli --json config set llm_base_url https://openrouter.ai/api/v1
browser-automation-cli --json config set llm_model openai/gpt-4o-mini
browser-automation-cli --json config get openrouter_api_key
```
- Keys are stored under XDG `config.toml` only
- `extract --llm` fails closed when `openrouter_api_key` is missing


## How To Diagnose Install Health
```bash
browser-automation-cli doctor --offline --quick --json
```
- Offline quick mode checks local Chrome discovery without network probes
- Use full doctor without `--quick` when you need deeper readiness checks
- Doctor also reports residual disk hygiene (check `residual_disk` + top-level `residual`)


## How To Verify Residual-Zero Disk Hygiene
```bash
# Path-light residual report (BORN may already have scavenged stale Singleton orphans)
# The binary reduces the payload; no JSON processor in the prompt.
browser-automation-cli --json --fields residual doctor --offline --quick

# Just the residual verdict, from a 26 KB envelope down to one line
browser-automation-cli --json --fields checks --filter-rows 'id=residual_disk' \
  doctor --offline --quick

# One-shot browser work should leave no CLI chrome markers
# Note: --url about:blank is intentional residual smoke (url present); not a blank PDF without url (GAP-013)
browser-automation-cli --json print-pdf --url about:blank --path /tmp/browser-automation-cli-residual-check.pdf

# Re-check residual fields after DIE
browser-automation-cli doctor --offline --quick --json | jaq '.residual'
```
- Top-level `residual` fields: `scanned_roots`, `cli_marker_dirs`, `chromium_tmp_singleton_orphans`, `scavenge_safe_candidates`, `live_cli_marker_processes` (legacy), `sibling_live_processes`, `orphan_marker_dirs`, `foreign_root_orphans`, `ghost_marker_processes`, `process_table_unavailable`
- Check id `residual_disk`: `fail` on `orphan_marker_dirs` or `ghost_marker_processes`; `warn` when marker dirs or Singleton orphans remain; else `pass`. A live sibling invocation is healthy and never fails.
- Residual-zero means zero live CLI marker processes, zero `browser-automation-cli-chrome-*` dirs, zero owned Singleton-only Chromium tmp litter after DIE
- Age floor for cross-run stale GC is 60s; host Flatpak Chrome temp is never wiped
- Maintainers (optional local gates, local maintainer scripts only):
  - `bash scripts/residual-check.sh`
  - `bash scripts/residual-stress.sh`


## How To Shrink an Envelope With --fields
```bash
# Full doctor envelope measures 26277 bytes on this host
browser-automation-cli --json doctor --offline --quick

# One dotted path takes the same answer down to 80 bytes
browser-automation-cli --json --fields residual.ghost_marker_processes doctor --offline --quick

# Paths are relative to data, so metadata resolves and data.metadata does not
browser-automation-cli --json --fields metadata scrape https://example.com --format metadata --engine http

# A path that resolves to nothing is reported, never silently dropped
browser-automation-cli --json --fields residual.nao_existe doctor --offline --quick
```
- `--fields` takes one CSV of dotted paths and is not repeatable
- Paths are rooted at `data`, so write `residual`, never `data.residual`
- The projection rebuilds the nesting each path implies
- Unresolved paths land in `agent_ops.unresolved_paths` with `flag` and `path`
- The reduction happens inside the binary, so no JSON processor is needed


## How To Count Rows With --count-only
```bash
# Replace the whole row payload with a single count
browser-automation-cli --json --fields checks --count-only doctor --offline --quick

# Count only the rows a filter keeps
browser-automation-cli --json --fields checks --filter-rows status=info --count-only \
  doctor --offline --quick
```
- `--count-only` emits `{"count": N}` in place of the rows
- `agent_ops.total` and `agent_ops.matched` still report the filter arithmetic
- Use it to size a result set before you pay for the rows


## How To Sort, Limit, and Dedupe Rows
```bash
# Deterministic order plus a hard row ceiling
browser-automation-cli --json --fields checks --sort-rows id --limit-rows 3 \
  --truncate-content 24 doctor --offline --quick

# Keep the first row of each distinct status
browser-automation-cli --json --fields checks --dedupe-by status \
  --truncate-content 24 doctor --offline --quick

# Narrow to one row by id, then cap it
browser-automation-cli --json --fields checks --filter-rows id=residual_disk --limit-rows 1 \
  doctor --offline --quick
```
- `--sort-rows` takes a dotted path and compares numbers numerically
- `--limit-rows` applies after filter, dedupe, and sort
- `--dedupe-by` keeps the first row for each distinct value
- `--filter-rows` accepts `key=value`, `key!=value`, and `key~substring`
- A missing field never matches, so absence is not difference under `!=`


## How To Cap Payload With --truncate-content and --max-output-bytes
```bash
# Cut every string in the payload to 24 characters
browser-automation-cli --json --fields checks --filter-rows id=chrome --truncate-content 24 \
  doctor --offline --quick

# Hard byte ceiling; rows are shed from the end
browser-automation-cli --json --fields checks --max-output-bytes 400 doctor --offline --quick
```
- `--truncate-content N` cuts strings and marks `agent_ops.truncated` true
- `--max-output-bytes` sheds whole rows and reports `agent_ops.omitted_rows`
- Both flags are global, so they work on any command that emits JSON
- Combine them with `--fields` when one projection is still too large


## How To Pin One Stealth Identity Across Processes
```bash
# Without a seed each of these is a different machine to the far end
browser-automation-cli --timeout 60 --json goto https://example.com

# With a seed the whole fleet of one-shot processes looks like one browser
browser-automation-cli --timeout 60 --json --stealth-seed fleet-01 goto https://example.com
browser-automation-cli --timeout 60 --json --stealth-seed fleet-01 scrape https://example.com --format text

# Make it durable instead of repeating the flag
browser-automation-cli --json config set stealth_seed fleet-01
browser-automation-cli --json config set stealth_profile chrome-linux
```
- Stealth is ON by default and masks the automation markers a real Chrome never exposes
- `--stealth-profile` accepts `auto`, `chrome-linux`, `chrome-win`, `chrome-mac`, and `auto` follows the host
- Without `--stealth-seed` every run draws a fresh identity, so a 50-URL crawl presents 50 different machines
- XDG keys are `stealth` (`true`), `stealth_profile` (`auto`), `stealth_seed` (no default)
- `browser_mode` (`auto`) is `auto|headed|headless`; `auto` resolves to headless and `doctor` reports the effective mode
- Turn the patches off for one run with `--no-stealth` when you are testing your own front end


## How To Route Through an Egress Proxy
```bash
browser-automation-cli --json --proxy socks5://127.0.0.1:1080 \
  scrape https://example.com --format text --engine http

browser-automation-cli --timeout 60 --json --proxy http://127.0.0.1:8888 \
  --proxy-bypass '127.0.0.1,localhost' goto https://example.com

# Credentials belong in XDG, never in argv
browser-automation-cli --json config set proxy_url http://127.0.0.1:8888
browser-automation-cli --json config set proxy_username agent
browser-automation-cli --json config set proxy_password secret
```
- `--proxy` accepts `http`, `https`, and `socks5`, and applies to Chrome and the HTTP engine alike
- `--proxy-bypass` uses Chrome's bypass-list syntax
- XDG keys are `proxy_url`, `proxy_bypass`, `proxy_username`, `proxy_password`
- Keep the credentials in XDG only, because argv shows up in the process table
- `cdp_proxy_bypass_loopback` (`true`) always bypasses loopback so the CDP control channel survives the proxy
- `robots_user_agent` sets the user-agent token robots.txt rules are matched against


## How To Shape Input Like a Human
```bash
# Reproducible human kinematics
browser-automation-cli --timeout 60 --json --input-profile human --input-seed 42 \
  goto https://example.com

# One event per action, exactly deterministic
browser-automation-cli --timeout 60 --json --input-profile direct goto https://example.com

browser-automation-cli --json config set input_profile human
```
- `human` is the default and interpolates pointer trajectories, dwells between press and release, and paces typing
- `--input-seed` seeds the jitter so a `human` run reproduces exactly
- Kinematics keys: `input_move_steps` (`24`), `input_move_gap_ms` (`12`), `input_click_dwell_ms` (`65`)
- Kinematics keys: `input_key_dwell_ms` (`45`), `input_type_delay_ms` (`95`), `input_target_jitter_px` (`3`)
- Scroll keys: `input_scroll_tick_px` (`100`), `input_scroll_max_ticks` (`40`), `input_scroll_settle_rounds` (`3`)


## How To Warm a Session Before the Target URL
```bash
# Land on the origin root first, then the deep URL
browser-automation-cli --timeout 60 --json --warmup goto https://example.com/deep/page

# Warm the real entry point when the edge hands out the session elsewhere
browser-automation-cli --timeout 60 --json --warmup-url https://example.com/login \
  goto https://example.com/app

# Headed on Linux without the private virtual display
browser-automation-cli --timeout 60 --json --headed --no-xvfb goto https://example.com
```
- `--warmup` gives the session cookies and a referrer chain before the target request
- `--warmup-url` implies `--warmup`, so passing it alone is enough
- `--no-xvfb` is only meaningful headed on Linux


## How To Keep the HTTP/2 Fingerprint Constant
```bash
browser-automation-cli --json config set http2_enabled true
browser-automation-cli --json config set http2_initial_stream_window_size 6291456
browser-automation-cli --json config set http2_initial_connection_window_size 15663105
browser-automation-cli --json config set http2_max_header_list_size 262144
browser-automation-cli --json config set http2_max_frame_size 16384
browser-automation-cli --json config set http2_adaptive_window false
```
- `http2_enabled` (`true`) negotiates HTTP/2 on the shared HTTP client
- The four window and size keys carry the defaults shown above
- `http2_adaptive_window` (`false`) stays off so the fingerprint stays constant


## How To Assert on the Emitted Payload
```bash
# Reported only: exit stays 0 and agent_ops.expectation_unmet lists the misses
browser-automation-cli --json --expect 'ok=true' doctor --offline --quick

# Opt in to failing the run
browser-automation-cli --json --expect 'ok=true' --expect-exit-code doctor --offline --quick
```
- `--expect` accepts `key=value`, `key!=value`, and `key~substring`, repeats, and ANDs every expression
- `--expect-exit-code` exits `65` when an expectation is unmet
- It stays off by default because changing an exit code on data content would silently break callers that already branch on it


## How To Open a Page and Snapshot
```bash
browser-automation-cli --timeout 60 --json goto https://example.com

cat > /tmp/goto-view.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"view"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/goto-view.browser-automation.jsonl
```
- Standalone `goto` navigates and ends the process
- Use `run` so `view` sees the same page in one lifecycle
- Accessibility snapshot emits `@eN` refs for later press and write steps


## How To Click and Fill in One Process
```bash
cat > /tmp/form.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"view"}
{"cmd":"write","target":"input","value":"hello"}
{"cmd":"press","target":"button"}
JSONL
browser-automation-cli --timeout 90 --json run --script /tmp/form.browser-automation.jsonl
```
- Keep click and fill in the same process so selectors and `@eN` refs stay valid
- Separate launches cannot share accessibility refs


## How To Scroll and Assert in a Run Script
```bash
cat > /tmp/scroll-assert.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"scroll","dy":1500}
{"cmd":"assert","url_contains":"example.com"}
{"cmd":"assert","text_contains":"Example Domain"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/scroll-assert.browser-automation.jsonl
```
- `dy` / `dx` are aliases for `delta_y` / `delta_x`
- `url_contains` / `text_contains` are assert aliases
- On fail-fast, the error envelope may include partial `data.steps`


## How To Capture a Full-page Screenshot
```bash
cat > /tmp/grab.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"grab","path":"/tmp/page.png","full_page":true}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/grab.browser-automation.jsonl

# Same flags on the grab subcommand after a prior step in the same process:
# browser-automation-cli --timeout 60 --json grab --path /tmp/page.png --full-page
```
- Path is the flag `--path`, not a positional argument
- `full_page` in NDJSON maps to `--full-page` on the CLI


## How To Print a Page to PDF
```bash
browser-automation-cli --json print-pdf --url https://example.com --path /tmp/page.pdf

# Inside multi-step run (GAP-001 / GAP-017)
cat > /tmp/pdf.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"print-pdf","path":"/tmp/page-from-run.pdf"}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/pdf.run.json
```
- Uses CDP `Page.printToPDF` in a one-shot process
- Pass `--url` to navigate before print, or print the current page inside a `run` script after `goto`
- Blank about:blank PDF is refused without navigated content or a step/CLI `url` (GAP-013); navigate with `goto` first (do not use view-only `allow_empty` here)


## How To Monitor Page Change Against a Baseline
```bash
browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/mon.base --write-baseline
browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/mon.base
```
- First call with `--write-baseline` stores the baseline hash/text
- Later calls compare against the baseline file without writing unless requested again


## How To Wait for Multi-text (OR)
```bash
cat > /tmp/wait-or.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","text":["Example Domain","Example"],"ms":5000}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/wait-or.browser-automation.jsonl

# CLI form with repeatable --text (OR semantics):
# browser-automation-cli --timeout 60 --json wait --text "Example Domain" --text "Example" --ms 5000
```
- Repeatable `--text` resolves when any listed value appears
- Combine with `ms` or `selector` or page `state` as needed


## How To Wait for Multi-selector or URL (v0.1.4)
```bash
cat > /tmp/wait-multi.browser-automation.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"wait","selector":"h1, body","ms":5000},
  {"cmd":"wait","url_contains":"example.com","ms":5000},
  {"cmd":"wait","url":"https://example.com/","ms":5000},
  {"cmd":"wait","navigation":true,"ms":5000}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/wait-multi.browser-automation.json

# CLI multi-selector CSS OR:
browser-automation-cli --timeout 60 --json wait --selector 'h1, body' --ms 5000
```
- CSS multi-selector OR: `#a, #b` or `selectors` arrays in run
- Run fields: `url` (exact), `url_contains`, `navigation: true` (boolean load lifecycle — not a string like `"load"`)
- Successful multi-selector wait may include `matched_selector` in result data
- Still combines with multi-text OR and `ms`


## How To Stream Run Steps With --json-steps
```bash
cat > /tmp/steps.array.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"wait","ms":200},
  {"cmd":"view"}
]
JSON
browser-automation-cli --timeout 60 --json --json-steps run --script /tmp/steps.array.json
```
- Global `--json-steps` streams one NDJSON line per step: `step`, `cmd`, `ok`, `result`
- Final `--json` envelope still includes `ok` and full `steps[].data`
- Useful for agent progressive feedback without re-spawning Chrome


## How To Pick / Select-option in Run
```bash
cat > /tmp/pick.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"pick","target":"[role=combobox]","option":"Option label"},
  {"cmd":"select-option","target":"select#country","option":"BR"}
]
JSON
# browser-automation-cli --timeout 90 --json run --script /tmp/pick.run.json
browser-automation-cli --json schema select-option
browser-automation-cli --json schema pick
```
- `pick` / `select-option` are agent inventory + run/exec/schema (not clap standalone subcommands)
- Require `target` (trigger) and `option` (text, selector, or role label)
- On native `<select>`, the CLI dispatches `input` then `change` and reports `via: native_select` (GAP-055)
- Discover argv with `schema pick` or `schema select-option`


## How To Accept a Dialog and Continue (dialog_settled)
```bash
cat > /tmp/dialog-settled.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"dialog","action":"accept"},
  {"cmd":"view"}
]
JSON
# browser-automation-cli --timeout 60 --json run --script /tmp/dialog-settled.run.json \
#   | jaq '.data.steps[] | select(.cmd=="dialog") | .data.dialog_settled'

browser-automation-cli --json config set dialog_settle_ms 2000
```
- After a real accept/dismiss, the data envelope includes boolean `dialog_settled` (GAP-054)
- Happy path is `true` when `Page.javascriptDialogClosed` was observed — do **not** invent a wait before the next page step
- Soft path: `dialog accept --if-present` when the dialog may be absent


## How To Isolate Dialogs Across Tabs (multi-tab)
```bash
cat > /tmp/dialog-multitab.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"page","action":"new","url":"https://example.org"},
  {"cmd":"page","action":"select","index":0},
  {"cmd":"dialog","action":"accept","if_present":true},
  {"cmd":"page","action":"select","index":1},
  {"cmd":"view"}
]
JSON
# browser-automation-cli --timeout 90 --json run --script /tmp/dialog-multitab.run.json
```
- Dialogs are keyed by CDP `session_id` (page forwarders stamp `Page::session_id`)
- Answering a dialog on one tab does not steal the map entry of another tab
- `tab_switch` domain enable is best-effort under a modal dialog budget


## How To Wait With wait_timeout_ms in Run
```bash
cat > /tmp/wait-timeout.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"wait","selector":"h1","wait_timeout_ms":2000},
  {"cmd":"wait","text":["Example Domain"],"wait_timeout_ms":5000}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/wait-timeout.run.json
```
- Public deadline key is `wait_timeout_ms` (GAP-053); the run parser honors it (not a silent discard)
- Also valid on CLI: `wait --selector h1 --wait-timeout-ms 2000`


## How To Scrape With format text Inside Run
```bash
cat > /tmp/scrape-text.run.json <<'JSON'
[
  {"cmd":"scrape","url":"https://example.com","format":"text","engine":"http"}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/scrape-text.run.json
```
- Run steps accept `format` / `formats` (GAP-057) with the same shape as top-level `scrape`
- Requesting only `text` must not dump a large `html` field in the step result


## How To Grab webp (Not avif)
```bash
cat > /tmp/grab-webp.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"grab","path":"/tmp/page.webp","format":"webp"}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/grab-webp.run.json

# CLI: grab --path /tmp/page.webp --format webp
# Encode formats: png | jpeg | webp only. AVIF is removed in v0.1.6.
```


## How To Submit a Form
```bash
# Target the <form> itself or any field inside it
# browser-automation-cli --timeout 60 --json submit "form#login" --timeout-ms 10000

cat > /tmp/submit.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"write","target":"input[name=q]","value":"hello"},
  {"cmd":"submit","target":"form","timeout_ms":10000}
]
JSON
# browser-automation-cli --timeout 90 --json run --script /tmp/submit.run.json
```
- `submit` waits for navigation or a completed request after form submission
- Discover argv with `schema submit --json`


## How To Export and Import Storage
```bash
# Export cookies + localStorage + sessionStorage to an explicit path (mode 0600)
# browser-automation-cli --timeout 60 --json storage export --path /tmp/auth-state.json --url https://example.com

# Import portable auth state, then navigate so restored state applies
# browser-automation-cli --timeout 60 --json storage import --path /tmp/auth-state.json --url https://example.com
```
- Path is always explicit (`--path`); never an implicit XDG default
- Optional `--url` navigates first (export) or after restore (import)
- Discover argv with `schema storage --json`


## How To Assert Console Empty or No Match
```bash
cat > /tmp/assert-console.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"assert","kind":"console_empty"},
  {"cmd":"assert","kind":"console_no_match","pattern":"TypeError"}
]
JSON
browser-automation-cli --capture-console --timeout 60 --json run --script /tmp/assert-console.run.json

# CLI forms (GAP-025):
# browser-automation-cli --capture-console --json assert console-empty
# browser-automation-cli --capture-console --json assert console-no-match --pattern TypeError
```
- Requires `--capture-console` on the same process
- Run kinds: `console_empty` / `console_no_match`; CLI: `console-empty` / `console-no-match`


## How To Use Schema Positional
```bash
browser-automation-cli --json schema run
browser-automation-cli --json schema wait
browser-automation-cli --json schema --cmd assert
```
- `schema <cmd>` positional and `schema --cmd <cmd>` are both valid (GAP-022)
- Prefer positional for agent UX


## How To View With --allow-empty
```bash
browser-automation-cli --json view --allow-empty

cat > /tmp/view-empty.run.json <<'JSON'
[
  {"cmd":"view","allow_empty":true}
]
JSON
browser-automation-cli --timeout 30 --json run --script /tmp/view-empty.run.json
```
- Empty about:blank refuses silent success unless `--allow-empty` / `allow_empty:true` (GAP-012)
- Prefer navigating with `goto` before `view` in normal flows


## How To Handle Beforeunload (GAP-003)
```bash
# Accept or dismiss beforeunload during navigation
browser-automation-cli --timeout 60 --json goto https://example.com --handle-before-unload accept
browser-automation-cli --timeout 60 --json goto https://example.com --handle-before-unload dismiss
browser-automation-cli --timeout 60 --json reload --handle-before-unload accept
browser-automation-cli --timeout 60 --json reload --ignore-cache --handle-before-unload dismiss

# Run step field handle_before_unload
cat > /tmp/beforeunload.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com","handle_before_unload":"accept"},
  {"cmd":"reload","ignore_cache":true,"handle_before_unload":"dismiss"}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/beforeunload.run.json
```
- Values: `accept` or `dismiss` (CLI `--handle-before-unload`; run field `handle_before_unload`)
- Arms CDP dialog auto-accept/dismiss during that navigation only
- Goto options also include `--init-script` and `--navigation-timeout-ms`


## How To Open Isolated Context (GAP-004)
```bash
# Flag alone → default-isolated; optional name after the flag
browser-automation-cli --timeout 60 --json page new --isolated-context
browser-automation-cli --timeout 60 --json page new --isolated-context my-ctx --url https://example.com

# Run: isolated_context string or true
cat > /tmp/page-iso.run.json <<'JSON'
[
  {"cmd":"page","action":"new","isolated_context":true},
  {"cmd":"page","action":"new","isolated_context":"agent-ctx","url":"https://example.com"}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/page-iso.run.json
```
- `page new --isolated-context` with no value uses `default-isolated`
- Run accepts `isolated_context: true` (→ `default-isolated`) or a named string
- Shared context when the field/flag is omitted


## How To fill-form in Run
```bash
cat > /tmp/fill-form.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"fill-form","fields":[{"target":"input","value":"hello"},{"target":"textarea","value":"world"}]}
]
JSON
# browser-automation-cli --timeout 90 --json run --script /tmp/fill-form.run.json

# CLI form (fields JSON via fill-form --fields-json; global --json is envelope only):
# browser-automation-cli --json fill-form --fields-json '[{"target":"input","value":"hello"}]'
```
- Run accepts `fields` array (or `json` string/array) of `{target|uid|selector|ref, value|text}`
- Prefer one process with `goto` so selectors stay valid


## How To console dump Empty Array (GAP-021)
```bash
browser-automation-cli --capture-console --json console dump --path /tmp/console.json
# Always a valid JSON array — [] when empty
jaq -e 'type == "array"' /tmp/console.json
```
- `console dump` always writes a valid JSON array (`[]` when empty)
- Enable `--capture-console` on the same process that produces messages when you need non-empty dumps


## How To List Network Requests
```bash
cat > /tmp/nav.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":400}
{"cmd":"net","action":"list","resource_types":"Document,XHR"}
JSONL
browser-automation-cli --capture-network --timeout 60 --json run --script /tmp/nav.jsonl
```
- Create the script file in the recipe before `run`
- Capture must be enabled on the same process that navigates
- `net list` after a separate process sees no prior capture


## How To Evaluate JavaScript
```bash
cat > /tmp/eval.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"eval","expression":"document.title"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/eval.browser-automation.jsonl

# Standalone eval runs against about:blank unless you already navigated in the same process
# browser-automation-cli --json eval 'document.title'
```
- Prefer `run` when the expression depends on page content
- Expression may be a plain value or a function declaration `() => ...`


## How To Emulate Mobile Viewport and Network
```bash
cat > /tmp/emulate.browser-automation.jsonl <<'JSONL'
{"cmd":"emulate","user_agent":"Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X)","viewport":"390x844x3,mobile,touch","network_conditions":"Slow 3G"}
{"cmd":"goto","url":"https://example.com"}
{"cmd":"resize","width":390,"height":844}
{"cmd":"view"}
JSONL
browser-automation-cli --timeout 90 --json run --script /tmp/emulate.browser-automation.jsonl

# Standalone compose (no --device preset flag):
# browser-automation-cli --json emulate \
#   --user-agent "Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X)" \
#   --viewport "390x844x3,mobile,touch" \
#   --network-conditions "Slow 3G"
```
- There is no `--device` preset flag
- Compose user agent, viewport, and network conditions yourself
- Network presets include Offline, No throttling, Slow 3G, Fast 3G, Slow 4G, Fast 4G


## How To Scrape With Markdown Over HTTP
```bash
browser-automation-cli --json scrape https://example.com --format markdown --engine http
# agent-native CLEAN STDOUT (project fields; no jq):
browser-automation-cli --json scrape https://example.com --engine http \
  --format markdown --only-main-content \
  --select source_url,title,markdown,status_code --max-text-chars 8000
```
- Formats: `text`, `markdown`, `html`, `links`, `metadata`, `summary`, `product`, `branding`, `raw-html`, `screenshot`, `images`
- Engine `http` uses reqwest and skips Chrome (prefer `http` when static HTML is enough)
- `--select` projects fields in the binary; `--max-text-chars` caps text/markdown/html (XDG `scrape_max_text_chars` default)
- Local one-shot scraping-oriented surface — **not** a hosted scraping SaaS (no CAPTCHA/proxy SaaS)

## How To Map With Sitemap and Path Filters
```bash
browser-automation-cli --json map https://example.com --limit 20 --use-sitemap \
  --include-path /docs --exclude-path /admin --select urls,count
browser-automation-cli --json crawl https://example.com --limit 10 --format markdown \
  --filter http_error=false --select source_url,title,markdown --output-mode json
```
- `--use-sitemap` default follows XDG `scrape_use_sitemap` (true)
- `--filter` is AND `key=value` / `key!=value` on batch/crawl pages
- `--output-mode ndjson` streams one page object per stdout line


## How To Scrape Multi-format
```bash
browser-automation-cli --json scrape https://example.com --format markdown,html,links --engine http
browser-automation-cli --timeout 60 --json scrape https://example.com --format markdown --format links --engine browser
browser-automation-cli --json scrape https://example.com --formats markdown,links --engine http
```
- CSV or repeatable `--format` returns multiple format fields in one invocation (GAP-009)
- Alias `--formats` is accepted where supported (GAP-018)
- Envelope includes per-format output when more than one format is requested


## How To Read Expanded metadata
```bash
# A page that declares Open Graph and Twitter card tags
browser-automation-cli --json --fields metadata \
  scrape https://blog.rust-lang.org/2024/02/08/Rust-1.76.0/ --format metadata --engine http

# A bare page: undeclared keys are absent, not null
browser-automation-cli --json --fields metadata scrape https://example.com \
  --format metadata --engine http
```
- `metadata` collects Open Graph, Dublin Core, `article:`, Twitter card, canonical, favicon, charset, and `html_lang`
- Keys are flattened as `og_title`, `dc_creator`, `article_published_time`, `twitter_card`
- Fields the page does not declare are omitted, never emitted as null
- Test presence with a key check, not with a null comparison


## How To Choose Between rawHtml and html
```bash
# rawHtml returns the document exactly as fetched
browser-automation-cli --json --fields rawHtml --truncate-content 90 \
  scrape https://docs.rs/serde/latest/serde/ --format rawHtml --engine http

# html returns the body after main-content extraction and selector filters
browser-automation-cli --json --fields html --truncate-content 90 \
  scrape https://docs.rs/serde/latest/serde/ --format html --engine http --only-main-content
```
- `rawHtml` lands under the `rawHtml` key and starts at the doctype
- `html` lands under the `html` key and starts at the extracted root
- On that docs page the raw document is 25628 chars against 8185 extracted
- They are no longer aliases of one another, so pick the one you mean
- Use `rawHtml` for fidelity and `html` for content you intend to reprocess


## How To Scrape With the Browser Engine and Formats
```bash
browser-automation-cli --timeout 60 --json scrape https://example.com --format markdown --engine browser
browser-automation-cli --timeout 60 --json scrape https://example.com --format links --engine browser
```
- Engine `browser` uses CDP through Chrome
- Browser engine captures `outerHTML` and applies `--format` (markdown/html/links/metadata/…)
- Use browser when content needs JS rendering


## How To POST Scrape Results to an Operator Webhook
```bash
browser-automation-cli --json scrape https://example.com --format markdown --engine http \
  --webhook-url https://127.0.0.1:9000/hook
```
- `--webhook-url` is a one-shot operator POST of the scrape result data
- It is not product telemetry; the destination is under operator control


## How To Batch-scrape From a URLs File
```bash
cat > /tmp/urls.txt <<'URLS'
# one URL per line
https://example.com
https://example.org
URLS
browser-automation-cli --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2
browser-automation-cli --timeout 120 --json batch-scrape --urls-file /tmp/urls.txt --format markdown --engine browser --concurrency 1
```
- Default engine is HTTP; pass `--engine browser` for CDP per URL (GAP-010)
- Create the URLs file before invoking the command


## How To Crawl With Same-host
```bash
browser-automation-cli --json crawl https://example.com --limit 20 --max-depth 2 --format text --same-host
browser-automation-cli --timeout 120 --json crawl https://example.com --limit 5 --max-depth 1 --engine browser --same-host
```
- `--same-host` is a boolean flag with no value
- Do not write `--same-host true`
- Default engine is HTTP BFS; pass `--engine browser` when JS rendering is required
- With `--same-host` the crawl stays on the seed host


## How To Map a Site
```bash
browser-automation-cli --json map https://example.com --limit 50 --max-depth 2
```
- Map discovers URLs from a seed without full page extraction
- HTTP path; no Chrome launch


## How To Search
```bash
browser-automation-cli --json search "example domain" --limit 10
```
- Local search returns HTTP SERP-style links or URL map results
- Limit caps result count


## How To Parse Local Files (HTML, PDF, DOCX, XLSX, ODS)
```bash
cat > /tmp/page.html <<'HTML'
<!doctype html>
<html><head><title>Demo</title></head>
<body><h1>Hello parse</h1><p>Local file text.</p></body></html>
HTML
browser-automation-cli --json parse /tmp/page.html
browser-automation-cli --json parse tests/fixtures/hello.pdf
browser-automation-cli --json parse tests/fixtures/hello.docx --redact-pii
# browser-automation-cli --json parse /tmp/sheet.xlsx
# browser-automation-cli --json parse /tmp/sheet.ods --redact-pii
```
- Parse extracts text from local html, md, txt, pdf, docx, xlsx, or ods
- `--redact-pii` redacts common PII patterns in the extracted text
- Create sample HTML before the first command; use repo fixtures for PDF/DOCX


## How To Extract With LLM
```bash
browser-automation-cli --json config set openrouter_api_key YOUR_KEY
browser-automation-cli --json config set llm_base_url https://openrouter.ai/api/v1
browser-automation-cli --json config set llm_model openai/gpt-4o-mini
browser-automation-cli --json extract https://example.com --llm --question 'What is the title?'
```
- Without the XDG key, the command fails closed with a usage envelope
- Optional `--schema-json` for structured extraction against a local schema file


## How To Encode and Decode QR Codes
```bash
browser-automation-cli --json qr encode --text 'hello' --format png --path /tmp/qr.png
browser-automation-cli --json qr decode --path /tmp/qr.png
```
- No Chrome required
- Encode formats include `png`, `svg`, and `terminal`


## How To Process Images Locally (agent-native)
```bash
# Download with SSRF + body cap + magic (no Chrome)
browser-automation-cli --json image download 'https://example.com/a.png' -o /tmp/a.png
# Compact envelope projection (anti-token)
browser-automation-cli --json image info --path /tmp/a.png --select format,width,height,sha256
# Convert (re-encode strips EXIF; local webp is lossless — quality applies to jpeg)
browser-automation-cli --json image convert --path /tmp/a.png --format webp -o /tmp/a.webp
# Screenshot without pixel base64; opt-in: grab --include-base64
browser-automation-cli --json grab --format webp --path /tmp/g.webp
# Upload converted file into a file input (Chrome one-shot / run)
# --script takes a file path or `-` for NDJSON on stdin; inline JSON is not a form
printf '%s\n' '{"cmd":"goto","url":"https://example.com"}' '{"cmd":"upload","target":"input[type=file]","path":"/tmp/a.webp"}' | browser-automation-cli --json run --script -
```
- Never dumps pixel base64 by default (agent-native stdout)
- Limits via XDG: `image_max_input_bytes`, `image_max_pixels`, `image_download_max_bytes`
- Magic bytes decide format (extension is not trusted); AVIF/HEIC rejected; GIF `frame_count` is 1 (no multi-frame reassemble)
- `image download` is a single image URL (SSRF + body cap) — not a whole-site tree `download`
- EXIF only (`kamadak-exif`); no IPTC/XMP; `--select tags` aliases to `exif`
- SVG: no resvg raster; use `--allow-non-image` only when raw non-image bytes are intentional
- No OCR action: the calling agent reads images natively, so embedded OCR was redundant middleware


## How To Process Videos Locally (agent-native)
```bash
# Probe magic + streams (ffprobe optional; JSON path/meta only — never raw media on stdout)
browser-automation-cli --json video info --path /tmp/in.mp4 --select container,duration_secs,streams,sha256
# agent aliases also work: --select format,bytes,path → container,size_bytes,path
# Convert/remux: smart copy when muxable; auto re-encode for WebM from H.264 (no manual ffmpeg)
browser-automation-cli --json video convert --path /tmp/in.mp4 --format webm -o /tmp/out.webm --select path_out,auto_reencoded,video_codec,bytes_out
# Extract audio
browser-automation-cli --json video to-mp3 --path /tmp/in.mp4 -o /tmp/a.mp3
# Trim + thumbnail frame (path→path)
browser-automation-cli --json video trim --path /tmp/in.mp4 --start 0 --duration 2 -o /tmp/clip.mp4
browser-automation-cli --json video thumbnail --path /tmp/in.mp4 --at 1 -o /tmp/thumb.png
# Summarise an HLS .m3u8 or DASH .mpd manifest without downloading any media
browser-automation-cli --json video manifest --path /tmp/master.m3u8
# Direct media URL download (SSRF + body cap + magic) — not a site player/yt-dlp
# browser-automation-cli --json video download 'https://example.com/clip.mp4' -o /tmp/in.bin
# Upload into a form (reuse existing CDP upload)
# --script takes a file path or `-` for NDJSON on stdin; inline JSON is not a form
printf '%s\n' '{"cmd":"goto","url":"https://example.com"}' '{"cmd":"upload","target":"input[type=file]","path":"/tmp/out.webm"}' | browser-automation-cli --json run --script -
```
- Requires optional OS `ffmpeg`/`ffprobe` (XDG `ffmpeg_path` / PATH); never links libav in the product crate
- Limits via XDG: `video_max_input_bytes`, `video_download_max_bytes`, `video_default_container`, `video_default_crf`, `video_default_audio_bitrate`, `ffmpeg_timeout_secs`
- Magic bytes decide container; extension is not trusted; path→path only (no full-file load in the CLI process)
- Agent honesty fields: `stream_copy`, `auto_reencoded`, `reencode_reason`, `faststart_applied`
- `video manifest` reads HLS `.m3u8` and DASH `.mpd` structure only; it never fetches segments
- Not in core: adaptive HLS/DASH playback, yt-dlp site extractors, pure-Rust H.264 encode

## How To Process Local Audio (path→path)
```bash
# Probe (magic + optional ffprobe) — no media dump
browser-automation-cli --json audio info --path /tmp/in.wav --select format,codec,duration,bytes,sha256
# Convert to MP3 (ffmpeg optional; smart copy when muxable)
browser-automation-cli --json audio convert --path /tmp/in.wav --format mp3 -o /tmp/a.mp3
# Extract audio from a video container (-vn)
browser-automation-cli --json audio convert --path /tmp/clip.mp4 --format m4a -o /tmp/a.m4a
# Trim
browser-automation-cli --json audio trim --path /tmp/a.mp3 --start 1 --duration 5 -o /tmp/cut.mp3
# Direct media URL download (SSRF + body cap + magic)
# browser-automation-cli --json audio download 'https://example.com/a.mp3' -o /tmp/a.mp3
# Upload into a form (existing CDP upload)
browser-automation-cli --json upload @e1 /tmp/a.mp3
```
- Requires optional OS `ffmpeg`/`ffprobe` (XDG `ffmpeg_path` / PATH); never links libav
- Limits via XDG: `audio_max_input_bytes`, `audio_download_max_bytes`, `audio_default_format`, `audio_default_bitrate`, `ffmpeg_timeout_secs`
- Magic bytes decide container; extension is not trusted; envelope may set `lossy_transcode` on lossy→lossy recompress
- Not in core: cpal device I/O, BPM/fingerprint, pure-Rust encode stack, yt-dlp/HLS


## How To Find Paths on Disk
```bash
browser-automation-cli --json find-paths 'Cargo.*' .
browser-automation-cli --json find-paths --glob '**/*.rs' .
```
- fd-like path discovery under the binary name `browser-automation-cli`
- Use `--glob` for shell-style filters (GAP-A011)
- No Chrome launch


## How To Localize Suggestions (pt-BR)
```bash
browser-automation-cli --lang pt-BR --json click-at --x 1 --y 1
browser-automation-cli --json config set lang pt-BR
```
- Human suggestions localize for `pt-BR` via `--lang` or XDG `lang`
- Successful coordinate clicks still require `--experimental-vision`


## How To MITM Capture
```bash
browser-automation-cli --json mitm init-ca
browser-automation-cli --json mitm start --seconds 30
browser-automation-cli --json mitm status
browser-automation-cli --json mitm list --limit 100
browser-automation-cli --json mitm har --out /tmp/capture.har
browser-automation-cli --json mitm redact --secrets
browser-automation-cli --json mitm domains
browser-automation-cli --json mitm apis
browser-automation-cli --json mitm graphql
browser-automation-cli --json mitm ws
```
- Binds only on 127.0.0.1 with an ephemeral port
- CA material lives under XDG data (`mitm/ca`)
- `start` keeps the one-shot proxy alive for `--seconds` then exits
- Export HAR with required `--out`


## How To MITM capture-url One-shot
```bash
browser-automation-cli --json mitm init-ca
browser-automation-cli --json mitm capture-url https://example.com --seconds 30 --har /tmp/cap.har
browser-automation-cli --json mitm list
browser-automation-cli --json mitm har --out /tmp/capture.har
```
- One-shot compose: local proxy + Chrome + navigate URL + capture (GAP-011)
- Optional `--hosts` allowlist for TLS intercept
- Global route-through-MITM flags also exist: `--mitm`, `--mitm-har`, `--mitm-redact-secrets`, …


## How To Workflow Run, Resume, and Status
```bash
cat > /tmp/wf.json <<'JSON'
{
  "name": "demo",
  "steps": [
    {"id": "ping", "cmd": "echo", "args": {"message": "start"}},
    {
      "id": "fetch",
      "cmd": "scrape",
      "args": {"url": "https://example.com", "engine": "http", "format": "text"},
      "depends_on": ["ping"]
    }
  ]
}
JSON
browser-automation-cli --json workflow run --manifest /tmp/wf.json
browser-automation-cli --json workflow resume --manifest /tmp/wf.json
browser-automation-cli --json workflow status --name demo
```
- Resume skips steps already `ok` in the SQLite journal
- Offline steps only; browser `@eN` multi-step remains `run --script`
- Supported offline commands include noop, echo, parse, scrape (http), batch-scrape


## How To Run a Lighthouse Audit
```bash
# Requires a real lighthouse binary on PATH
browser-automation-cli --timeout 180 --json lighthouse https://example.com

# Mock binary for local smoke without a real lighthouse install
browser-automation-cli --timeout 60 --json lighthouse https://example.com \
  --lighthouse-path ./scripts/mock-lighthouse.sh
```
- Resolve order: flag `--lighthouse-path` → XDG `lighthouse_path` → PATH
- Envelope reports `binary_source` as `real` or `mock`
- Pass `--lighthouse-path` or XDG `lighthouse_path` to an external binary or mock script
- Lighthouse itself is not embedded in the CLI


## How To Inspect Heap Snapshots
```bash
cat > /tmp/heap.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"heap","action":"take","path":"/tmp/snap.heapsnapshot"}
JSONL
browser-automation-cli --category-memory --timeout 120 --json run --script /tmp/heap.browser-automation.jsonl
browser-automation-cli --category-memory --json heap summary --path /tmp/snap.heapsnapshot
```
- Deep heap analysis requires `--category-memory`
- Summary reads an existing snapshot path via `--path`


## How To Generate Shell Completions
```bash
browser-automation-cli completions bash
browser-automation-cli completions zsh
browser-automation-cli completions fish
```
- Completions path is light and does not launch Chrome
- Redirect stdout into your shell completion directory as needed



## How To Write Spreadsheets (sheet-write)
```bash
printf 'name,score\nalice,10\nbob,9\n' > /tmp/rows.csv
browser-automation-cli --json sheet-write /tmp/rows.csv -o /tmp/out.xlsx --sheet Data
```
- Writes a simple XLSX workbook from CSV or JSON array-of-objects
- No Chrome required
- Use `--sheet` to name the worksheet (default `Sheet1`)


## How To Structural-Lint With sg-scan
```bash
browser-automation-cli --json sg-scan . --limit 100
```
- One-shot structural lint for forbidden product patterns
- No Chrome required
- `--limit 0` means unlimited findings


## How To Dry-run and Apply sg-rewrite
```bash
browser-automation-cli --json sg-rewrite .
browser-automation-cli --json sg-rewrite . --apply
```
- Default is dry-run report only
- Pass `--apply` to write known-safe fixes
- No Chrome required


## How To Find Paths With --glob
```bash
browser-automation-cli --json find-paths --glob '**/*.rs' .
browser-automation-cli --json find-paths 'Cargo.*' . --extension rs
```
- `--glob` is shell-style glob filter (GAP-A011)
- Regex `pattern` and `--glob` can be combined with other filters
- No Chrome required


## How To Run a JSON Array Script
```bash
cat > /tmp/demo.array.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"view"}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/demo.array.json
```
- `run --script` accepts NDJSON **or** a top-level JSON array of step objects
- Same process lifecycle: BORN EXECUTE FINALIZE DIE
- Fail-fast errors may still include partial `data.steps`
- Final envelope includes full `steps[].data` when `--json` is set


## How To Read Lighthouse binary_source
```bash
browser-automation-cli --timeout 60 --json lighthouse https://example.com \
  --lighthouse-path ./scripts/mock-lighthouse.sh \
  | jaq '.data.binary_source // .binary_source // .'
```
- Resolve order: flag `--lighthouse-path` → XDG `lighthouse_path` → PATH
- Envelope reports `binary_source` as `real` or `mock`
- Mock is for e2e/smoke honesty, not production audits


## How To Configure Redis Cache Honestly
```bash
browser-automation-cli --json config set cache_backend redis
browser-automation-cli --json config set cache_redis_url redis://127.0.0.1:6379
browser-automation-cli doctor --offline --quick --json
```
- Cache settings are XDG-only via `config set` / `config get` / `config list-keys`
- Use `redis://` only; `rediss://` is fail-closed (plain TCP client)
- Doctor reports `cache_redis` when Redis cache is configured


## How To Cover Remaining Interaction and Page Commands
```bash
# keys / type / hover / drag / upload (same process as navigation)
cat > /tmp/interact.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"keys","keys":"Tab"}
{"cmd":"type","text":"hello"}
{"cmd":"hover","target":"a"}
{"cmd":"text"}
{"cmd":"attr","selector":"a","name":"href"}
{"cmd":"page","action":"list"}
JSONL
browser-automation-cli --timeout 90 --json run --script /tmp/interact.browser-automation.jsonl

# dialog accept/dismiss subcommands (not --action); soft path when optional
browser-automation-cli --timeout 60 --json reload --ignore-cache
browser-automation-cli --json dialog accept --if-present
browser-automation-cli --json dialog dismiss --if-present
browser-automation-cli --json exec --help >/dev/null

# dialog inside run (NDJSON shape uses action + optional if_present)
cat > /tmp/dialog.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"dialog","action":"accept","if_present":true}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/dialog.run.json

# category-gated surfaces (explicit flags)
browser-automation-cli --category-extensions --json extension list
browser-automation-cli --category-third-party --json devtools3p list
browser-automation-cli --category-webmcp --json webmcp list
browser-automation-cli --experimental-screencast --json screencast --help >/dev/null
browser-automation-cli --category-memory --json heap --help >/dev/null
browser-automation-cli --json perf --help >/dev/null
browser-automation-cli --json resize --help >/dev/null
browser-automation-cli completions bash >/dev/null
```
- Every agent name appears in `commands --json` (**69**)
- `select-option` / `pick` appear in inventory and run/schema only
- Prefer `schema <name>` before inventing argv for gated surfaces


## How To Discover Command Schemas
```bash
browser-automation-cli commands --json
browser-automation-cli schema goto --json
browser-automation-cli schema --cmd scrape --json
browser-automation-cli schema print-pdf --json
browser-automation-cli schema monitor --json
browser-automation-cli schema qr --json
browser-automation-cli schema find-paths --json
browser-automation-cli schema sheet-write --json
browser-automation-cli schema sg-scan --json
browser-automation-cli schema sg-rewrite --json
browser-automation-cli schema run --json
browser-automation-cli schema pick --json
browser-automation-cli schema select-option --json
browser-automation-cli schema submit --json
browser-automation-cli schema storage --json
browser-automation-cli schema batch-scrape --json
browser-automation-cli schema config --json
browser-automation-cli schema mitm --json
browser-automation-cli schema workflow --json
browser-automation-cli schema locale --json
browser-automation-cli schema man --json
```
- `commands` lists the agent-facing surface (**69** names)
- `schema <cmd>` or `schema --cmd` prints a JSON Schema fragment for one command
- Useful for tool registration in agent frameworks


## How To Pipe JSON With jaq
```bash
browser-automation-cli doctor --offline --quick --json | jaq -e '.ok == true'
browser-automation-cli --json scrape https://example.com --format metadata --engine http \
  | jaq '.data // .'
browser-automation-cli commands --json | jaq '.data.commands // .commands // .'
```
- Prefer `--json` so stdout is machine-readable
- `jaq` filters keep agent glue small and deterministic


## How To Bypass robots.txt With Dual Flags
```bash
# Honor robots by default (no bypass flags)
browser-automation-cli --json scrape https://example.com --format text --engine http

# Bypass only when both flags are present together
browser-automation-cli --ignore-robots --i-accept-robots-risk --json \
  scrape https://example.com --format text --engine http
```
- Default policy honors robots.txt
- `--ignore-robots` alone fails; `--i-accept-robots-risk` alone fails
- Both flags are required when you accept the risk of bypass


## How To List Cookies
```bash
cat > /tmp/cookie.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"cookie","action":"list"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/cookie.browser-automation.jsonl
```
- Cookie helpers operate on the active page in the same process
- Optional URL filter exists on `cookie list --url`


## How To List Console Messages
```bash
cat > /tmp/console.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"eval","expression":"console.log('hello-cookbook')"}
{"cmd":"console","action":"list"}
JSONL
browser-automation-cli --capture-console --timeout 60 --json run --script /tmp/console.browser-automation.jsonl
```
- Enable `--capture-console` on the same process that produces messages
- Filter types with `--types log,warning,error,info,debug` on the CLI form
- `console dump` always writes a valid JSON array (`[]` when empty)


## How To Assert URL or Text
```bash
cat > /tmp/assert.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"assert","kind":"url","value":"example.com","contains":true}
{"cmd":"assert","kind":"text","value":"Example Domain"}
{"cmd":"assert","url_contains":"example.com"}
{"cmd":"assert","text_contains":"Example Domain"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/assert.browser-automation.jsonl
```
- Assert fails the process when the condition is not met
- URL assert supports exact match or contains semantics (`contains` or `url_contains`)
- Text assert can target a selector via `target` or use `text_contains`

## Full Command Inventory (69)
- Live source of truth: `browser-automation-cli commands --json` (**69** agent-facing names)
- Clap product surface is **66** names (excludes agent-only `select-option` / `pick`; those two are run/exec/schema inventory)
- DevTools tool-ref e2e covers **53** tools (`scripts/e2e_all_52_tools.sh` filename is legacy; suite runs 53; lighthouse mock SKIP)
- Full agent command list (all **69**):
  - Meta / discovery: `doctor`, `commands`, `schema`, `version`, `locale`, `completions`, `man`
  - Navigate: `goto`, `back`, `forward`, `reload`, `page`, `wait`, `dialog`
  - Interact: `press`, `click-at`, `write`, `keys`, `type`, `hover`, `drag`, `submit`, `fill-form`, `upload`, `scroll`
  - Agent inventory + run/exec/schema (not clap standalone): `select-option`, `pick`
  - Observe: `view`, `eval`, `text`, `attr`, `assert`, `cookie`, `storage`, `console`, `net`
  - Capture: `grab`, `print-pdf`, `monitor`, `screencast`, `lighthouse`
  - Multi-step: `run`, `exec`, `record`
  - Extract / scrape: `extract`, `scrape`, `batch-scrape`, `crawl`, `map`, `search`, `parse`
  - Local IO (no Chrome): `qr`, `image`, `video`, `audio`, `find-paths`, `sheet-write`, `sg-scan`, `sg-rewrite`
  - Infra: `config`, `mitm`, `workflow`
  - Emulation / perf: `emulate`, `resize`, `perf`, `heap`
  - Category gates: `extension`, `devtools3p`, `webmcp`
- Complete flat list: `doctor`, `commands`, `schema`, `version`, `locale`, `goto`, `view`, `press`, `click-at`, `write`, `keys`, `type`, `wait`, `hover`, `drag`, `submit`, `fill-form`, `select-option`, `pick`, `upload`, `back`, `forward`, `reload`, `eval`, `grab`, `print-pdf`, `monitor`, `run`, `exec`, `extract`, `text`, `scroll`, `cookie`, `storage`, `attr`, `assert`, `console`, `net`, `page`, `dialog`, `scrape`, `batch-scrape`, `crawl`, `map`, `search`, `parse`, `qr`, `image`, `video`, `audio`, `find-paths`, `sg-scan`, `sg-rewrite`, `sheet-write`, `mitm`, `workflow`, `config`, `emulate`, `resize`, `perf`, `lighthouse`, `screencast`, `heap`, `extension`, `devtools3p`, `webmcp`, `completions`, `man`
- Discover argv with `schema <name> --json` for any name above
