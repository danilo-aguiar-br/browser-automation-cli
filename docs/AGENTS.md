[English](AGENTS.md) | [Português Brasileiro](AGENTS.pt-BR.md)

# Agents Guide — browser-automation-cli

> Cut browser-tool glue. Keep one Chrome lifecycle under your agent. Lifecycle: BORN EXECUTE FINALIZE DIE.


## Why Agents Choose This CLI
- Subprocess ownership is explicit and short-lived
- JSON envelopes reduce brittle stdout scraping
- Multi-step scripts preserve accessibility refs without a daemon
- Category gates keep experimental surfaces opt-in
- Local scrape / crawl / map / search / parse surface ships as first-class subcommands
- Artifact helpers (`print-pdf`, `monitor`, `qr`, `image`, `video`, `audio`, `find-paths`, `sheet-write`, `sg-scan`, `sg-rewrite`) and XDG LLM keys extend agent workflows without daemons
- Durable defaults live in flags and XDG `config path|init|show|set|get`
- v0.1.8 agent-first: anti-detection family shipped; scrape envelope unified; inventory **69** live; **204** XDG keys
- Carried forward: `dialog_settled` boolean after real dialog answer; XDG `dialog_settle_ms`; grab **png|jpeg|webp** only (AVIF removed); run `wait_timeout_ms` + scrape `format`/`formats` (0.1.6 added `submit`/`storage`; 0.1.7 added `image`+`video`+`audio`+`record`)
- Multi-tab dialog isolation via `Page::session_id` / `dialog_map_key`; native select `via: native_select` (input then change)
- Residual-zero disk law from v0.1.5 remains current: BORN + FINALIZE Singleton GC, doctor `residual_disk` / JSON `residual`, meta cmds `locale` and `man`
- Product config: flags + XDG only (never product env vars); discover keys via `config list-keys --json`
- GAP-021 partial: unit LHR fixtures; e2e lighthouse mock **SKIP**. GAP-022 residual ~53 multi-version dups accepted. GAP-023/024 intentional PRD divergences in `parity_intentional_divergences.json`
- Carry-forward from v0.1.4 agent contracts: `--json-steps`, wait multi/url, pick/select-option, assert console, schema positional, MITM capture-url, clap JSON usage errors


## Economy
- Avoid long-lived browser servers that leak across agent turns
- Pay Chrome launch cost only when the task needs a real page
- Prefer HTTP `scrape` / `batch-scrape` / `crawl` / `map` when content alone is enough
- Agent CLEAN STDOUT scrape: always pass `--select` (e.g. `source_url,title,markdown,status_code`); default engine is `http`; prefer `--format markdown` + `--only-main-content`; use `--max-text-chars` / XDG `scrape_max_text_chars`; optional `--include-selector`/`--exclude-selector`, `--redact-pii`, `--with-content-hash`, `--header "Name: value"`, browser `--wait-ms`; multi-format + `--select` promotes nested fields (markdown/jsonld) to top-level; format `json` + `--schema-json`/`--question` uses OpenRouter via XDG (fail-closed without key); batch/crawl: `--filter http_error=false` (OK pages keep), `--sort`, `--dedup-key` (URL-normalized), `--output-mode json|ndjson|csv`
- Map/crawl: `--use-sitemap`, `--sitemap-only`, `--include-path` / `--exclude-path`, `--search` on map; batch/crawl `--filter http_error=false`, optional `--output-mode ndjson|csv`
- Local scraping-oriented one-shot only — not a hosted scraping SaaS (CAPTCHA/proxy/async SaaS TREATED out of product)
- Collapse multi-step flows into one `run` process when refs matter
- Stream progressive feedback with `--json-steps` instead of re-spawning for status
- Reuse `schema <cmd>` once per session instead of re-deriving argv by guesswork


## Sovereignty
- No npm runtime dependency for the product binary
- No remote telemetry path in the CLI
- System Chrome remains under the operator host policy
- Product settings live in flags and XDG `config` only
- Product logging uses `--verbose` / `--debug` / `-q` and XDG `log_level`
- Color uses `config set color`; Chrome path uses `config set chrome_path`


