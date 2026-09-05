[English](ROADMAP.md) | [Português Brasileiro](ROADMAP.pt-BR.md)

# Roadmap (notas de mantenedor)

- Este projeto embarca como CLI one-shot estável
- O roadmap é intencionalmente curto

## Curto prazo (qualidade local)

- A v0.1.9 é o release CORRENTE e as linhas abaixo descrevem o estado vivo
- A v0.1.8 entregou a família anti-detecção e fechou os gaps G2, G4, G8, G9, G11 e G13; a 0.1.9 fechou identidade/tela/FTL, fecha emulate só-screen via CDP, o plano webdriver do `--no-stealth` e o cartão da frente do README, nomeia o recorte vivo do fingerprint e acrescenta gates chrome-mac / eval-nav
- Superfície XDG viva: 217 chaves documentadas em `docs/CONFIGURATION.md`
- Inventário vivo de agentes: 71 nomes via `commands --json`
- O inventário inclui `submit`, `storage`, `image`, `video`, `audio`, `record`, `locale` e `man`
- A v0.1.9 acrescentou os verbos `sitemap` e `feed`, que levaram o inventário vivo de 69 para **71**
- Os dois delegam para um motor que já respondia, sem uma linha de lógica duplicada
- `map --sitemap-only` já devolvia as URLs do sitemap e `scrape --formats feed` já devolvia o feed processado
- A razão para nomeá-los como verbos foi descobribilidade, nunca comportamento novo
- `download`, `agent` e `stats` NÃO estão implementados
- O PRD dá uma linha de descrição para cada, sem assinatura, sem envelope e sem critério de aceite
- Isso é fronteira declarada, não débito escondido
- Settings de produto são só flags mais XDG `config`, nunca variáveis de ambiente de produto
- Descubra chaves com `config list-keys --json` em vez de confiar em lista estática
- Entregue desde a v0.1.7: `scrape --format attributes` com `--attribute-selector` e `--attribute-name`
- Entregue: `parse --format` deriva formatos de scrape do arquivo processado
- Entrada não-HTML do `parse` não tem DOM, então aceita só text, markdown e summary
- Manter gates `scripts/*-check.sh` verdes em cada passe de auditoria
- Os gates de residual-zero em disco são `scripts/residual-check.sh` e `scripts/residual-stress.sh`
- Suite opcional de confiança: reexecutar `dialog_multitab_gate`, `option_pick_gate`, `wait_conditions_gate` e `scrape_step_gate`
- Reexecutar também as fixtures unit LHR de lighthouse após refactors grandes
- O encode do `grab` é só png, jpeg e webp, porque o AVIF foi removido
- Crescer cobertura unit de helpers puros como `dialog_map_key` e `scores_from_lhr`
- Opcional: extrair famílias grandes de handlers de `commands` quando um domínio novo aterrissar

### Família anti-detecção (v0.1.8)

- A v0.1.8 aplica patches no browser antes da primeira navegação sob XDG `stealth`, padrão true
- A v0.1.8 acrescentou `stealth_profile`, padrão `auto`, e `stealth_seed`, que não tem padrão
- `stealth_seed` fixa a identidade personificada entre processos quando você o define
- As flags globais `--no-stealth`, `--stealth-profile` e `--stealth-seed` sobrescrevem esses valores XDG
- A v0.1.8 acrescentou controle de fingerprint HTTP/2 sob `http2_enabled`, padrão true
- `http2_initial_stream_window_size` tem padrão 6291456
- `http2_initial_connection_window_size` tem padrão 15663105
- `http2_max_header_list_size` tem padrão 262144 e `http2_max_frame_size` tem 16384
- `http2_adaptive_window` completa essa família de fingerprint HTTP/2
- A v0.1.8 acrescentou as chaves de proxy `proxy_url`, `proxy_bypass`, `proxy_username` e `proxy_password`
- Credenciais de proxy pertencem ao XDG, porque o argv é visível na tabela de processos
- `cdp_proxy_bypass_loopback` tem padrão true para o canal de controle CDP sobreviver ao proxy
- As flags globais `--proxy` e `--proxy-bypass` cobrem o lado argv da mesma família
- A v0.1.8 acrescentou cinemática humana de input sob `input_profile`, padrão `human`
- `input_move_steps` é 24, `input_move_gap_ms` é 12 e `input_click_dwell_ms` é 65
- `input_key_dwell_ms` é 45, `input_type_delay_ms` é 95 e `input_scroll_tick_px` é 100
- `input_scroll_max_ticks` é 40, `input_target_jitter_px` é 3 e `input_scroll_settle_rounds` é 3
- As flags globais `--input-profile` e `--input-seed` sobrescrevem a cinemática por processo
- A v0.1.8 acrescentou `browser_mode`, padrão `auto`, alcançável por XDG e NÃO por flag
- A v0.1.8 acrescentou também `robots_user_agent`, `scrape_no_cache` e `monitor_diff_max_bytes`, padrão 65536
- A v0.1.8 deu consumidores reais a `--mitm-max-body-bytes`, `--mitm-no-media-bodies` e `--mitm-redact-secrets`
- A v0.1.8 acrescentou `--mitm-no-redact-secrets`, a única forma de desligar a mascaração de segredos
- Pedir mascarar e desmascarar ao mesmo tempo resolve para mascarar, que é a leitura segura
- A v0.1.8 unificou o envelope de `scrape`, então a aridade de `--format` não muda mais o conjunto de chaves
- `formats` e `format_list` estão sempre presentes, e `--fields` agora projeta nos dois casos
- O envelope de `scrape` com um formato agora reporta `stealth`, `http2_profile` e `tls_impersonation`
- Ele reporta também `header_order_controlled`, `fingerprint_stable_across_processes` e `profile_contradicts_host`
- `cookie_jar_persistent` fecha esse bloco de telemetria, medido em 2026-08-10

