[English](README.md) | [Português Brasileiro](README.md)

# JSON Schemas — browser-automation-cli

This is a single bilingual file (English and Português Brasileiro sections below). There is no separate `README.pt-BR.md` in this directory.

Cross-language note: keep both language sections in this file when editing. Schema filenames stay kebab-case English matching CLI subcommands.


## English

- This directory versions machine-readable JSON contracts for agents
- Source of truth for per-command input fragments: live CLI  
  `browser-automation-cli schema <cmd> --json`  
  (positional preferred; also `schema --cmd <cmd> --json`)
- Static `*.schema.json` command files are generated from that live surface via  
  `bash scripts/generate_command_schemas.sh`
- Check mode (no write): `bash scripts/generate_command_schemas.sh --check`
- Static snapshots may lag if the binary is older than `src/commands_prd/meta.rs` — **regenerate when schemas are stale**
- Prefer live `schema <cmd>` when generating argv after upgrades
- Envelope files are hand-maintained and are not overwritten by the generator
- Inventory size: **63** top-level command names (`commands --json`), including `locale`, `man`, `select-option`, and `pick`

### How to regenerate

```bash
cargo build --release --locked
bash scripts/generate_command_schemas.sh
bash scripts/generate_command_schemas.sh --check
```

### Generator notes
- Generator reads the live inventory from `commands --json` / `schema --cmd` / `schema <cmd>`
- Writes one `docs/schemas/<cmd>.schema.json` per inventory command
- Does **not** overwrite `envelope-success.schema.json`, `envelope-error.schema.json`, or `run-script-step.schema.json`
- After adding or renaming inventory commands (for example `print-pdf`, `monitor`, `qr`, `find-paths`, `sheet-write`, `sg-scan`, `sg-rewrite`, `select-option`, `pick`), re-run the generator
- If a static schema disagrees with live `schema <cmd> --json`, treat the live CLI as authoritative and regenerate

### Envelopes and non-command contracts
- `envelope-success.schema.json` — success stdout envelope
- `envelope-error.schema.json` — error stdout envelope under `--json` (may include partial `data` for fail-fast multi-step)
- `run-script-step.schema.json` — one step for `run --script` (NDJSON line or JSON array element)

### Command input schemas (63 — full inventory)

#### Meta and discovery
- `doctor.schema.json` — `doctor` (envelope may include top-level `residual` / check `residual_disk`)
- `commands.schema.json` — `commands`
- `schema.schema.json` — `schema` (positional `<cmd>` or `--cmd`)
- `version.schema.json` — `version`
- `locale.schema.json` — `locale` (UI locale diagnostics; human suggestions only)
- `completions.schema.json` — `completions`
- `man.schema.json` — `man` (clap_mangen roff; optional `--out`)

#### Navigation and page state
- `goto.schema.json` — `goto` (`handle_before_unload` / `--handle-before-unload accept|dismiss`; GAP-003)
- `back.schema.json` — `back`
- `forward.schema.json` — `forward`
- `reload.schema.json` — `reload` (`--ignore-cache`; `handle_before_unload`)
- `view.schema.json` — `view` (`--allow-empty`; GAP-012)
- `page.schema.json` — `page` (`isolated_context` string or true on new; flag alone → `default-isolated`; GAP-004)
- `wait.schema.json` — `wait` (multi-selector OR; run `url` / `url_contains` / `navigation: true` boolean; result may include `matched_selector`)
- `dialog.schema.json` — `dialog` (`if_present` / `--if-present` soft path)

#### Interaction
- `press.schema.json` — `press`
- `click-at.schema.json` — `click-at`
- `write.schema.json` — `write`
- `keys.schema.json` — `keys`
- `type.schema.json` — `type`
- `hover.schema.json` — `hover`
- `drag.schema.json` — `drag`
- `fill-form.schema.json` — `fill-form`
- `select-option.schema.json` — `select-option` (custom select / badge / popover / role=option; also via `run`/`exec`)
- `pick.schema.json` — `pick` (alias surface of select-option)
- `upload.schema.json` — `upload`
- `scroll.schema.json` — `scroll` (`dy`/`dx` aliases)

#### Extract and assert
- `eval.schema.json` — `eval`
- `extract.schema.json` — `extract` (includes `--llm` / `--question` / XDG LLM keys)
- `text.schema.json` — `text`
- `attr.schema.json` — `attr`
- `assert.schema.json` — `assert` (`url` / `text` / `console` / `console-empty` / `console-no-match`; run `kind` dual surface)
- `cookie.schema.json` — `cookie`

#### Capture and artifacts
- `grab.schema.json` — `grab`
- `print-pdf.schema.json` — `print-pdf` (also valid as `run` step)
- `monitor.schema.json` — `monitor` (`check`)
- `console.schema.json` — `console` (dump always valid JSON array, including `[]`)
- `net.schema.json` — `net`
- `screencast.schema.json` — `screencast`