## Compatible Agents and Orchestrators
- Integration mode for every entry below is one-shot subprocess plus `--json`
- This project validates locally with cargo and e2e scripts
- Claude Code
- Codex
- Gemini CLI
- Opencode
- Cursor
- Windsurf
- VS Code Copilot
- GitHub Copilot CLI
- Cline
- Continue
- Aider
- Zed AI assistant
- JetBrains AI Assistant
- Local shell scripts and Makefiles
- Any orchestrator that can spawn a process and read stdout exit codes


## Agent Integration Details
- Spawn `browser-automation-cli` as a one-shot subprocess
- Always pass `--json` for machine parsing
- Read success and error envelopes from stdout
- Keep stderr for human or debug logs only
- Use `commands --json` to discover the live inventory (**69 agent names**)
- Inventory includes config, mitm, workflow, scrape, batch-scrape, crawl, map, search, parse, print-pdf, monitor, qr, find-paths, sheet-write, sg-scan, sg-rewrite, extract, submit, storage, select-option, pick, locale, man, and DevTools-parity tools (**69** total, includes `image`, `video`, `audio`; e2e 53 tools with lighthouse mock SKIP)
- Note: `select-option` and `pick` are in the **69** agent inventory (`commands --json`) and are used via `run` / `exec` / `schema`; they are **not** clap standalone subcommands (clap product surface is **67** names excluding `help`)
- Use `schema <name> --json` or `schema --cmd <name> --json` before generating argv for unfamiliar commands
- Prefer flags for one-off control
- Use `config init|set|get|path|show|list-keys` for durable XDG defaults
- Discover live config keys via `config list-keys --json` (do not hard-code a fixed count; includes `dialog_settle_ms` and more)
- Resolve paths with `config path --json`
- For multi-step work that needs shared `@eN` refs, use one `run --script` process (NDJSON **or** JSON array of steps)
- `run --script -` reads NDJSON steps from **stdin**, one step per line, against a single live session
- Prefer stdin over shell process substitution: `run --script <(printf ...)` is rejected, because the path lands in `/proc/<pid>/fd/<n>` and the file jail refuses reads outside the allowed roots
- Final `run --json` envelope includes `ok` and full `steps[].data`
- Stream per-step NDJSON with global `--json-steps` (`step`, `cmd`, `ok`, `result`)
- Wait with OR text: `wait --text A --text B`
- Wait multi-selector CSS OR and run fields `url` / `url_contains` / `navigation: true` (boolean) and public **`wait_timeout_ms`**; may return `matched_selector`
- After real `dialog accept|dismiss`, read **`dialog_settled`** (boolean). When true, do **not** insert an artificial wait before the next page step
- Configure dialog settle budget only with `config set dialog_settle_ms` (XDG; never a product env var)
- Pick option menus: `{"cmd":"pick","target":"…","option":"…"}` or `select-option` (native `<select>` → `input`+`change`, `via: native_select`)
- Submit form: `submit <target>` or `{"cmd":"submit","target":"…"}`
- Storage portable auth: `storage export|import --path <file>` (cookies + localStorage + sessionStorage)
- Grab encode formats: **png | jpeg | webp** only — never `avif`
- Scroll aliases in NDJSON: `{"cmd":"scroll","dy":1500}`
- Assert aliases: `{"cmd":"assert","url_contains":"example.com"}` / `text_contains`
- Assert console: `{"cmd":"assert","kind":"console_empty"}` or `console_no_match` + `pattern` (needs `--capture-console`)
- CLI assert: `assert console-empty` / `assert console-no-match --pattern …`
- On `run` fail-fast errors, inspect partial `data.steps` when present
- Scrape with multi-format `--format text|markdown|html|rawHtml|links|metadata|summary|product|branding|screenshot` and `--engine http|browser`
- `html` is the processed body (main-content extraction and selector filters applied); `rawHtml` is the response body untouched, under its own `rawHtml` key
- `metadata` harvests what the document declares: `og_*`, `dc_*`, `article_*`, `twitter_*`, `canonical`, `favicon`, `charset`, `html_lang`, plus title/description/status_code/source_url/link_count
- Absent metadata fields are omitted, never emitted as null
- Open Graph arrives as `og_title`, `og_description`, `og_image`, `og_site_name`, `og_type`, `og_url`
- Dublin Core arrives as `dc_creator`, `dc_title`, `dc_subject`, `dc_publisher`, `dc_date`
- Article timestamps arrive as `article_published_time`, `article_modified_time`, `article_author`, `article_section`
- Twitter card arrives as `twitter_card`, `twitter_title`, `twitter_description`, `twitter_image`, `twitter_site`
- Never index a metadata key blindly; read the key only after checking it is present
- Run scrape steps honor `format` / `formats` without dumping HTML when only text was requested
- Scrape envelope shape is unified since v0.1.8: one format and many produce the same keys
- `formats` always exists and maps every requested format to its content
- Each format is mirrored to its own top-level key, so single-format readers keep working
- Diagnosis fields such as `status_code` and `source_url` survive a multi-format request
- Before v0.1.8 a multi-format request dropped those diagnosis fields; never rely on that shape
- Batch/crawl: optional `--engine browser` (default http)
- Optional operator webhook on scrape: `--webhook-url` (one-shot POST, not product telemetry)
- Capture screenshots with `grab --path <file>` (not a positional path)
- Print PDF with `print-pdf --url … --path …` (also inside `run`)
- View blank pages: pass `--allow-empty` only when intentional
- LLM extract fails closed without XDG `openrouter_api_key`
- Localize human suggestions with `--lang pt-BR` or `config set lang pt-BR` (flags + XDG only)
- Inspect resolved locale with `locale --json`; generate man page with `man`
- After browser work, expect residual-zero disk when alone: doctor check `residual_disk` not `fail` and top-level `residual` zeros for `orphan_marker_dirs`, `ghost_marker_processes`, and (after DIE alone) `cli_marker_dirs` + `chromium_tmp_singleton_orphans`; `sibling_live_processes` is informational concurrency; do **not** require zero `live_cli_marker_processes`
- Clap usage errors emit JSON when `--json` is already on argv (GAP-002)
- Beforeunload (GAP-003): `goto`/`reload --handle-before-unload accept|dismiss`; run field `handle_before_unload`
- Isolated context (GAP-004): `page new --isolated-context [name]` (flag alone → `default-isolated`); run `isolated_context` string or `true`
- Extension install/uninstall intentionally outside `run` (GAP-007); discover via `schema`/`commands`
- Assert dual surface (GAP-014): CLI `assert url|text|console|console-empty|console-no-match` vs run kinds
- `console dump` always writes a valid JSON array (`[]` when empty) (GAP-021)
- Wait multi-selector success may include `matched_selector`; run `navigation` is boolean `true`
- Scrape multi-format alias `--formats` where supported (GAP-018)
- `print-pdf` refuses blank without navigated content/`url` (GAP-013)


