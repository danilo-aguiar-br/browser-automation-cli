[English](ROADMAP.md) | [Português Brasileiro](ROADMAP.pt-BR.md)

# Roadmap (notas de mantenedor)

- Este projeto embarca como CLI one-shot estável
- O roadmap é intencionalmente curto

## Curto prazo (qualidade local)

- **DoD residual da v0.1.6 alcançado** (medido 2026-07-31): GAP-054 settle de diálogo + multi-aba (booleano `dialog_settled`; XDG `dialog_settle_ms`), GAP-055 select nativo, GAP-057 format de scrape em run, GAP-053 `wait_timeout_ms`, lei residual-zero de disco da 0.1.5 ainda corrente
- A v0.1.7 fechou o erro de teto que o `doctor` engolia
- A v0.1.7 fechou o caminho não resolvido silencioso em agent-ops, agora em `unresolved_paths`
- A v0.1.7 fechou a sugestão i18n que citava uma flag inexistente
- A v0.1.7 fechou o alias falso de `rawHtml`, agora bruto contra `html` processado
- A v0.1.7 expandiu o `metadata` além dos cinco campos originais
- A v0.1.7 fechou o `--urls-file` que aceitava entrada sem teto
- A v0.1.7 promoveu nove chaves XDG e documentou toda a superfície XDG
- A v0.1.7 acrescentou dois gates novos ao passe de auditoria
- Superfície XDG viva: 176 chaves documentadas em `docs/CONFIGURATION.md`
- Manter gates `scripts/*-check.sh` verdes em cada passe de auditoria (incl. `scripts/residual-check.sh` / `scripts/residual-stress.sh` para residual-zero em disco)
- Suite opcional de confiança: reexecutar `dialog_multitab_gate`, `option_pick_gate`, `wait_conditions_gate`, `scrape_step_gate`, fixtures unit LHR de lighthouse após refactors grandes
- Inventário vivo de agentes: **69** nomes via `commands --json` (inclui `submit`, `storage`, `image`+`video`+`audio`+`record`, `locale`, `man`)
- Settings de produto: só flags + XDG `config` (sem variáveis de ambiente de produto); descubra chaves via `config list-keys --json`
- **Encode do `grab`:** só png|jpeg|webp; AVIF removido (breaking, manter residual anotado)
- Crescer cobertura unit de helpers puros (filter, JSON, residual ledger, `dialog_map_key`, `scores_from_lhr`)
- Opcional: extrair famílias grandes de handlers de `commands` quando um domínio novo aterrissar

## Residuais intencionais (não alegar fechamento como paridade completa)

- **GAP-021 parcial:** confiança do parser lighthouse é fixtures unit (minimal + chrome-captured LHR); e2e mock permanece **SKIP** — nunca alegar PASS completo do parser lighthouse em e2e
- **GAP-022 dups residuais:** ~53 multi-versão medidas; poda barata esgotada; residual aceito
- **GAP-023 / GAP-024:** flags/comandos wishlist do PRD permanecem divergências intencionais em `parity_intentional_divergences.json` — não paridade PRD completa
- **Encode AVIF:** removido de `grab` (webp permanece); documentar como residual breaking intencional da 0.1.6
- Decode de AVIF segue fechado por limite físico, não por prioridade
- Encode de HEIC segue fechado pelo mesmo limite físico
- Extração de mídia que exige execução de JavaScript ofuscado segue fechada
- Todo recurso que depende de serviço remoto segue fechado por design

## Aberto, sem prazo assumido
- `scrape` não tem o formato `attributes`
- `scrape` não tem o formato `changeTracking`
- `search` não tem filtro temporal; dez dimensões continuam faltando
- `crawl` não aceita regex em include e exclude, e não há `regexOnFullURL`
- `parse` não aplica formatos de scrape ao arquivo processado
- `crawl` e `batch-scrape` não têm `--webhook-url`, que o `scrape` já tem
- Estes itens não têm data e NÃO DEVEM ser lidos como promessa

## Inventário completo de agente (69)

Descubra ao vivo: `browser-automation-cli commands --json`

```
assert attr back batch-scrape click-at commands completions config console cookie
crawl devtools3p dialog doctor drag emulate eval exec extension extract fill-form
find-paths forward goto grab heap hover image video audio keys lighthouse locale man map mitm monitor
net page parse perf pick press print-pdf qr reload resize run schema scrape screencast
scroll search select-option sg-rewrite sg-scan sheet-write storage submit text type
upload version view wait webmcp workflow write
```

Nota: `pick` e `select-option` são nomes multi-passo de inventário usados em scripts `run`; a contagem de subcomandos clap de produto é **67** (69 nomes de agente − 2 só-run).

### Mídia local (image/video) — não-metas intencionais (Wave C TREATED)

- **No produto agora:** path→path `image` / `video` (magic, download SSRF, convert/remux, to-mp3, trim, thumbnail, manifest); ffmpeg/ffprobe opcional via XDG `ffmpeg_path` (sem libav linkado).
- **No produto agora:** `video manifest` resume a estrutura de HLS `.m3u8` e DASH `.mpd` sem baixar mídia.
- **Fora do produto (honesty):** playback adaptativo HLS/DASH, downloaders yt-dlp/site, encode pure-Rust de produção, batch multi-file JoinSet. Agentes usam ferramentas externas ou design futuro — não alegar como shipped.

## Explicitamente fora de escopo

- Daemon / serviço de browser long-lived
- OpenTelemetry remoto / dashboards SaaS
- Embedding de servidor MCP
- Orquestração remota de release in-repo / matriz multi-arch cargo-dist
- HLS/DASH / yt-dlp core / encode video pure-Rust (ver Wave C TREATED acima)

## Profiling (sob demanda)

```bash
./scripts/profile-cdp.sh
# ou: cargo flamegraph --bin browser-automation-cli -- goto about:blank
```

- Artefatos de captura não são commitados
- Use-os localmente para justificar micro-opts
