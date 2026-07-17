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
- Testes de robots e comportamento de pipe (`tests/robots_http.rs`, `tests/pipe_broken.rs`)
- Helpers de golden i18n e cold-start (`tests/golden_i18n.rs`, `tests/cold_start.rs`)
- Cobertura e2e opcional de eventos CDP quando Chrome está disponível (`tests/e2e_cdp_events.rs`)
- Script e2e completo das **52 tools** DevTools (ainda): `scripts/e2e_all_52_tools.sh`
- Inventário vivo da CLI é **56 comandos** (`commands --json`) — mais amplo que o conjunto e2e de 52 tool-ref
- Fixture vendored de tool-ref: `tests/fixtures/tool-reference.md`


## Como Rodar
```bash
timeout 300 cargo test --locked
timeout 300 cargo test --lib --locked
timeout 120 cargo clippy --all-targets --locked -- -D warnings
cargo fmt --check
```
- Rode um arquivo com `cargo test --test doctor_cli --locked`
- Use `-- --nocapture` só durante debug
- Prefira library e gates de schema primeiro ao iterar contratos


## E2E 52 Tools
```bash
cargo build --release --locked
bash scripts/e2e_all_52_tools.sh
```
- Exige binário release em `target/release/browser-automation-cli` (faça `cargo build --release --locked` antes)
- Exercita tools de paridade DevTools na página fixture em `scripts/fixtures/e2e_page/`
- Escreve relatório em workdir temp e imprime contagens PASS/FAIL/SKIP
- Evidência do mantenedor para v0.1.2: 52 PASS / 0 FAIL em host local com Chrome
- A suite de 52 tools não substitui smokes residuais de comandos fora do conjunto tool-ref


## Smokes Residuais de PRD (além das 52 tools)
Rode após o e2e ao validar o inventário completo de 56 comandos:

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

# parse PDF / DOCX with optional PII redact
browser-automation-cli --json parse tests/fixtures/hello.pdf
browser-automation-cli --json parse tests/fixtures/hello.docx --redact-pii

# extract --llm fail-closed without XDG key
browser-automation-cli --json extract https://example.com --llm --question 'What is the title?'
# expect usage envelope requiring: config set openrouter_api_key
```
- Também úteis: scrape browser com format, `config path`, `mitm start`, doctor XDG, i18n `--lang pt-BR`


## Mock de Lighthouse
```bash
browser-automation-cli --json lighthouse https://example.com \
  --lighthouse-path ./scripts/mock-lighthouse.sh
```
- Use `--lighthouse-path` ou XDG `lighthouse_path` apontando para `scripts/mock-lighthouse.sh` quando não houver Lighthouse real
- O mock grava reports HTML/JSON mínimos para paths de smoke
- Doctor reporta presença de lighthouse como informativo quando o binário está ausente


## Perfis de Validação Local
- Rode fmt, clippy e testes de contrato sem browser primeiro na sua máquina
- Testes com browser exigem Chrome ou Chromium instalado localmente
- A validação roda localmente com cargo e scripts e2e na máquina do mantenedor
- Mantenha publish no crates.io bloqueado sem aprovação explícita do mantenedor
- Smokes opcionais de pilares após e2e: `run` + scrape, comandos residuais de PRD acima, `config path`, `mitm start`, doctor XDG


## Auditoria de Schemas e Documentação Bilíngue
```bash
cargo build --release --locked
bash scripts/generate_command_schemas.sh
bash scripts/generate_command_schemas.sh --check
bash scripts/audit_bilingual_docs.sh
```
- `generate_command_schemas.sh` grava um `docs/schemas/<cmd>.schema.json` por comando do inventário a partir do `schema --cmd` ao vivo (superfície de meta.rs)
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
- Drift de inventário: reconcilie com `commands --json` (56) e `tests/fixtures/tool-reference.md` (52 tools)
- Script e2e sem binário: rode `cargo build --release --locked` primeiro para existir `target/release/browser-automation-cli`
- Path de lighthouse ausente: passe `--lighthouse-path ./scripts/mock-lighthouse.sh` ou defina XDG `lighthouse_path`
- Extract LLM fail-closed: esperado sem `config set openrouter_api_key`
- Problemas de bind MITM: garanta loopback livre e revise `mitm status --json`
- Confusão de journal de workflow: inspecione `workflow status` e o XDG `workflow_dir` de `config path --json`
