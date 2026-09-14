[English](#english) | [Português Brasileiro](#português-brasileiro)

# JSON Schemas — browser-automation-cli


## English
- This directory versions machine-readable JSON contracts for agents
- The source of truth for per-command input fragments is the live CLI with `browser-automation-cli schema <cmd> --json`
- The positional form is preferred, and `schema --cmd <cmd> --json` answers the same contract
- The static command files `*.schema.json` are generated from that live surface by `bash scripts/generate_command_schemas.sh`
- Check mode writes nothing and runs as `bash scripts/generate_command_schemas.sh --check`
- Static snapshots lag when the binary is older than `src/commands/meta/`, so regenerate them when a schema is stale
- Prefer the live `schema <cmd>` when composing argv after an upgrade
- Envelope files are hand-maintained and the generator never overwrites them
- A property may carry `step_key_aliases`, the alternative spellings a `run --script` step accepts for that key beyond the `step_key` itself
- Those aliases are camelCase for agents that emit JSON in the casing of their own language
- They are declared in `STEP_KEY_ALIASES` (`src/commands/run/inventory.rs`), which the step handler and this generator both read
- An alias absent from that table is absent from the code as well, because `tests/step_key_alias_gate.rs` fails when a step handler reaches a camelCase key by inline literal
- The directory holds 74 files, which are 71 command input schemas plus `envelope-success.schema.json`, `envelope-error.schema.json` and `run-script-step.schema.json`
- The inventory holds 71 top-level command names in `commands --json`, including `submit`, `storage`, `locale`, `man`, `select-option` and `pick`

### Cross-platform drift check
- `cargo test --test schema_input_drift_gate` compares every `docs/schemas/<cmd>.schema.json` with the live `schema --cmd <cmd>` and needs no `bash`
- The gate compares only `properties` and `required`, because `schema --cmd` also answers `output_schema`, `surfaces` and `error_schema`
- The gate exempts `envelope-error`, `envelope-success` and `run-script-step` by name, so a new file with no live command fails instead of being skipped
- The gate reads the directory instead of a frozen list, so a new command schema is checked with no edit to the gate
- Use this gate to detect drift on Linux, macOS and Windows alike
- Use the generator below only to rewrite the files, and only on a host that has `bash`

### How to regenerate
- The generator is a `bash` script, so it does not run on a host without `bash`
- The generator reads the newer of `target/release/browser-automation-cli` and `target/debug/browser-automation-cli`, and refuses a binary older than the sources

```bash
cargo build --release --locked
bash scripts/generate_command_schemas.sh
bash scripts/generate_command_schemas.sh --check
```

### Generator notes
- The generator reads the live inventory from `commands --json`, `schema --cmd` and `schema <cmd>`
- It writes one `docs/schemas/<cmd>.schema.json` per inventory command
- It does NOT overwrite `envelope-success.schema.json`, `envelope-error.schema.json` or `run-script-step.schema.json`
- Re-run the generator after adding or renaming inventory commands such as `print-pdf`, `monitor`, `qr`, `record`, `find-paths`, `sheet-write`, `sg-scan`, `sg-rewrite`, `select-option`, `pick`, `submit` and `storage`
- If a static schema disagrees with the live `schema <cmd> --json`, treat the live CLI as authoritative and regenerate

### Witness fields live in the envelope and not in these schemas
- Every browser envelope publishes five witness fields inside `data`, which are `browser_mode_requested`, `browser_mode_effective`, `browser_mode_source`, `display_backend` and `runtime_enable_used`
- `display_backend` takes `headless`, `xvfb` or `host`
- Since `0.2.0`, `display_backend` reports the display the launch actually used, and no longer the display it intended
- `run` strips the five fields from each step and publishes one copy at the top of its envelope
- No command input schema in this directory lists these fields, because they describe what a run did and never its argv
- `envelope-success.schema.json` does not list them, and the live `output_schema` of `schema goto` does not list them either
- Read the full contract in `docs/AGENTS.md`, section `Browser Witness Fields on Every Envelope`

### Envelopes and non-command contracts
- `envelope-success.schema.json` — success stdout envelope, including the optional `agent_ops` report (`total`, `matched`, `truncated`, `omitted_rows`, `unresolved_paths`) emitted only when a universal envelope flag ran
- `envelope-error.schema.json` — error stdout envelope under `--json`, which may include partial `data` for fail-fast multi-step
- `run-script-step.schema.json` — one step for `run --script`, as an NDJSON line or a JSON array element

### Command input schemas (71 — full inventory)
- The 71 command input schemas are listed below in groups by area, one file per command

### Command input schemas — Meta and discovery
- `doctor.schema.json` — `doctor` (envelope may include top-level `residual` / check `residual_disk`)
- `commands.schema.json` — `commands`
- `schema.schema.json` — `schema` (positional `<cmd>` or `--cmd`)
- `version.schema.json` — `version`
- `locale.schema.json` — `locale` (UI locale diagnostics; human suggestions only)
- `completions.schema.json` — `completions`
- `man.schema.json` — `man` (clap_mangen roff; optional `--out`)

### Command input schemas — Navigation and page state
- `goto.schema.json` — `goto` (`handle_before_unload` / `--handle-before-unload accept|dismiss`; GAP-003)
- `back.schema.json` — `back`
- `forward.schema.json` — `forward`
- `reload.schema.json` — `reload` (`--ignore-cache`; `handle_before_unload`)
- `view.schema.json` — `view` (`--allow-empty`; GAP-012)
- `page.schema.json` — `page` (`isolated_context` string or true on new; flag alone → `default-isolated`; GAP-004)
- `wait.schema.json` — `wait` (multi-selector OR; run `url` / `url_contains` / `navigation: true` boolean; public `wait_timeout_ms`; result may include `matched_selector`)
- `dialog.schema.json` — `dialog` (`if_present` / `--if-present` soft path; data may include `dialog_settled` boolean after real answer)

### Command input schemas — Interaction
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

### Command input schemas — Extract and assert
- `eval.schema.json` — `eval`
- `extract.schema.json` — `extract` (includes `--llm` / `--question` / XDG LLM keys)
- `text.schema.json` — `text`
- `attr.schema.json` — `attr`
- `assert.schema.json` — `assert` (`url` / `text` / `console` / `console-empty` / `console-no-match`; run `kind` dual surface)
- `cookie.schema.json` — `cookie`
- `storage.schema.json` — `storage` (`export|import --path`; cookies + localStorage + sessionStorage)

### Command input schemas — Capture and artifacts
- `grab.schema.json` — `grab` (encode png|jpeg|webp only; AVIF removed in v0.1.6)
- `print-pdf.schema.json` — `print-pdf` (also valid as `run` step)
- `monitor.schema.json` — `monitor` (`check`)
- `console.schema.json` — `console` (dump always valid JSON array, including `[]`)
- `net.schema.json` — `net`
- `screencast.schema.json` — `screencast`

### Command input schemas — Multi-step
- `run.schema.json` — `run` (script path; body is NDJSON or JSON array; global `--json-steps`)
- `exec.schema.json` — `exec`

### Command input schemas — Local scrape, crawl and parse surface
- `scrape.schema.json` — `scrape` (multi `--format` / CSV / alias `--formats`; run steps honor `format`/`formats`)
- `batch-scrape.schema.json` — `batch-scrape` (`--engine http|browser`)
- `crawl.schema.json` — `crawl` (`--engine http|browser`)
- `map.schema.json` — `map`
- `sitemap.schema.json` — `sitemap` (discovery verb; delegates to `map --sitemap-only`)
- `feed.schema.json` — `feed` (delegates to `scrape --formats feed --engine http`)
- `search.schema.json` — `search`
- `parse.schema.json` — `parse` (`--redact-pii`; pdf/docx/xlsx/ods)
- `record.schema.json` — `record` (`--url` / `--path`; emits a replayable `run --script` NDJSON file)

### Command input schemas — Local IO helpers with no Chrome
- `qr.schema.json` — `qr` (`encode` / `decode`)
- `image.schema.json` — `image` (`info` / `convert` / `resize` / `download` / `exif`)
- `video.schema.json` — `video` (`info` / `download` / `convert` / `to-mp3` / `trim` / `thumbnail` / `manifest`)
- `audio.schema.json` — `audio` (`info` / `download` / `convert` / `trim`)
- `find-paths.schema.json` — `find-paths` (`--glob`)
- `sheet-write.schema.json` — `sheet-write`
- `sg-scan.schema.json` — `sg-scan`
- `sg-rewrite.schema.json` — `sg-rewrite`

### Command input schemas — Config, MITM and workflow
- `config.schema.json` — `config` (discover live XDG keys via `config list-keys --json`; includes `dialog_settle_ms`)
- `mitm.schema.json` — `mitm` (includes `capture-url`)
- `workflow.schema.json` — `workflow`

### Command input schemas — Emulation and performance
- `emulate.schema.json` — `emulate`
- `resize.schema.json` — `resize`
- `perf.schema.json` — `perf`
- `lighthouse.schema.json` — `lighthouse` (input; envelope may include `binary_source` real|mock; e2e mock is SKIP)
- `heap.schema.json` — `heap`

### Command input schemas — Category-gated surfaces
- `extension.schema.json` — `extension`
- `devtools3p.schema.json` — `devtools3p`
- `webmcp.schema.json` — `webmcp`

### Live CLI vs static snapshots
- Always treat `schema <cmd> --json` or `schema --cmd <cmd> --json` as authoritative for the installed binary
- After upgrading the CLI, run `cargo test --test schema_input_drift_gate` to detect drift, then re-run `scripts/generate_command_schemas.sh` on a host with `bash` to rewrite the files
- Use `commands --json` to confirm inventory membership after upgrades, which lists 71 commands
- The DevTools e2e suite remains 53 tools with the lighthouse mock as SKIP, while the inventory schemas cover the full 71-command surface
- `submit.schema.json` and `storage.schema.json` already exist in this directory
- The bilingual fence audit is the `bash` script `bash scripts/audit_bilingual_docs.sh`


## Português Brasileiro
- Este diretório versiona contratos JSON legíveis por máquina para agentes
- A fonte da verdade dos fragments de input por comando é a CLI ao vivo com `browser-automation-cli schema <cmd> --json`
- A forma posicional é a preferida, e `schema --cmd <cmd> --json` responde o mesmo contrato
- Os arquivos estáticos de comando `*.schema.json` são gerados dessa superfície por `bash scripts/generate_command_schemas.sh`
- O modo verificação não grava nada e roda como `bash scripts/generate_command_schemas.sh --check`
- Snapshots estáticos atrasam quando o binário é mais antigo que `src/commands/meta/`, então regenere quando um schema estiver defasado
- Prefira `schema <cmd>` ao vivo ao montar argv depois de um upgrade
- Os arquivos de envelope são mantidos à mão e o gerador nunca os sobrescreve
- Uma propriedade pode trazer `step_key_aliases`, as grafias alternativas que um passo de `run --script` aceita para aquela chave além do próprio `step_key`
- Esses aliases são camelCase para agentes que emitem JSON na convenção da própria linguagem
- Eles são declarados em `STEP_KEY_ALIASES` (`src/commands/run/inventory.rs`), lido tanto pelo handler do passo quanto por este gerador
- Alias ausente dessa tabela está ausente do código também, porque `tests/step_key_alias_gate.rs` reprova quando um handler alcança chave camelCase por literal inline
- O diretório tem 74 arquivos, que são 71 schemas de input de comando mais `envelope-success.schema.json`, `envelope-error.schema.json` e `run-script-step.schema.json`
- O inventário tem 71 nomes de comando de topo em `commands --json`, incluindo `submit`, `storage`, `locale`, `man`, `select-option` e `pick`

### Verificação de drift multiplataforma
- `cargo test --test schema_input_drift_gate` compara todo `docs/schemas/<cmd>.schema.json` com o `schema --cmd <cmd>` ao vivo e não precisa de `bash`
- O gate compara só `properties` e `required`, porque `schema --cmd` também responde `output_schema`, `surfaces` e `error_schema`
- O gate isenta `envelope-error`, `envelope-success` e `run-script-step` pelo nome, então um arquivo novo sem comando vivo reprova em vez de ser pulado
- O gate lê o diretório em vez de uma lista congelada, então um schema de comando novo é verificado sem editar o gate
- Use este gate para detectar drift igualmente em Linux, macOS e Windows
- Use o gerador abaixo só para reescrever os arquivos, e só num host que tenha `bash`

### Como regenerar
- O gerador é um script `bash`, então ele não roda num host sem `bash`
- O gerador lê o mais novo entre `target/release/browser-automation-cli` e `target/debug/browser-automation-cli`, e recusa binário mais antigo que o código-fonte

```bash
cargo build --release --locked
bash scripts/generate_command_schemas.sh
bash scripts/generate_command_schemas.sh --check
```

### Notas do gerador
- O gerador lê o inventário vivo de `commands --json`, `schema --cmd` e `schema <cmd>`
- Ele grava um `docs/schemas/<cmd>.schema.json` por comando do inventário
- Ele NÃO sobrescreve `envelope-success.schema.json`, `envelope-error.schema.json` nem `run-script-step.schema.json`
- Reexecute o gerador depois de adicionar ou renomear comandos do inventário como `print-pdf`, `monitor`, `qr`, `record`, `find-paths`, `sheet-write`, `sg-scan`, `sg-rewrite`, `select-option`, `pick`, `submit` e `storage`
- Se um schema estático divergir de `schema <cmd> --json` ao vivo, trate a CLI como autoritativa e regenere

### Campos de testemunho ficam no envelope e não nestes schemas
- Todo envelope de browser publica cinco campos de testemunho dentro de `data`, que são `browser_mode_requested`, `browser_mode_effective`, `browser_mode_source`, `display_backend` e `runtime_enable_used`
- `display_backend` vale `headless`, `xvfb` ou `host`
- Desde a `0.2.0`, `display_backend` informa o display que o launch realmente usou, e não mais o display que ele pretendia usar
- `run` remove os cinco campos de cada passo e publica uma cópia única no topo do próprio envelope
- Nenhum schema de input de comando deste diretório lista esses campos, porque eles descrevem o que uma execução fez e nunca o argv dela
- `envelope-success.schema.json` não os lista, e o `output_schema` ao vivo de `schema goto` também não
- Leia o contrato completo em `docs/AGENTS.pt-BR.md`, seção `Campos de Testemunho do Browser em Todo Envelope`

### Envelopes e contratos fora de comando
- `envelope-success.schema.json` — envelope de sucesso no stdout, incluindo o relatório opcional `agent_ops` (`total`, `matched`, `truncated`, `omitted_rows`, `unresolved_paths`) emitido apenas quando uma flag universal de envelope rodou
- `envelope-error.schema.json` — envelope de erro no stdout com `--json`, que pode incluir `data` parcial em fail-fast multi-passo
- `run-script-step.schema.json` — um passo para `run --script`, como linha NDJSON ou elemento de array JSON

### Schemas de input de comando (71 — inventário completo)
- Os 71 schemas de input de comando estão listados abaixo em grupos por área, um arquivo por comando

### Schemas de input de comando — Meta e descoberta
- `doctor.schema.json` — `doctor` (envelope pode incluir `residual` de topo / check `residual_disk`)
- `commands.schema.json` — `commands`
- `schema.schema.json` — `schema` (posicional `<cmd>` ou `--cmd`)
- `version.schema.json` — `version`
- `locale.schema.json` — `locale` (diagnósticos de locale de UI; só sugestões humanas)
- `completions.schema.json` — `completions`
- `man.schema.json` — `man` (clap_mangen roff; `--out` opcional)

### Schemas de input de comando — Navegação e estado de página
- `goto.schema.json` — `goto` (`handle_before_unload` / `--handle-before-unload accept|dismiss`; GAP-003)
- `back.schema.json` — `back`
- `forward.schema.json` — `forward`
- `reload.schema.json` — `reload` (`--ignore-cache`; `handle_before_unload`)
- `view.schema.json` — `view` (`--allow-empty`; GAP-012)
- `page.schema.json` — `page` (`isolated_context` string ou true em new; flag sozinha → `default-isolated`; GAP-004)
- `wait.schema.json` — `wait` (multi-seletor OR; run `url` / `url_contains` / `navigation: true` boolean; prazo público `wait_timeout_ms`; resultado pode incluir `matched_selector`)
- `dialog.schema.json` — `dialog` (`if_present` / `--if-present` soft path; dados podem incluir booleano `dialog_settled` após resposta real)

### Schemas de input de comando — Interação
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

### Schemas de input de comando — Extração e assert
- `eval.schema.json` — `eval`
- `extract.schema.json` — `extract` (inclui `--llm` / `--question` / chaves LLM XDG)
- `text.schema.json` — `text`
- `attr.schema.json` — `attr`
- `assert.schema.json` — `assert` (`url` / `text` / `console` / `console-empty` / `console-no-match`; superfície dual em run `kind`)
- `cookie.schema.json` — `cookie`
- `storage.schema.json` — `storage` (`export|import --path`; cookies + localStorage + sessionStorage)

### Schemas de input de comando — Captura e artefatos
- `grab.schema.json` — `grab` (encode só png|jpeg|webp; AVIF removido na v0.1.6)
- `print-pdf.schema.json` — `print-pdf` (também válido como passo de `run`)
- `monitor.schema.json` — `monitor` (`check`)
- `console.schema.json` — `console` (dump sempre array JSON válido, inclusive `[]`)
- `net.schema.json` — `net`
- `screencast.schema.json` — `screencast`

### Schemas de input de comando — Multi-passo
- `run.schema.json` — `run` (path do script; body NDJSON ou array JSON; global `--json-steps`)
- `exec.schema.json` — `exec`

### Schemas de input de comando — Superfície local de scrape, crawl e parse
- `scrape.schema.json` — `scrape` (multi `--format` / CSV / alias `--formats`; passos run honram `format`/`formats`)
- `batch-scrape.schema.json` — `batch-scrape` (`--engine http|browser`)
- `crawl.schema.json` — `crawl` (`--engine http|browser`)
- `map.schema.json` — `map`
- `sitemap.schema.json` — `sitemap` (verbo de descoberta; delega para `map --sitemap-only`)
- `feed.schema.json` — `feed` (delega para `scrape --formats feed --engine http`)
- `search.schema.json` — `search`
- `parse.schema.json` — `parse` (`--redact-pii`; pdf/docx/xlsx/ods)
- `record.schema.json` — `record` (`--url` / `--path`; gera um arquivo NDJSON reexecutável por `run --script`)

### Schemas de input de comando — Helpers de IO local sem Chrome
- `qr.schema.json` — `qr` (`encode` / `decode`)
- `image.schema.json` — `image` (`info` / `convert` / `resize` / `download` / `exif`)
- `video.schema.json` — `video` (`info` / `download` / `convert` / `to-mp3` / `trim` / `thumbnail` / `manifest`)
- `audio.schema.json` — `audio` (`info` / `download` / `convert` / `trim`)
- `find-paths.schema.json` — `find-paths` (`--glob`)
- `sheet-write.schema.json` — `sheet-write`
- `sg-scan.schema.json` — `sg-scan`
- `sg-rewrite.schema.json` — `sg-rewrite`

### Schemas de input de comando — Config, MITM e workflow
- `config.schema.json` — `config` (descubra chaves XDG vivas via `config list-keys --json`; inclui `dialog_settle_ms`)
- `mitm.schema.json` — `mitm` (inclui `capture-url`)
- `workflow.schema.json` — `workflow`

### Schemas de input de comando — Emulação e performance
- `emulate.schema.json` — `emulate`
- `resize.schema.json` — `resize`
- `perf.schema.json` — `perf`
- `lighthouse.schema.json` — `lighthouse` (input; o envelope pode incluir `binary_source` real|mock; e2e mock é SKIP)
- `heap.schema.json` — `heap`

### Schemas de input de comando — Superfícies com gate de categoria
- `extension.schema.json` — `extension`
- `devtools3p.schema.json` — `devtools3p`
- `webmcp.schema.json` — `webmcp`

### CLI ao vivo vs snapshots estáticos
- Trate sempre `schema <cmd> --json` ou `schema --cmd <cmd> --json` como autoritativo para o binário instalado
- Depois de atualizar a CLI, rode `cargo test --test schema_input_drift_gate` para detectar drift e então reexecute `scripts/generate_command_schemas.sh` num host com `bash` para reescrever os arquivos
- Use `commands --json` para confirmar o inventário depois de upgrades, que lista 71 comandos
- A suíte e2e DevTools permanece com 53 tools e o mock do lighthouse como SKIP, enquanto os schemas de inventário cobrem a superfície completa de 71 comandos
- `submit.schema.json` e `storage.schema.json` já existem neste diretório
- A auditoria bilíngue de fences é o script `bash` `bash scripts/audit_bilingual_docs.sh`


## Cross-language note / Nota entre idiomas
- English and Português Brasileiro live in this same `README.md`, with no `README.pt-BR.md` in this directory
- Inglês e Português Brasileiro ficam neste mesmo `README.md`, sem `README.pt-BR.md` neste diretório
- Keep both language sections in this file when editing
- Mantenha as duas seções de idioma neste arquivo ao editar
- Schema file names remain English kebab-case matching CLI subcommands, for tooling stability
- Nomes dos arquivos de schema permanecem em inglês kebab-case iguais aos subcomandos da CLI, para estabilidade de tooling
