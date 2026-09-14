[English](README.md) | [Português Brasileiro](README.pt-BR.md)

# browser-automation-cli

> One-shot Chrome CDP automation for AI agents. BORN, EXECUTE, FINALIZE, DIE.

[![docs.rs](https://img.shields.io/docsrs/browser-automation-cli)](https://docs.rs/browser-automation-cli)
[![crates.io](https://img.shields.io/crates/v/browser-automation-cli)](https://crates.io/crates/browser-automation-cli)
[![License](https://img.shields.io/crates/l/browser-automation-cli)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.88.0-orange)](Cargo.toml)
[![Downloads](https://img.shields.io/crates/d/browser-automation-cli)](https://crates.io/crates/browser-automation-cli)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-blue)](https://www.rust-lang.org)
[![GitHub](https://img.shields.io/badge/github-browser--automation--cli-black.svg)](https://github.com/danilo-aguiar-br/browser-automation-cli)

```bash
cargo install browser-automation-cli
```

Agent discovery map: [llms.txt](llms.txt) (short) and [llms-full.txt](llms-full.txt) (expanded).

## What is it
- Single-process browser automation CLI for AI agents
- Talks to system Chrome or Chromium through chromiumoxide CDP
- No daemon, no npm packaging, no remote telemetry
- Lifecycle is always BORN, EXECUTE, FINALIZE, DIE
- JSON envelopes on stdout for programmatic agents
- XDG config and paths via `config` commands only

## The Pain
- Agent workflows need multi-step browser work without a sticky daemon
- Node and npm browser stacks add runtime weight and supply-chain surface
- Session-based tools leave orphan Chrome processes and unclear ownership
- JSON contracts often drift from real CLI flags and exit codes
- Product settings outside XDG `config` make agent prompts fragile

## Why browser-automation-cli
- One process owns one Chrome lifecycle from launch to kill fallback
- Multi-step work uses `run --script` NDJSON or a JSON array of steps in the same process
- Accessibility snapshot refs `@eN` stay valid only inside that process
- `--json` envelopes are stable for programmatic agents; clap usage errors also emit JSON when `--json` is on argv
- Install path is pure Rust via cargo
- v0.2.0 is current: a Chrome started by the CLI drives DevTools over a pipe instead of a TCP port, and a headed Linux launch stays inside a private Xvfb protected by a cookie; inventory **71** agent names via `commands --json`; **217** XDG keys

## Superpowers
- Navigation and page lifecycle: `goto` (init-script, beforeunload accept|dismiss), `back`, `forward`, `reload`, `page`
- Input: `press`, `write`, `type`, `keys`, `hover`, `drag`, `fill-form`, `select-option`, `pick` (native select + HIG badge/popover / `role=option` with pick events), `submit`, `upload`
- Observation: `view` (refuses empty about:blank unless `--allow-empty`), `grab` (formats `png|jpeg|webp` only; AVIF removed), `extract`, `text`, `attr`, `scroll`, `assert`
- Wait: multi `--text` OR; CSS multi-selector OR (`#a, #b`); run fields `url` / `url_contains` / `navigation` / `wait_timeout_ms`
- Assert: `url` / `text` / `console` plus `console_empty` / `console_no_match` (CLI `console-empty` / `console-no-match`)
- Scrape: multi-format `--format` / `--formats` (CSV or repeatable) with `--engine http|browser`; 15 live formats `text|markdown|html|rawHtml|links|metadata|screenshot|summary|product|branding|images|jsonld|json|feed|attributes` (`raw-html` stays an accepted alias of `rawHtml`); browser applies formats via outerHTML; `format`/`formats` also accepted in `run` scrape steps
- Local scrape/crawl/map/search/parse: `batch-scrape` and `crawl` accept `--engine http|browser`, `map`, `search` (cleans `uddg=`), `parse` (PDF/DOCX/xlsx/ods + `--redact-pii`)
- Extract LLM: `extract --llm --question --schema-json` (XDG `openrouter_api_key`, `llm_base_url`, `llm_model`)
- Capture: `console` (dump always writes `[]` when empty) and `net` with optional global capture flags
- Dialogs: `dialog accept|dismiss` returns `.data.dialog_settled`; XDG `dialog_settle_ms`; multi-tab dialog `session_id` isolation with e2e gate
- Storage: `storage export|import` for cookies + per-origin state within one process
- DevTools depth: `eval`, `emulate`, `resize`, `perf`, `lighthouse` (flag → XDG → PATH; `binary_source` real|mock; unit fixtures include chrome-captured LHR 13.4.1; e2e mock remains SKIP), `heap`
- PDF print: `print-pdf` one-shot and multi-step `run`; refuses blank PDF without navigated content
- Monitor: `monitor check --url --baseline [--write-baseline]`
- Utilities (no Chrome): `qr encode|decode`, `image info|convert|resize|download|exif`, `video info|download|convert|to-mp3|trim|thumbnail|manifest`, `audio info|download|convert|trim`, `find-paths` (`--glob`), `sheet-write`, `sg-scan`, `sg-rewrite`
- Assert aliases: `url_contains` / `text_contains`; `attr` falls back to DOM properties
- Scroll aliases in `run`: `dy`/`dx` for `delta_y`/`delta_x`
- Optional categories: memory, extensions, third-party, webmcp
- Experimental: vision `click-at`, screencast with ffmpeg export
- Anti-detection: stealth is ON by default, with `--no-stealth`, `--stealth-profile`, `--stealth-seed`, `--proxy`, `--proxy-bypass`, `--input-profile human|direct`, `--warmup`, `--no-xvfb`
- MITM one-shot: `status|list|get|har|export|domains|apis|init-ca|start|capture-url|graphql|ws|block|allow|redact` (binds `127.0.0.1`; global `--mitm*`)
- Workflow DAG: `workflow run|resume|status` with SQLite journal (resume skips ok)
- XDG config: `config path|init|show|set|get|unset|list-keys` for config.toml (discover full keys via `config list-keys --json`)
- Discovery: `doctor` (incl. `residual_disk`), `commands` (**71** agent names), `schema <cmd>` or `schema --cmd`, `version`, `locale`, `man`, `completions`
- Global flags: the global help declares **57** long flags, **55** of them product flags plus `--help` and `--version`; `browser-automation-cli --help` is the source of truth
- Multi-step observability: `run --json` final envelope includes `ok` + full `steps[].data`; global `--json-steps` streams one NDJSON line per step
- Fail-fast multi-step: `run` returns partial `data.steps` on error envelopes
- Residual-zero disk (still true from 0.1.5 RES-01…12): BORN auto-GC of stale Singleton-only Chromium dirs under `/tmp` older than 60s; FINALIZE dual scavenge + re-scan; never kills host Flatpak Chrome; marker prefix `browser-automation-cli-chrome-`
- Lifecycle: BORN + FINALIZE scavenge owned Chromium `/tmp` orphans; product law is residual-zero process + disk
- Cache: XDG `cache_backend` (`sqlite|memory|redis`) and `cache_redis_url`; `rediss://` fail-closed
- Intentional residual: GAP-022 ~53 transitive multi-version dups; GAP-023/024 PRD divergences registered

## What's New in 0.2.0
- Full history in [CHANGELOG.md](CHANGELOG.md)
- The pipe bridge and the X11 pin below were validated live on Linux only
### DevTools and Xvfb Security
- A Chrome started by the CLI no longer opens a DevTools TCP port, so a local process can no longer read `/json/version` and drive the browser
- Chrome now runs with `--remote-debugging-pipe`, and a loopback WebSocket bridge relays exactly one valid client on a path holding the 122 random bits of a v4 UUID
- The bridge caps each message at 256 MiB and waits at most two seconds for its pipe threads at teardown
- The private Xvfb requires a `MIT-MAGIC-COOKIE-1`, kept in a mode 0600 file that teardown removes
- A cookie file left by a CLI killed with `SIGKILL` is removed by the next launch
- The Lightpanda engine and the legacy launch path selected by `chrome_legacy_oxide_launch` keep their previous behaviour
### Headed Linux Under Wayland
- A headed Chrome inside the private Xvfb no longer opens a window on the Wayland desktop
- The launch pins `--ozone-platform=x11` whenever the private display started, and no flag or XDG key passes a platform switch to Chrome, so `--no-xvfb` is the way to keep your own display
- The pin follows the Xvfb that actually started, so a missing Xvfb no longer forces X11 onto a launch with no X server
- `display_backend` reports the display really used: `headless`, `xvfb` or `host`
- The extension launch path also starts the private display on headed runs
- The `virtual_display` message of `doctor` names the value `auto` resolves to on this host
### Concurrent Displays
- Two concurrent headed launches no longer share one private display
- Readiness requires the lock file to name the pid of the Xvfb this launch started, and a server that exits is retried on the next free number
- A display number whose lock names a dead process is reused
- The private Xvfb stops with `SIGTERM` and a grace period before `SIGKILL`, so it removes its own lock and socket
- The private Xvfb writes to the null device instead of a pipe nobody read
### Teardown and Exit 124
- A launch cancelled between the fork and the first readiness wait no longer leaves a Chrome without an owner
- A failed launch kills Chrome's whole process group, not only its pid
- `--timeout` during that teardown ends the command with exit 124 instead of 69
- The Chrome and Lightpanda output drainers wait at most two seconds at teardown, so a descendant holding the output no longer holds the command

## Quick Start
```bash
cargo install --path . --locked
browser-automation-cli --version
browser-automation-cli doctor --offline --quick --json
browser-automation-cli doctor --offline --quick --json | jaq '.residual // .data.residual // .'
browser-automation-cli locale --json
browser-automation-cli goto https://example.com --json
browser-automation-cli view --json
```

## Installation
- Local development install:
```bash
git clone https://github.com/danilo-aguiar-br/browser-automation-cli
cargo install --path browser-automation-cli --locked
```
- From crates.io after the first publish:
```bash
cargo install browser-automation-cli --locked
```
- Runtime needs Chrome or Chromium on the shell path (or `config set chrome_path`)
- Optional: `ffmpeg` for screencast file export
- Optional: `lighthouse` binary for lighthouse audits (or `config set lighthouse_path`)

## Usage
- Always pass `--json` for agent pipelines
- Keep human diagnostics on stderr with `-q` when piping
- Use `--timeout` for wall-clock process budget in seconds
- Use `run --script` (NDJSON lines or a JSON array of steps) for multi-step sessions that need shared `@eN` refs
- Stream per-step progress with global `--json-steps` (NDJSON lines: `step`, `cmd`, `ok`, `result`)
- Prefer CLI flags for one-off agent calls; use `config` for durable XDG defaults
- Logging detail: `--verbose` / `--debug` / `-q`, or `config set log_level`
- Localize human suggestions with `--lang pt-BR` or `config set lang pt-BR`
- Optional scrape `--webhook-url` posts the result once to an operator URL (not product telemetry)
- Optional MITM: global `--mitm`, `--mitm-ca-dir`, `--mitm-har`, `--mitm-hosts`, `--mitm-ws`, `--mitm-max-body-bytes`, `--mitm-no-media-bodies`, `--mitm-redact-secrets`

```bash
browser-automation-cli config set openrouter_api_key sk-or-...
browser-automation-cli --json goto https://example.com
browser-automation-cli --json wait --text Hello --text Welcome --ms 5000
browser-automation-cli --json scrape https://example.com --format markdown --engine http
browser-automation-cli --json scrape https://example.com --format markdown,html,links --engine browser
browser-automation-cli --json scrape https://example.com --format markdown --engine http --webhook-url https://example.com/hook
browser-automation-cli --json sitemap https://example.com --limit 200
browser-automation-cli --json feed https://example.com/feed.xml
browser-automation-cli --json extract --llm --question "What is the title?" https://example.com
browser-automation-cli --category-memory heap summary --path snap.heapsnapshot --json
browser-automation-cli --json mitm start --seconds 30
browser-automation-cli --json mitm capture-url https://example.com --seconds 30
browser-automation-cli --json mitm capture-url https://example.com --seconds 30 --har /tmp/browser-automation-cli-artifacts/cap.har
browser-automation-cli --json mitm har --out /tmp/browser-automation-cli-artifacts/capture.har
browser-automation-cli --json workflow resume --manifest workflow.toml
browser-automation-cli --json print-pdf --url https://example.com --path /tmp/browser-automation-cli-artifacts/page.pdf
browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/browser-automation-cli-artifacts/base.txt --write-baseline
browser-automation-cli --json parse ./doc.pdf --redact-pii
browser-automation-cli --json parse ./doc.ods
browser-automation-cli --json qr encode --text "hello" --path /tmp/browser-automation-cli-artifacts/qr.png
browser-automation-cli --json qr decode --path /tmp/browser-automation-cli-artifacts/qr.png
browser-automation-cli --json find-paths --glob '**/*.rs' '' src
browser-automation-cli --json sheet-write rows.csv --out /tmp/browser-automation-cli-artifacts/out.xlsx
browser-automation-cli --json sg-scan src
browser-automation-cli --json schema run
browser-automation-cli --json schema --cmd wait
browser-automation-cli --json batch-scrape --urls-file /tmp/urls.txt --format text --engine browser --concurrency 2
browser-automation-cli --capture-console --json assert console-empty
browser-automation-cli --json record --url https://example.com --path /tmp/steps.jsonl --seconds 30 --max-events 200
```

- `--script` takes a file path, never inline JSON; write the steps file first (NDJSON, one step per line):
```json
{"cmd":"goto","url":"https://example.com"}
{"cmd":"view"}
```
- Then run that file in one process:
```bash
browser-automation-cli --json run --script /tmp/steps.jsonl
browser-automation-cli --json --json-steps run --script /tmp/steps.jsonl
```
- Reading an API payload needs the capture and the navigation in the same process, so put the `net` step in the script (`/tmp/net.jsonl`):
```json
{"cmd":"goto","url":"https://example.com"}
{"cmd":"net","action":"get","id":"0","response_path":"/tmp/browser-automation-cli-artifacts/res.json"}
```
```bash
browser-automation-cli --capture-network --json run --script /tmp/net.jsonl
```

### Global Flag Reference
- Global flags are listed by `<cmd> --help`, for example `version --help`, and the root `--help` lists none of them
- Persist a durable default with `browser-automation-cli config set <key> <value>` in XDG instead of repeating a flag
- Output and payload reduction
  - `--json` emits machine-readable JSON success and error envelopes on stdout
  - `--json-steps` streams one NDJSON object per `run` step on stdout with `step`, `cmd`, `ok` and `result`
  - `--correlation-id <ID>` sets an id echoed on JSON envelopes and NDJSON steps, and it is not a secret
  - `--fields <PATHS>` projects `data` down to ONE CSV of dotted paths relative to `data`, and repeating the flag exits 2
  - `--filter-rows <EXPR>` keeps rows matching `key=value`, `key!=value` or `key~substring`, repeatable and ANDed
  - A missing field never matches a `--filter-rows` expression, including under `!=`
  - `--dedupe-by <PATH>` drops rows whose dotted-path value repeats and keeps the first
  - `--sort-rows <PATH>` sorts rows by a dotted path and compares numbers numerically
  - `--limit-rows <N>` emits at most N rows after filter, dedupe and sort
  - `--max-items <N>` is an accepted alias of `--limit-rows` and limits what is EMITTED, while a command's `--limit` limits what is FETCHED
  - `--count-only` emits only `{"count": N}` instead of the rows
  - `--truncate-content <CHARS>` cuts every string in the payload to N characters and marks `truncated`
  - `--max-output-bytes <BYTES>` is a hard ceiling on emitted bytes that sheds rows from the end and marks `truncated`
  - `--expect <EXPR>` asserts the emitted payload with the `--filter-rows` grammar, repeatable and ANDed
  - `--expect` is evaluated AFTER `--fields` and `--filter-rows`, over the payload the caller actually receives
  - An unmet `--expect` is listed in `agent_ops.expectation_unmet` and the exit code stays 0
  - `--expect-exit-code` exits 65 when any `--expect` is unmet
  - `--help` prints help and `--version` prints the version
- Time and concurrency
  - `--timeout <SECS>` sets the global wall-clock timeout in seconds, where 0 means no override and the maximum is 86400
  - `--step-timeout <SECS>` sets the per-step timeout of `run` scripts, where 0 inherits the global timeout and the maximum is 86400
  - `--max-concurrency <N>` caps concurrent I/O tasks for batch, crawl and CDP fan-out, where 0 means auto
  - `--min-delay-ms <MS>` sets a per-invocation floor between same-origin requests in milliseconds
  - The effective wait of `--min-delay-ms` is the maximum of the flag, XDG `scrape_min_delay_ms` and the site `Crawl-delay`
- Window and display
  - `--browser-mode <MODE>` picks `auto`, `headless` or `headed` for this run and wins over XDG `browser_mode`
  - `--headed` shows the browser window on your own display for debugging
  - `--headless` requires a headless browser for this run and overrides any persisted mode
  - `--no-xvfb` skips the private virtual display on Linux and uses the current display
- Capture and artifacts
  - `--artifacts-dir <DIR>` sets the directory for screenshots, PDFs and other one-shot artifacts
  - `--capture-console` captures console messages during browser commands
  - `--capture-network` captures network requests during browser commands
  - `--dump-on-failure` writes the captured console and network evidence to the artifacts dir when the command fails
- Robots and roots
  - `--ignore-robots` skips robots.txt policy checks and requires risk acceptance for blocked hosts
  - `--i-accept-robots-risk` explicitly accepts the robots.txt override risk together with `--ignore-robots`
  - `--allow-outside-roots` permits local reads and artifact writes outside the allowed roots
- Stealth and input
  - `--no-stealth` turns off the anti-detection patches for this run
  - `--stealth-profile <PROFILE>` picks `auto`, `chrome-linux`, `chrome-win` or `chrome-mac` as the impersonated identity
  - `--stealth-seed <SEED>` pins the stealth identity across processes
  - `--warmup` visits the origin root before the target URL so the session carries cookies
  - `--warmup-url <URL>` warms this URL instead of the origin root and implies `--warmup`
  - `--input-profile <PROFILE>` picks `human` (default) or `direct` input shaping
  - `--input-seed <SEED>` seeds the input jitter so a `human` run reproduces exactly
- Proxy
  - `--proxy <URL>` sets the egress proxy for Chrome and the HTTP engine with `http`, `https` or `socks5`
  - `--proxy-bypass <HOSTS>` lists the hosts that bypass the proxy in Chrome bypass-list syntax
  - Proxy credentials belong in XDG through `config set proxy_url`, never in argv
- MITM
  - `--mitm` enables the one-shot local MITM proxy and routes Chrome through it
  - `--mitm-ca-dir <DIR>` sets the directory for the MITM CA key and cert PEM, defaulting to XDG data
  - `--mitm-har <FILE>` writes HAR 1.2 to this path on FINALIZE when `--mitm` is active
  - `--mitm-hosts <HOSTS>` lists the comma-separated hosts to decrypt, and empty means all
  - `--mitm-ws` restates the default, because WebSocket frames are always captured under `--mitm`
  - `--mitm-max-body-bytes <BYTES>` caps the body bytes retained per exchange
  - `--mitm-no-media-bodies` drops image, video and audio bodies from the capture
  - `--mitm-redact-secrets` restates the default redaction of Authorization and Cookie secrets
  - `--mitm-no-redact-secrets` keeps Authorization and Cookie values readable in the capture
- Category gates
  - `--category-memory` enables the deep heap analysis tools
  - `--category-extensions` enables the extension management tools
  - `--category-third-party` enables the third-party developer tool surface
  - `--category-webmcp` enables the `webmcp` tool surface
  - `--experimental-screencast` enables the experimental screencast, which may require ffmpeg for file export
  - `--experimental-vision` enables the coordinate `click-at` vision tools
- Language and log
  - `--lang <LANG>` forces the UI language `en` or `pt-BR`, and machine JSON stays English
  - `-q, --quiet` suppresses non-error human logs on stderr
  - `-v, --verbose` raises stderr verbosity to info
  - `--debug` turns on maximum tracing detail on stderr
  - `--plain` forces plain stderr with no ANSI colors

## Commands
Full agent inventory (**71** names via `commands --json`, sorted):
- `assert` — Assertions on url, text or console
- `attr` — Read one attribute from a target
- `audio` — Local audio pipeline one-shot without Chrome: info, download, convert, trim
- `back` — History back
- `batch-scrape` — Scrape many URLs from a file, HTTP or browser engine, one-shot
- `click-at` — Click at page CSS coordinates, requires `--experimental-vision`
- `commands` — List available commands
- `completions` — Generate shell completions, path-level, no Chrome
- `config` — XDG config and path management, no `.env` at runtime
- `console` — Captured console messages, requires `--capture-console`
- `cookie` — Cookie jar helpers for the active page through the Network domain
- `crawl` — Crawl from a seed URL, HTTP BFS or browser, one-shot
- `devtools3p` — Third-party developer tools surface, requires `--category-third-party`
- `dialog` — Accept or dismiss dialogs
- `doctor` — Diagnose Chrome install and one-shot readiness
- `drag` — Drag from one target to another with HTML5 drag-and-drop
- `emulate` — Emulate device, network, UA, geolocation or CPU
- `eval` — Evaluate JavaScript as an expression or a function declaration
- `exec` — Single-step inline command with the same surface as `run` steps
- `extension` — Chrome extension tools, requires `--category-extensions`
- `extract` — Extract text or an attribute from a target, or run an LLM extract with `--llm`
- `feed` — Read an RSS, Atom or JSON Feed document over HTTP
- `fill-form` — Fill multiple form fields from a JSON array of target and value pairs
- `find-paths` — Discover filesystem paths with an fd-like UX
- `forward` — History forward
- `goto` — Navigate to a URL, one-shot
- `grab` — Capture a screenshot
- `heap` — Heap snapshot tools, deep analysis requires `--category-memory`
- `hover` — Hover an element
- `image` — Local image pipeline one-shot without Chrome: info, convert, resize, download, exif
- `keys` — Press a keyboard key
- `lighthouse` — Run a Lighthouse audit through the external binary
- `locale` — Show the resolved UI locale and its detection diagnostics
- `man` — Generate a roff man page, path-level, no Chrome
- `map` — Map site URLs from a seed over HTTP
- `mitm` — MITM capture, CA and HAR, local one-shot
- `monitor` — One-shot change check against a baseline file by hash or text
- `net` — Captured network requests, requires `--capture-network`
- `page` — Page info or multi-tab management
- `parse` — Extract text from a local html, md, txt, pdf, docx or xlsx file
- `perf` — Performance trace and metrics
- `pick` — Pick an option from a custom select, badge popover or `role=option`
- `press` — Click an element by selector or `@eN` ref
- `print-pdf` — Print the current page to PDF through CDP `Page.printToPDF`, one-shot
- `qr` — QR encode and decode one-shot without Chrome
- `record` — Record page interactions as a replayable `run --script` NDJSON file
- `reload` — Reload the current page
- `resize` — Resize the page viewport
- `run` — Run a multi-step script in one process, NDJSON or JSON array of steps
- `schema` — JSON Schema fragment for a command, accepting `schema run` or `schema --cmd run`
- `scrape` — Navigate and return body text or formats, local HTTP or CDP
- `screencast` — Screencast start and stop, experimental
- `scroll` — Scroll the page or an element by delta pixels
- `search` — Local search over HTTP SERP links or a URL map
- `select-option` — Pick an option from a custom select, badge popover or `role=option`
- `sg-rewrite` — Structural rewrite for known-safe fixes, dry-run by default and `--apply` writes
- `sg-scan` — Structural lint scan for forbidden product patterns, one-shot
- `sheet-write` — Write a simple XLSX workbook from CSV or JSON, one-shot
- `sitemap` — List the URLs declared by a site's sitemap.xml over HTTP
- `storage` — Export or import portable auth state: cookies, localStorage and sessionStorage
- `submit` — Submit a form, or the form owning a field, and wait for its outcome
- `text` — Extract visible text from a target
- `type` — Type text into `--target` or into the focused element with `--focus-only`
- `upload` — Upload a file to a file input
- `version` — Print the CLI version
- `video` — Local video pipeline one-shot without Chrome: info, download, convert, to-mp3, trim, thumbnail, manifest
- `view` — Accessibility snapshot with `@eN` refs
- `wait` — Wait for milliseconds, text, selector or load state
- `webmcp` — Web surface tools, requires `--category-webmcp`
- `workflow` — Workflow journal DAG backed by SQLite
- `write` — Fill an input value with smart fill for select, checkbox, radio and text

Grouped for humans:
- Discovery: `doctor`, `commands`, `schema`, `version`, `locale`, `man`, `completions`
- Navigate: `goto`, `back`, `forward`, `reload`
- Interact: `press`, `write`, `type`, `keys`, `wait`, `hover`, `drag`, `fill-form`, `select-option`, `pick`, `submit`, `upload`, `click-at`
- Observe: `view`, `extract`, `text`, `scroll`, `attr`, `assert`, `grab`
- Scrape: `scrape`, `batch-scrape`, `crawl`, `map`, `sitemap`, `feed`, `search`, `parse`
- Capture: `console`, `net`, `print-pdf`, `monitor`, `screencast`
- Tabs/Dialogs: `page`, `dialog`, `cookie`, `storage`
- Utils: `qr`, `image`, `video`, `audio`, `find-paths`, `sheet-write`, `sg-scan`, `sg-rewrite`
- Advanced: `eval`, `emulate`, `resize`, `perf`, `lighthouse`, `heap`, `extension`, `devtools3p`, `webmcp`, `mitm`, `workflow`
- Config: `config path|init|show|set|get|unset|list-keys`
- Multi-step: `run`, `exec`, `record`
- Record teaching: `browser-automation-cli --json record --url https://example.com --path /tmp/steps.jsonl --seconds 30 --max-events 200` writes page interactions as replayable NDJSON, then `browser-automation-cli --json run --script /tmp/steps.jsonl` replays them in one process
- Audio teaching: `browser-automation-cli --json audio info|download|convert|trim` runs the local audio pipeline without Chrome
- Sitemap teaching: `browser-automation-cli --json sitemap https://example.com --limit 200` reads the DECLARED sitemap — the `robots.txt` `Sitemap:` hints, the document itself, and nested `sitemapindex` descent — and never walks the link graph, so there is no `--depth` to pass
- Feed teaching: `browser-automation-cli --json feed https://example.com/feed.xml` parses RSS, Atom and JSON Feed from the RAW body over the HTTP engine; the HTML-shaping flags are absent because a selector would destroy the document, and Chrome is not offered because it would render the browser's XML viewer instead of the feed
- Inventory note: **71** agent-facing names via `commands --json` (includes `select-option`, `pick`, `submit`, `storage`, `image`+`video`+`audio`+`record`); DevTools e2e covers 53 tools (lighthouse mock SKIP)

## Anti-Detection, Proxy and Input Shaping
- Stealth is ON by default and masks the automation markers a real Chrome never exposes
- `--no-stealth` turns the anti-detection patches off for one run
- `--stealth-profile <PROFILE>` picks the impersonated identity: `auto`, `chrome-linux`, `chrome-win`, `chrome-mac` (`list` prints the tokens)
- `--stealth-seed <SEED>` pins `hardwareConcurrency`, `deviceMemory`, GPU vendor/renderer and `history.length` — not UA, platform, languages, timezone, screen, plugins or the Chrome build, which comes from the identity crate with a User-Agent override and from the installed Chrome without one
- `doctor --fingerprint` compares webdriver, platform vs UA, and screen vs viewport; without `--quick` it scores the live page and fails if the page contradicts the plan
- Launch applies 1920×1080 device metrics so `screen` is not the headless 800×600 default; `config set screen WxH` and run-step `screen` are the explicit knobs
- `auto` follows the host and is almost always right
- `--stealth-seed <SEED>` pins that identity across processes
- With a seed the Chrome major of the last launch is also stored in `state_dir/stealth/host-major-<hash>.txt`, so the HTTP engine announces that major; without a seed, or under `--no-stealth`, nothing about it touches the disk
- `scrape` reports where the announced Chrome major came from in `user_agent_major_source`: `projected`, `host_binary`, `host_unprobed`, or `null` under `--no-stealth`
- Without a seed every run draws a fresh identity, so a 50-URL crawl of 50 one-shot processes presents 50 different machines
- `--proxy <URL>` sets the egress proxy for Chrome **and** for the HTTP engine, accepting `http`, `https`, and `socks5`
- `--proxy-bypass <HOSTS>` lists the hosts that skip the proxy, in Chrome's bypass-list syntax
- `--input-profile <PROFILE>` is `human` (default) or `direct`
- `human` interpolates pointer trajectories, dwells between press and release, and paces typing
- Measured 2026-09-04 on this tree: the `human` pacing cost grows superlinearly with the typed length, at 2281 ms for 1 character, 14236 ms for 2 and 95781 ms for 4, so a long `type` can exhaust `--timeout` and return exit 124
- Pass `--input-profile direct` when the field is long and the pacing does not matter; this is an OPEN defect, tracked in `gaps.md`, and the workaround is stated here rather than left for the operator to discover through a timeout
- `--input-seed <SEED>` seeds the input jitter so a `human` run reproduces exactly
- `--warmup` visits the origin root before the target URL, so the session already carries cookies and a referrer chain
- `--warmup-url <URL>` warms that URL instead of the target's origin root
- `--no-xvfb` skips the private virtual display on Linux and uses the current one (only meaningful headed on Linux)
- `--expect <EXPR>` asserts the emitted payload matches `key=value`, `key!=value`, or `key~substring`; it repeats and every expression is ANDed
- `--expect-exit-code` exits `65` when an expectation is unmet, instead of only reporting it
- It stays off by default because changing an exit code on data content would silently break callers that already branch on it
- Durable XDG keys: `stealth` (`true`), `stealth_profile` (`auto`), `stealth_seed`, `browser_mode` (`auto`), `input_profile` (`human`)
- `browser_mode` is `auto|headed|headless`; `auto` resolves to headed inside a private virtual display on Linux with Xvfb on PATH and without `--no-xvfb`, and to headless in every other case; `doctor` reports the effective mode
- Proxy XDG keys: `proxy_url`, `proxy_bypass`, `proxy_username`, `proxy_password`, `cdp_proxy_bypass_loopback` (`true`)
- Keep proxy credentials in XDG only, because argv shows up in the process table
- `cdp_proxy_bypass_loopback` always bypasses loopback so the CDP control channel survives a proxy
- `robots_user_agent` sets the user-agent token robots.txt rules are matched against
- HTTP/2 fingerprint keys: `http2_enabled` (`true`), `http2_initial_stream_window_size` (`6291456`), `http2_initial_connection_window_size` (`15663105`), `http2_max_header_list_size` (`262144`), `http2_max_frame_size` (`16384`), `http2_adaptive_window` (`false`)
- `http2_adaptive_window` stays off so the fingerprint stays constant
- Input kinematics keys: `input_move_steps` (`24`), `input_move_gap_ms` (`12`), `input_click_dwell_ms` (`65`), `input_key_dwell_ms` (`45`), `input_type_delay_ms` (`95`), `input_scroll_tick_px` (`100`), `input_scroll_max_ticks` (`40`), `input_target_jitter_px` (`3`), `input_scroll_settle_rounds` (`3`)
- Input dispersion and rhythm keys: `input_timing_distribution` (`lognormal`), `input_move_steps_stddev` (`6`), `input_move_gap_stddev_ms` (`5`), `input_click_dwell_stddev_ms` (`26`), `input_key_dwell_stddev_ms` (`18`), `input_type_delay_stddev_ms` (`40`), `input_scroll_tick_stddev_px` (`25`), `input_word_pause_ms` (`320`), `input_word_pause_permille` (`120`), `input_typo_permille` (`0`)
- `input_timing_distribution` is `lognormal|normal|uniform` and governs the fast rhythm only, because the long-pause tail is `input_word_pause_permille`
- `input_word_pause_permille` accepts `0` as a legitimate value, which removes the long word-boundary pause tail entirely
- `input_typo_permille` stays at `0` because a mistyped character corrected with Backspace changes what the page sees mid-word
- `user_data_dir` has no default and stays absent, and that absence is what upholds the residual-zero disk guarantee
- Turning `user_data_dir` on is opt-in and gives up that guarantee, because the Chrome profile then persists across runs
- `capture_preserved_rings` (`3`) is the number of navigation boundaries kept for `console` and `net --include-preserved`

```bash
browser-automation-cli --json --stealth-seed fleet-01 goto https://example.com
browser-automation-cli --json --proxy socks5://127.0.0.1:1080 scrape https://example.com --format text --engine http
browser-automation-cli --json --input-profile human --input-seed 42 goto https://example.com
browser-automation-cli --json --warmup goto https://example.com/deep/page
browser-automation-cli --json --warmup-url https://example.com/login goto https://example.com/app
browser-automation-cli --json --no-stealth goto http://127.0.0.1:8080
browser-automation-cli --json config set stealth_profile chrome-linux
browser-automation-cli --json config set proxy_url http://user:pass@127.0.0.1:8888
browser-automation-cli --json config unset stealth_seed
```

## Configuration
- Prefer CLI flags for one-off agent calls
- Product settings only via flags and XDG `config path|init|show|set|get|unset|list-keys`
- Discover the full key list (count is not fixed at 16) with `config list-keys --json`
- Every key with its default and description: [XDG Key Reference](#xdg-key-reference)
- Important keys: `dialog_settle_ms`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`, `cache_backend`, `cache_redis_url`, `lang`, `log_level`
- Logging: `--verbose` / `--debug` / `-q`, or XDG `config set log_level` / `log_to_file`
- Color: `config set color true|false`
- Chrome binary: shell path or XDG `config set chrome_path`
- Lighthouse binary: flag `--lighthouse-path`, XDG `config set lighthouse_path`, or PATH (envelope reports `binary_source`)
- Dialog settle budget: XDG `config set dialog_settle_ms <ms>` (agent-visible `dialog_settled` on dialog accept|dismiss)
- Cache: `config set cache_backend sqlite|memory|redis` and optional `cache_redis_url` (`redis://` only; `rediss://` fail-closed)
- `config init` creates XDG layout and default config.toml
- `config unset <KEY>` restores one key to its built-in default and is the real inverse of `set`
- `config set <key> ""` is not an inverse: on a string key it writes an empty value the normal path never produces, and on a numeric key it is a parse error
- Unsetting a key that is already absent succeeds, so a script never needs to know the previous state
- `config path` prints resolved config, data, cache, state, and browsers_dir paths
- CLI flags override values stored in config.toml
- Doctor reports browsers_dir, lighthouse source, `cache_redis`, and `residual_disk` among readiness checks
- Doctor JSON top-level field `residual` reports: `scanned_roots`, `cli_marker_dirs`, `chromium_tmp_singleton_orphans`, `scavenge_safe_candidates`, `live_cli_marker_processes` (legacy), `sibling_live_processes`, `orphan_marker_dirs`, `foreign_root_orphans`, `ghost_marker_processes`, `process_table_unavailable`

## XDG Key Reference
- This section lists all 217 XDG keys grouped by family, each with its built-in default and its description
- The source of truth is `browser-automation-cli --json config list-keys`, which answers with the same key, default and description
- Set a key with `config set <key> <value>`, read it with `config get <key>` and restore its default with `config unset <key>`
- A default of none means the key stays absent until you set it
- The descriptions mirror [docs/CONFIGURATION.md](docs/CONFIGURATION.md), and the defaults come from the binary
### `audio_*` (4)
- `audio_default_bitrate` — Default bitrate for lossy audio encode. Default: `192k`
- `audio_default_format` — Default audio convert format `mp3`, `m4a`, `ogg`, `opus`, `flac`, `wav` or `aac`. Default: `mp3`
- `audio_download_max_bytes` — Max HTTP body bytes for audio download. Default: `256000000`
- `audio_max_input_bytes` — Max bytes for audio stdin materialization or path pre-check. Default: `256000000`

### `browser_*` (3)
- `browser_close_wait_secs` — `Browser.close` and process wait budget during FINALIZE in seconds. Default: `5`
- `browser_mode` — Window mode: `auto` resolves to `headed` inside a private virtual display on Linux with Xvfb on PATH and without `--no-xvfb`, and to `headless` in every other case; `headed` puts a real window on your display, and on Linux that window is rendered into a private virtual display when Xvfb is available; `headless` is cheapest and most detectable. `--headed` still wins. Inverting the `auto` default carries a latency bill and is a separate decision; `doctor` reports what `auto` resolves to on this host under the `virtual_display` check, so the answer never drifts from the binary. Default: `auto`
- `browser_scrape_max_body_bytes` — Max body bytes for browser-engine scrape helpers. Default: `2000000`

### `cache_*` (4)
- `cache_backend` — Cache backend `sqlite`, `memory` or `redis`. Default: `sqlite`
- `cache_max_resp_bulk_bytes` — Redis RESP bulk string size ceiling in bytes. Default: `16777216`
- `cache_max_resp_line_bytes` — Redis RESP line size ceiling in bytes. Default: `16777216`
- `cache_redis_url` — Redis URL required when the backend is `redis`. Default: none

### `capture_*` (1)
- `capture_preserved_rings` — Navigation boundaries kept for console and net `--include-preserved`. Default: `3`

### `cdp_*` (8)
- `cdp_connection_probe_timeout_secs` — CDP `Browser.getVersion` liveness probe timeout in seconds. Default: `3`
- `cdp_discovery_max_body_bytes` — Max CDP discovery HTTP body bytes for `/json/version` and `/json/list`. Only the Lightpanda readiness probe reads it, because the self-spawned Chrome uses the DevTools pipe. Default: `1048576`
- `cdp_discovery_timeout_secs` — CDP HTTP discovery timeout for `/json/version` probes in seconds. No launch path reads it: the self-spawned Chrome uses the DevTools pipe and exposes no `/json/version`, and Lightpanda readiness uses `lightpanda_discovery_timeout_ms`. Default: `2`
- `cdp_event_broadcast_capacity` — Process-local CDP event broadcast channel capacity. Default: `4096`
- `cdp_event_drain_poll_ms` — CDP event drain poll slice during navigation wait in milliseconds. Default: `100`
- `cdp_network_idle_settle_ms` — CDP network-idle settle window in milliseconds. Default: `500`
- `cdp_proxy_bypass_loopback` — Always bypass loopback when Chrome runs behind `--proxy`. The CDP control channel is loopback, so a proxy that captures it produces a browser that never answers — reported as a Chrome startup timeout, which blames the wrong component. Default: `true`
- `cdp_target_event_wait_ms` — CDP target event short wait in milliseconds. Default: `600`

### `chrome_*` (5)
- `chrome_default_timeout_ms` — Default per-operation timeout for the Chrome engine in milliseconds. Default: `25000`
- `chrome_legacy_oxide_launch` — Launch Chrome through the legacy path instead of the self-spawn path as a stabilization fallback that loses the residual kill target, reopens an unauthenticated loopback DevTools port and never starts the private Xvfb. Default: `false`
- `chrome_path` — Absolute Chrome or Chromium path. Default: none
- `chrome_search_paths` — Ordered Chrome or Chromium discovery paths, platform-separated; empty uses the built-in per-OS layout. Default: none
- `chrome_startup_timeout_secs` — Chrome self-spawn CDP readiness wait in seconds, measured on the DevTools pipe bridge; the bridge gives the CDP client twice this budget to connect. Default: `20`

### `default_*` (3)
- `default_jpeg_quality` — JPEG quality in range `1..=100` when `grab` omits `--quality`. Default: `80`
- `default_viewport_height` — Default headless Chrome window height (`--window-size`) when launch options omit the viewport. Default: `1080`
- `default_viewport_width` — Default headless Chrome window width (`--window-size`) when launch options omit the viewport. Default: `1920`

### `heap_*` (10)
- `heap_dominator_max_states` — Dominator visited-state ceiling against pathological graphs. Default: `50000`
- `heap_final_iters` — Heap snapshot final drain iterations. Default: `20`
- `heap_inner_iters` — Heap snapshot inner drain iterations after finished. Default: `10`
- `heap_max_class_nodes` — Heap `class_nodes` list cap. Default: `500`
- `heap_max_edges` — Heap node-op max edges returned. Default: `200`
- `heap_max_path_depth` — Heap paths max depth. Default: `8`
- `heap_max_paths` — Heap paths enumeration max paths. Default: `32`
- `heap_max_retainers` — Heap node-op max retainers returned. Default: `200`
- `heap_outer_iters` — Heap snapshot outer poll max iterations. Default: `200`
- `heap_snapshot_max_bytes` — Offline heap snapshot file size ceiling in bytes. Default: `536870912`

### `http_*` (5)
- `http_connect_timeout_secs` — HTTP connect-phase timeout in seconds. Default: `10`
- `http_pool_max_idle_per_host` — HTTP pool max idle connections per host. Default: `4`
- `http_redirect_max` — Max HTTP redirects followed by product clients. Default: `10`
- `http_ssrf_mode` — HTTP SSRF policy `strict`, `allow_loopback` or `off`. Default: `strict`
- `http_timeout_secs` — Shared HTTP client total timeout in seconds. Default: `30`

### `http2_*` (6)
- `http2_adaptive_window` — Let the HTTP/2 stack resize windows dynamically. Off keeps the advertised values fixed, which is what makes the fingerprint reproducible. Default: `false`
- `http2_enabled` — Offer `h2` in ALPN. ALPN is visible in the clear during the TLS handshake and Chrome always lists `h2`, so a client that offers only `http/1.1` has answered "not a browser" before sending a byte. Default: `true`
- `http2_initial_connection_window_size` — Connection-level flow-control window advertised to the peer. Default: `15663105`
- `http2_initial_stream_window_size` — `SETTINGS_INITIAL_WINDOW_SIZE` advertised to the peer. Library defaults are three orders of magnitude away from Chrome's. Default: `6291456`
- `http2_max_frame_size` — `SETTINGS_MAX_FRAME_SIZE` advertised to the peer. Default: `16384`
- `http2_max_header_list_size` — `SETTINGS_MAX_HEADER_LIST_SIZE` advertised to the peer. Default: `262144`

### `image_*` (6)
- `image_avif_speed` — AVIF encoder speed in range `1..=10` where one is slowest and best, needs the `image-avif` feature. Default: `6`
- `image_default_format` — Default image convert format `png`, `jpeg`, `webp` or `gif`. Default: `png`
- `image_default_quality` — Default lossy quality in range `1..=100` for image convert and resize. Default: `85`
- `image_download_max_bytes` — Max HTTP body bytes for image download. Default: `32000000`
- `image_max_input_bytes` — Max bytes for local image decode, convert or resize input. Default: `32000000`
- `image_max_pixels` — Max width times height for image decode as anti-bomb guard. Default: `64000000`

### `input_*` (20)
- `input_click_dwell_ms` — Hold time between `mousePressed` and `mouseReleased` in milliseconds. Default: `65`
- `input_click_dwell_stddev_ms` — Standard deviation of the press-to-release hold in milliseconds. Default: `26`
- `input_key_dwell_ms` — Hold time between `keyDown` and `keyUp` in milliseconds. Default: `45`
- `input_key_dwell_stddev_ms` — Standard deviation of the `keyDown`-to-`keyUp` hold in milliseconds. Default: `18`
- `input_move_gap_ms` — Delay between synthesized pointer positions in milliseconds. Default: `12`
- `input_move_gap_stddev_ms` — Standard deviation of the delay between synthesized pointer positions in milliseconds. Default: `5`
- `input_move_steps` — Intermediate pointer positions synthesized for one move (human profile). Default: `24`
- `input_move_steps_stddev` — Standard deviation of the per-gesture pointer sample budget, so two moves over the same distance do not carry the same number of intermediate positions. A drag rescales this to its own smaller budget instead of carrying the absolute value across. Default: `6`
- `input_profile` — Default input shaping when `--input-profile` is absent: `human` synthesizes a trajectory, wheel ticks and key events; `direct` keeps the pre-0.1.8 dispatch. The flag still wins. Default: `human`
- `input_scroll_max_ticks` — Ceiling on the number of wheel ticks one scroll gesture synthesizes. Each tick is a CDP round trip, so without a ceiling the cost of a scroll grows linearly with the distance requested and a large `--delta-y` exhausts the command timeout. Past the ceiling the ticks carry more pixels each; total travel is unchanged and only the granularity degrades. Default: `40`
- `input_scroll_settle_rounds` — Extra rounds allowed to deliver a wheel delta the renderer dropped. Default: `3`
- `input_scroll_tick_px` — Scroll distance carried by one synthesized wheel tick in CSS pixels. Default: `100`
- `input_scroll_tick_stddev_px` — Standard deviation of the distance one synthesized wheel tick carries in CSS pixels. Default: `25`
- `input_target_jitter_px` — Radius of the random offset applied to a click target in CSS pixels. Default: `3`
- `input_timing_distribution` — Shape of the dispersion drawn around every input delay: `lognormal`, `normal` or `uniform`. `lognormal` is the default because human inter-key intervals are right-skewed, and a symmetric draw reproduces the width of the human distribution without its asymmetry. It governs the fast rhythm only; the long-pause tail is `input_word_pause_permille`. Every mean is floored at 5% dispersion and truncated between a quarter and four times itself, so setting a standard deviation to `0` does not buy the zero variance a detector reads as machine. Default: `lognormal`
- `input_type_delay_ms` — Delay between characters while typing in milliseconds. Default: `95`
- `input_type_delay_stddev_ms` — Standard deviation of the delay between characters in milliseconds. A caller that asks for its own typing rhythm gets this dispersion rescaled by the same ratio, so halving the mean halves the spread instead of leaving an absolute value that no longer fits. Default: `40`
- `input_typo_permille` — Chance in a thousand that a character is typed wrong, erased with `Backspace` and retyped. The field always ends up holding exactly the requested text. `0` by default, and the only humanisation key that is: every other one disperses TIMING, which a page cannot read as a different value, while this one changes the CHARACTER STREAM, so an `input` listener sees the wrong prefix and may autocomplete or navigate on it. The wrong key is always a physical neighbour on the QWERTY row. Default: `0`
- `input_word_pause_ms` — Mean of the extra pause taken at a word or sentence boundary in milliseconds, itself dispersed by half of its value. This pause is what produces the long right tail of a typing trace, which no amount of jitter around the per-character mean can create. Default: `320`
- `input_word_pause_permille` — Chance in a thousand that a word or sentence boundary earns that long pause. `0` removes the tail and leaves only the fast rhythm. Default: `120`

### `lightpanda_*` (8)
- `lightpanda_cdp_connect_timeout_secs` — Lightpanda CDP connect attempt timeout in seconds. Default: `5`
- `lightpanda_discovery_timeout_ms` — Per-probe CDP discovery timeout while waiting for Lightpanda in milliseconds. Default: `500`
- `lightpanda_max_log_lines` — Bounded Lightpanda launch log ring in lines per stream. Default: `40`
- `lightpanda_poll_interval_ms` — Lightpanda CDP readiness poll interval in milliseconds. Default: `100`
- `lightpanda_ready_slice_ms` — Drain slice after Lightpanda child exit before snapshotting logs in milliseconds. Default: `25`
- `lightpanda_session_timeout_secs` — Lightpanda `--timeout` session max in seconds, range `1..=604800`. Default: `604800`
- `lightpanda_startup_timeout_secs` — Lightpanda process startup wait in seconds. Default: `10`
- `lightpanda_target_init_timeout_secs` — Lightpanda target init wait after connect in seconds. Default: `10`

### `llm_*` (3)
- `llm_base_url` — OpenAI-compatible base URL. Default: none
- `llm_http_timeout_secs` — LLM and webhook blocking HTTP timeout in seconds. Default: `60`
- `llm_model` — Default LLM model id. Default: none

### `log_*` (3)
- `log_level` — Tracing `EnvFilter` used when argv flags stay quiet. Default: `error`
- `log_rotation` — Rolling policy `daily`, `hourly` or `never`. Default: `daily`
- `log_to_file` — Rotated local JSON logs under XDG state, never remote. Default: `false`

### `max_*` (6)
- `max_cli_json_payload_bytes` — Max bytes for CLI flag JSON payloads. Default: `4194304`
- `max_json_file_bytes` — Max bytes for JSON or NDJSON script and manifest files. Default: `33554432`
- `max_log_files` — Retained rotated log files in range `1..=90`. Default: `14`
- `max_ndjson_line_bytes` — Max bytes for one NDJSON line in run scripts and traces. Default: `1048576`
- `max_sg_file_bytes` — Max bytes for one source file read by `sg-scan` and `sg-rewrite`. Default: `16777216`
- `max_urls_file_bytes` — Max bytes for the `batch-scrape --urls-file` list. Default: `8388608`

### `mitm_*` (9)
- `mitm_ca_cache_size` — MITM dynamic certificate cache size in hosts. Default: `1000`
- `mitm_capture_wait_max_ms` — MITM capture wait ceiling after navigate in milliseconds. Default: `8000`
- `mitm_capture_wait_min_ms` — MITM capture wait floor after navigate in milliseconds. Default: `800`
- `mitm_chrome_settle_ms` — MITM Chrome launch settle before navigation in milliseconds. Default: `150`
- `mitm_list_limit_max` — MITM list and query max items clamp. Default: `10000`
- `mitm_proxy_seconds_max` — MITM proxy one-shot max window in seconds. Default: `600`
- `mitm_rebind_attempts` — MITM proxy bind retries when the port is transiently in use. Default: `3`
- `mitm_ws_frames_cap` — Cap on in-memory WebSocket frames per capture process. Default: `500`
- `mitm_ws_preview_chars` — WebSocket text preview truncation in Unicode chars. Default: `256`

### `perf_*` (5)
- `perf_autostop_settle_ms` — Perf auto-stop settle after load or reload in milliseconds. Default: `500`
- `perf_trace_inner_iters` — Perf trace inner drain iterations after complete. Default: `5`
- `perf_trace_inner_slice_ms` — Perf trace poll inner slice in milliseconds. Default: `20`
- `perf_trace_outer_iters` — Perf trace outer poll max iterations. Default: `100`
- `perf_trace_outer_slice_ms` — Perf trace outer poll interval in milliseconds. Default: `50`

### `proxy_*` (4)
- `proxy_bypass` — Hosts bypassing the proxy, in Chrome's bypass-list syntax. Default: none
- `proxy_password` — Proxy password, sent as basic auth. Never echoed by `config get` or `config show`. Default: none
- `proxy_url` — Egress proxy for both Chrome and the HTTP engine (`http`, `https`, `socks5`, `socks5h`). Put credentials here rather than in `--proxy`, where the process table exposes them. Default: none
- `proxy_username` — Proxy account name, sent as basic auth. Kept here rather than in argv, where the process table would expose it. Default: none

### `redis_*` (3)
- `redis_allow_remote` — Allow non-loopback Redis hosts, false when unset. Default: `false`
- `redis_connect_timeout_secs` — Redis TCP connect timeout in seconds. Default: `2`
- `redis_io_timeout_secs` — Redis RESP stream I/O timeout in seconds. Default: `3`

### `retry_*` (16)
- `retry_base_delay_ms` — Default retry base delay in milliseconds. Default: `50`
- `retry_budget_secs` — Default retry wall budget in seconds. Default: `10`
- `retry_cdp_base_delay_ms` — CDP retry base delay in milliseconds. Default: `100`
- `retry_cdp_budget_secs` — CDP retry wall budget in seconds. Default: `15`
- `retry_cdp_max_attempts` — CDP retry max attempts. Default: `4`
- `retry_cdp_max_delay_secs` — CDP retry max delay in seconds. Default: `3`
- `retry_default_max_attempts` — Default retry max attempts including the first try. Default: `3`
- `retry_http_base_delay_ms` — HTTP scrape retry base delay in milliseconds. Default: `75`
- `retry_http_budget_secs` — HTTP scrape retry wall budget in seconds. Default: `12`
- `retry_http_max_attempts` — HTTP scrape retry max attempts. Default: `3`
- `retry_http_max_delay_secs` — HTTP scrape retry max delay in seconds. Default: `2`
- `retry_llm_base_delay_ms` — LLM HTTP retry base delay in milliseconds. Default: `200`
- `retry_llm_budget_secs` — LLM HTTP retry wall budget in seconds. Default: `20`
- `retry_llm_max_attempts` — LLM HTTP retry max attempts. Default: `2`
- `retry_llm_max_delay_secs` — LLM HTTP retry max delay in seconds. Default: `4`
- `retry_max_delay_secs` — Default retry max delay in seconds. Default: `2`

### `robots_*` (4)
- `robots_loopback_exempt` — Loopback hosts skip `robots.txt`; set false to enforce against localhost. Default: `true`
- `robots_max_body_bytes` — Max `robots.txt` body bytes as anti-OOM guard. Default: `524288`
- `robots_probe_timeout_secs` — `robots.txt` request timeout in seconds. Default: `5`
- `robots_user_agent` — User-agent token that `robots.txt` rules are matched against. Set it when stealth sends a browser User-Agent, so the rules evaluated are the ones that apply to the request actually sent. Default: none

### `scrape_*` (21)
- `scrape_charset_peek_bytes` — Charset sniffing peek window in bytes. Default: `4096`
- `scrape_crawl_limit_max` — Max crawl page budget as anti-DoS clamp for `--limit`. Default: `500`
- `scrape_crawl_max_depth` — Max BFS depth for `crawl` and `map`. Default: `10`
- `scrape_dedup_similar` — Collapse near-duplicate pages by content similarity in `crawl` and `batch-scrape`. Default: `false`
- `scrape_dedup_similar_distance` — SimHash Hamming distance in range `0..=64` under which pages are near-duplicates. Default: `3`
- `scrape_default_engine` — Default scrape engine when the CLI omits `--engine`, `http` or `browser`. Default: `http`
- `scrape_delay_jitter_ratio` — Politeness delay jitter ratio in range `0.0..=1.0`, zero disables. Default: `0.2`
- `scrape_feed_max_entries` — Max entries kept by scrape format `feed` for RSS, Atom and JSON Feed. Default: `50`
- `scrape_follow_rel_next` — Follow `rel=next` pagination links during crawl. Default: `false`
- `scrape_honor_meta_robots` — Honor meta robots and `X-Robots-Tag` noindex. Default: `true`
- `scrape_honor_nofollow` — Skip `rel=nofollow` links in crawl discovery. Default: `true`
- `scrape_http_cache_ttl_secs` — HTTP scrape response L2 cache TTL in seconds. Default: `3600`
- `scrape_max_body_bytes` — Max HTTP scrape body bytes. Default: `5000000`
- `scrape_max_parse_bytes` — Max local file parse size in bytes before reject. Default: `50000000`
- `scrape_max_text_chars` — Max text or markdown chars in scrape envelopes, zero means no cap. Default: `32768`
- `scrape_min_delay_ms` — Floor delay between same-origin GETs in milliseconds. Default: `0`
- `scrape_no_cache` — Ignore the response cache on READ and always fetch from origin. The fresh response is still written, so a bypassing call refreshes the entry for later callers instead of leaving a stale one. `--no-cache` on `scrape` overrides this per invocation. There is no way to express the same thing with `scrape_http_cache_ttl_secs`: a TTL of `0` already means "never expires", which is the opposite, and the key rejects it. `monitor check` bypasses unconditionally and ignores this key, because a cached body made it compare a stored page with itself and report `changed: false`. Default: `false`
- `scrape_search_limit_max` — Max search result budget as anti-DoS clamp. Default: `50`
- `scrape_sitemap_max_bytes` — Max sitemap body bytes. Default: `2000000`
- `scrape_summary_chars` — Max chars for scrape format `summary`. Default: `400`
- `scrape_use_sitemap` — Prefer `sitemap.xml` when mapping a site. Default: `true`

### `screencast_*` (4)
- `screencast_ffmpeg_framerate` — Screencast `ffmpeg` input framerate in frames per second. Default: `10`
- `screencast_jpeg_quality` — Screencast CDP JPEG quality in range `1..=100`. Default: `60`
- `screencast_start_pump_iters` — Immediate pump iterations after `Page.startScreencast`. Default: `15`
- `screencast_stop_pump_iters` — Drain pump iterations before `Page.stopScreencast`. Default: `40`

### `state_*` (3)
- `state_collect_deadline_secs` — CDP storage collect outer deadline in seconds. Default: `5`
- `state_event_recv_secs` — CDP storage event recv slice in seconds. Default: `2`
- `state_load_settle_ms` — Settle delay after `load_state` navigation in milliseconds. Default: `500`

### `stealth_*` (3)
- `stealth` — Anti-detection patches applied before the first navigation. `--no-stealth` turns them off for one run. Default: `true`
- `stealth_profile` — Impersonated identity: `auto`, `chrome-linux`, `chrome-win`, `chrome-mac`. `auto` follows the host, which is the only value that cannot contradict the Canvas and WebGL hashes the real GPU produces. Default: `auto`
- `stealth_seed` — Pins the stealth identity so the same fingerprint is reproduced across processes. Absent means a fresh identity each run, which is the default precisely because caching an identity writes it to disk. Default: none

### `svg_*` (3)
- `svg_max_bytes` — Max SVG source bytes accepted before rasterisation. Default: `4000000`
- `svg_max_depth` — Max XML nesting depth accepted in an SVG source. Default: `128`
- `svg_max_entities` — Max `<!ENTITY>` declarations tolerated in an SVG DTD, zero rejects any. Default: `0`

### `video_*` (5)
- `video_default_audio_bitrate` — Default bitrate for video `to-mp3`. Default: `192k`
- `video_default_container` — Default video convert container `mp4`, `webm`, `mkv`, `mov`, `avi` or `m4v`. Default: `mp4`
- `video_default_crf` — Default CRF in range `1..=51` for lossy video re-encode. Default: `23`
- `video_download_max_bytes` — Max HTTP body bytes for video download. Default: `512000000`
- `video_max_input_bytes` — Max bytes for video stdin materialization or path pre-check. Default: `512000000`

### `webhook_*` (3)
- `webhook_max_attempts` — Webhook max attempts including the first try. Default: `3`
- `webhook_post_timeout_secs` — Operator webhook POST timeout in seconds. Default: `15`
- `webhook_retry_base_delay_ms` — Webhook retry base delay in milliseconds, doubled each attempt. Default: `50`

### Standalone keys (39)
- `allowed_roots` — Extra allowed roots for local reads and artifact writes, platform-separated; defaults cover cwd, XDG dirs and temp. Default: none
- `artifacts_dir` — Artifacts output directory. Default: none
- `color` — ANSI colors on human stderr. Default: none
- `dialog_settle_ms` — Max wait after a JS dialog answer for `javascriptDialogClosed` in milliseconds. Default: `2000`
- `dom_stable_window_ms` — Quiet window for `wait --dom-stable-ms` in milliseconds. Default: `500`
- `drag_move_gap_ms` — Delay between synthesized drag positions in milliseconds. Default: `16`
- `drag_move_steps` — Intermediate mouse positions synthesized for one HTML5 drag. Default: `6`
- `encryption_key` — Session encryption key material. Default: none
- `eval_drain_slice_ms` — Eval drain slice while waiting for `Runtime.evaluate` results in milliseconds. Default: `40`
- `event_pump_slice_ms` — Wait and eval event pump slice in milliseconds. Default: `50`
- `event_tracker_max_entries` — In-memory console and network tracker ring size per page session. Default: `1000`
- `extension_attach_poll_iters` — Extension attach poll iterations; slice x iterations is the total wait. Default: `20`
- `extension_attach_poll_ms` — Extension attach poll slice in milliseconds. Default: `150`
- `ffmpeg_path` — Absolute `ffmpeg` path for screencast encode and video convert or `to-mp3`. Default: none
- `ffmpeg_timeout_secs` — Wall-clock `ffmpeg` encode timeout in seconds, range `1..=3600`. Default: `120`
- `file_parse_cache_ttl_secs` — Local file-parse L2 cache TTL in seconds. Default: `86400`
- `gif_max_frames` — Max animation frames decoded from a GIF. Default: `2000`
- `ignore_robots` — Default robots ignore, with both CLI risk flags still required. Default: `false`
- `interact_settle_ms` — UI settle delay after click, type or extension action in milliseconds. Default: `200`
- `lang` — Message locale override (`en` or `pt-BR`; bare `pt` rejected). Default: none
- `lighthouse_path` — Absolute `lighthouse` CLI path. Default: none
- `lighthouse_timeout_secs` — Wall-clock `lighthouse` CLI timeout in seconds, range `1..=3600`. Default: `300`
- `manifest_max_bytes` — Max bytes accepted for an HLS or DASH manifest body. Default: `8000000`
- `manifest_max_variants` — Max variant or representation entries emitted per manifest envelope. Default: `500`
- `monitor_diff_max_bytes` — Byte ceiling for the `monitor check --diff-mode` payload. A page rewritten wholesale diffs to the whole page twice, and the caller asked what changed, not for everything. `diff_truncated` says when the ceiling applied, and `added_count` / `removed_count` keep reporting the real size. Default: `65536`
- `namespace` — Isolated state namespace. Default: none
- `nav_micro_settle_ms` — Navigation micro-settle after page transitions in milliseconds. Default: `100`
- `network_idle_window_ms` — Quiet window for `wait --network-idle` in milliseconds. Default: `500`
- `openrouter_api_key` — LLM API key stored with permission `0600`. Default: none
- `platform_child_poll_ms` — Child-process exit poll interval during FINALIZE in milliseconds. Default: `50`
- `platform_child_wait_secs` — Platform child wait deadline in seconds. Default: `5`
- `residual_orphan_min_age_secs` — Age floor in seconds before a dead-owner marker profile is collectable. Default: `60`
- `run_max_include_depth` — Max nesting depth for `run --script` include chains. Default: `16`
- `screen` — Default page screen `WxH` for `Emulation.setDeviceMetricsOverride` (`screen.width`/`screen.height`). Absent = mirror the viewport. Argv `--screen` and a `run` emulate/resize `screen` field still win. Never smaller than the viewport. Default: none
- `search_base_url` — HTML search endpoint base with `?q=` appended. Default: `https://html.duckduckgo.com/html/`
- `shutdown_deadline_secs` — Shutdown hard deadline waiting for browser exit in seconds. Default: `30`
- `support_settle_ms` — Support-thread settle for sync helpers in milliseconds. Default: `80`
- `timeout` — Global timeout in seconds. Default: `0`
- `user_data_dir` — persistent Chrome profile directory, opt-in. Unset by default, and unset is what keeps residual-zero: the launch gets a throwaway profile and the run leaves nothing on disk. Set it only when a detector attests session across invocations, because a persistent profile is a directory this CLI will never delete for you. Created with mode 0700 on Unix. `--profile` on argv wins over this key. Default: none

## Features
- This crate has no Cargo feature flags
- Optional categories are process flags, not compile-time features
- `--category-memory` enables deep heap tools
- `--category-extensions` enables extension tools
- `--category-third-party` enables third-party DevTools helpers
- `--category-webmcp` enables webmcp tools
- `--experimental-vision` enables `click-at`
- `--experimental-screencast` enables screencast export with ffmpeg

## Targets
- Documented for `x86_64-unknown-linux-gnu`
- Documented for `x86_64-apple-darwin`
- Documented for `aarch64-apple-darwin`
- Documented for `x86_64-pc-windows-msvc`
- Documented for `aarch64-unknown-linux-musl`
- Not supported on `wasm32-unknown-unknown` (Chrome CDP requires a desktop browser)
- docs.rs metadata declares these targets explicitly after the 2026-05-01 multi-target change

## MSRV
- Minimum Supported Rust Version is 1.88.0
- Policy: bump MSRV only in minor or major releases with CHANGELOG note
- Local docs: `timeout 180 cargo doc --no-deps`

## Integration Patterns
- Claude Code, Codex, Cursor, and shell agents spawn one process per action
- Multi-step agent plans must use `run --script` (NDJSON or JSON array) instead of chaining separate processes
- Parse stdout with `jaq` and ignore stderr unless diagnosing failures
- Stream step progress with `--json-steps` when agents need progressive feedback
- Persist durable defaults with `config set` under XDG
- See [INTEGRATIONS.md](INTEGRATIONS.md) and [docs/AGENTS.md](docs/AGENTS.md)

## Performance
- Cold start is dominated by Chrome launch, not Rust binary size
- Prefer `doctor --offline --quick` for install checks without network
- Reuse multi-step scripts to avoid repeated Chrome launches
- Prefer `scrape --engine http` when CDP is not required
- Use `batch-scrape` concurrency for parallel fetches (`--engine http` default; `--engine browser` when JS render is required)

## Memory Requirements
- Expect Chrome process memory far above the CLI binary itself
- Heap tools need `--category-memory` and larger snapshots increase RAM use
- Screencast export may invoke ffmpeg as an external helper
- Workflow journals and MITM captures land under XDG state/data paths

## Troubleshooting FAQ
- Chrome not found: install Chromium or Google Chrome, ensure it is on the shell path, or `config set chrome_path`, then re-run `doctor`
- Config / XDG: run `config init` then `config path` to inspect layout; use `config set|get` for values
- Product settings only via flags and `config set` (XDG)
- Exit 69 unavailable: browser binary missing, blocked, or not launchable
- Exit 124 timeout: raise `--timeout` or shorten the script
- Exit 2 usage: re-check flags with `browser-automation-cli help <cmd>`; with `--json` on argv, clap usage errors emit JSON envelopes
- `@eN` refs invalid across commands: keep steps inside one `run` process; refs do not span processes
- Network empty: pass `--capture-network` on the same process that navigates
- API payload read: `net get <IDX>` writes bodies with `--response-path` and `--request-path`, but `net list` and `net get` only see traffic captured in the same process, so a standalone `net get 0` after a separate `goto` refuses with exit 2; put a `net` step next to the `goto` step in one script and run `browser-automation-cli --capture-network --json run --script /tmp/net.jsonl`
- Wait multi-text: repeat `--text` for OR semantics (any listed text unblocks)
- Wait multi-selector / URL: CSS OR `#a, #b`; in `run` use `url` / `url_contains` / `navigation`
- View empty blank: empty about:blank refuses silent success unless `--allow-empty` / `allow_empty:true`
- MITM bind: `mitm start` and `mitm capture-url` listen on `127.0.0.1` only with an ephemeral port
- MITM HAR: `mitm har --out <path>` (required); or global `--mitm-har` on FINALIZE; or `capture-url --har`
- MITM redact: `mitm redact` SHOWS the effective policy, `mitm redact --secrets true|false` persists a default, and the global `--mitm-redact-secrets` overrides both; CA under XDG data
- Workflow resume: `workflow resume` skips steps already `ok` in the journal
- Scrape multi-format: `--format markdown,html,links` (CSV or repeatable) returns per-format fields; 15 live formats are `text`, `markdown`, `html`, `rawHtml`, `links`, `metadata`, `screenshot`, `summary`, `product`, `branding`, `images`, `jsonld`, `json`, `feed`, `attributes` (`raw-html` remains an accepted alias of `rawHtml`)
- Scrape browser formats: `--engine browser` applies `--format` via outerHTML
- Batch/crawl browser engine: `batch-scrape --engine browser` and `crawl --engine browser` (GAP-010)
- Scroll aliases: in `run` scripts use `dy`/`dx` as aliases for `delta_y`/`delta_x`
- Schema discovery: `schema run` or `schema --cmd run`; expanded fragments for goto/eval/type/scroll/assert/wait
- Lang: `--lang pt-BR` or `config set lang pt-BR` localizes human suggestions
- Fail-fast partial steps: failed `run` error envelopes may include partial `data.steps`
- JSON steps stream: `--json-steps` emits one NDJSON object per step; final `--json` envelope still includes full `steps[]`
- Lighthouse path: flag, `config set lighthouse_path`, or PATH; envelope `binary_source` is `real` or `mock` (mock is e2e-only honesty, not production)
- Search redirects: `search` cleans `uddg=` wrappers to destination URLs
- Parse documents: `parse` supports PDF/DOCX/xlsx/ods and `--redact-pii`
- Extract LLM: requires XDG `openrouter_api_key` (optional `llm_base_url`, `llm_model`)
- Print PDF: `print-pdf --url <url> --path <file>` one-shot CDP; also valid inside `run`
- Monitor baseline: `monitor check --url <url> --baseline <file> [--write-baseline]`
- Assert console: `assert console-empty` / `assert console-no-match --pattern …` (needs `--capture-console`)
- Assert aliases: `url_contains` / `text_contains`; `attr` uses DOM property fallback when HTML attribute is null
- Pick / select-option: agent inventory names; native select dispatches input+change; HIG badge/popover / `role=option` via `pick`
- Submit / storage: `submit` for form submit; `storage export|import` for cookies + per-origin state
- Inventory size: `commands --json` lists **71** agent names (includes `select-option`, `pick`, `submit`, `storage`, `image`+`video`+`audio`+`record`)
- Locale: `locale --json` diagnoses resolved language; set with `--lang pt-BR` or `config set lang pt-BR`
- `file://` + `scrape --engine http`: Usage error — use browser engine or `parse` for local files
- `reload --ignore-cache`: CDP `Page.reload` with `ignoreCache` (not a JS no-op)
- `run` script formats: `--script` is always a file path (inline JSON is rejected with exit 66); the file is NDJSON one object per line, or a single JSON array of steps; supports `wait_timeout_ms` and scrape `format`/`formats`
- Grab formats: `png|jpeg|webp` only (AVIF removed in 0.1.6)
- Redis cache: set `cache_backend redis` and `cache_redis_url`; never use `rediss://`
- Residual /tmp disk hygiene (0.1.5 RES-01…12 still true in 0.1.7):
  - BORN auto-GC: `scavenge_stale_singleton_orphans` removes `/tmp` `org.chromium.Chromium.*` Singleton-only dirs older than 60s
  - FINALIZE dual scavenge + re-scan of owned marker dirs (`browser-automation-cli-chrome-` prefix)
  - Never kills host Flatpak Chrome or non-CLI browser processes
  - Doctor check `residual_disk` + top-level JSON field `residual` (`scanned_roots`, `cli_marker_dirs`, `chromium_tmp_singleton_orphans`, `scavenge_safe_candidates`, `live_cli_marker_processes` (legacy), `sibling_live_processes`, `orphan_marker_dirs`, `foreign_root_orphans`, `ghost_marker_processes`, `process_table_unavailable`)
  - Local gates: `scripts/residual-check.sh`, `scripts/residual-stress.sh`
- Dialog settle: `dialog accept|dismiss` → read `.data.dialog_settled`; budget via XDG `dialog_settle_ms`; multi-tab isolation by `session_id` (e2e gated)
- Lighthouse: unit fixtures include chrome-captured LHR 13.4.1; e2e mock path remains SKIP (contract-only)
- Intentional residual: GAP-022 ~53 transitive multi-version dups; GAP-023/024 PRD divergences registered
- Sheet/lint utils: `sheet-write <input> --out <file>`, `sg-scan <paths>`, `sg-rewrite <paths>` take positional inputs; `find-paths --glob` for shell globs
- `find-paths` positional order is `[PATTERN] [PATHS]...`, so a lone positional is read as the regex PATTERN and the roots silently fall back to the current directory; pass an empty pattern to target a root: `find-paths --glob '**/*.rs' '' src`
- Dialog soft path: `dialog accept --if-present` / run `if_present:true` soft-ok when no dialog is showing

## Exit Codes
- `0` success
- `2` usage or clap parse failure
- `6` blocked — the origin served a bot check instead of content
- `64` capability disabled — a category or experimental gate flag is missing
- `65` data error
- `66` no input
- `69` unavailable
- `70` software, browser, or protocol failure
- `74` I/O failure
- `75` precondition — the page or session does not satisfy the command
- `78` config error
- `124` timeout
- `130` cancelled by SIGINT
- `141` broken pipe
- `255` unexpected fatal path — plausible panic route, but not mapped by any `error.kind` and not observable through the discovery surface

## Documentation Map
- [docs/HOW_TO_USE.md](docs/HOW_TO_USE.md) first command in 60 seconds
- [docs/AGENTS.md](docs/AGENTS.md) agent integration contract
- [docs/COOKBOOK.md](docs/COOKBOOK.md) practical recipes
- [docs/CONFIGURATION.md](docs/CONFIGURATION.md) every XDG key, its default and its purpose
- [docs/CROSS_PLATFORM.md](docs/CROSS_PLATFORM.md) platform matrix
- [docs/STEALTH_PARITY.md](docs/STEALTH_PARITY.md) anti-detection parity against the reference implementations
- [docs/MIGRATION.md](docs/MIGRATION.md) version migration notes
- [docs/TESTING.md](docs/TESTING.md) test categories
- [docs/schemas/README.md](docs/schemas/README.md) JSON schema index
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) module layout and lifecycle internals
- [docs/ROADMAP.md](docs/ROADMAP.md) what is planned and what is closed by physical limit
- [PRIVACY.md](PRIVACY.md) what stays local and what is never uploaded
- [skills/browser-automation-cli-en/SKILL.md](skills/browser-automation-cli-en/SKILL.md) imperative agent skill
- [CHANGELOG.md](CHANGELOG.md) Keep a Changelog history
- [SECURITY.md](SECURITY.md) vulnerability reporting
- [CONTRIBUTING.md](CONTRIBUTING.md) contributor workflow
- [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) Contributor Covenant 2.1
- [llms.txt](llms.txt) short LLM discovery map

## Contributing
- Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a PR
- Follow the Code of Conduct in [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)

## Security
- Report vulnerabilities privately via [SECURITY.md](SECURITY.md)
- Maintainer contact: daniloaguiarbr@proton.me

## Changelog
- Version history lives only in [CHANGELOG.md](CHANGELOG.md)

## Acknowledgments
- The Chrome DevTools Protocol team, whose published contract is what makes a one-shot CDP client possible without a daemon
- `chromiumoxide`, `hudsucker`, `clap`, `tokio`, `reqwest` and `feed-rs`, the crates this CLI is built on
- The Rust project, for a toolchain where `clippy -D warnings` and `cargo deny` are cheap enough to run on every gate
- Reporters of security issues are credited in [SECURITY.md](SECURITY.md) after coordinated disclosure; none yet
- No external contributors to credit yet; [CONTRIBUTING.md](CONTRIBUTING.md) describes how that changes

## License
- Dual licensed under MIT OR Apache-2.0
- See [LICENSE](LICENSE), [LICENSE-MIT](LICENSE-MIT), and [LICENSE-APACHE](LICENSE-APACHE)
