[English](TESTING.md) | [Português Brasileiro](TESTING.pt-BR.md)

# Testes — browser-automation-cli

> Rode a suite certa para o risco, não todo path de browser por default.


## Por que Testes Categorizados
- Testes de runtime de browser são mais lentos e dependentes do host
- Testes de schema e inventário pegam drift de contrato sem Chrome
- Manter categorias explícitas protege a velocidade de iteração local
- Prefira validação local com cargo e scripts e2e


## Categorias de Teste
- Testes unitários e de library em `src/` (`cargo test --lib`)
- Smokes de CLI como `tests/doctor_cli.rs` e `tests/goto_smoke.rs`
- Gates de envelope e schema como `tests/envelope_schema.rs` e `tests/parity_toolref_schema.rs`
- Testes de inventário e matriz de paridade (`tests/parity_inventory.rs`, `tests/parity_matrix.rs`)
- Gate de inventário run: `tests/parity_run_inventory.rs` enforce `RUN_DISPATCHED_CMDS` ∪ exclude intencional (inclui `print-pdf`, `select-option`, `pick`)
- Gate de superfície clap: `tests/clap_command_debug_assert.rs` roda `Cli::command().debug_assert()`
- Testes de robots e comportamento de pipe (`tests/robots_http.rs`, `tests/pipe_broken.rs`)
- Helpers de golden i18n e cold-start (`tests/golden_i18n.rs`, `tests/cold_start.rs`)
- Cobertura e2e opcional de eventos CDP quando Chrome está disponível (`tests/e2e_cdp_events.rs`)
- Script e2e completo das **53 tools** DevTools (nome legado do arquivo): `scripts/e2e_all_52_tools.sh`
- Inventário vivo da CLI é **65 nomes de agente** (`commands --json`) — mais amplo que o conjunto e2e de 53 tool-ref; inclui inventário de agente `select-option` e `pick` (run/exec/schema, não clap), meta `locale` e `man`, mais clap `submit` e `storage`
- Gates de produto v0.1.6 (locais; Chrome serial quando necessário):
  - `tests/dialog_multitab_gate.rs` — isolamento multi-aba de diálogo + `dialog_settled` (GAP-054)
  - `tests/option_pick_gate.rs` — select nativo `input`+`change` (GAP-055)
  - `tests/wait_conditions_gate.rs` — honesty de prazo `wait_timeout_ms` (GAP-053)
  - `tests/scrape_step_gate.rs` — scrape `format`/`formats` em run sem monstro HTML (GAP-057)
  - Fixtures unit de lighthouse: `scripts/fixtures/lighthouse/minimal_lhr.json` + `chrome_captured_lhr.json` (parse LHR real de scores_from_lhr; GAP-021 parcial)
- Suite de integração residual: `tests/residual_one_shot.rs` (marker zero, não-crescimento de Singleton, wipe de fixture no BORN, campos residual no doctor)
- Gates locais residual: `scripts/residual-check.sh`, `scripts/residual-stress.sh` (só scripts locais do mantenedor)
- Fixture vendored de tool-ref: `tests/fixtures/tool-reference.md`


## Como Rodar
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
- Rode um arquivo com `cargo test --test doctor_cli --locked`
- Use `-- --nocapture` só durante debug
- Prefira library e gates de schema primeiro ao iterar contratos


## E2E 53 Tools
```bash
cargo build --release --locked
bash scripts/e2e_all_52_tools.sh
```
- Exige binário release em `target/release/browser-automation-cli` (faça `cargo build --release --locked` antes)
- Exercita tools de paridade DevTools na página fixture em `scripts/fixtures/e2e_page/`
- Escreve relatório em workdir temp e imprime contagens PASS/FAIL/SKIP
- Evidência do mantenedor para v0.1.4: 53 PASS / 0 FAIL em host local com Chrome (residual A001 fechado; GAP-001…025 hard-close)
- Evidência do mantenedor para v0.1.5: residual-zero em disco fechado (RES-01…12); `cargo test --lib residual::` + `cargo test --test residual_one_shot` + residual-check local PASS
- **Evidência do mantenedor para v0.1.6 (honesta):** `TOTAL=53 PASS=52 FAIL=0 SKIP=1` — caminho mock de lighthouse é **SKIP** (CONTRACT-ONLY). Nunca alegue PASS completo do parser lighthouse em e2e
- Confiança do parser lighthouse é em unit: `scores_from_lhr` contra `minimal_lhr.json` e `chrome_captured_lhr.json` real sanitizado (forma Lighthouse 13.4.1)
- A suite de 52-tools não substitui smokes residuais de comandos fora do conjunto tool-ref


