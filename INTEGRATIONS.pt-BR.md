[English](INTEGRATIONS.md) | [Português Brasileiro](INTEGRATIONS.pt-BR.md)

# Integrações — browser-automation-cli

> Um processo, um Chrome, um envelope JSON. Feito para subprocessos de agente.

## Snapshot de Cobertura
- Funciona com qualquer agente que spawna subprocesso e lê stdout e stderr
- Superfícies primárias: Claude Code, Codex, Cursor, shell CI, GitHub Actions
- Helpers de descoberta: `commands --json`, `schema --cmd`, `doctor --json`

## Aliases de Flags e Notas de Versão
- Nomes de produto permanecem fixos: `view`, `press`, `write`, `grab`
- Evite inventar aliases como `click` ou `screenshot` em prompts de agente
- `0.1.0` entrega a superfície de paridade DevTools default-on e gates de categoria
- Tools experimentais exigem `--experimental-vision` ou `--experimental-screencast`

## Tabela Resumo

| Surface | Integration style | Required flags | Notes |
|---------|-------------------|----------------|-------|
| Claude Code | subprocess | `--json` | multi-passo via `run --script` |
| Codex | subprocess | `--json -q` | stderr quieto para transcripts limpos |
| Cursor | shell tool | `--json` | timeouts explícitos |
| GitHub Actions | workflow step | `--json` | instale Chrome no runner |
| Shell CI | script | `--json` | parse com `jaq` |
| Continue / Cline | editor shell | `--json -q` | apenas one-shot |

## Claude Code
- Spawne um processo da CLI por ação atômica
- Use `run --script` quando refs `@eN` precisarem sobreviver a vários passos
```bash
browser-automation-cli doctor --offline --quick --json
browser-automation-cli --json goto https://example.com
browser-automation-cli --json view
```

## Codex
- Prefira `-q --json` para que só envelopes cheguem ao transcript
```bash
browser-automation-cli -q --json goto https://example.com
```

## Cursor
- Chame o binário pela shell tool com `--timeout` explícito
```bash
browser-automation-cli --timeout 60 --json scrape https://example.com
```

## GitHub Actions
```yaml
- name: Install CLI
  run: cargo install --path . --locked
- name: Doctor
  run: browser-automation-cli doctor --offline --quick --json
- name: Smoke goto
  run: browser-automation-cli --timeout 60 --json goto https://example.com
```

## Shell CI
- Sempre capture exit codes antes de parsear JSON
```bash
out=$(browser-automation-cli --json version)
echo "$out" | jaq -e '.ok == true'
```

## Continue e Cline
- Use modo JSON quieto para manter transcripts do editor limpos
- Não espere stickiness de sessão entre launches separados

## Novas Flags por Versão
- `0.1.0`: gates de categoria, vision e screencast experimentais, flags de captura, descoberta de schema
