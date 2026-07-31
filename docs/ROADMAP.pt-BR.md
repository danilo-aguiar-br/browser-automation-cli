[English](ROADMAP.md) | [Português Brasileiro](ROADMAP.pt-BR.md)

# Roadmap (notas de mantenedor)

- Este projeto embarca como CLI one-shot estável
- O roadmap é intencionalmente curto

## Curto prazo (qualidade local)

- **DoD residual da v0.1.6 alcançado** (medido 2026-07-31): GAP-054 settle de diálogo + multi-aba (booleano `dialog_settled`; XDG `dialog_settle_ms`), GAP-055 select nativo, GAP-057 format de scrape em run, GAP-053 `wait_timeout_ms`, lei residual-zero de disco da 0.1.5 ainda corrente
- Manter gates `scripts/*-check.sh` verdes em cada passe de auditoria (incl. `scripts/residual-check.sh` / `scripts/residual-stress.sh` para residual-zero em disco)
- Suite opcional de confiança: reexecutar `dialog_multitab_gate`, `option_pick_gate`, `wait_conditions_gate`, `scrape_step_gate`, fixtures unit LHR de lighthouse após refactors grandes
- Inventário vivo de agentes: **65** nomes via `commands --json` (inclui `submit`, `storage`, `locale`, `man`)
- Settings de produto: só flags + XDG `config` (sem variáveis de ambiente de produto); descubra chaves via `config list-keys --json`
- **Encode do `grab`:** só png|jpeg|webp; AVIF removido (breaking, manter residual anotado)
- Crescer cobertura unit de helpers puros (filter, JSON, residual ledger, `dialog_map_key`, `scores_from_lhr`)
- Opcional: extrair famílias grandes de handlers de `commands` quando um domínio novo aterrissar

## Residuais intencionais (não alegar fechamento como paridade completa)

- **GAP-021 parcial:** confiança do parser lighthouse é fixtures unit (minimal + chrome-captured LHR); e2e mock permanece **SKIP** — nunca alegar PASS completo do parser lighthouse em e2e
- **GAP-022 dups residuais:** ~53 multi-versão medidas; poda barata esgotada; residual aceito
- **GAP-023 / GAP-024:** flags/comandos wishlist do PRD permanecem divergências intencionais em `parity_intentional_divergences.json` — não paridade PRD completa
- **Encode AVIF:** removido de `grab` (webp permanece); documentar como residual breaking intencional da 0.1.6

## Inventário completo de agente (65)

Descubra ao vivo: `browser-automation-cli commands --json`

```
assert attr back batch-scrape click-at commands completions config console cookie
crawl devtools3p dialog doctor drag emulate eval exec extension extract fill-form
find-paths forward goto grab heap hover keys lighthouse locale man map mitm monitor
net page parse perf pick press print-pdf qr reload resize run schema scrape screencast
scroll search select-option sg-rewrite sg-scan sheet-write storage submit text type
upload version view wait webmcp workflow write
```

Nota: `pick` e `select-option` são nomes multi-passo de inventário usados em scripts `run`; a contagem de subcomandos clap de produto é 63.

## Explicitamente fora de escopo

- Daemon / serviço de browser long-lived
- OpenTelemetry remoto / dashboards SaaS
- Embedding de servidor MCP
- Orquestração remota de release in-repo / matriz multi-arch cargo-dist

## Profiling (sob demanda)

```bash
./scripts/profile-cdp.sh
# ou: cargo flamegraph --bin browser-automation-cli -- goto about:blank
```

- Artefatos de captura não são commitados
- Use-os localmente para justificar micro-opts
