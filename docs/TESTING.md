[English](TESTING.md) | [Português Brasileiro](TESTING.pt-BR.md)

# Testing — browser-automation-cli

> Run the right suite for the risk, not every browser path by default.


## Why Categorized Tests
- Browser runtime tests are slower and host-dependent
- Schema and inventory tests catch contract drift without Chrome
- Keeping categories explicit protects local iteration speed
- Prefer local validation with cargo and e2e scripts


## Test Categories
- Unit and library tests in `src/` (`cargo test --lib`)
- CLI smoke tests such as `tests/doctor_cli.rs` and `tests/goto_smoke.rs`
- Envelope and schema gates such as `tests/envelope_schema.rs` and `tests/parity_toolref_schema.rs`
- Parity inventory and matrix tests (`tests/parity_inventory.rs`, `tests/parity_matrix.rs`)
- Run inventory gate: `tests/parity_run_inventory.rs` enforces `RUN_DISPATCHED_CMDS` ∪ intentional exclude (includes `print-pdf`, `select-option`, `pick`)
- Clap surface gate: `tests/clap_command_debug_assert.rs` runs `Cli::command().debug_assert()`
- Robots and pipe behaviour tests (`tests/robots_http.rs`, `tests/pipe_broken.rs`)
- Golden i18n and cold-start helpers (`tests/golden_i18n.rs`, `tests/cold_start.rs`)
- Optional e2e CDP event coverage when Chrome is available (`tests/e2e_cdp_events.rs`)
- Full **53-tool** DevTools e2e script (legacy filename): `scripts/e2e_all_52_tools.sh`
- Live CLI inventory is **65 agent names** (`commands --json`) — broader than the 53 tool-ref e2e set; includes agent-inventory `select-option` and `pick` (run/exec/schema, not clap), meta `locale` and `man`, plus clap `submit` and `storage`
- v0.1.6 product gates (local, Chrome serial when required):
  - `tests/dialog_multitab_gate.rs` — multi-tab dialog isolation + `dialog_settled` (GAP-054)
  - `tests/option_pick_gate.rs` — native select `input`+`change` (GAP-055)
  - `tests/wait_conditions_gate.rs` — `wait_timeout_ms` deadline honesty (GAP-053)
  - `tests/scrape_step_gate.rs` — scrape `format`/`formats` in run without HTML monster (GAP-057)
  - Lighthouse unit fixtures: `scripts/fixtures/lighthouse/minimal_lhr.json` + `chrome_captured_lhr.json` (real LHR-shaped scores_from_lhr parse; GAP-021 partial)
- Residual integration suite: `tests/residual_one_shot.rs` (marker zero, Singleton non-growth, BORN fixture wipe, doctor residual fields)
- Local residual gates: `scripts/residual-check.sh`, `scripts/residual-stress.sh` (local maintainer scripts only)
- Vendored tool-ref fixture: `tests/fixtures/tool-reference.md`


## How to Run
```bash
timeout 300 cargo test --locked
timeout 300 cargo test --lib --locked
timeout 120 cargo test --lib residual:: --locked
timeout 120 cargo test --test residual_one_shot --locked
timeout 120 cargo test --test parity_run_inventory --locked
timeout 120 cargo test --test clap_command_debug_assert --locked
timeout 180 cargo test --test dialog_multitab_gate --locked
timeout 180 cargo test --test option_pick_gate --locked
timeout 180 cargo test --test wait_conditions_gate --locked
timeout 120 cargo test --test scrape_step_gate --locked
timeout 120 cargo test --lib scores_from --locked
timeout 120 cargo clippy --all-targets --locked -- -D warnings
cargo fmt --check
```
- Run a single file with `cargo test --test doctor_cli --locked`
- Use `-- --nocapture` only while debugging
- Prefer library and schema gates first when iterating on contracts


