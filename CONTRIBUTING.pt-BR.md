[English](CONTRIBUTING.md) | [Português Brasileiro](CONTRIBUTING.pt-BR.md)

# Contribuindo para browser-automation-cli

## Boas-vindas
- Obrigado por melhorar a automação one-shot de browser para agentes
- Este guia cobre setup, branching, commits, PRs e higiene de release

## Início Rápido
```bash
git clone https://github.com/danilo-aguiar-br/browser-automation-cli
cd browser-automation-cli
cargo build --locked
cargo test --locked
browser-automation-cli doctor --offline --quick --json
```

## Setup de Desenvolvimento
- Instale Rust 1.88.0 ou mais novo
- Instale Chrome ou Chromium para comandos de runtime
- Tools opcionais: `ffmpeg`, `lighthouse`, `cargo-deny`, `cargo-audit`
- Prefira `cargo run -q -- <args>` durante o desenvolvimento local

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
- Veja [docs/TESTING.pt-BR.md](docs/TESTING.pt-BR.md)

## Documentação
- Atualize docs públicas em inglês e português no mesmo PR
- Mantenha identificadores técnicos sem tradução
- Atualize `docs/schemas/` quando contratos JSON mudarem
- Atualize skill packages em `skills/` quando a superfície de comandos mudar
- Documente settings de produto só como flags mais `config` XDG
- Não invente nem documente variáveis de ambiente de produto para settings
- Ao adicionar comandos, atualize README Commands, INTEGRATIONS New Flags, llms.txt / llms-full Command Surface (EN+pt-BR), receitas COOKBOOK, skills, MIGRATION e contagens de inventário (61 de topo / 53 tools e2e em 0.1.4)

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
