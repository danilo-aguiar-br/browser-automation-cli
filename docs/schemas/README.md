[English](README.md) | [Português Brasileiro](README.md)

# JSON Schemas — browser-automation-cli

This is a single bilingual file (English and Português Brasileiro sections below). There is no separate `README.pt-BR.md` in this directory.

Cross-language note: keep both language sections in this file when editing. Schema filenames stay kebab-case English matching CLI subcommands.


## English

- This directory versions machine-readable JSON contracts for agents
- Source of truth for per-command input fragments: live CLI  
  `browser-automation-cli schema --cmd <name> --json`
- Static `*.schema.json` command files are generated from that live surface via  
  `bash scripts/generate_command_schemas.sh`
- Check mode (no write): `bash scripts/generate_command_schemas.sh --check`
- Static snapshots may lag if the binary is older than `src/commands_prd/meta.rs`
- Prefer live `schema --cmd` when generating argv after upgrades
- Envelope files are hand-maintained and are not overwritten by the generator

### How to regenerate

```bash
cargo build --release --locked
bash scripts/generate_command_schemas.sh
bash scripts/generate_command_schemas.sh --check
```

### Envelopes and non-command contracts
- `envelope-success.schema.json` — success stdout envelope
- `envelope-error.schema.json` — error stdout envelope under `--json`
- `run-script-step.schema.json` — one NDJSON step for `run --script`

### Command input schemas (52 — full inventory)

#### Meta and discovery
- `doctor.schema.json` — `doctor`
- `commands.schema.json` — `commands`
- `schema.schema.json` — `schema`
- `version.schema.json` — `version`
- `completions.schema.json` — `completions`

#### Navigation and page state
- `goto.schema.json` — `goto`
- `back.schema.json` — `back`
- `forward.schema.json` — `forward`
- `reload.schema.json` — `reload`
- `view.schema.json` — `view`
- `page.schema.json` — `page`
- `wait.schema.json` — `wait`
- `dialog.schema.json` — `dialog`

#### Interaction
- `press.schema.json` — `press`
- `click-at.schema.json` — `click-at`
- `write.schema.json` — `write`
- `keys.schema.json` — `keys`
- `type.schema.json` — `type`
- `hover.schema.json` — `hover`
- `drag.schema.json` — `drag`
- `fill-form.schema.json` — `fill-form`
- `upload.schema.json` — `upload`
- `scroll.schema.json` — `scroll`

#### Extract and assert
- `eval.schema.json` — `eval`
- `extract.schema.json` — `extract`
- `text.schema.json` — `text`
- `attr.schema.json` — `attr`
- `assert.schema.json` — `assert`
- `cookie.schema.json` — `cookie`

#### Capture
- `grab.schema.json` — `grab`
- `console.schema.json` — `console`
- `net.schema.json` — `net`
- `screencast.schema.json` — `screencast`

#### Multi-step
- `run.schema.json` — `run`
- `exec.schema.json` — `exec`

#### Local scrape / crawl / parse (Firecrawl-parity)
- `scrape.schema.json` — `scrape`
- `batch-scrape.schema.json` — `batch-scrape`
- `crawl.schema.json` — `crawl`
- `map.schema.json` — `map`
- `search.schema.json` — `search`
- `parse.schema.json` — `parse`

#### Config, MITM, workflow
- `config.schema.json` — `config`
- `mitm.schema.json` — `mitm`
- `workflow.schema.json` — `workflow`

#### Emulation and performance
- `emulate.schema.json` — `emulate`
- `resize.schema.json` — `resize`
- `perf.schema.json` — `perf`
- `lighthouse.schema.json` — `lighthouse`
- `heap.schema.json` — `heap`

#### Category-gated surfaces
- `extension.schema.json` — `extension`
- `devtools3p.schema.json` — `devtools3p`
- `webmcp.schema.json` — `webmcp`

### Live CLI vs static snapshots
- Always treat `schema --cmd <name> --json` as authoritative for the installed binary
- After upgrading the CLI, re-run `scripts/generate_command_schemas.sh`
- Use `commands --json` to confirm inventory membership after upgrades
- Bilingual fence audit: `bash scripts/audit_bilingual_docs.sh`


## Português Brasileiro

