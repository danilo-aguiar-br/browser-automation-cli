[English](CONTRIBUTING.md) | [Português Brasileiro](CONTRIBUTING.pt-BR.md)

# Contribuindo para browser-automation-cli

## Boas-vindas
- Obrigado por melhorar a automação one-shot de browser para agentes
- Este guia cobre setup, branching, commits, PRs e higiene de release

## Início Rápido
```bash
git clone https://github.com/danilo-aguiar-br/browser-automation-cli
cargo build --locked --manifest-path browser-automation-cli/Cargo.toml
cargo test --locked --manifest-path browser-automation-cli/Cargo.toml
browser-automation-cli doctor --offline --quick --json
```

## Setup de Desenvolvimento
- Instale Rust 1.88.0 ou mais novo
- Instale Chrome ou Chromium para comandos de runtime
- Tools opcionais: `ffmpeg`, `lighthouse`, `cargo-deny`, `cargo-audit`
- Prefira `cargo run -q -- <args>` durante o desenvolvimento local
- Se o checkout estiver dentro de pasta sincronizada (Dropbox, OneDrive, iCloud, Drive), exclua `target/` dessa sincronização ANTES do primeiro build
- No Dropbox para Linux isso é `mkdir -p target && setfattr -n user.com.dropbox.ignored -v 1 target`
- O atributo vive no DIRETÓRIO, então ele se perde sempre que `target/` é apagado e recriado, e precisa ser reaplicado nessa hora
- Medido em 2026-08-31: sem ele o cliente de sincronização apagou `target/` no meio da compilação, o que apareceu como `couldn't create a temp dir`, como um `cargo build --release` verde cujo artefato sumiu minutos depois, e como `SIGBUS` no `rustc` quando uma `.rlib` mapeada foi removida sob o processo

## Estratégia de Branch
- Faça branch a partir de `main`
- Use nomes curtos como `fix/goto-timeout` ou `docs/agents-guide`
- Mantenha cada PR focado em um único concern

## Convenção de Commit
- Prefira subjects no imperativo: `fix doctor offline path`
- Mantenha commits pequenos e revisáveis
- Nunca adicione trailers `Co-authored-by` sem pedido explícito do usuário
- Nunca commite secrets, cookies ou chaves de state cifrado

## Processo de PR
- Abra um PR contra `main`
- Descreva o que mudou, por que e como validou
- Linke issues relacionadas quando existirem
- Mantenha o diff livre de reformatação drive-by

## Testes
- Rode suites com `timeout 300 cargo test --locked`
- Rode clippy com `timeout 120 cargo clippy --all-targets --locked -- -D warnings`
- Rode format check com `cargo fmt --check`
- Adicione cobertura de regressão para cada bugfix
- Gates de contrato: `tests/parity_run_inventory.rs` (RUN_DISPATCHED_CMDS ∪ exclusões intencionais) e `tests/clap_command_debug_assert.rs` (`Cli::command().debug_assert()`)
- Gates locais residual-zero: `scripts/residual-check.sh` e `scripts/residual-stress.sh` (somente local; não é requisito de CI de produto)
- Veja [docs/TESTING.pt-BR.md](docs/TESTING.pt-BR.md)

## Documentação
- Atualize docs públicas em inglês e português no mesmo PR
- Mantenha identificadores técnicos sem tradução
- Atualize `docs/schemas/` quando contratos JSON mudarem
- Atualize skill packages em `skills/` quando a superfície de comandos mudar
- Documente settings de produto só como flags mais `config` XDG
- Não invente nem documente variáveis de ambiente de produto para settings
- Ao adicionar comandos, atualize README Commands, INTEGRATIONS New Flags, llms.txt / llms-full Command Surface (EN+pt-BR), receitas COOKBOOK, skills, MIGRATION e contagens de inventário
- Ao adicionar chave de configuração XDG ou flag global, não apenas comando, atualize também `docs/CONFIGURATION.md` e `docs/CONFIGURATION.pt-BR.md`, as duas skills embarcadas em `skills/` incluindo `references/xdg-keys.md`, e a entrada de CHANGELOG da versão
- O `scripts/doc-coverage-check.sh` lê o binário vivo e reprova quando a prosa deriva da superfície entregue
- Tip de inventário ao vivo (0.1.9): **71** nomes de agente via `commands --json` (0.1.6 acrescentou `submit`/`storage` → 65; 0.1.7 acrescenta `image`+`video`+`audio` → 68 e depois `record` → 69; também `select-option`, `pick`, `locale`, `man` — remeça sempre com `commands --json`); **53** tools e2e do DevTools com placar PASS=52 SKIP=1 quando o mock do lighthouse é o único skip

## Reportar Bugs
- Abra issue no GitHub com `browser-automation-cli --version`
- Inclua a linha de comando exata e URL redigida quando necessário
- Anexe envelopes `--json` quando a falha for estruturada

## Pedir Features
- Descreva o problema do usuário antes de propor superfície de API
- Prefira estender subcomandos existentes em vez de inventar aliases
- Mantenha ownership one-shot do processo como restrição dura

## Processo de Release
- Bump SemVer no `Cargo.toml`
- Atualize ambos CHANGELOGs em `[Unreleased]` e depois corte a seção da versão
- Mantenha a ordem Keep a Changelog: Unreleased primeiro, depois versões decrescentes
- Sincronize docs públicas com a superfície de comandos shipável antes da tag
- Confirme que `cargo package --list` inclui `docs/`, `skills/` e docs públicas da raiz
- Mantenha publish crates.io e GitHub Release bloqueados até aprovação explícita
- Valide com build, clippy, fmt e testes antes da tag

## Reconhecimento
- Contribuidores são creditados no histórico Git e nas notas de release
- Reporters de segurança entram em SECURITY após disclosure coordenado

## Perguntas
- Abra discussion ou issue no GitHub
- Contate o maintainer em daniloaguiarbr@proton.me para tópicos privados
