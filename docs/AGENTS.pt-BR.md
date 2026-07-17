[English](AGENTS.md) | [Português Brasileiro](AGENTS.pt-BR.md)

# Guia de Agentes — browser-automation-cli

> Corte cola de browser-tool. Mantenha um ciclo de vida de Chrome sob seu agente. Ciclo de vida: BORN EXECUTE FINALIZE DIE.


## Por que Agentes Escolhem Esta CLI
- Ownership de subprocesso é explícito e de curta duração
- Envelopes JSON reduzem scraping frágil de stdout
- Scripts multi-passo preservam refs de acessibilidade sem daemon
- Gates de categoria mantêm superfícies experimentais opt-in
- Superfície de descoberta firecrawl-local embarca como subcomandos de primeira classe
- Config XDG substitui variáveis de ambiente de produto


## Economia
- Evite servers de browser long-lived que vazam entre turns do agente
- Pague o custo de launch do Chrome só quando a tarefa precisa de página real
- Prefira `scrape` / `batch-scrape` / `crawl` / `map` HTTP quando só conteúdo basta
- Colapse fluxos multi-passo em um processo `run` quando refs importam
- Reutilize `schema --cmd` uma vez por sessão em vez de adivinhar argv


## Soberania
- Sem dependência de runtime npm no binário do produto
- Sem caminho de telemetria remota na CLI
- Chrome do sistema permanece sob a política do host do operador
- Settings de produto vivem em flags e `config` XDG, não em variáveis de ambiente de produto
- Não existe contrato de env de produto como `BROWSER_AUTOMATION_CLI_*`


## Agentes e Orquestradores Compatíveis
- O modo de integração de cada entrada abaixo é subprocesso one-shot com `--json`
- Este projeto valida localmente com cargo e scripts e2e; não afirma cobertura CI hospedada por agente
- Claude Code
- Codex
- Gemini CLI
- Opencode
- Cursor
- Windsurf
- VS Code Copilot
- GitHub Copilot CLI
- Cline
- Continue
- Aider
- Zed AI assistant
- JetBrains AI Assistant
- Scripts de shell local e Makefiles
- Qualquer orquestrador que possa spawnar um processo e ler stdout e exit codes


## Detalhes de Integração de Agente
- Spawne `browser-automation-cli` como subprocesso one-shot
- Passe sempre `--json` para parsing por máquina
- Leia envelopes de sucesso e erro no stdout
- Mantenha stderr só para logs humanos ou debug
- Use `commands --json` para descobrir o inventário vivo
- O inventário inclui config, mitm, workflow, scrape, batch-scrape, crawl, map, search, parse
- Use `schema --cmd <name> --json` antes de gerar argv de comandos pouco familiares
- Prefira flags para controle pontual
- Use `config init|set|get|path|show` para defaults XDG duráveis
- Env de SO só quando necessário: `RUST_LOG` para tracing, `NO_COLOR` para desligar cor
- Para multi-passo que precisa de refs `@eN` compartilhadas, use um processo `run --script`
- Wait com texto OR: `wait --text A --text B`
- Scrape com `--format text|markdown|html|links|metadata` e `--engine http|browser`
- Capture screenshots com `grab --path <file>` (não path posicional)


## Integrações do Crate
- O nome do binário é sempre `browser-automation-cli`
- Instale com `cargo install browser-automation-cli --locked` após publish no crates.io
- Em desenvolvimento, instale por path ou git
- Qualquer crate Rust de agente integra via `std::process::Command`
- Crates de padrão compatível incluem `rig-core`, `genai`, `async-openai`, `ollama-rs`, `anthropic-sdk`, `agentai`, `autoagents`, `swarms-rs`, `graphbit`, `llm-agent-runtime`
- A CLI não é dependência de library Rust desses crates
- O contrato compartilhado é argv mais JSON no stdout mais exit codes no estilo sysexits

### Exemplo Mínimo em Rust com Command
```rust
use std::process::Command;

fn main() {
    let out = Command::new("browser-automation-cli")
        .args(["-q", "--json", "version"])
        .output()
        .expect("spawn browser-automation-cli");
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], true);
}
```


## Descoberta de Superfície para Agentes
- Inventário: `browser-automation-cli commands --json`
- Fragments de input: `browser-automation-cli schema --cmd <name> --json`
- Paths de config: `browser-automation-cli config path --json`
- Chaves de config: `config set|get|show` para lang, timeout, artifacts_dir, ignore_robots, namespace, encryption_key, color
- MITM: `mitm status|init-ca|start|list|get|har|export|domains|apis`
- Workflow: `workflow run|resume|status`
- Firecrawl-local: `scrape`, `batch-scrape`, `crawl`, `map`, `search`, `parse`
- Saúde: `doctor --json` (reporta descoberta de Chrome e XDG browsers_dir)


## Ciclo de Vida
- Slogan (English): BORN EXECUTE FINALIZE DIE
- Um processo possui uma sessão Chrome do launch até o FINALIZE
- FINALIZE é idempotente (Browser.close, wait, kill fallback)
- Não espere sessão ou refs `@eN` sobreviverem ao exit do processo


## Contrato Técnico
### REQUIRED
- Passe `--json` para consumo programático
- Trate um processo como um ciclo de vida de Chrome (BORN EXECUTE FINALIZE DIE)
- Use `run --script` para multi-passo que precisa de refs `@eN` compartilhadas
- Cheque exit code do processo antes de confiar no stdout
- Ramifique no campo `ok` do envelope
- Mantenha gates de categoria e experimental explícitos quando necessários
- Configure settings duráveis de produto só via `config` / flags
- Descubra comandos desconhecidos com `commands --json` e `schema --cmd`

### FORBIDDEN
- Não mantenha daemon entre turns do agente
- Não invente aliases de produto como `bac`, `click` ou `screenshot`
- Não reutilize refs `@eN` entre launches de processo separados
- Não parseie stderr como canal primário de sucesso
- Não habilite bypass de robots sem a política dual-flag
- Não dependa de variáveis de ambiente de produto `BROWSER_AUTOMATION_CLI_*`
- Não passe path posicional para `grab`; use `--path`
- Não invente preset `--device` em `emulate`; use `--user-agent`, `--viewport`, `--network-conditions`

### Correct Pattern
```bash
browser-automation-cli -q --timeout 60 --json goto https://example.com
browser-automation-cli -q --json view
out=$(browser-automation-cli -q --json version)
echo "$out" | jaq -e '.ok == true'
browser-automation-cli -q --json commands
browser-automation-cli -q --json config path
browser-automation-cli -q --json wait --text Example --text Domain --ms 5000
browser-automation-cli -q --json scrape https://example.com --format markdown --engine http
browser-automation-cli -q --json grab --path /tmp/page.png --full-page
```


## Envelope JSON
- Sucesso: `{"schema_version":1,"ok":true,"data":...}`
- Erro: `{"schema_version":1,"ok":false,"error":{...}}`
- Objetos de erro incluem `kind`, `message` e `exit_code` quando `--json` está ativo
- Índice de schemas: [docs/schemas/README.md](schemas/README.md)
- Fragments vivos de input sempre vêm de `schema --cmd`; arquivos estáticos podem atrasar


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
