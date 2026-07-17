[English](MIGRATION.md) | [Português Brasileiro](MIGRATION.pt-BR.md)

# Migração — browser-automation-cli

> Migre para o modelo de processo one-shot sem adivinhar o mapa de comandos. Ciclo de vida: BORN EXECUTE FINALIZE DIE.


## O Que Muda
- `0.1.0` é a primeira linha pública do produto
- Nomes canônicos de comando são `view`, `press`, `write` e `grab`
- Automação multi-passo deve usar `run --script` em um processo
- Superfícies de categoria e experimental são opt-in
- Slogan de lifecycle é só em inglês: BORN EXECUTE FINALIZE DIE


## Baseline 0.1.0
- Launch one-shot do Chrome e cleanup FINALIZE em um único processo
- Navegação e interação centrais: `goto`, `view`, `press`, `write`, `grab`, `run`
- Superfície de paridade DevTools para input, snapshot, network, console, pages, wait, perf, lighthouse, screencast, heap, extensions
- Descoberta de schema via `schema --cmd` e inventário via `commands --json`
- Política dual-flag de robots para bypass explícito
- Gates de categoria como `--category-memory` e `--category-extensions`
- Gates experimentais como `--experimental-vision` e `--experimental-screencast`
- Explicitamente fora só de 0.1.0: MITM local, journal de workflow e superfície Firecrawl crawl/map/search


## 0.1.0 → 0.1.1
### Configuração e XDG
- Contrato de variáveis de ambiente de produto foi removido
- Não use `BROWSER_AUTOMATION_CLI_*` para settings
- Configure com flags da CLI e XDG via `config init|set|get|path|show`
- `config path --json` reporta `config_dir`, `data_dir`, `state_dir`, `mitm_ca_dir`, `mitm_capture_dir`, `workflow_dir` e paths relacionados
- Chave de cifragem vai para `config set encryption_key`, não env
- Env de SO permanece só para convenções do host: `RUST_LOG`, `NO_COLOR`
- Doctor ganha check XDG de `browsers_dir`

### MITM
- Nova superfície MITM local em hudsucker
- `mitm start` faz bind em `127.0.0.1` com porta efêmera em modo one-shot
- Comandos relacionados: `status`, `init-ca`, `list`, `get`, `har`, `export`, `domains`, `apis`
- Material de CA fica sob XDG data; capturas sob XDG state

### Workflow
- Novo journal DAG de workflow (petgraph + SQLite)
- Comandos: `workflow run`, `workflow resume`, `workflow status`
- Journals ficam sob XDG state
- `workflow resume` pula passos já marcados `ok`

### Superfície firecrawl-local
- Novos comandos: `batch-scrape`, `crawl`, `map`, `search`, `parse`
- `scrape` ganha `--format` (`text|markdown|html|links|metadata`)
- `scrape` ganha `--engine` (`http|browser`) e `--only-main-content`
- Batch scrape usa concorrência limitada via Tokio `JoinSet`

### Flags de interação e captura
- `wait` aceita `--text` repetível com semântica OR (qualquer match resolve)
- `grab` usa `--path` (não path posicional)
- `emulate` usa `--user-agent`, `--viewport`, `--network-conditions` (sem preset `--device`)
- `run` ganha opções de paridade com scrape e aplica gates de categoria dentro dos passos do script

### Empacotamento e docs
- Documentação e skills bilíngues públicas para o pacote crates.io
- Dual license `MIT OR Apache-2.0`
- Workflows hospedados do GitHub Actions removidos; a validação é local


## Migração Passo a Passo
- Instale ou rebuild o binário para `0.1.1`
- Substitua chamadas de session-daemon por invocações one-shot
- Reescreva planos multi-passo de agente em scripts NDJSON para `run`
- Mude consumidores de output para envelopes `--json`
- Remova scripts que exportam settings de produto `BROWSER_AUTOMATION_CLI_*`
- Mova defaults duráveis para `config set` ou mantenha-os como flags explícitas
- Mova material de cifragem para `config set encryption_key <secret>`
- Mapeie nomes antigos de tools via `commands --json` e o tool map DevTools
- Atualize callers de screenshot para `grab --path <file>`
- Atualize waits que precisam de textos alternativos para `--text` repetível (OR)
- Atualize callers de scrape para passar `--format` e `--engine` de forma explícita quando necessário
- Descubra superfícies novas de v0.1.1 com `schema --cmd <name> --json`
- Confirme saúde de paths XDG do doctor com `doctor --json`
- Reexecute validação local: `cargo test --lib`, script e2e e smokes dos pilares que importam


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
- Envelopes de erro também carregam `kind` e `exit_code` para ramificação programática
- Fragments vivos de input por comando vêm de `schema --cmd`
- Snapshots estáticos em `docs/schemas/` são um índice de conveniência e podem atrasar o binário
- Adições estáticas de v0.1.1 incluem `config`, `mitm`, `workflow`, `scrape`, `batch-scrape`, `crawl`, `map`, `search`, `parse` e `wait`
- Prefira `schema --cmd` ao vivo após upgrades para confirmar o binário instalado


## Notas de Compatibilidade
- Não existe linha estável prévia no crates.io para este repositório antes de `0.1.0`
- Limpeza de branding e histórico recriou um root commit público limpo
- O primeiro publish no crates.io ainda exige aprovação explícita do mantenedor
- Agentes que hardcoded env vars de produto devem migrar para flags + `config`
- Integração por subprocesso permanece o único path de agente suportado
- Exit codes permanecem no estilo sysexits: `0`, `2`, `65`, `66`, `69`, `70`, `74`, `78`, `124`, `130`, `141`


## Rollback
- Fixe o commit local anterior ou o path do binário instalado
- Mantenha scripts compatíveis com os campos `ok` e `schema_version` do envelope
- Se reverter de `0.1.1` para `0.1.0`, remova o uso de config, mitm, workflow, batch-scrape, crawl, map, search, parse
- Se reverter, também remova premissas de scrape `--format`/`--engine` que dependem de `0.1.1`
- Se reverter, restaure wrappers de wait ou grab que assumiam argv antigo só se o seu fork os tinha
- Não reintroduza env vars de produto; mesmo em trees antigas, prefira flags quando possível


## Veja Também
- [CHANGELOG.pt-BR.md](../CHANGELOG.pt-BR.md)
- [docs/AGENTS.pt-BR.md](AGENTS.pt-BR.md)
- [docs/CROSS_PLATFORM.pt-BR.md](CROSS_PLATFORM.pt-BR.md)
- [docs/TESTING.pt-BR.md](TESTING.pt-BR.md)
- [docs/schemas/README.md](schemas/README.md)
