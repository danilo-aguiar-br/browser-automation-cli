[English](TESTING.md) | [Português Brasileiro](TESTING.pt-BR.md)

# Testes — browser-automation-cli

> Rode a suite certa para o risco, não todo path de browser por default.

## Por que Testes Categorizados
- Testes de runtime de browser são mais lentos e dependentes do host
- Testes de schema e inventário pegam drift de contrato sem Chrome
- Manter categorias explícitas protege a velocidade de iteração local

## Categorias de Teste
- Testes unitários e de library em `src/`
- Smokes de CLI como `tests/doctor_cli.rs` e `tests/goto_smoke.rs`
- Gates de envelope e schema como `tests/envelope_schema.rs` e `tests/parity_toolref_schema.rs`
- Testes de inventário e matriz de paridade
- Testes de robots e comportamento de pipe
- Cobertura e2e opcional de eventos CDP quando Chrome está disponível

## Como Rodar
```bash
timeout 300 cargo test --locked
timeout 120 cargo clippy --all-targets --locked -- -D warnings
cargo fmt --check
```
- Rode um arquivo com `cargo test --test doctor_cli --locked`
- Use `-- --nocapture` só durante debug

## Perfis de CI
- CI default deve rodar fmt, clippy e testes de contrato sem browser primeiro
- Testes com browser exigem Chrome ou Chromium na imagem do runner
- Mantenha jobs de publish e release bloqueados sem aprovação do maintainer

## Variáveis de Ambiente
- `RUST_LOG` para tracing mais profundo em testes falhando
- `BROWSER_AUTOMATION_CLI_DEBUG` para máximo detalhe no stderr da CLI
- Variáveis de path do Chrome só quando a descoberta precisar de override

## Troubleshooting
- Doctor falha em chrome: instale Chromium ou Google Chrome primeiro
- Timeouts no goto smoke: eleve o timeout do processo ou inspecione política de rede
- Falhas de schema gate: atualize código e `docs/schemas/` na mesma mudança