## Gates Residual-Zero de Disco (v0.1.5 — ainda corrente na 0.1.6)
```bash
cargo build --release --locked
cargo test --lib residual:: --locked
cargo test --test residual_one_shot --locked
bash scripts/residual-check.sh
# stress opcional de N one-shots:
# bash scripts/residual-stress.sh
```
- `residual_one_shot` cobre: marker CLI zero após goto, não-crescimento de Singleton Chromium após print-pdf, wipe BORN de fixture Singleton stale, campos residual no doctor
- `residual-check.sh` roda doctor (BORN GC path-light) + print-pdf one-shot + assert zero markers CLI e JSON `residual` do doctor
- `residual-stress.sh` repete trabalho one-shot para estressar higiene residual localmente
- Check id do doctor sob teste: `residual_disk` (higiene residual de disco path-light)
- Campo JSON de topo do doctor sob teste: `residual` (`ResidualDiskReport`)
- Campos residual do doctor sob teste: `cli_marker_dirs`, `chromium_tmp_singleton_orphans`, `scavenge_safe_candidates`, `live_cli_marker_processes`
- Age floor do GC stale de produção é 60s; testes podem usar helpers de library com age zero para fixtures


## Gates de Produto v0.1.6 (dialog / select / wait / scrape / units lighthouse)
```bash
cargo test --test dialog_multitab_gate --locked
cargo test --test option_pick_gate --locked
cargo test --test wait_conditions_gate --locked
cargo test --test scrape_step_gate --locked
# Parse puro de lighthouse (sem alegar PASS e2e):
cargo test --lib --locked scores_from
# Residual ainda obrigatório:
bash scripts/residual-check.sh
```
- `dialog_multitab_gate`: isolamento tab1 + accept owner via gate multi-aba `Page::session_id`; asserta `dialog_settled` sem wait inventado (GAP-054)
- `option_pick_gate`: eventos nativos de select + `via: native_select` (GAP-055)
- `wait_conditions_gate`: prazo honra `wait_timeout_ms` (~2s, não default silencioso) (GAP-053)
- `scrape_step_gate`: scrape em run com `format=text` sem dump de HTML (GAP-057)
- Lighthouse e2e mock permanece SKIP; fixtures unit são o gate honesto do parser (GAP-021 parcial)
- **Encode do `grab`:** só png|jpeg|webp; AVIF removido (breaking) — smokes residuais não devem passar `--format avif`
- **Residual intencional GAP-024:** divergências wishlist de PRD ficam em `parity_intentional_divergences.json` (não alegue paridade PRD completa)
- **Não** trate dashboards remotos de orquestração como superfície de produto; use só cargo local e `scripts/*-check.sh`

## Inventário completo de agente (65)

Descubra ao vivo: `browser-automation-cli commands --json`

```
assert attr back batch-scrape click-at commands completions config console cookie
crawl devtools3p dialog doctor drag emulate eval exec extension extract fill-form
find-paths forward goto grab heap hover keys lighthouse locale man map mitm monitor
net page parse perf pick press print-pdf qr reload resize run schema scrape screencast
scroll search select-option sg-rewrite sg-scan sheet-write storage submit text type
upload version view wait webmcp workflow write
```

Nota: `pick` e `select-option` são nomes multi-passo de inventário usados em scripts `run`; a contagem de subcomandos clap de produto é 63.

## Smokes Residuais de PRD (além das 53 tools)
Rode após o e2e ao validar o inventário completo de **65** nomes:

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

# locale / man meta (inventário 65)
browser-automation-cli --json locale
browser-automation-cli --json man >/tmp/browser-automation-cli.1

# campos residual do doctor (v0.1.5)
browser-automation-cli doctor --offline --quick --json | jaq '.residual'
browser-automation-cli --json schema submit
browser-automation-cli --json schema storage
browser-automation-cli --json config list-keys
browser-automation-cli --json config set dialog_settle_ms 2000
```
- Também úteis: scrape browser com format, `config path`, `mitm start`, doctor XDG, i18n `--lang pt-BR`
- Testes de contrato a citar em evidência: `parity_run_inventory`, `clap_command_debug_assert`, `residual_one_shot`, testes lib residual, `dialog_multitab_gate`, `option_pick_gate`, `wait_conditions_gate`, `scrape_step_gate`
- Também: `dialog --if-present`, `console dump` (array `[]` quando vazio), `print-pdf` dentro de `run`


## Mock de Lighthouse (honesty SKIP em e2e)
```bash
browser-automation-cli --json lighthouse https://example.com \
  --lighthouse-path ./scripts/mock-lighthouse.sh
