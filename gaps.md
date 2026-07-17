# gaps.md — Status v0.1.1 (CLOSED)

Data: 2026-07-17  
Binário: `target/release/browser-automation-cli` 0.1.1  
Método: compile local, `cargo test --lib` (208 ok), e2e 52 tools, smokes pilares, auditoria de documentação pública.

## Resumo

| Check | Resultado |
|-------|-----------|
| `cargo build --release` | OK 0.1.1 |
| `cargo test --lib` | 208 passed |
| E2E 52 tools | 52 PASS / 0 FAIL |
| `run` + scrape | OK |
| Help product env | ZERO `[env: BROWSER_AUTOMATION_CLI_*=]` |
| XDG doctor/config | OK |
| MITM start | bind 127.0.0.1 ephemeral OK |
| Telemetria | none |
| Documentação pública alinhada à superfície 0.1.1 | OK (auditoria 2026-07-17) |
| Skills no tarball (`cargo package --list`) | OK (`skills/` não excluído) |

## GAP status (18/18 CLOSED)

| ID | Status v0.1.1 |
|----|----------------|
| 001 RunFlags E0609 | CLOSED |
| 002 Selector E0597 | CLOSED |
| 003 run scrape | CLOSED |
| 004 MITM hudsucker | CLOSED |
| 005 Workflow resume | CLOSED |
| 006 Firecrawl local surface | CLOSED |
| 007 XDG / no product env | CLOSED |
| 008 Matrix | CLOSED |
| 009 wait multi-text OR | CLOSED |
| 010 scrape formats paths | CLOSED |
| 011 English comments | CLOSED |
| 012 tool-ref fixture | CLOSED |
| 013 lto fat + JoinSet + hudsucker | CLOSED |
| 014 doctor XDG | CLOSED |
| 015 commands complete | CLOSED |
| 016 lighthouse e2e mock path | CLOSED |
| 017 DoD expanded | CLOSED |
| 018 no env suggestions | CLOSED |

## Documentação pública (DoD v0.1.1)

- README EN/PT: Commands/Superpowers/Configuration com config, mitm, workflow, batch-scrape, crawl, map, search, parse; sem env de produto
- CHANGELOG Keep a Changelog: Unreleased → 0.1.1 → 0.1.0
- docs/* e pares .pt-BR: HOW_TO_USE, AGENTS, COOKBOOK, CROSS_PLATFORM, MIGRATION, TESTING
- INTEGRATIONS New Flags 0.1.1
- SECURITY: encryption via config; práticas MITM
- llms.txt / llms-full.txt / llms.pt-BR.txt: paths `skills/`, superfície completa
- skills EN/PT: skill consolidada imperativa com catálogo completo dos 52 comandos e fórmula executável por comando; playbooks multi-passo A–J; XDG/MITM/workflow/scrape; envelopes; exit codes; `evals/queries.json` 20 queries por idioma; sem histórico de versões na skill
- CLAUDE.md bloco de produto `browser-automation-cli`: catálogo completo dos 52 comandos com fórmulas; config XDG; MITM 127.0.0.1; workflow JSON; Firecrawl-local; política anti-product-env (só RUST_LOG/NO_COLOR/PATH); `grab --path`; wait multi-text OR; linguagem DEVE/NUNCA
- docs/schemas: 52 command schemas gerados de meta/`schema --cmd` + envelopes + run-script-step; `scripts/generate_command_schemas.sh` + `--check`
- Auditoria bilíngue de fences: `scripts/audit_bilingual_docs.sh` (exit 0 nos pares públicos)
- Links de skill: `skills/` (plural), não `skill/`
- Slogan de lifecycle de produto: inglês BORN → EXECUTE → FINALIZE → DIE

## Evidências

- version 0.1.1
- e2e TOTAL=52 PASS=52
- run scrape ok
- config path layout=xdg
- mitm start 127.0.0.1:PORT
- docs e skills sincronizados com a superfície shipável

Nenhum GAP-00x deixado para versão futura.