## Crate Integrations
- Binary name is always `browser-automation-cli`
- Install with `cargo install browser-automation-cli --locked` after crates.io publish
- During development install from path or git
- Any Rust agent crate integrates through `std::process::Command`
- Compatible pattern crates include `rig-core`, `genai`, `async-openai`, `ollama-rs`, `anthropic-sdk`, `agentai`, `autoagents`, `swarms-rs`, `graphbit`, `llm-agent-runtime`
- The CLI is not a Rust library dependency of those crates
- The shared contract is argv plus JSON stdout plus sysexits-style exit codes

### Minimal Rust Command Example
```rust
use std::process::Command;

fn main() {
    let out = Command::new("browser-automation-cli")
        .args(["-q", "--json", "version"])
        .output()
        .expect("spawn browser-automation-cli");
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], true);
}
```


## Surface Discovery for Agents
- Inventory: `browser-automation-cli commands --json` (**69** agent names)
- Input fragments: `browser-automation-cli schema <name> --json` or `schema --cmd <name> --json`
- Config paths: `browser-automation-cli config path --json`
- Config keys: discover with `config list-keys --json` (includes `dialog_settle_ms`; never invent product env vars)
- MITM: `mitm status|list|get|har|export|domains|apis|init-ca|start|capture-url|graphql|ws|block|allow|redact`
- Global MITM: `--mitm`, `--mitm-ca-dir`, `--mitm-har`, `--mitm-hosts`, `--mitm-ws`, `--mitm-max-body-bytes`, `--mitm-no-media-bodies`, `--mitm-redact-secrets`, `--mitm-no-redact-secrets`
- Workflow: `workflow run|resume|status`
- Local scrape surface: `scrape`, `batch-scrape`, `crawl`, `map`, `search`, `parse`
- Artifacts and local IO: `print-pdf`, `monitor check`, `qr encode|decode`, `image info|convert|resize|download|exif`, `video info|download|convert|to-mp3|trim|thumbnail|manifest`, `audio info|download|convert|trim`, `find-paths` (`--glob`), `sheet-write`, `sg-scan`, `sg-rewrite`
- Forms / state: `submit`, `storage export|import`, `select-option` / `pick` (inventory + run/exec; not clap standalone)
- Meta: `locale` (UI locale diagnostics), `man` (roff man page; no Chrome)
- LLM extract: `extract --llm --question …` (XDG keys only)
- Health: `doctor --json` (Chrome discovery, XDG browsers_dir, lighthouse source, `cache_redis` when configured, residual disk hygiene)
- Residual: top-level `residual` + check `residual_disk` with fields `cli_marker_dirs`, `chromium_tmp_singleton_orphans`, `scavenge_safe_candidates`, `live_cli_marker_processes` (legacy), `sibling_live_processes`, `orphan_marker_dirs`, `ghost_marker_processes`, `foreign_root_orphans`, `scanned_roots`
- Cache: XDG `cache_backend` (`sqlite|memory|redis`) and `cache_redis_url` (`redis://` only; `rediss://` fail-closed)
- Lighthouse: flag → XDG `lighthouse_path` → PATH; envelope `binary_source` is `real` or `mock`; e2e mock is SKIP (never claim full e2e lighthouse parser PASS)


