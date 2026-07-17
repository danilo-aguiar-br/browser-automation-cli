[English](INTEGRATIONS.md) | [Português Brasileiro](INTEGRATIONS.pt-BR.md)

# Integrações — browser-automation-cli

> Um processo, um Chrome, um envelope JSON. Feito para subprocessos de agentes.

## Snapshot de Cobertura
- Funciona com qualquer agente que dispare subprocesso e leia stdout mais stderr
- Superfícies primárias: Claude Code, Codex, Cursor, shell local, agentes de editor
- Helpers de descoberta: `commands --json`, `schema --cmd`, `doctor --json`
- O caminho de integração é apenas subprocesso local

## Aliases de Flags e Notas de Versão
- Nomes de produto ficam fixos: `view`, `press`, `write`, `grab`
- Evite inventar aliases como `click` ou `screenshot` em prompts de agente
- `0.1.0` entrega a superfície de paridade DevTools default-on mais gates de categoria
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
browser-automation-cli --timeout 60 --json scrape https://example.com
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
