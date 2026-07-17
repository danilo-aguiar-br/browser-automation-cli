[English](MIGRATION.md) | [Português Brasileiro](MIGRATION.pt-BR.md)

# Migração — browser-automation-cli

> Migre para o modelo de processo one-shot sem adivinhar o mapa de comandos.

## O Que Muda
- `0.1.0` é a primeira linha pública do produto
- Nomes canônicos de comando são `view`, `press`, `write` e `grab`
- Automação multi-passo deve usar `run --script` em um processo
- Superfícies de categoria e experimental são opt-in

## Migração Passo a Passo
- Instale o binário por path ou git
- Substitua chamadas de session-daemon por invocações one-shot
- Reescreva planos multi-passo de agente em scripts NDJSON para `run`
- Mude consumidores de output para envelopes `--json`
- Mapeie nomes antigos de tools via `commands --json` e o tool map DevTools

## Mudanças de JSON Schema
- Antes: prosa livre ou JSON ad-hoc sem `schema_version`
- Depois no sucesso:
```json
{"schema_version":1,"ok":true,"data":{}}
```
- Depois no erro com `--json`:
```json
{"schema_version":1,"ok":false,"error":{"message":"..."}}
```
- Fragments vivos de input por comando vêm de `schema --cmd`

## Notas de Compatibilidade
- Não existe linha estável prévia no crates.io para este repositório
- Limpeza de branding e histórico recriou um root commit público limpo
- O primeiro publish no crates.io ainda exige aprovação explícita do mantenedor

## Rollback
- Fixe o commit local anterior ou o path do binário instalado
- Mantenha scripts compatíveis com os campos `ok` e `schema_version` do envelope

## Veja Também
- [CHANGELOG.pt-BR.md](../CHANGELOG.pt-BR.md)
- [docs/AGENTS.pt-BR.md](AGENTS.pt-BR.md)
- [docs/schemas/README.md](schemas/README.md)