## Full Command Inventory (69)
- Live source of truth: `browser-automation-cli commands --json` (**69** agent-facing names)
Clap product surface is **67** names (excludes agent-only `select-option` / `pick`)
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
- Complete flat list: `doctor`, `commands`, `schema`, `version`, `locale`, `goto`, `view`, `press`, `click-at`, `write`, `keys`, `type`, `wait`, `hover`, `drag`, `submit`, `fill-form`, `select-option`, `pick`, `upload`, `back`, `forward`, `reload`, `eval`, `grab`, `print-pdf`, `monitor`, `run`, `exec`, `record`, `extract`, `text`, `scroll`, `cookie`, `storage`, `attr`, `assert`, `console`, `net`, `page`, `dialog`, `scrape`, `batch-scrape`, `crawl`, `map`, `search`, `parse`, `qr`, `image`, `video`, `audio`, `find-paths`, `sg-scan`, `sg-rewrite`, `sheet-write`, `mitm`, `workflow`, `config`, `emulate`, `resize`, `perf`, `lighthouse`, `screencast`, `heap`, `extension`, `devtools3p`, `webmcp`, `completions`, `man`
- Discover argv with `schema <name> --json` for any name above

## Lifecycle
- Slogan (English): BORN EXECUTE FINALIZE DIE
- One process owns one Chrome session from launch through FINALIZE
- BORN scavenges stale Singleton-only Chromium tmp (age floor 60s)
- FINALIZE is idempotent (Browser.close, wait, kill fallback) and dual-scavenges invocation-window + stale Singleton orphans
- Residual contract for agents: after DIE alone expect zero `orphan_marker_dirs`, zero `ghost_marker_processes`, zero CLI marker dirs, zero owned Singleton-only Chromium tmp litter; `sibling_live_processes>0` is healthy concurrency; do not require zero `live_cli_marker_processes`
- Host Flatpak Chrome is never killed or wiped by product residual GC
- Do not expect session or `@eN` refs to survive process exit
- Verify with `doctor --offline --quick --json` → `residual` / check `residual_disk`