#### Multi-step
- `run.schema.json` — `run` (script path; body is NDJSON or JSON array; global `--json-steps`)
- `exec.schema.json` — `exec`

#### Local scrape / crawl / parse surface
- `scrape.schema.json` — `scrape` (multi `--format` / CSV / alias `--formats`)
- `batch-scrape.schema.json` — `batch-scrape` (`--engine http|browser`)
- `crawl.schema.json` — `crawl` (`--engine http|browser`)
- `map.schema.json` — `map`
- `search.schema.json` — `search`
- `parse.schema.json` — `parse` (`--redact-pii`; pdf/docx/xlsx/ods)

#### Local IO helpers (no Chrome)
- `qr.schema.json` — `qr` (`encode` / `decode`)
- `find-paths.schema.json` — `find-paths` (`--glob`)
- `sheet-write.schema.json` — `sheet-write`
- `sg-scan.schema.json` — `sg-scan`
- `sg-rewrite.schema.json` — `sg-rewrite`

#### Config, MITM, workflow
- `config.schema.json` — `config` (16 XDG keys)
- `mitm.schema.json` — `mitm` (includes `capture-url`)
- `workflow.schema.json` — `workflow`

#### Emulation and performance
- `emulate.schema.json` — `emulate`
- `resize.schema.json` — `resize`
- `perf.schema.json` — `perf`
- `lighthouse.schema.json` — `lighthouse` (input; envelope may include `binary_source` real|mock)
- `heap.schema.json` — `heap`

#### Category-gated surfaces
- `extension.schema.json` — `extension`
- `devtools3p.schema.json` — `devtools3p`
- `webmcp.schema.json` — `webmcp`

### Live CLI vs static snapshots
- Always treat `schema <cmd> --json` (or `schema --cmd <cmd> --json`) as authoritative for the installed binary
- After upgrading the CLI, re-run `scripts/generate_command_schemas.sh`
- Use `commands --json` to confirm inventory membership after upgrades (**63** commands)
- DevTools e2e suite remains 53 tools; inventory schemas cover the full 63-command surface
- Bilingual fence audit: `bash scripts/audit_bilingual_docs.sh`


## Português Brasileiro

- Este diretório versiona contratos JSON legíveis por máquina para agentes
- Fonte da verdade dos fragments de input por comando: CLI ao vivo  
  `browser-automation-cli schema <cmd> --json`  
  (posicional preferido; também `schema --cmd <cmd> --json`)
- Arquivos estáticos `*.schema.json` de comando são gerados dessa superfície via  
  `bash scripts/generate_command_schemas.sh`
- Modo verificação (sem gravar): `bash scripts/generate_command_schemas.sh --check`
- Snapshots estáticos podem atrasar se o binário for mais antigo que `src/commands_prd/meta.rs` — **regenere quando os schemas estiverem defasados**
- Prefira `schema <cmd>` ao vivo ao gerar argv após upgrades
- Arquivos de envelope são mantidos à mão e não são sobrescritos pelo gerador
- Tamanho do inventário: **63** nomes de comando de topo (`commands --json`), incluindo `locale`, `man`, `select-option` e `pick`

### Como regenerar

```bash
cargo build --release --locked
bash scripts/generate_command_schemas.sh
bash scripts/generate_command_schemas.sh --check
```

### Notas do gerador
- O gerador lê o inventário vivo de `commands --json` / `schema --cmd` / `schema <cmd>`
- Grava um `docs/schemas/<cmd>.schema.json` por comando do inventário
- **Não** sobrescreve `envelope-success.schema.json`, `envelope-error.schema.json` ou `run-script-step.schema.json`
- Após adicionar ou renomear comandos do inventário (por exemplo `print-pdf`, `monitor`, `qr`, `find-paths`, `sheet-write`, `sg-scan`, `sg-rewrite`, `select-option`, `pick`), reexecute o gerador
- Se um schema estático divergir de `schema <cmd> --json` ao vivo, trate a CLI como autoritativa e regenere

### Envelopes e contratos fora de comando
- `envelope-success.schema.json` — envelope de sucesso no stdout
- `envelope-error.schema.json` — envelope de erro no stdout com `--json` (pode incluir `data` parcial em fail-fast multi-passo)
- `run-script-step.schema.json` — um passo para `run --script` (linha NDJSON ou elemento de array JSON)

### Schemas de input de comando (63 — inventário completo)

#### Meta e descoberta
- `doctor.schema.json` — `doctor` (envelope pode incluir `residual` de topo / check `residual_disk`)
- `commands.schema.json` — `commands`
- `schema.schema.json` — `schema` (posicional `<cmd>` ou `--cmd`)
- `version.schema.json` — `version`
- `locale.schema.json` — `locale` (diagnósticos de locale de UI; só sugestões humanas)
- `completions.schema.json` — `completions`
- `man.schema.json` — `man` (clap_mangen roff; `--out` opcional)

