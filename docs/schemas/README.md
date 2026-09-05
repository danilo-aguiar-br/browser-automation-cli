[English](#english) | [Português Brasileiro](#português-brasileiro)

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
- Static snapshots may lag if the binary is older than `src/commands/meta/` — **regenerate when schemas are stale**
- Prefer live `schema <cmd>` when generating argv after upgrades
- Envelope files are hand-maintained and are not overwritten by the generator
- A property may carry `step_key_aliases`: alternative spellings a `run --script` step accepts for that key, beyond the `step_key` itself
- Those aliases are camelCase for agents that emit JSON in the casing of their own language, and they are declared in `STEP_KEY_ALIASES` (`src/commands/run/inventory.rs`), which the step handler and this generator both read
- An alias absent from that table is absent from the code as well: `tests/step_key_alias_gate.rs` fails when a step handler reaches a camelCase key by inline literal
- Inventory size: **71** top-level command names (`commands --json`), including `submit`, `storage`, `locale`, `man`, `select-option`, and `pick`

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
- After adding or renaming inventory commands (for example `print-pdf`, `monitor`, `qr`, `record`, `find-paths`, `sheet-write`, `sg-scan`, `sg-rewrite`, `select-option`, `pick`), re-run the generator
- If a static schema disagrees with live `schema <cmd> --json`, treat the live CLI as authoritative and regenerate

### Envelopes and non-command contracts
- `envelope-success.schema.json` — success stdout envelope, including the optional `agent_ops` report (`total`, `matched`, `truncated`, `omitted_rows`, `unresolved_paths`) emitted only when a universal envelope flag ran
- `envelope-error.schema.json` — error stdout envelope under `--json` (may include partial `data` for fail-fast multi-step)
- `run-script-step.schema.json` — one step for `run --script` (NDJSON line or JSON array element)

### Command input schemas (71 — full inventory)

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
- `wait.schema.json` — `wait` (multi-selector OR; run `url` / `url_contains` / `navigation: true` boolean; public `wait_timeout_ms`; result may include `matched_selector`)
- `dialog.schema.json` — `dialog` (`if_present` / `--if-present` soft path; data may include `dialog_settled` boolean after real answer)

#### Interaction
- `press.schema.json` — `press`
- `click-at.schema.json` — `click-at`
- `write.schema.json` — `write`
- `keys.schema.json` — `keys`
- `type.schema.json` — `type`
- `hover.schema.json` — `hover`
- `drag.schema.json` — `drag`
- `submit.schema.json` — `submit` (form or field target; wait for navigation/request)
- `fill-form.schema.json` — `fill-form`
- `select-option.schema.json` — `select-option` (custom select / badge / popover / role=option; native `<select>` → `input`+`change`; also via `run`/`exec`)
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
- `storage.schema.json` — `storage` (`export|import --path`; cookies + localStorage + sessionStorage)

#### Capture and artifacts
- `grab.schema.json` — `grab` (encode **png|jpeg|webp** only; AVIF removed in v0.1.6)
- `print-pdf.schema.json` — `print-pdf` (also valid as `run` step)
- `monitor.schema.json` — `monitor` (`check`)
- `console.schema.json` — `console` (dump always valid JSON array, including `[]`)
- `net.schema.json` — `net`
- `screencast.schema.json` — `screencast`

#### Multi-step
- `run.schema.json` — `run` (script path; body is NDJSON or JSON array; global `--json-steps`)
- `exec.schema.json` — `exec`

#### Local scrape / crawl / parse surface
- `scrape.schema.json` — `scrape` (multi `--format` / CSV / alias `--formats`; run steps honor `format`/`formats`)
- `batch-scrape.schema.json` — `batch-scrape` (`--engine http|browser`)
- `crawl.schema.json` — `crawl` (`--engine http|browser`)
- `map.schema.json` — `map`
- `sitemap.schema.json` — `sitemap` (discovery verb; delegates to `map --sitemap-only`)
- `feed.schema.json` — `feed` (delegates to `scrape --formats feed --engine http`)
- `search.schema.json` — `search`
- `parse.schema.json` — `parse` (`--redact-pii`; pdf/docx/xlsx/ods)
- `record.schema.json` — `record` (`--url` / `--path`; emits a replayable `run --script` NDJSON file)

#### Local IO helpers (no Chrome)
- `qr.schema.json` — `qr` (`encode` / `decode`)
- `image.schema.json` — `image` (`info` / `convert` / `resize` / `download` / `exif`)
- `video.schema.json` — `video` (`info` / `download` / `convert` / `to-mp3` / `trim` / `thumbnail` / `manifest`)
- `audio.schema.json` — `audio` (`info` / `download` / `convert` / `trim`)
- `find-paths.schema.json` — `find-paths` (`--glob`)
- `sheet-write.schema.json` — `sheet-write`
- `sg-scan.schema.json` — `sg-scan`
- `sg-rewrite.schema.json` — `sg-rewrite`

#### Config, MITM, workflow
- `config.schema.json` — `config` (discover live XDG keys via `config list-keys --json`; includes `dialog_settle_ms`)
- `mitm.schema.json` — `mitm` (includes `capture-url`)
- `workflow.schema.json` — `workflow`

#### Emulation and performance
- `emulate.schema.json` — `emulate`
- `resize.schema.json` — `resize`
- `perf.schema.json` — `perf`
- `lighthouse.schema.json` — `lighthouse` (input; envelope may include `binary_source` real|mock; e2e mock is SKIP)
- `heap.schema.json` — `heap`

#### Category-gated surfaces
- `extension.schema.json` — `extension`
- `devtools3p.schema.json` — `devtools3p`
- `webmcp.schema.json` — `webmcp`

### Live CLI vs static snapshots
- Always treat `schema <cmd> --json` (or `schema --cmd <cmd> --json`) as authoritative for the installed binary
- After upgrading the CLI, re-run `scripts/generate_command_schemas.sh`
- Use `commands --json` to confirm inventory membership after upgrades (**71** commands)
- DevTools e2e suite remains 53 tools (lighthouse mock SKIP); inventory schemas cover the full 71-command surface
- After adding `submit` / `storage`, regenerate so static snapshots exist for those names
- Bilingual fence audit: `bash scripts/audit_bilingual_docs.sh`


## Português Brasileiro

- Este diretório versiona contratos JSON legíveis por máquina para agentes
- Fonte da verdade dos fragments de input por comando: CLI ao vivo  
  `browser-automation-cli schema <cmd> --json`  
  (posicional preferido; também `schema --cmd <cmd> --json`)
- Arquivos estáticos `*.schema.json` de comando são gerados dessa superfície via  
  `bash scripts/generate_command_schemas.sh`
- Modo verificação (sem gravar): `bash scripts/generate_command_schemas.sh --check`
- Snapshots estáticos podem atrasar se o binário for mais antigo que `src/commands/meta/` — **regenere quando os schemas estiverem defasados**
- Prefira `schema <cmd>` ao vivo ao gerar argv após upgrades
- Arquivos de envelope são mantidos à mão e não são sobrescritos pelo gerador
- Uma propriedade pode trazer `step_key_aliases`: grafias alternativas que um passo de `run --script` aceita para aquela chave, além do próprio `step_key`
- Esses aliases são camelCase para agentes que emitem JSON na convenção da própria linguagem, e são declarados em `STEP_KEY_ALIASES` (`src/commands/run/inventory.rs`), lido tanto pelo handler do passo quanto por este gerador
- Alias ausente dessa tabela está ausente do código também: `tests/step_key_alias_gate.rs` reprova quando um handler alcança chave camelCase por literal inline
- Tamanho do inventário: **71** nomes de comando de topo (`commands --json`), incluindo `submit`, `storage`, `locale`, `man`, `select-option` e `pick`

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
- Após adicionar ou renomear comandos do inventário (por exemplo `print-pdf`, `monitor`, `qr`, `find-paths`, `sheet-write`, `sg-scan`, `sg-rewrite`, `select-option`, `pick`, `submit`, `storage`), reexecute o gerador
- Se um schema estático divergir de `schema <cmd> --json` ao vivo, trate a CLI como autoritativa e regenere

### Envelopes e contratos fora de comando
- `envelope-success.schema.json` — envelope de sucesso no stdout, incluindo o relatório opcional `agent_ops` (`total`, `matched`, `truncated`, `omitted_rows`, `unresolved_paths`) emitido apenas quando uma flag universal de envelope rodou
- `envelope-error.schema.json` — envelope de erro no stdout com `--json` (pode incluir `data` parcial em fail-fast multi-passo)
- `run-script-step.schema.json` — um passo para `run --script` (linha NDJSON ou elemento de array JSON)

### Schemas de input de comando (71 — inventário completo)

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
- `wait.schema.json` — `wait` (multi-seletor OR; run `url` / `url_contains` / `navigation: true` boolean; prazo público `wait_timeout_ms`; resultado pode incluir `matched_selector`)
- `dialog.schema.json` — `dialog` (`if_present` / `--if-present` soft path; dados podem incluir booleano `dialog_settled` após resposta real)

#### Interação
- `press.schema.json` — `press`
- `click-at.schema.json` — `click-at`
- `write.schema.json` — `write`
- `keys.schema.json` — `keys`
- `type.schema.json` — `type`
- `hover.schema.json` — `hover`
- `drag.schema.json` — `drag`
- `submit.schema.json` — `submit` (form ou campo; espera navegação/requisição)
- `fill-form.schema.json` — `fill-form`
- `select-option.schema.json` — `select-option` (select custom / badge / popover / role=option; `<select>` nativo → `input`+`change`; também via `run`/`exec`)
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
- `storage.schema.json` — `storage` (`export|import --path`; cookies + localStorage + sessionStorage)

#### Captura e artefatos
- `grab.schema.json` — `grab` (encode só **png|jpeg|webp**; AVIF removido na v0.1.6)
- `print-pdf.schema.json` — `print-pdf` (também válido como passo de `run`)
- `monitor.schema.json` — `monitor` (`check`)
- `console.schema.json` — `console` (dump sempre array JSON válido, inclusive `[]`)
- `net.schema.json` — `net`
- `screencast.schema.json` — `screencast`

#### Multi-passo
- `run.schema.json` — `run` (path do script; body NDJSON ou array JSON; global `--json-steps`)
- `exec.schema.json` — `exec`

#### Superfície local de scrape / crawl / parse
- `scrape.schema.json` — `scrape` (multi `--format` / CSV / alias `--formats`; passos run honram `format`/`formats`)
- `batch-scrape.schema.json` — `batch-scrape` (`--engine http|browser`)
- `crawl.schema.json` — `crawl` (`--engine http|browser`)
- `map.schema.json` — `map`
- `sitemap.schema.json` — `sitemap` (verbo de descoberta; delega para `map --sitemap-only`)
- `feed.schema.json` — `feed` (delega para `scrape --formats feed --engine http`)
- `search.schema.json` — `search`
- `parse.schema.json` — `parse` (`--redact-pii`; pdf/docx/xlsx/ods)
- `record.schema.json` — `record` (`--url` / `--path`; gera um arquivo NDJSON reexecutável por `run --script`)

#### Helpers de IO local (sem Chrome)
- `qr.schema.json` — `qr` (`encode` / `decode`)
- `image.schema.json` — `image` (`info` / `convert` / `resize` / `download` / `exif`)
- `video.schema.json` — `video` (`info` / `download` / `convert` / `to-mp3` / `trim` / `thumbnail` / `manifest`)
- `audio.schema.json` — `audio` (`info` / `download` / `convert` / `trim`)
- `find-paths.schema.json` — `find-paths` (`--glob`)
- `sheet-write.schema.json` — `sheet-write`
- `sg-scan.schema.json` — `sg-scan`
- `sg-rewrite.schema.json` — `sg-rewrite`

#### Config, MITM, workflow
- `config.schema.json` — `config` (descubra chaves XDG vivas via `config list-keys --json`; inclui `dialog_settle_ms`)
- `mitm.schema.json` — `mitm` (inclui `capture-url`)
- `workflow.schema.json` — `workflow`

#### Emulação e performance
- `emulate.schema.json` — `emulate`
- `resize.schema.json` — `resize`
- `perf.schema.json` — `perf`
- `lighthouse.schema.json` — `lighthouse` (input; o envelope pode incluir `binary_source` real|mock; e2e mock é SKIP)
- `heap.schema.json` — `heap`

#### Superfícies com gate de categoria
- `extension.schema.json` — `extension`
- `devtools3p.schema.json` — `devtools3p`
- `webmcp.schema.json` — `webmcp`

### CLI ao vivo vs snapshots estáticos
- Trate sempre `schema <cmd> --json` (ou `schema --cmd <cmd> --json`) como autoritativo para o binário instalado
- Após atualizar a CLI, reexecute `scripts/generate_command_schemas.sh`
- Use `commands --json` para confirmar inventário após upgrades (**71** comandos)
- A suite e2e DevTools permanece com 53 tools (lighthouse mock SKIP); os schemas de inventário cobrem a superfície completa de 71 comandos
- Após adicionar `submit` / `storage`, regenere para que existam snapshots estáticos desses nomes
- Auditoria bilíngue de fences: `bash scripts/audit_bilingual_docs.sh`


### Nota entre idiomas / Cross-language note
- English and Português Brasileiro live in this same `README.md` (no `README.pt-BR.md` here)
- Inglês e Português Brasileiro ficam neste mesmo `README.md` (sem `README.pt-BR.md` neste diretório)
- Schema file names remain English kebab-case for tooling stability
- Nomes dos arquivos de schema permanecem em inglês kebab-case para estabilidade de tooling