## Technical Contract (v0.1.8)
### REQUIRED
- Pass `--json` for programmatic consumption
- Treat one process as one Chrome lifecycle (BORN EXECUTE FINALIZE DIE)
- Use `run --script` for multi-step work that needs shared `@eN` refs (NDJSON or JSON array)
- Prefer `--json-steps` when the agent needs progressive step feedback (stream per-step NDJSON)
- Prefer schema positional: `schema <cmd> --json` (also `schema --cmd <cmd> --json`)
- Use dialog soft path when optional: `dialog accept --if-present` / `dialog dismiss --if-present`
- After a real dialog answer, read `dialog_settled`; when true, proceed to the next page step without inventing a wait
- Configure dialog settle only via XDG `config set dialog_settle_ms` (flags + XDG only; no product env)
- Honor `wait_timeout_ms` on run wait steps as the public deadline key
- Honor scrape `format` / `formats` in run steps (text-only must not emit HTML monsters)
- Read `formats` on scrape envelopes; the shape no longer changes with the format count
- Treat stealth, HTTP/2 fingerprinting and human input pacing as ON by default
- Keep MITM secret masking on unless the operator explicitly asked for `--mitm-no-redact-secrets`
- Check process exit code before trusting stdout
- Branch on envelope field `ok`
- Keep category and experimental gates explicit when needed
- Configure durable product settings via `config` / flags only (`--lang` + XDG for language)
- Discover unknown commands with `commands --json` (**69**) and `schema <cmd>` or `schema --cmd`
- Discover config keys with `config list-keys --json` (never hard-code a fixed key count)
- After browser one-shots, treat residual-zero as part of success: inspect doctor `residual` when diagnosing leaks

### FORBIDDEN
- Do not keep a daemon between agent turns
- Do not invent product aliases such as `bac`, `click`, or `screenshot`
- Do not reuse `@eN` refs across separate process launches
- Do not parse stderr as the primary success channel
- Do not enable robots bypass without the dual-flag policy
- Use only flags and `config` for product settings
- Do not invent product environment variables for config (flags + XDG `config` only)
- Do not pass a positional path to `grab`; use `--path`
- Do not pass `grab --format avif` — AVIF encode is removed (png|jpeg|webp only)
- Do not invent a `--device` preset on `emulate`; use `--user-agent`, `--viewport`, `--network-conditions`
- Do not invoke `select-option` / `pick` as clap top-level subcommands; use `run` / `exec` steps (they remain in `commands --json` inventory)
- Do not invent an artificial wait after `dialog_settled: true`
- Do not assume silent success for empty `view` on about:blank without `--allow-empty`
- Do not assume `print-pdf` succeeds without a navigated page or an explicit `url` (GAP-013); residual smokes may use `print-pdf --url about:blank` as a light one-shot when `url` is present
- Do not kill or ask the CLI to wipe host Flatpak Chrome residual
- Do not claim full e2e lighthouse parser PASS when the suite SKIPs the mock path
- Do not disable MITM secret masking to make a capture easier to read
- Do not tune HTTP/2 or input pacing keys without an explicit operator decision
- Do not assume a multi-format scrape drops diagnosis fields; the envelope shape is unified