## E2E 53 Tools
```bash
cargo build --release --locked
bash scripts/e2e_all_52_tools.sh
```
- Requires a release binary at `target/release/browser-automation-cli` (build with `cargo build --release --locked` first)
- Exercises DevTools-parity tools against the local fixture page under `scripts/fixtures/e2e_page/`
- Writes a report under a temp workdir and prints PASS/FAIL/SKIP counts
- Maintainer evidence for v0.1.4: 53 PASS / 0 FAIL on a local host with Chrome (residual A001 closed; GAP-001…025 hard-close)
- Maintainer evidence for v0.1.5: residual-zero disk closed (RES-01…12); `cargo test --lib residual::` + `cargo test --test residual_one_shot` + local residual-check PASS
- **Maintainer evidence for v0.1.6 (honest):** `TOTAL=53 PASS=52 FAIL=0 SKIP=1` — lighthouse mock path is **SKIP** (CONTRACT-ONLY). Never claim full e2e lighthouse parser PASS
- Lighthouse parser confidence is unit-level: `scores_from_lhr` against `minimal_lhr.json` and real sanitized `chrome_captured_lhr.json` (Lighthouse 13.4.1 shape)
- The 52-tool suite does not replace residual smokes for commands outside the tool-ref set


## Residual-Zero Disk Gates (v0.1.5 — still current in 0.1.6)
```bash
cargo build --release --locked
cargo test --lib residual:: --locked
cargo test --test residual_one_shot --locked
bash scripts/residual-check.sh
# optional stress of N one-shots:
# bash scripts/residual-stress.sh
```
- `residual_one_shot` covers: CLI marker zero after goto, Chromium Singleton non-growth after print-pdf, BORN wipe of stale Singleton fixture, doctor residual fields
- `residual-check.sh` runs doctor (BORN GC path-light) + one-shot print-pdf + asserts zero CLI markers and doctor JSON `residual`
- `residual-stress.sh` repeats one-shot work to stress residual hygiene locally
- Doctor check id under test: `residual_disk` (path-light residual disk hygiene)
- Doctor top-level JSON field under test: `residual` (`ResidualDiskReport`)
- Doctor residual fields under test: `cli_marker_dirs`, `chromium_tmp_singleton_orphans`, `scavenge_safe_candidates`, `live_cli_marker_processes`
- Age floor for production stale GC is 60s; tests may use zero-age library helpers for fixtures


## v0.1.6 Product Gates (dialog / select / wait / scrape / lighthouse units)
```bash
cargo test --test dialog_multitab_gate --locked
cargo test --test option_pick_gate --locked
cargo test --test wait_conditions_gate --locked
cargo test --test scrape_step_gate --locked
# Lighthouse pure parse (no claim of e2e PASS):
cargo test --lib --locked scores_from
# Residual still required:
bash scripts/residual-check.sh
```
- `dialog_multitab_gate`: isolation tab1 + accept owner via `Page::session_id` multi-tab gate; asserts `dialog_settled` without invented wait (GAP-054)
- `option_pick_gate`: native select events + `via: native_select` (GAP-055)
- `wait_conditions_gate`: deadline honors `wait_timeout_ms` (~2s, not silent default) (GAP-053)
- `scrape_step_gate`: run scrape `format=text` without HTML dump (GAP-057)
- Lighthouse e2e mock remains SKIP; unit fixtures are the honest parser gate (GAP-021 partial)
- **`grab` encode:** png|jpeg|webp only; AVIF removed (breaking) — residual smokes must not pass `--format avif`
- **GAP-024 intentional residual:** PRD wishlist divergences stay in `parity_intentional_divergences.json` (do not claim full PRD parity)
- Do **not** treat remote orchestration dashboards as product surface; use local cargo and `scripts/*-check.sh` only

## Full agent inventory (65)

Discover live: `browser-automation-cli commands --json`

```
assert attr back batch-scrape click-at commands completions config console cookie
crawl devtools3p dialog doctor drag emulate eval exec extension extract fill-form
find-paths forward goto grab heap hover keys lighthouse locale man map mitm monitor
net page parse perf pick press print-pdf qr reload resize run schema scrape screencast
scroll search select-option sg-rewrite sg-scan sheet-write storage submit text type
upload version view wait webmcp workflow write
```

Note: `pick` and `select-option` are multi-step inventory names used in `run` scripts; clap product subcommand count is 63.

## Residual PRD Smokes (beyond 53 tools)
Run after e2e when validating the full **65**-name inventory:

