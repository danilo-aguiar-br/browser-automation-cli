[English](INTEGRATIONS.md) | [Português Brasileiro](INTEGRATIONS.pt-BR.md)

# Integrations — browser-automation-cli

> One process, one Chrome, one JSON envelope. Built for agent subprocesses.

## Coverage Snapshot
- Works with any agent that can spawn a subprocess and read stdout plus stderr
- Primary surfaces: Claude Code, Codex, Cursor, local shell, editor agents
- Discovery helpers: `commands --json`, `schema --cmd`, `doctor --json`
- Integration path is local subprocess only
- Product settings are flags plus XDG config only

## Flag Aliases and Version Notes
- Product names stay fixed: `view`, `press`, `write`, `grab`
- Avoid inventing aliases such as `click` or `screenshot` in agent prompts (use `grab` for screenshots; scrape may accept a `screenshot` format token)
- Use `grab --path <file>` (not a bare positional path)
- Use repeatable `wait --text` for OR semantics across multiple strings
- Use `scrape --format` / `scrape --engine` for local scrape formats
- Browser scrape applies `--format` via outerHTML (markdown/html/links/metadata/raw-html/screenshot/summary/product/branding)
- `0.1.0` ships the default-on DevTools parity surface plus category gates
- `0.1.1` adds XDG `config`, local MITM, workflow journal, and local scrape/crawl/map/search/parse surface (`batch-scrape`, `crawl`, `map`, `search`, `parse`, expanded `scrape`)
- `0.1.2` closes agent-first gaps and adds `print-pdf`, `monitor`, `qr`, `find-paths`, parse document types, extract LLM, and expanded config keys
- Experimental tools require `--experimental-vision` or `--experimental-screencast`

## Summary Table

| Surface | Integration style | Required flags | Notes |
|---------|-------------------|----------------|-------|
| Claude Code | subprocess | `--json` | multi-step via `run --script` |
| Codex | subprocess | `--json -q` | quiet stderr for cleaner transcripts |
| Cursor | shell tool | `--json` | keep timeouts explicit |
| Local shell | script | `--json` | parse with `jaq` |
| Continue / Cline | editor shell | `--json -q` | one-shot only |

## Claude Code
- Spawn one CLI process per atomic action
- Use `run --script` when `@eN` refs must survive multiple steps
- Prefer XDG `config set` for durable defaults
```bash
browser-automation-cli doctor --offline --quick --json
browser-automation-cli --json goto https://example.com
browser-automation-cli --json view
```

## Codex
- Prefer `-q --json` so only envelopes reach the agent transcript
```bash
browser-automation-cli -q --json goto https://example.com
```

## Cursor
- Call the binary from the shell tool with explicit `--timeout`
```bash
browser-automation-cli --timeout 60 --json scrape https://example.com --format markdown --engine http
```

## Local Shell
- Always capture exit codes before parsing JSON
- Run validations on your local machine before release
```bash
out=$(browser-automation-cli --json version)
echo "$out" | jaq -e '.ok == true'
```

## Continue and Cline
- Use quiet JSON mode to keep editor transcripts clean
- Do not expect session stickiness between separate process launches

## New Flags by Version
- `0.1.0`: category gates, experimental vision and screencast, capture flags, schema discovery
- `0.1.1`: XDG `config` (`init`/`path`/`show`/`get`/`set`), `mitm` (local CA + one-shot `127.0.0.1` proxy), `workflow` (`run`/`resume`/`status`), local scrape surface (`scrape --format/--engine`, `batch-scrape`, `crawl`, `map`, `search`, `parse`), multi-text `wait --text` OR, `grab --path`
- `0.1.2`:
  - `scrape --engine browser` applies `--format` (incl. `raw-html`, `screenshot`, `summary`, `product`, `branding`) via outerHTML
  - `run` scroll aliases `dy`/`dx` for `delta_y`/`delta_x`; fail-fast error envelopes may include partial `data.steps`
  - `schema --cmd` expanded for `goto`/`eval`/`type`/`scroll`/`assert`
  - `--lang pt-BR` and `config set lang` localize human suggestions
  - Logging via `--verbose`/`--debug` and XDG `log_level`/`chrome_path`/`lighthouse_path` only
  - `search` cleans `uddg=` SERP redirects
  - `print-pdf` one-shot CDP; `monitor check --url --baseline [--write-baseline]`
  - `parse` PDF/DOCX/xlsx/ods + `--redact-pii`; `extract --llm --question --schema-json` (XDG `openrouter_api_key`, `llm_base_url`, `llm_model`)
  - `qr encode|decode`, `find-paths`
  - `assert` aliases `url_contains`/`text_contains`; `attr` DOM property fallback
  - Config keys: `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`
  - Command inventory is 56 top-level names (`commands --json`), including `print-pdf`, `monitor`, `qr`, `find-paths`