### Correct Pattern
```bash
browser-automation-cli -q --timeout 60 --json goto https://example.com
browser-automation-cli -q --json view
out=$(browser-automation-cli -q --json version)
echo "$out" | jaq -e '.ok == true'
browser-automation-cli -q --json commands
browser-automation-cli -q --json config path
browser-automation-cli -q --json wait --text Example --text Domain --ms 5000
browser-automation-cli -q --timeout 60 --json scrape https://example.com --format markdown --engine browser
browser-automation-cli -q --json grab --path /tmp/page.png --full-page
browser-automation-cli -q --json print-pdf --url https://example.com --path /tmp/page.pdf
browser-automation-cli -q --json find-paths 'Cargo.*' .
browser-automation-cli -q --json find-paths --glob '**/*.rs' .
browser-automation-cli -q --json sheet-write /tmp/rows.csv -o /tmp/out.xlsx
browser-automation-cli -q --json sg-scan . --limit 50
browser-automation-cli -q --json config list-keys
browser-automation-cli -q --json schema run
browser-automation-cli -q --json --json-steps run --script '[{"cmd":"goto","url":"https://example.com"},{"cmd":"view"}]'
browser-automation-cli -q --json mitm capture-url https://example.com --seconds 20
browser-automation-cli -q --capture-console --json assert console-empty
browser-automation-cli -q --timeout 60 --json goto https://example.com --handle-before-unload accept
browser-automation-cli -q --json page new --isolated-context
browser-automation-cli -q --json dialog accept --if-present
browser-automation-cli -q --json config set dialog_settle_ms 2000
browser-automation-cli -q --capture-console --json console dump --path /tmp/console.json
browser-automation-cli -q --json schema pick
browser-automation-cli -q --json schema submit
browser-automation-cli -q --json schema storage
browser-automation-cli -q --json locale
browser-automation-cli -q --json doctor --offline --quick
```


## JSON Envelope
- Success: `{"schema_version":1,"ok":true,"data":...}`
- Error: `{"schema_version":1,"ok":false,"error":{...}}`
- Error objects include `kind`, `message`, and `exit_code` when `--json` is set
- Multi-step fail-fast errors may also include partial `data.steps`
- `run --json` success includes `ok` and full `steps[].data`
- `--json-steps` streams one NDJSON object per step: `step`, `cmd`, `ok`, `result`
- Clap usage errors with `--json` on argv emit JSON error envelopes
- Schema index: [docs/schemas/README.md](schemas/README.md)
- Live input fragments always come from `schema <cmd>` / `schema --cmd`; static files may lag


## Reducing the Payload (never pipe through a JSON processor)
- These flags are GLOBAL and work on every one of the 69 commands
- The binary applies them to `data` before writing, so the model never receives what it would discard
- `--fields PATHS` projects dotted paths (CSV) and keeps the documented nesting
- `--filter-rows EXPR` keeps rows matching `key=value`, `key!=value` or `key~substring`; repeatable and ANDed
- `--limit-rows N` caps rows after filter, dedupe and sort
- `--sort-rows PATH` orders rows; numbers compare numerically, not as text
- `--dedupe-by PATH` drops repeats, keeping the first
- `--count-only` returns `{"count": N}` instead of the rows
- `--truncate-content CHARS` cuts every string in the payload
- `--max-output-bytes BYTES` is a hard ceiling that sheds rows from the end
- Measured: `doctor --offline --quick` is 26_277 bytes; with `--fields residual.ghost_marker_processes` it is 80
- When a row operation ran, the envelope gains `agent_ops` with `total`, `matched`, `truncated`, `omitted_rows`
- `--fields` operates on PATHS, not rows, so it reports no row counters — a `total` there would be meaningless
- `agent_ops` is omitted entirely when there is nothing to report, so a clean projection keeps the previous envelope shape
- `unresolved_paths` lists every dotted path that did not resolve, each with the `flag` that asked for it
- Read it: `--fields typo` returns `data:{}` and `--sort-rows typo` returns the rows untouched with `matched == total`
- Both are indistinguishable from success without this field
- `truncated` is the only way to tell a short payload from a cut one — always read it
- Untouched envelopes never grow the field, so existing parsers are unaffected
- A filter matching nothing is an empty list with `ok: true`, never an error and never the unfiltered list
- A missing field never matches, including under `!=`: absence is not difference
- Row operations need one list; when `data` holds several, the error names them and `--fields` narrows first
- MUST NOT pipe stdout through `jaq` to shrink a payload — that work belongs in the binary
- `--select`, `--filter`, `--limit` and `--sort` are LOCAL flags of some commands, never these globals
- Passing a local flag as a global one fails at argv time with an unexpected-argument error