```bash
# print-pdf artifact (one-shot + run)
browser-automation-cli --json print-pdf --url https://example.com --path /tmp/page.pdf

# monitor baseline check
browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/mon.base --write-baseline

# QR encode/decode (no Chrome)
browser-automation-cli --json qr encode --text 'hello' --format png --path /tmp/qr.png
browser-automation-cli --json qr decode --path /tmp/qr.png

# find-paths (no Chrome)
browser-automation-cli --json find-paths 'Cargo.*' .
browser-automation-cli --json find-paths --glob '**/*.rs' .

# sheet-write / sg-scan / sg-rewrite (no Chrome)
printf 'a,b\n1,2\n' > /tmp/rows.csv
browser-automation-cli --json sheet-write /tmp/rows.csv -o /tmp/out.xlsx
browser-automation-cli --json sg-scan . --limit 20
browser-automation-cli --json sg-rewrite .

# run JSON array + json-steps stream (GAP-020)
cat > /tmp/demo.array.json <<'JSON'
[{"cmd":"goto","url":"https://example.com"},{"cmd":"view"}]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/demo.array.json
browser-automation-cli --timeout 60 --json --json-steps run --script /tmp/demo.array.json

# wait multi-selector / url_contains (GAP-019/024)
cat > /tmp/wait.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"wait","selector":"h1, body","ms":3000},
  {"cmd":"wait","url_contains":"example.com","ms":3000}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/wait.json

# pick / select-option (run-only inventory; GAP-023)
# browser-automation-cli --timeout 60 --json run --script '[{"cmd":"goto","url":"…"},{"cmd":"pick","target":"…","option":"…"}]'

# assert console kinds (GAP-025)
# browser-automation-cli --capture-console --timeout 60 --json run --script '[{"cmd":"goto","url":"https://example.com"},{"cmd":"assert","kind":"console_empty"}]'

# schema positional (GAP-022)
browser-automation-cli --json schema run
browser-automation-cli --json schema --cmd wait

# view --allow-empty (GAP-012)
browser-automation-cli --json view --allow-empty

# multi-format scrape + batch/crawl browser engine (GAP-009/010)
browser-automation-cli --json scrape https://example.com --format markdown,html,links --engine http
printf '%s\n' 'https://example.com' > /tmp/urls.txt
browser-automation-cli --json batch-scrape --urls-file /tmp/urls.txt --format text --engine http --concurrency 1
# browser-automation-cli --timeout 120 --json batch-scrape --urls-file /tmp/urls.txt --format text --engine browser --concurrency 1

# MITM capture-url + har --out (GAP-011)
browser-automation-cli --json mitm init-ca
# browser-automation-cli --json mitm capture-url https://example.com --seconds 15 --har /tmp/cap.har
# browser-automation-cli --json mitm har --out /tmp/capture.har
# browser-automation-cli --json mitm redact --secrets

# config list-keys + redis honesty (no rediss)
browser-automation-cli --json config list-keys
# browser-automation-cli --json config set cache_backend redis
# browser-automation-cli --json config set cache_redis_url redis://127.0.0.1:6379

# lighthouse binary_source (mock)
browser-automation-cli --json lighthouse https://example.com \
  --lighthouse-path ./scripts/mock-lighthouse.sh | jaq '.data.binary_source // .'

# parse PDF / DOCX with optional PII redact
browser-automation-cli --json parse tests/fixtures/hello.pdf
browser-automation-cli --json parse tests/fixtures/hello.docx --redact-pii

# extract --llm fail-closed without XDG key
browser-automation-cli --json extract https://example.com --llm --question 'What is the title?'
# expect usage envelope requiring: config set openrouter_api_key

# clap JSON usage error (GAP-002)
browser-automation-cli --json not-a-real-command 2>/dev/null | jaq -e '.ok == false' || true

# dialog soft path
browser-automation-cli --json dialog accept --if-present
# console dump always []
browser-automation-cli --capture-console --json console dump --path /tmp/console.json
# beforeunload flag help surface
browser-automation-cli goto --help | rg handle-before-unload
# page isolated context
browser-automation-cli page new --help | rg isolated-context
# print-pdf in run
# cat > /tmp/pdf.run.json <<'JSON'
# [{"cmd":"goto","url":"https://example.com"},{"cmd":"print-pdf","path":"/tmp/page-from-run.pdf"}]
# JSON
# browser-automation-cli --timeout 60 --json run --script /tmp/pdf.run.json
# schema already covered

# locale / man meta + submit/storage (inventory 65)
browser-automation-cli --json locale
browser-automation-cli --json man >/tmp/browser-automation-cli.1
browser-automation-cli --json schema submit
browser-automation-cli --json schema storage
browser-automation-cli --json config list-keys
browser-automation-cli --json config set dialog_settle_ms 2000

# residual doctor fields (v0.1.5 law still current)
browser-automation-cli doctor --offline --quick --json | jaq '.residual'
```
- Also useful: browser format scrape, `config path`, `mitm start`, doctor XDG, i18n `--lang pt-BR`
- Contract tests to cite in evidence: `parity_run_inventory`, `clap_command_debug_assert`, `residual_one_shot`, residual lib tests, `dialog_multitab_gate`, `option_pick_gate`, `wait_conditions_gate`, `scrape_step_gate`


