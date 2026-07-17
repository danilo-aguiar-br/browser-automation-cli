[English](INTEGRATIONS.md) | [Português Brasileiro](INTEGRATIONS.pt-BR.md)

# Integrações — browser-automation-cli

> Um processo, um Chrome, um envelope JSON. Feito para subprocessos de agentes.

## Snapshot de Cobertura
- Funciona com qualquer agente que dispare subprocesso e leia stdout mais stderr
- Superfícies primárias: Claude Code, Codex, Cursor, shell local, agentes de editor
- Helpers de descoberta: `commands --json`, `schema --cmd`, `doctor --json`
- O caminho de integração é apenas subprocesso local
- Settings de produto são flags mais config XDG apenas (sem env vars de produto)

## Aliases de Flags e Notas de Versão
- Nomes de produto ficam fixos: `view`, `press`, `write`, `grab`
- Evite inventar aliases como `click` ou `screenshot` em prompts de agente
- Use `grab --path <file>` (não path posicional bare)
- Use `wait --text` repetível para semântica OR entre várias strings
- Use `scrape --format` / `scrape --engine` para formatos de paridade Firecrawl local
- `0.1.0` entrega a superfície de paridade DevTools default-on mais gates de categoria
- `0.1.1` adiciona `config` XDG, MITM local, journal de workflow e superfície Firecrawl-local (`batch-scrape`, `crawl`, `map`, `search`, `parse`, `scrape` expandido)
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
- Prefira XDG `config set` para defaults duráveis em vez de inventar env vars
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
- Use vars de SO só para tracing/cor: `RUST_LOG`, `NO_COLOR`
```bash
out=$(browser-automation-cli --json version)
echo "$out" | jaq -e '.ok == true'
```

## Continue e Cline
- Use modo JSON quieto para manter transcripts do editor limpos
- Não espere stickiness de sessão entre launches de processos separados

## Novas Flags por Versão
- `0.1.0`: gates de categoria, vision e screencast experimentais, flags de capture, schema discovery
- `0.1.1`: `config` XDG (`init`/`path`/`show`/`get`/`set`), `mitm` (CA local + proxy one-shot em `127.0.0.1`), `workflow` (`run`/`resume`/`status`), superfície Firecrawl-local (`scrape --format/--engine`, `batch-scrape`, `crawl`, `map`, `search`, `parse`), `wait --text` multi OR, `grab --path`
