[English](COOKBOOK.md) | [Português Brasileiro](COOKBOOK.pt-BR.md)

# Cookbook — browser-automation-cli

> Practical recipes with copy-ready commands for one-shot browser work. Lifecycle: BORN EXECUTE FINALIZE DIE.


## Latency Note
- Chrome launch dominates cold start on browser-engine commands
- Prefer one `run` script over many separate launches when steps share state
- HTTP scrape, crawl, map, search, and parse avoid Chrome when you only need content
- Each process is BORN, EXECUTE, FINALIZE, DIE with no shared browser across invocations


## Default Values Reference
- Global timeout default is `0` meaning no process wall budget unless set by flag or XDG config
- Step timeout default is `0` meaning inherit global timeout
- Headless mode is default unless `--headed`
- JSON is off unless `--json`
- Product settings come from flags and `config` (XDG), not product env vars
- There are no product `BROWSER_AUTOMATION_CLI_*` environment settings
- OS env only: `RUST_LOG`, `NO_COLOR`


## How To Init XDG Config
```bash
browser-automation-cli --json config init
browser-automation-cli --json config path
browser-automation-cli --json config show
browser-automation-cli --json config set timeout 60
browser-automation-cli --json config set lang en
browser-automation-cli --json config set namespace demo
browser-automation-cli --json config set artifacts_dir /tmp/bac-artifacts
browser-automation-cli --json config set ignore_robots false
browser-automation-cli --json config set encryption_key "replace-me-with-a-secret"
browser-automation-cli --json config set color true
browser-automation-cli --json config get timeout
browser-automation-cli --json config get encryption_key
browser-automation-cli --json config get color
```
- `config init` creates XDG dirs and default `config.toml`
- Supported keys include `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`
- Flags always override file config for that invocation
- Product settings do not use product env vars


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
- Formats: `text`, `markdown`, `html`, `links`, `metadata`
- Engine `http` uses reqwest and skips Chrome


## How To Scrape With the Browser Engine
```bash
browser-automation-cli --timeout 60 --json scrape https://example.com --format text --engine browser
```
- Engine `browser` uses CDP through Chrome
- Use browser when content needs JS rendering


## How To Batch-scrape From a URLs File
```bash
cat > /tmp/urls.txt <<'EOF'
# one URL per line
https://example.com
https://example.org
EOF
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


## How To Parse Local HTML
```bash
cat > /tmp/page.html <<'HTML'
<!doctype html>
<html><head><title>Demo</title></head>
<body><h1>Hello parse</h1><p>Local file text.</p></body></html>
HTML
browser-automation-cli --json parse /tmp/page.html
```
- Parse extracts text from local html, md, txt, or pdf
- Create the sample file before the command


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
- Pass `--lighthouse-path` to an external binary or mock script
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


## How To Discover Command Schemas
```bash
browser-automation-cli commands --json
browser-automation-cli schema --cmd goto --json
browser-automation-cli schema --cmd scrape --json
browser-automation-cli schema --cmd batch-scrape --json
browser-automation-cli schema --cmd config --json
browser-automation-cli schema --cmd mitm --json
browser-automation-cli schema --cmd workflow --json
```
- `commands` lists the agent-facing surface
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
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/assert.browser-automation.jsonl
```
- Assert fails the process when the condition is not met
- URL assert supports exact match or contains semantics
- Text assert can target a selector via `target` or `ref` in the step