## Lighthouse Mock (e2e SKIP honesty)
```bash
browser-automation-cli --json lighthouse https://example.com \
  --lighthouse-path ./scripts/mock-lighthouse.sh
```
- Use `--lighthouse-path` or XDG `lighthouse_path` to point at `scripts/mock-lighthouse.sh` when a real Lighthouse install is unavailable
- Resolve order: flag → XDG `lighthouse_path` → PATH
- Envelope reports `binary_source` as `real` or `mock`
- The mock writes minimal HTML/JSON reports for smoke paths
- Doctor reports lighthouse presence/source as informational when the binary is missing
- **v0.1.6 honesty:** e2e suite **SKIPs** the lighthouse mock path — never report that as a full parser PASS
- Parser confidence: unit tests on `scripts/fixtures/lighthouse/minimal_lhr.json` and `chrome_captured_lhr.json`


## Local Validation Profiles
- Run fmt, clippy, and non-browser contract tests first on your machine
- Browser-backed tests require Chrome or Chromium installed locally
- Validation runs locally with cargo and e2e scripts on the maintainer machine
- Keep crates.io publish blocked without explicit maintainer approval
- Optional pillar smokes after e2e: `run` + `--json-steps`, residual PRD commands above, residual-check, `config path`, `mitm capture-url`, doctor XDG + residual


## Documentation Schema and Bilingual Audit
```bash
cargo build --release --locked
bash scripts/generate_command_schemas.sh
bash scripts/generate_command_schemas.sh --check
bash scripts/audit_bilingual_docs.sh
```
- `generate_command_schemas.sh` writes one `docs/schemas/<cmd>.schema.json` per inventory command from live `schema --cmd` (meta.rs surface)
- `--check` fails when static command schemas drift from the installed binary
- Envelopes and `run-script-step.schema.json` are preserved and not overwritten by the generator
- `audit_bilingual_docs.sh` compares `browser-automation-cli` invocations inside code fences for EN and `.pt-BR` pairs
- Exit `0` means fence multisets match; exit `1` means drift; exit `2` means a missing pair file


## Logging and Paths During Tests
- Product logging in the CLI under test: `--verbose` / `--debug` / `-q` or XDG `config set log_level`
- Color defaults via `config set color`
- Host-specific Chrome path overrides via `config set chrome_path` when discovery needs it
- Resolve XDG layout with `config path --json`


## Troubleshooting
- Doctor fails on chrome: install Chromium or Google Chrome first, or set `config set chrome_path`
- Timeouts in goto smoke: raise process timeout or inspect network policy
- Schema gate failures: update both code and `docs/schemas/` in the same change
- Command schema drift: re-run `bash scripts/generate_command_schemas.sh` after changing `meta.rs`
- Bilingual fence drift: re-run `bash scripts/audit_bilingual_docs.sh` and align EN and `.pt-BR` command blocks
- Inventory drift: refresh against `commands --json` (65) and `tests/fixtures/tool-reference.md` (53 tools)
- Residual disk leaks: re-run `cargo test --test residual_one_shot` and `bash scripts/residual-check.sh`; inspect doctor `residual`
- Run inventory drift: refresh `RUN_DISPATCHED_CMDS` and re-run `cargo test --test parity_run_inventory`
- Clap assert failures: fix `GlobalOpts` / subcommand definitions then re-run `cargo test --test clap_command_debug_assert`
- E2E script missing binary: run `cargo build --release --locked` first so `target/release/browser-automation-cli` exists
- Lighthouse path missing: pass `--lighthouse-path ./scripts/mock-lighthouse.sh` or set XDG `lighthouse_path`
- LLM extract fail-closed: expected without `config set openrouter_api_key`
- MITM bind issues: ensure local loopback is free and review `mitm status --json`
- Workflow journal confusion: inspect `workflow status` and XDG `workflow_dir` from `config path --json`