## Asserting on the Payload
- `--expect EXPR` states what the emitted payload must contain, using the `--filter-rows` grammar
- Repeatable and ANDed, so several assertions can hold at once
- Evaluated LAST, over the payload you actually receive, so projection or truncation cannot hide a failure
- An expectation holds when at least ONE row satisfies it: `--expect status=200` asks "is there a 200 here?"
- Filter first when you need every row to match — `--filter-rows` narrows, `--expect` then asserts
- Unmet expectations arrive in `agent_ops.expectation_unmet`, echoed exactly as you typed them
- The exit code stays `0` by default, because changing it on data content would break pipelines that branch on it
- `--expect-exit-code` opts in to exit `65` when any expectation is unmet
- The envelope is still written first: the payload is what explains the failure
- A malformed expression fails at argv time with exit `2`, never as a silently empty match


## Scrape Additions
### Reading exact attributes
- `--format attributes` with paired `--attribute-selector CSS` and `--attribute-name NAME`, both repeatable
- Answers "what is at these exact places?", which no other format asks
- Without it, reading one attribute off a list meant pulling `rawHtml` and parsing outside the binary
- Pairs positionally; unequal counts fail at argv time rather than dropping a question silently
- Each row carries `selector`, `attribute`, `values` and `count`; a bad selector adds `error` and the other rows survive
- Read from the full document, so `--only-main-content` cannot remove the elements you named
### Acting before scraping
- `--action JSON`, repeatable, runs one `run --script` step before extraction
- Example: `--action '{"cmd":"press","target":"#load-more"}'`
- Same grammar as `run --script` on purpose, so a `record` capture stays replayable here
- Runs in this session, between navigation and extraction — a separate invocation would lose the effect
- Browser engine only; with `--engine http` it is rejected with exit `2` rather than silently ignored
- A failing action fails the scrape: it was a precondition you stated for the extraction
### Seeing what changed
- `monitor check --diff-mode git|json` reports WHAT moved, not just that it did
- `git` emits a unified diff as text; `json` emits `added` and `removed` as lists an agent reads directly
- A diff needs the previous content, and the baseline file holds only a hash
- So the content is kept in `<baseline>.content`, written whenever the flag is on
- The first run with the flag has nothing to compare against and says so via `diff_available: false`
- `added_count` and `removed_count` report the real size even when `diff_truncated` is set
- `config set monitor_diff_max_bytes` moves the ceiling


