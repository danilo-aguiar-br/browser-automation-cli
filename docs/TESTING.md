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
- Robots and pipe behaviour tests (`tests/robots_http.rs`, `tests/pipe_broken.rs`)
- Golden i18n and cold-start helpers (`tests/golden_i18n.rs`, `tests/cold_start.rs`)
- Optional e2e CDP event coverage when Chrome is available (`tests/e2e_cdp_events.rs`)
- Full **53-tool** DevTools e2e script (legacy filename): `scripts/e2e_all_52_tools.sh`
- Live CLI inventory is **59 commands** (`commands --json`) — broader than the 53 tool-ref e2e set
- Vendored tool-ref fixture: `tests/fixtures/tool-reference.md`


## How to Run
```bash
timeout 300 cargo test --locked
timeout 300 cargo test --lib --locked
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
- Maintainer evidence for v0.1.3: 53 PASS / 0 FAIL on a local host with Chrome (residual A001 closed)
- The 52-tool suite does not replace residual smokes for commands outside the tool-ref set


## Residual PRD Smokes (beyond 53 tools)
Run after e2e when validating the full 59-command inventory:

```bash
# print-pdf artifact
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

# find-paths --glob
browser-automation-cli --json find-paths --glob '**/*.rs' .

# run JSON array
cat > /tmp/demo.array.json <<'JSON'
[{"cmd":"goto","url":"https://example.com"},{"cmd":"view"}]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/demo.array.json

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
```
- Also useful: browser format scrape, `config path`, `mitm start`, doctor XDG, i18n `--lang pt-BR`


## Lighthouse Mock
```bash
browser-automation-cli --json lighthouse https://example.com \
  --lighthouse-path ./scripts/mock-lighthouse.sh
```
- Use `--lighthouse-path` or XDG `lighthouse_path` to point at `scripts/mock-lighthouse.sh` when a real Lighthouse install is unavailable
- Resolve order: flag → XDG `lighthouse_path` → PATH
- Envelope reports `binary_source` as `real` or `mock`
- The mock writes minimal HTML/JSON reports for smoke paths
- Doctor reports lighthouse presence/source as informational when the binary is missing


## Local Validation Profiles
- Run fmt, clippy, and non-browser contract tests first on your machine
- Browser-backed tests require Chrome or Chromium installed locally
- Validation runs locally with cargo and e2e scripts on the maintainer machine
- Keep crates.io publish blocked without explicit maintainer approval
- Optional pillar smokes after e2e: `run` + scrape, residual PRD commands above, `config path`, `mitm start`, doctor XDG


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
- Inventory drift: refresh against `commands --json` (59) and `tests/fixtures/tool-reference.md` (53 tools)
- E2E script missing binary: run `cargo build --release --locked` first so `target/release/browser-automation-cli` exists
- Lighthouse path missing: pass `--lighthouse-path ./scripts/mock-lighthouse.sh` or set XDG `lighthouse_path`
- LLM extract fail-closed: expected without `config set openrouter_api_key`
- MITM bind issues: ensure local loopback is free and review `mitm status --json`
- Workflow journal confusion: inspect `workflow status` and XDG `workflow_dir` from `config path --json`
