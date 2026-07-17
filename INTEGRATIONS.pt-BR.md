[English](INTEGRATIONS.md) | [Português Brasileiro](INTEGRATIONS.pt-BR.md)

# Integrações — browser-automation-cli

> Um processo, um Chrome, um envelope JSON. Feito para subprocessos de agentes.

## Snapshot de Cobertura
- Funciona com qualquer agente que dispare subprocesso e leia stdout mais stderr
- Superfícies primárias: Claude Code, Codex, Cursor, shell local, agentes de editor
- Helpers de descoberta: `commands --json`, `schema --cmd`, `doctor --json`
- O caminho de integração é apenas subprocesso local
- Settings de produto são flags mais config XDG apenas

## Aliases de Flags e Notas de Versão
- Nomes de produto ficam fixos: `view`, `press`, `write`, `grab`
- Evite inventar aliases como `click` ou `screenshot` em prompts de agente (use `grab` para screenshots; scrape pode aceitar token de format `screenshot`)
- Use `grab --path <file>` (não path posicional bare)
- Use `wait --text` repetível para semântica OR entre várias strings
- Use `scrape --format` / `scrape --engine` para formatos de scrape local
- Scrape browser aplica `--format` via outerHTML (markdown/html/links/metadata/raw-html/screenshot/summary/product/branding)
- `0.1.0` entrega a superfície de paridade DevTools default-on mais gates de categoria
- `0.1.1` adiciona `config` XDG, MITM local, journal de workflow e superfície local scrape/crawl/map/search/parse (`batch-scrape`, `crawl`, `map`, `search`, `parse`, `scrape` expandido)
- `0.1.2` fecha gaps agent-first e adiciona `print-pdf`, `monitor`, `qr`, `find-paths`, tipos de documento no parse, extract LLM e chaves de config expandidas
- Ferramentas experimentais exigem `--experimental-vision` ou `--experimental-screencast`

## Tabela Resumo

| Superfície | Estilo de integração | Flags exigidas | Notas |
|------------|----------------------|----------------|-------|
| Claude Code | subprocesso | `--json` | multi-passo via `run --script` |
| Codex | subprocesso | `--json -q` | stderr quieto para transcripts limpos |
| Cursor | shell tool | `--json` | deixe timeouts explícitos |
| Shell local | script | `--json` | parse com `jaq` |
| Continue / Cline | shell do editor | `--json -q` | apenas one-shot |

## Claude Code
- Dispare um processo CLI por ação atômica
- Use `run --script` quando refs `@eN` precisarem sobreviver a vários passos
- Prefira XDG `config set` para defaults duráveis
```bash
browser-automation-cli doctor --offline --quick --json
browser-automation-cli --json goto https://example.com
browser-automation-cli --json view
```

## Codex
- Prefira `-q --json` para que só envelopes cheguem ao transcript do agente
```bash
browser-automation-cli -q --json goto https://example.com
```

## Cursor
- Chame o binário da shell tool com `--timeout` explícito
```bash
browser-automation-cli --timeout 60 --json scrape https://example.com --format markdown --engine http
```

## Shell Local
- Sempre capture exit codes antes de parsear JSON
- Rode validações na sua máquina local antes do release
```bash
out=$(browser-automation-cli --json version)
echo "$out" | jaq -e '.ok == true'
```

## Continue e Cline
- Use modo JSON quieto para manter transcripts do editor limpos
- Não espere stickiness de sessão entre launches de processos separados

## Novas Flags por Versão
- `0.1.0`: gates de categoria, vision e screencast experimentais, flags de capture, schema discovery
- `0.1.1`: `config` XDG (`init`/`path`/`show`/`get`/`set`), `mitm` (CA local + proxy one-shot em `127.0.0.1`), `workflow` (`run`/`resume`/`status`), superfície local de scrape (`scrape --format/--engine`, `batch-scrape`, `crawl`, `map`, `search`, `parse`), `wait --text` multi OR, `grab --path`
- `0.1.2`:
  - `scrape --engine browser` aplica `--format` (incl. `raw-html`, `screenshot`, `summary`, `product`, `branding`) via outerHTML
  - Aliases de scroll em `run` `dy`/`dx` para `delta_y`/`delta_x`; envelopes de erro fail-fast podem incluir `data.steps` parciais
  - `schema --cmd` expandido para `goto`/`eval`/`type`/`scroll`/`assert`
  - `--lang pt-BR` e `config set lang` localizam sugestões humanas
  - Logging via `--verbose`/`--debug` e XDG `log_level`/`chrome_path`/`lighthouse_path` apenas
  - `search` limpa redirects SERP `uddg=`
  - `print-pdf` one-shot CDP; `monitor check --url --baseline [--write-baseline]`
  - `parse` PDF/DOCX/xlsx/ods + `--redact-pii`; `extract --llm --question --schema-json` (XDG `openrouter_api_key`, `llm_base_url`, `llm_model`)
  - `qr encode|decode`, `find-paths`
  - Aliases de `assert` `url_contains`/`text_contains`; fallback de property DOM em `attr`
  - Chaves de config: `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`
  - Inventário de comandos com 56 nomes de topo (`commands --json`), incluindo `print-pdf`, `monitor`, `qr`, `find-paths`
