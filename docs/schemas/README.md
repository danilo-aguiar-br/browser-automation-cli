[English](README.md) | [Português Brasileiro](README.md)

# JSON Schemas — browser-automation-cli

## English
- This directory versions machine-readable JSON contracts for agents
- Live per-command input fragments also come from `browser-automation-cli schema --cmd <name> --json`
- Keep schema files in kebab-case matching command or envelope names

### Schema Index
- `envelope-success.schema.json` — success stdout envelope
- `envelope-error.schema.json` — error stdout envelope under `--json`
- `version.schema.json` — `version` command data payload
- `doctor.schema.json` — `doctor` command data payload
- `goto.schema.json` — `goto` input contract
- `run-script-step.schema.json` — one NDJSON step for `run --script`
- `commands.schema.json` — inventory payload for `commands --json`

## Português Brasileiro
- Este diretório versiona contratos JSON legíveis por máquina para agentes
- Fragments vivos de input por comando também vêm de `browser-automation-cli schema --cmd <name> --json`
- Mantenha arquivos de schema em kebab-case espelhando comando ou envelope

### Índice de Schemas
- `envelope-success.schema.json` — envelope de sucesso no stdout
- `envelope-error.schema.json` — envelope de erro no stdout com `--json`
- `version.schema.json` — payload de dados do comando `version`
- `doctor.schema.json` — payload de dados do comando `doctor`
- `goto.schema.json` — contrato de input de `goto`
- `run-script-step.schema.json` — um passo NDJSON para `run --script`
- `commands.schema.json` — payload de inventário de `commands --json`