### Histórico (NÃO ler como estado corrente)

- A v0.1.6 fechou o GAP-054 de settle e multi-aba, com `dialog_settled` e XDG `dialog_settle_ms`
- A v0.1.6 fechou o GAP-055 select nativo, o GAP-057 format em run e o GAP-053 `wait_timeout_ms`
- A v0.1.6 manteve a lei residual-zero de disco herdada da v0.1.5
- A v0.1.7 fechou o erro de teto que o `doctor` engolia
- A v0.1.7 fechou o caminho não resolvido silencioso em agent-ops, agora em `unresolved_paths`
- A v0.1.7 fechou a sugestão i18n que citava uma flag inexistente
- A v0.1.7 fechou o alias falso de `rawHtml`, agora bruto contra `html` processado
- A v0.1.7 expandiu o `metadata` além dos cinco campos originais
- A v0.1.7 fechou o `--urls-file` que aceitava entrada sem teto
- A v0.1.7 promoveu nove chaves XDG e documentou toda a superfície XDG
- A v0.1.7 acrescentou dois gates novos ao passe de auditoria

## Residuais intencionais (não alegar fechamento como paridade completa)

- **GAP-021 parcial:** confiança do parser lighthouse é fixtures unit (minimal + chrome-captured LHR); e2e mock permanece **SKIP** — nunca alegar PASS completo do parser lighthouse em e2e
- **GAP-022 dups residuais:** ~53 multi-versão medidas; poda barata esgotada; residual aceito
- **GAP-023 / GAP-024:** flags/comandos wishlist do PRD permanecem divergências intencionais — não paridade PRD completa
- **Encode AVIF:** removido de `grab` (webp permanece); documentar como residual breaking intencional da 0.1.6
- Decode de AVIF segue fechado por limite físico, não por prioridade
- Encode de HEIC segue fechado pelo mesmo limite físico
- Extração de mídia que exige execução de JavaScript ofuscado segue fechada
- Todo recurso que depende de serviço remoto segue fechado por design
- Anti-detecção é melhor esforço, e NENHUM perfil stealth garante evasão de um detector dado

### Capacidade local de scrape vs fronteira

- Local e entregue: scrape multi-formato, crawl, map, extract via LLM (OpenRouter no XDG), webhook one-shot, MITM local + HAR, `change_status` local (`fresh`/`unchanged`), `--with-content-hash` opt-in
- Fronteira (não é débito 0.1.9): serviço remoto de change, JA4 no motor HTTP (exige TLS C), ordem de headers no motor HTTP (`HeaderMap` do reqwest não itera por inserção), solvers de desafio
- Reconhecimento de texto em imagem dentro do processo não é fronteira; foi removido de propósito e não volta, porque toda LLM que chama já tem visão

## Aberto, sem prazo assumido
- `scrape` não tem o formato `changeTracking`
- `search` não tem filtro temporal, e dez dimensões continuam faltando
- `browser_mode` só é alcançável por XDG, porque nenhuma flag CLI o expõe
- Estes itens não têm data e NÃO DEVEM ser lidos como promessa

## Inventário completo de agente (71)

Descubra ao vivo: `browser-automation-cli commands --json`

```
assert attr back batch-scrape click-at commands completions config console cookie
crawl devtools3p dialog doctor drag emulate eval exec extension extract feed fill-form
find-paths forward goto grab heap hover image video audio keys lighthouse locale man map mitm monitor
net page parse perf pick press print-pdf qr record reload resize run schema scrape screencast
scroll search select-option sg-rewrite sg-scan sheet-write sitemap storage submit text type
upload version view wait webmcp workflow write
```

Nota: `pick` e `select-option` são nomes multi-passo de inventário usados em scripts `run`; a contagem de subcomandos clap de produto é **69** (71 nomes de agente − 2 só-run).

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