## Other Global Flags
- Every flag below applies to all 69 commands and is accepted before or after the subcommand
### Output and diagnostics
- `--json` emits the machine envelope; `--json-steps` adds a per-step envelope inside `run`
- `-q` / `--quiet` silences stderr prose; `--plain` drops ANSI from human output
- `--verbose` and `--debug` raise tracing on stderr, never on stdout
- `--correlation-id ID` stamps the envelope so a run can be traced across tools
- `--artifacts-dir DIR` chooses where files land; `--dump-on-failure` writes console and network evidence there
- Pair `--dump-on-failure` with `--capture-console` or `--capture-network`, since the capture dies with the process
### Time and concurrency
- `--timeout SECS` bounds the whole run; `--step-timeout SECS` bounds one step of `run`
- `--max-concurrency N` caps fan-out for the commands that have any
### Browser mode and anti-detection
- `--headed` renders a real window; on Linux it goes into a private virtual display when `Xvfb` is available
- `--no-xvfb` keeps a headed launch on the operator's own display instead
- `doctor` reports `xvfb` with the install command for the detected distribution; the CLI never installs anything
- `--no-stealth` turns the disguise off; `--stealth-profile` picks `auto`, `chrome-linux`, `chrome-win` or `chrome-mac`
- `auto` follows the host platform, and a headless launch still gets a User-Agent override so it does not announce `HeadlessChrome`
- `--stealth-seed SEED` pins the identity so it is stable across processes
- `--input-profile human|direct` and `--input-seed SEED` govern pointer and keyboard timing
- `--warmup` visits the origin root first; `--warmup-url URL` names a different entry point and implies `--warmup`
- The cookie jar lives for one process only; the scrape envelope states that as `cookie_jar_persistent: false`
- `doctor` repeats that scope as the `cookie_jar_scope` check, so the limit is discoverable without a scrape
- Use `storage export` and `storage import` to carry a session between invocations
- The envelope reports `profile_contradicts_host: true` when the stealth profile claims another platform
- Read that field before blaming a block: TLS and HTTP/2 carry the real stack whatever the User-Agent says
- Anti-detection defaults, all set with `config set` and never with an environment variable
- `stealth` is `true`, `stealth_profile` is `auto`, `browser_mode` is `auto`
- `stealth_seed` has no default; set it only when a stable identity is required
- `http2_enabled` is `true`, so HTTP/2 fingerprinting is on before you ask for it
- `http2_initial_stream_window_size` is 6291456 and `http2_initial_connection_window_size` is 15663105
- `http2_max_header_list_size` is 262144 and `http2_max_frame_size` is 16384
- `http2_adaptive_window` governs window growth during a live connection
- Any HTTP/2 change moves the observable fingerprint, so it needs an explicit operator decision
- `input_profile` is `human`, `input_move_steps` is 24, `input_move_gap_ms` is 12
- `input_click_dwell_ms` is 65, `input_key_dwell_ms` is 45, `input_type_delay_ms` is 95
- `input_scroll_tick_px` is 100, `input_scroll_max_ticks` is 40, `input_scroll_settle_rounds` is 3
- `input_target_jitter_px` is 3 and spreads the pointer landing point
- `input_profile direct` removes that pacing and is observable to the origin
- `robots_user_agent` names the identity used when fetching robots
- `scrape_no_cache` opts out of the scrape response cache
- `monitor_diff_max_bytes` is 65536 and bounds the stored diff content
### Network
- `--proxy URL` routes both engines; credentials belong in XDG via `config set proxy_url`, never in argv
- `--proxy-bypass HOSTS` adds hosts that skip the proxy
- Loopback is bypassed automatically under `--proxy`, because the CDP control channel is loopback
- Without it, a proxy failure surfaces as a Chrome startup timeout and blames the wrong component
- `config set cdp_proxy_bypass_loopback false` opts out
- `proxy_url`, `proxy_bypass`, `proxy_username` and `proxy_password` are XDG keys, never argv secrets
- `cdp_proxy_bypass_loopback` is `true`, so the CDP control channel stays off the proxy
- `--mitm` and its `--mitm-*` companions intercept traffic; `--allow-outside-roots` permits reads and writes outside the allowed roots
- MITM secret masking is ON by default, so a capture never carries raw secrets by accident
- `--mitm-redact-secrets` restates that default explicitly and changes nothing
- `--mitm-no-redact-secrets` is the only way to turn the masking off
- Asking for both resolves to MASK, because the safe reading of a contradiction about secrets is to mask
- `--mitm-max-body-bytes` caps a captured body; the default ceiling is 65536 bytes
- `--mitm-no-media-bodies` drops image, video and audio bodies from the capture
- `--ignore-robots` needs `--i-accept-robots-risk` as well; one flag alone does not bypass robots
### Feature gates
- `--category-memory` for `heap`, `--category-extensions` for `extension`
- `--category-third-party` for `devtools3p`, `--category-webmcp` for `webmcp`
- `--experimental-vision` for `click-at`, `--experimental-screencast` for `screencast`
- `--lang en|pt-BR` selects the message language


## Exit Codes
- `0` success
- `2` usage
- `6` blocked — the origin served a bot check instead of content. Transport succeeded (HTTP 200, valid HTML), so `status_code` and `http_error` report success while the body carries a challenge. Read `error.suggestion`; retrying the same request escalates toward a ban
- `65` data
- `66` no input
- `69` unavailable
- `70` software, browser, protocol
- `74` I/O
- `78` config
- `124` timeout
- `130` cancelled
- `141` broken pipe
