[English](AGENTS.md) | [Português Brasileiro](AGENTS.pt-BR.md)

# Guia de Agentes — browser-automation-cli

> Corte cola de browser-tool. Mantenha um ciclo de vida de Chrome sob seu agente.

## Por que Agentes Escolhem Esta CLI
- Ownership de subprocesso é explícito e de curta duração
- Envelopes JSON reduzem scraping frágil de stdout
- Scripts multi-passo preservam refs de acessibilidade sem daemon
- Gates de categoria mantêm superfícies experimentais opt-in

## Economia
- Evite servers de browser long-lived que vazam entre turns do agente
- Pague o custo de launch do Chrome só quando a tarefa precisa de página real
- Colapse fluxos multi-passo em um processo `run` quando refs importam

## Soberania
- Sem dependência de runtime npm no binário do produto
- Sem caminho de telemetria remota na CLI
- Chrome do sistema permanece sob a política do host do operador

## Agentes e Orquestradores Compatíveis
- Claude Code
- Codex
- Cursor
- Continue
- Cline
- Scripts de shell local e agentes de editor

## Detalhes de Integração de Agente
- Spawne `browser-automation-cli` como subprocesso one-shot
- Passe sempre `--json` para parsing por máquina
- Leia envelopes de sucesso e erro no stdout
- Mantenha stderr só para logs humanos ou debug
- Use `commands --json` para descobrir o inventário vivo
- Use `schema --cmd <name> --json` antes de gerar argv de comandos pouco familiares

## Integrações do Crate
- O nome do binário é sempre `browser-automation-cli`
- Install por git/path em desenvolvimento ou `cargo install browser-automation-cli --locked` após publish no crates.io
- Após release no crates.io use `cargo install browser-automation-cli --locked`

## Contrato Técnico
### REQUIRED
- Passe `--json` para consumo programático
- Trate um processo como um ciclo de vida de Chrome
- Use `run --script` para multi-passo que precisa de refs `@eN` compartilhadas
- Cheque exit code do processo antes de confiar no stdout
- Ramifique no campo `ok` do envelope
- Mantenha gates de categoria e experimental explícitos quando necessários

### FORBIDDEN
- Não mantenha daemon entre turns do agente
- Não invente aliases de produto como `bac`, `click` ou `screenshot`
- Não reutilize refs `@eN` entre launches de processo separados
- Não parseie stderr como canal primário de sucesso
- Não habilite bypass de robots sem a política dual-flag

### Correct Pattern
```bash
browser-automation-cli -q --timeout 60 --json goto https://example.com
browser-automation-cli -q --json view
out=$(browser-automation-cli -q --json version)
echo "$out" | jaq -e '.ok == true'
```

## Envelope JSON
- Sucesso: `{"schema_version":1,"ok":true,"data":...}`
- Erro: `{"schema_version":1,"ok":false,"error":{...}}`
- Índice de schemas: [docs/schemas/README.md](schemas/README.md)

## Códigos de Saída
- `0` sucesso
- `2` usage
- `65` data
- `66` no input
- `69` unavailable
- `70` software, browser, protocol
- `74` I/O
- `78` config
- `124` timeout
- `130` cancelled
- `141` broken pipe