```
- Use `--lighthouse-path` ou XDG `lighthouse_path` apontando para `scripts/mock-lighthouse.sh` quando não houver Lighthouse real
- Ordem de resolve: flag → XDG `lighthouse_path` → PATH
- Envelope reporta `binary_source` como `real` ou `mock`
- O mock grava reports HTML/JSON mínimos para paths de smoke
- Doctor reporta presença/origem de lighthouse como informativo quando o binário está ausente
- **Honesty v0.1.6:** a suite e2e faz **SKIP** do caminho mock de lighthouse — nunca reporte isso como PASS completo do parser
- Confiança do parser: testes unit em `scripts/fixtures/lighthouse/minimal_lhr.json` e `chrome_captured_lhr.json`


## Perfis de Validação Local
- Rode fmt, clippy e testes de contrato sem browser primeiro na sua máquina
- Testes com browser exigem Chrome ou Chromium instalado localmente
- A validação roda localmente com cargo e scripts e2e na máquina do mantenedor
- Mantenha publish no crates.io bloqueado sem aprovação explícita do mantenedor
- Smokes opcionais de pilares após e2e: `run` + `--json-steps`, comandos residuais de PRD acima, residual-check, `config path`, `mitm capture-url`, doctor XDG + residual


## Auditoria de Schemas e Documentação Bilíngue
```bash
cargo build --release --locked
bash scripts/generate_command_schemas.sh
bash scripts/generate_command_schemas.sh --check
bash scripts/audit_bilingual_docs.sh
```
- `generate_command_schemas.sh` grava um `docs/schemas/<cmd>.schema.json` por comando do inventário a partir de `schema --cmd` ao vivo (superfície de meta.rs)
- `--check` falha quando schemas estáticos de comando divergem do binário instalado
- Envelopes e `run-script-step.schema.json` são preservados e não sobrescritos pelo gerador
- `audit_bilingual_docs.sh` compara invocações de `browser-automation-cli` dentro de fences de código entre pares EN e `.pt-BR`
- Exit `0` significa multisets de fences iguais; exit `1` significa drift; exit `2` significa par de arquivo ausente


## Logging e Paths Durante Testes
- Logging de produto na CLI sob teste: `--verbose` / `--debug` / `-q` ou XDG `config set log_level`
- Defaults de cor via `config set color`
- Overrides de path do Chrome via `config set chrome_path` quando a descoberta precisar
- Resolva o layout XDG com `config path --json`


## Troubleshooting
- Doctor falha em chrome: instale Chromium ou Google Chrome primeiro, ou defina `config set chrome_path`
- Timeouts no goto smoke: eleve o timeout do processo ou inspecione política de rede
- Falhas de schema gate: atualize código e `docs/schemas/` na mesma mudança
- Drift de schema de comando: reexecute `bash scripts/generate_command_schemas.sh` após mudar `meta.rs`
- Drift bilíngue de fences: reexecute `bash scripts/audit_bilingual_docs.sh` e alinhe blocos de comando EN e `.pt-BR`
- Drift de inventário: reconcilie com `commands --json` (65) e `tests/fixtures/tool-reference.md` (53 tools)
- Leaks residual de disco: reexecute `cargo test --test residual_one_shot` e `bash scripts/residual-check.sh`; inspecione `residual` do doctor
- Drift de inventário run: atualize `RUN_DISPATCHED_CMDS` e reexecute `cargo test --test parity_run_inventory`
- Falhas de clap assert: corrija `GlobalOpts` / definições de subcomando e reexecute `cargo test --test clap_command_debug_assert`
- Script e2e sem binário: rode `cargo build --release --locked` primeiro para existir `target/release/browser-automation-cli`
- Path de lighthouse ausente: passe `--lighthouse-path ./scripts/mock-lighthouse.sh` ou defina XDG `lighthouse_path`
- Extract LLM fail-closed: esperado sem `config set openrouter_api_key`
- Problemas de bind MITM: garanta loopback livre e revise `mitm status --json`
- Confusão de journal de workflow: inspecione `workflow status` e o XDG `workflow_dir` de `config path --json`