- Este diretório versiona contratos JSON legíveis por máquina para agentes
- Fonte da verdade dos fragments de input por comando: CLI ao vivo  
  `browser-automation-cli schema --cmd <name> --json`
- Arquivos estáticos `*.schema.json` de comando são gerados dessa superfície via  
  `bash scripts/generate_command_schemas.sh`
- Modo verificação (sem gravar): `bash scripts/generate_command_schemas.sh --check`
- Snapshots estáticos podem atrasar se o binário for mais antigo que `src/commands_prd/meta.rs`
- Prefira `schema --cmd` ao vivo ao gerar argv após upgrades
- Arquivos de envelope são mantidos à mão e não são sobrescritos pelo gerador

### Como regenerar

```bash
cargo build --release --locked
bash scripts/generate_command_schemas.sh
bash scripts/generate_command_schemas.sh --check
```

### Envelopes e contratos fora de comando
- `envelope-success.schema.json` — envelope de sucesso no stdout
- `envelope-error.schema.json` — envelope de erro no stdout com `--json`
- `run-script-step.schema.json` — um passo NDJSON para `run --script`

### Schemas de input de comando (52 — inventário completo)

#### Meta e descoberta
- `doctor.schema.json` — `doctor`
- `commands.schema.json` — `commands`
- `schema.schema.json` — `schema`
- `version.schema.json` — `version`
- `completions.schema.json` — `completions`

#### Navegação e estado de página
- `goto.schema.json` — `goto`
- `back.schema.json` — `back`
- `forward.schema.json` — `forward`
- `reload.schema.json` — `reload`
- `view.schema.json` — `view`
- `page.schema.json` — `page`
- `wait.schema.json` — `wait`
- `dialog.schema.json` — `dialog`

#### Interação
- `press.schema.json` — `press`
- `click-at.schema.json` — `click-at`
- `write.schema.json` — `write`
- `keys.schema.json` — `keys`
- `type.schema.json` — `type`
- `hover.schema.json` — `hover`
- `drag.schema.json` — `drag`
- `fill-form.schema.json` — `fill-form`
- `upload.schema.json` — `upload`
- `scroll.schema.json` — `scroll`

#### Extração e assert
- `eval.schema.json` — `eval`
- `extract.schema.json` — `extract`
- `text.schema.json` — `text`
- `attr.schema.json` — `attr`
- `assert.schema.json` — `assert`
- `cookie.schema.json` — `cookie`

#### Captura
- `grab.schema.json` — `grab`
- `console.schema.json` — `console`
- `net.schema.json` — `net`
- `screencast.schema.json` — `screencast`

#### Multi-passo
- `run.schema.json` — `run`
- `exec.schema.json` — `exec`

#### Scrape / crawl / parse local (Firecrawl-parity)
- `scrape.schema.json` — `scrape`
- `batch-scrape.schema.json` — `batch-scrape`
- `crawl.schema.json` — `crawl`
- `map.schema.json` — `map`
- `search.schema.json` — `search`
- `parse.schema.json` — `parse`

#### Config, MITM, workflow
- `config.schema.json` — `config`
- `mitm.schema.json` — `mitm`
- `workflow.schema.json` — `workflow`

#### Emulação e performance
- `emulate.schema.json` — `emulate`
- `resize.schema.json` — `resize`
- `perf.schema.json` — `perf`
- `lighthouse.schema.json` — `lighthouse`
- `heap.schema.json` — `heap`

#### Superfícies com gate de categoria
- `extension.schema.json` — `extension`
- `devtools3p.schema.json` — `devtools3p`
- `webmcp.schema.json` — `webmcp`

### CLI ao vivo vs snapshots estáticos
- Trate sempre `schema --cmd <name> --json` como autoritativo para o binário instalado
- Após atualizar a CLI, reexecute `scripts/generate_command_schemas.sh`
- Use `commands --json` para confirmar inventário após upgrades
- Auditoria bilíngue de fences: `bash scripts/audit_bilingual_docs.sh`


### Nota entre idiomas / Cross-language note
- English and Português Brasileiro live in this same `README.md` (no `README.pt-BR.md` here)
- Inglês e Português Brasileiro ficam neste mesmo `README.md` (sem `README.pt-BR.md` neste diretório)
- Schema file names remain English kebab-case for tooling stability
- Nomes dos arquivos de schema permanecem em inglês kebab-case para estabilidade de tooling