#### Navegação e estado de página
- `goto.schema.json` — `goto` (`handle_before_unload` / `--handle-before-unload accept|dismiss`; GAP-003)
- `back.schema.json` — `back`
- `forward.schema.json` — `forward`
- `reload.schema.json` — `reload` (`--ignore-cache`; `handle_before_unload`)
- `view.schema.json` — `view` (`--allow-empty`; GAP-012)
- `page.schema.json` — `page` (`isolated_context` string ou true em new; flag sozinha → `default-isolated`; GAP-004)
- `wait.schema.json` — `wait` (multi-seletor OR; run `url` / `url_contains` / `navigation: true` boolean; resultado pode incluir `matched_selector`)
- `dialog.schema.json` — `dialog` (`if_present` / `--if-present` soft path)

#### Interação
- `press.schema.json` — `press`
- `click-at.schema.json` — `click-at`
- `write.schema.json` — `write`
- `keys.schema.json` — `keys`
- `type.schema.json` — `type`
- `hover.schema.json` — `hover`
- `drag.schema.json` — `drag`
- `fill-form.schema.json` — `fill-form`
- `select-option.schema.json` — `select-option` (select custom / badge / popover / role=option; também via `run`/`exec`)
- `pick.schema.json` — `pick` (superfície alias de select-option)
- `upload.schema.json` — `upload`
- `scroll.schema.json` — `scroll` (aliases `dy`/`dx`)

#### Extração e assert
- `eval.schema.json` — `eval`
- `extract.schema.json` — `extract` (inclui `--llm` / `--question` / chaves LLM XDG)
- `text.schema.json` — `text`
- `attr.schema.json` — `attr`
- `assert.schema.json` — `assert` (`url` / `text` / `console` / `console-empty` / `console-no-match`; superfície dual em run `kind`)
- `cookie.schema.json` — `cookie`

#### Captura e artefatos
- `grab.schema.json` — `grab`
- `print-pdf.schema.json` — `print-pdf` (também válido como passo de `run`)
- `monitor.schema.json` — `monitor` (`check`)
- `console.schema.json` — `console` (dump sempre array JSON válido, inclusive `[]`)
- `net.schema.json` — `net`
- `screencast.schema.json` — `screencast`

#### Multi-passo
- `run.schema.json` — `run` (path do script; body NDJSON ou array JSON; global `--json-steps`)
- `exec.schema.json` — `exec`

#### Superfície local de scrape / crawl / parse
- `scrape.schema.json` — `scrape` (multi `--format` / CSV / alias `--formats`)
- `batch-scrape.schema.json` — `batch-scrape` (`--engine http|browser`)
- `crawl.schema.json` — `crawl` (`--engine http|browser`)
- `map.schema.json` — `map`
- `search.schema.json` — `search`
- `parse.schema.json` — `parse` (`--redact-pii`; pdf/docx/xlsx/ods)

#### Helpers de IO local (sem Chrome)
- `qr.schema.json` — `qr` (`encode` / `decode`)
- `find-paths.schema.json` — `find-paths` (`--glob`)
- `sheet-write.schema.json` — `sheet-write`
- `sg-scan.schema.json` — `sg-scan`
- `sg-rewrite.schema.json` — `sg-rewrite`

#### Config, MITM, workflow
- `config.schema.json` — `config` (16 chaves XDG)
- `mitm.schema.json` — `mitm` (inclui `capture-url`)
- `workflow.schema.json` — `workflow`

#### Emulação e performance
- `emulate.schema.json` — `emulate`
- `resize.schema.json` — `resize`
- `perf.schema.json` — `perf`
- `lighthouse.schema.json` — `lighthouse` (input; o envelope pode incluir `binary_source` real|mock)
- `heap.schema.json` — `heap`

#### Superfícies com gate de categoria
- `extension.schema.json` — `extension`
- `devtools3p.schema.json` — `devtools3p`
- `webmcp.schema.json` — `webmcp`

### CLI ao vivo vs snapshots estáticos
- Trate sempre `schema <cmd> --json` (ou `schema --cmd <cmd> --json`) como autoritativo para o binário instalado
- Após atualizar a CLI, reexecute `scripts/generate_command_schemas.sh`
- Use `commands --json` para confirmar inventário após upgrades (**63** comandos)
- A suite e2e DevTools permanece com 53 tools; os schemas de inventário cobrem a superfície completa de 63 comandos
- Auditoria bilíngue de fences: `bash scripts/audit_bilingual_docs.sh`


### Nota entre idiomas / Cross-language note
- English and Português Brasileiro live in this same `README.md` (no `README.pt-BR.md` here)
- Inglês e Português Brasileiro ficam neste mesmo `README.md` (sem `README.pt-BR.md` neste diretório)
- Schema file names remain English kebab-case for tooling stability
- Nomes dos arquivos de schema permanecem em inglês kebab-case para estabilidade de tooling
