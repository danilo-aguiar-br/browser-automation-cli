[English](COOKBOOK.md) | [Português Brasileiro](COOKBOOK.pt-BR.md)

# Cookbook — browser-automation-cli

> Practical recipes with copy-ready commands for one-shot browser work. Lifecycle: BORN EXECUTE FINALIZE DIE.


## Latency Note
- Chrome launch dominates cold start on browser-engine commands
- Prefer one `run` script over many separate launches when steps share state
- HTTP scrape, crawl, map, search, parse, qr, find-paths, sheet-write, sg-scan, and sg-rewrite avoid Chrome when you only need content or local IO
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
browser-automation-cli --json config get timeout
browser-automation-cli --json config get encryption_key
browser-automation-cli --json config get color
```
- `config init` creates XDG dirs and default `config.toml`
- Supported keys (16): `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `log_to_file`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`, `cache_backend`, `cache_redis_url`
- Flags always override file config for that invocation
- Product settings use only flags and `config path|init|show|set|get`


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
```
- Uses CDP `Page.printToPDF` in a one-shot process
- Pass `--url` to navigate before print, or print the current page inside a `run` script after `goto`


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
```
- Formats: `text`, `markdown`, `html`, `links`, `metadata`, `summary`, `product`, `branding`, `raw-html`, `screenshot`
- Engine `http` uses reqwest and skips Chrome


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
```
- HTTP engine only for batch-scrape
- Create the URLs file before invoking the command


## How To Crawl With Same-host
```bash
browser-automation-cli --json crawl https://example.com --limit 20 --max-depth 2 --format text --same-host
```
- `--same-host` is a boolean flag with no value
- Do not write `--same-host true`
- HTTP BFS crawl stays on the seed host when the flag is set


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
```
- Binds only on 127.0.0.1 with an ephemeral port
- CA material lives under XDG data (`mitm/ca`)
- `start` keeps the one-shot proxy alive for `--seconds` then exits
- Export HAR with required `--out`


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

# dialog auto-handling, reload cache bypass, exec step surface
browser-automation-cli --timeout 60 --json reload --ignore-cache
browser-automation-cli --json dialog --action accept
browser-automation-cli --json exec --help >/dev/null

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
- Every top-level name appears in `commands --json` (59)
- Prefer `schema --cmd <name>` before inventing argv for gated surfaces


## How To Discover Command Schemas
```bash
browser-automation-cli commands --json
browser-automation-cli schema --cmd goto --json
browser-automation-cli schema --cmd scrape --json
browser-automation-cli schema --cmd print-pdf --json
browser-automation-cli schema --cmd monitor --json
browser-automation-cli schema --cmd qr --json
browser-automation-cli schema --cmd find-paths --json
browser-automation-cli schema --cmd sheet-write --json
browser-automation-cli schema --cmd sg-scan --json
browser-automation-cli schema --cmd sg-rewrite --json
browser-automation-cli schema --cmd run --json
browser-automation-cli schema --cmd batch-scrape --json
browser-automation-cli schema --cmd config --json
browser-automation-cli schema --cmd mitm --json
browser-automation-cli schema --cmd workflow --json
```
- `commands` lists the agent-facing surface (59 commands)
- `schema --cmd` prints a JSON Schema fragment for one command
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

## Full Command Inventory (59)
- Live source of truth: `browser-automation-cli commands --json` (59 top-level names)
- DevTools tool-ref e2e covers **53** tools (`scripts/e2e_all_52_tools.sh` filename is legacy; suite runs 53)
- Full top-level command list (every name is a real subcommand):
  - Meta: `doctor`, `commands`, `schema`, `version`, `completions`
  - Navigate: `goto`, `back`, `forward`, `reload`, `page`, `wait`, `dialog`
  - Interact: `press`, `click-at`, `write`, `keys`, `type`, `hover`, `drag`, `fill-form`, `upload`, `scroll`
  - Observe: `view`, `eval`, `text`, `attr`, `assert`, `cookie`, `console`, `net`
  - Capture: `grab`, `print-pdf`, `monitor`, `screencast`, `lighthouse`
  - Multi-step: `run`, `exec`
  - Extract/scrape: `extract`, `scrape`, `batch-scrape`, `crawl`, `map`, `search`, `parse`
  - Local IO (no Chrome): `qr`, `find-paths`, `sheet-write`, `sg-scan`, `sg-rewrite`
  - Infra: `config`, `mitm`, `workflow`
  - Emulation/perf: `emulate`, `resize`, `perf`, `heap`
  - Category gates: `extension`, `devtools3p`, `webmcp`
- Discover argv with `schema --cmd <name> --json` for any name above

