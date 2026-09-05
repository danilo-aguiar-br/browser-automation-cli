[English](INTEGRATIONS.md) | [Português Brasileiro](INTEGRATIONS.pt-BR.md)

# Integrações — browser-automation-cli

> Um processo, um Chrome, um envelope JSON. Feito para subprocessos de agentes.

## Snapshot de Cobertura
- Funciona com qualquer agente que dispare subprocesso e leia stdout mais stderr
- Superfícies primárias: Claude Code, Codex, Cursor, shell local, agentes de editor
- Helpers de descoberta: `commands --json`, `schema <cmd>` ou `schema --cmd`, `doctor --json`
- O caminho de integração é apenas subprocesso local
- Settings de produto são flags mais config XDG apenas

## Aliases de Flags e Notas de Versão
- Nomes de produto ficam fixos: `view`, `press`, `write`, `grab`
- Evite inventar aliases como `click` ou `screenshot` em prompts de agente (use `grab` para screenshots; scrape pode aceitar token de format `screenshot`)
- Use `grab --path <file>` (não path posicional bare)
- Use `wait --text` repetível para semântica OR entre várias strings
- Use `scrape --format` / `scrape --engine` para formatos de scrape local
- Scrape browser aplica `--format` via outerHTML; 15 formatos vivos: `text`, `markdown`, `html`, `rawHtml`, `links`, `metadata`, `screenshot`, `summary`, `product`, `branding`, `images`, `jsonld`, `json`, `feed`, `attributes` (`raw-html` continua alias aceito de `rawHtml`)
- `0.1.0` entrega a superfície de paridade DevTools default-on mais gates de categoria
- `0.1.1` adiciona `config` XDG, MITM local, journal de workflow e superfície local scrape/crawl/map/search/parse (`batch-scrape`, `crawl`, `map`, `search`, `parse`, `scrape` expandido)
- `0.1.2` fecha gaps agent-first e adiciona `print-pdf`, `monitor`, `qr`, `find-paths`, tipos de documento no parse, extract LLM e chaves de config expandidas
- `0.1.3` fecha residual-zero e contratos de agente: `run` NDJSON|array JSON, reload/beforeunload/init_script CDP, honestidade Redis/Lighthouse, `sheet-write`/`sg-scan`/`sg-rewrite`, `find-paths --glob` (59 comandos de topo; 53 tools DevTools e2e)
- `0.1.4` fecha gaps agent-first: `--json-steps`, `wait` url/navigation/multi-seletor, `select-option`/`pick` (run/schema), assert `console_*`, `schema <cmd>` posicional, MITM `capture-url` + `--mitm*`, scrape multi-formato, batch/crawl `--engine browser`, `print-pdf` no `run`
- `0.1.5` fecha residual-zero de disco (RES-01…12): BORN auto-GC de dirs Chromium Singleton-only em `/tmp` (piso de idade 60s), FINALIZE dual scavenge + re-scan, `doctor residual_disk` + campo de topo `residual` (`ResidualDiskReport`), nunca mata Chrome Flatpak do host; honestidade de inventário com `locale`/`man`
- `0.1.6` fecha confiança agent-first de diálogo/select/scrape/wait: booleano `dialog_settled` + XDG `dialog_settle_ms`, isolamento multi-aba de diálogo por `session_id` com gate e2e, select nativo `input`+`change`, `wait_timeout_ms` em `run`, scrape `format`/`formats` em `run`, grab só `png|jpeg|webp` (encode AVIF removido); inventário tip 0.1.8 era 69 via `commands --json` (0.1.6: `submit`/`storage` → 65; 0.1.7: `image`+`video`+`audio` → 68 depois `record` → 69; também `select-option`, `pick`); e2e TOTAL=53 PASS=52 SKIP=1 (mock lighthouse SKIP honesto)
- `0.1.8` fecha anti-detecção e controle de saída: família stealth (`--no-stealth`, `--stealth-profile`, `--stealth-seed`), modo de janela pela chave XDG `browser_mode` mais `--no-xvfb`, proxy de saída (`--proxy`, `--proxy-bypass`) valendo para o Chrome e para o motor HTTP, chaves de fingerprint HTTP/2 constante, cinemática humana de input (`--input-profile`, `--input-seed`), warmup de sessão (`--warmup`, `--warmup-url`), asserções sobre o payload (`--expect`, `--expect-exit-code`) e `config unset <KEY>`; a superfície de configuração cresce de 176 para **204** chaves enquanto o inventário tip da 0.1.8 permanecia 69 via `commands --json`
- Superfície viva (v0.1.9): **217** chaves XDG via `config list-keys --json` (o 204 fica no parágrafo da 0.1.8 acima); `doctor --fingerprint` acrescenta `measurement_scope` / `unmeasured_os` (não são chaves XDG); `emulate`/`resize` `screen` aplica CDP; o plano `--no-stealth` do fingerprint casa com a página
- Ferramentas experimentais exigem `--experimental-vision` ou `--experimental-screencast`

## Tabela Resumo

| Superfície | Estilo de integração | Flags exigidas | Notas |
|------------|----------------------|----------------|-------|
| Claude Code | subprocesso | `--json` | multi-passo via `run --script` (NDJSON ou array JSON; opcional `--json-steps`) |
| Codex | subprocesso | `--json -q` | stderr quieto para transcripts limpos |
| Cursor | shell tool | `--json` | deixe timeouts explícitos |
| Shell local | script | `--json` | parse com `jaq` |
| Continue / Cline | shell do editor | `--json -q` | apenas one-shot |

## Claude Code
- Dispare um processo CLI por ação atômica
- Use `run --script` (NDJSON ou array JSON) quando refs `@eN` precisarem sobreviver a vários passos
- Prefira XDG `config set` para defaults duráveis
```bash
browser-automation-cli doctor --offline --quick --json
browser-automation-cli --json goto https://example.com
browser-automation-cli --json view
browser-automation-cli --json run --script /tmp/steps.jsonl
browser-automation-cli --json --json-steps run --script /tmp/steps.jsonl
```
- `--script` é caminho de arquivo, nunca JSON inline; `/tmp/steps.jsonl` guarda um objeto de passo por linha:
```json
{"cmd":"goto","url":"https://example.com"}
{"cmd":"view"}
```

## Codex
- Prefira `-q --json` para que só envelopes cheguem ao transcript do agente
```bash
browser-automation-cli -q --json goto https://example.com
```

## Cursor
- Chame o binário da shell tool com `--timeout` explícito
```bash
browser-automation-cli --timeout 60 --json scrape https://example.com --format markdown --engine http
```

## Shell Local
- Sempre capture exit codes antes de parsear JSON
- Rode validações na sua máquina local antes do release
```bash
out=$(browser-automation-cli --json version)
echo "$out" | jaq -e '.ok == true'
```

## Continue e Cline
- Use modo JSON quieto para manter transcripts do editor limpos
- Não espere stickiness de sessão entre launches de processos separados

## Novas Flags por Versão
- `0.1.0`: gates de categoria, vision e screencast experimentais, flags de capture, schema discovery
- `0.1.1`: `config` XDG (`init`/`path`/`show`/`get`/`set`), `mitm` (CA local + proxy one-shot em `127.0.0.1`), `workflow` (`run`/`resume`/`status`), superfície local de scrape (`scrape --format/--engine`, `batch-scrape`, `crawl`, `map`, `search`, `parse`), `wait --text` multi OR, `grab --path`
- `0.1.2`:
  - `scrape --engine browser` aplica `--format` via outerHTML nos 15 formatos vivos `text`, `markdown`, `html`, `rawHtml`, `links`, `metadata`, `screenshot`, `summary`, `product`, `branding`, `images`, `jsonld`, `json`, `feed`, `attributes` (`raw-html` continua alias aceito de `rawHtml`)
  - Aliases de scroll em `run` `dy`/`dx` para `delta_y`/`delta_x`; envelopes de erro fail-fast podem incluir `data.steps` parciais
  - `schema --cmd` expandido para `goto`/`eval`/`type`/`scroll`/`assert`
  - `--lang pt-BR` e `config set lang` localizam sugestões humanas
  - Logging via `--verbose`/`--debug` e XDG `log_level`/`chrome_path`/`lighthouse_path` apenas
  - `search` limpa redirects SERP `uddg=`
  - `print-pdf` one-shot CDP; `monitor check --url --baseline [--write-baseline]`
  - `parse` PDF/DOCX/xlsx/ods + `--redact-pii`; `extract --llm --question --schema-json` (XDG `openrouter_api_key`, `llm_base_url`, `llm_model`)
  - `qr encode|decode`, `image info|convert|resize|download|exif`, `video info|download|convert|to-mp3|trim|thumbnail|manifest`, `find-paths`
  - Aliases de `assert` `url_contains`/`text_contains`; fallback de property DOM em `attr`
  - Chaves de config: `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`
  - Inventário de comandos com 56 nomes de topo (`commands --json`), incluindo `print-pdf`, `monitor`, `qr`, `find-paths`
- `0.1.3`:
  - `run --script` aceita NDJSON ou um array JSON de passos; fail-fast pode devolver `data.steps` parciais
  - `reload --ignore-cache` usa CDP `Page.reload` + `ignoreCache`
  - `init_script` é removido após navegação/reload; `handle_before_unload` auto-aceita via diálogo CDP (sem inject de preventDefault)
  - `scrape --engine http` rejeita `file://` com Usage + sugestão browser/parse
  - `find-paths --glob`; `sheet-write` CSV/JSON→XLSX; `sg-scan` / `sg-rewrite` lint estrutural (dry-run por padrão)
  - Lighthouse resolve flag → XDG `lighthouse_path` → PATH; envelope `binary_source` real|mock; doctor reporta origem
  - Redis: XDG `cache_backend` / `cache_redis_url`; `rediss://` fail-closed; doctor `cache_redis`
  - FINALIZE faz scavenge de órfãos Chromium em `/tmp` owned; e2e residual residual-zero
  - Config: `config list-keys`; chaves novas `log_to_file`, `cache_backend`, `cache_redis_url`
  - Inventário de comandos com 59 nomes de topo (`commands --json`), incluindo `sheet-write`, `sg-scan`, `sg-rewrite`
- `0.1.4`:
  - Global `--json-steps`: stream NDJSON por passo (`step`, `cmd`, `ok`, `result`) durante `run`
  - `wait` multi-seletor CSS OR (`#a, #b`), arrays `selectors`, `url` / `url_contains` / `navigation`
  - Multi-passo `select-option` / `pick` (badge/popover / `role=option`; descobertos via `schema` e inventário run)
  - Assert `console_empty` / `console_no_match` (CLI `assert console-empty` / `assert console-no-match --pattern`)
  - `schema <cmd>` posicional além de `schema --cmd`
  - `goto`/`reload` `--handle-before-unload accept|dismiss` (`BeforeUnloadAction`)
  - MITM `capture-url` one-shot + flags globais `--mitm`, `--mitm-ca-dir`, `--mitm-har`, `--mitm-hosts`, `--mitm-ws`, `--mitm-max-body-bytes`, `--mitm-no-media-bodies`, `--mitm-redact-secrets`
  - MITM subcomandos: `status|list|get|har|export|domains|apis|init-ca|start|capture-url|graphql|ws|block|allow|redact`
  - Scrape multi-formato (`--format` repetível/CSV); `batch-scrape` e `crawl` aceitam `--engine browser` (default http)
  - `view --allow-empty`; `print-pdf` no multi-passo `run`; diálogo soft com `--if-present` (GAP-006)
  - Inventário de comandos com 61 nomes de topo (`commands --json`), incluindo `select-option` e `pick`
- `0.1.5`:
  - Higiene residual-zero de disco (product law: residual-zero de processo + disco)
  - BORN auto-GC: `scavenge_stale_singleton_orphans` de dirs `/tmp` `org.chromium.Chromium.*` Singleton-only com mais de 60s
  - FINALIZE dual scavenge + re-scan de dirs marker owned (prefixo `browser-automation-cli-chrome-`); nunca mata Chrome Flatpak do host
  - Checagem doctor `residual_disk` + campo JSON de topo `residual` (`ResidualDiskReport`): `scanned_roots`, `cli_marker_dirs`, `chromium_tmp_singleton_orphans`, `scavenge_safe_candidates`, `live_cli_marker_processes` (legado), `sibling_live_processes`, `orphan_marker_dirs`, `foreign_root_orphans`, `ghost_marker_processes`, `process_table_unavailable`
  - Gates locais de residual: `scripts/residual-check.sh`, `scripts/residual-stress.sh` (somente local)
  - Honestidade de descoberta: inventário inclui `locale` e `man`
  - Inventário (histórico 0.1.5): **63** nomes de agente via `commands --json`
- `0.1.6`:
  - Diálogo: `dialog accept|dismiss` emite booleano `.data.dialog_settled` no happy path; XDG `config set dialog_settle_ms` orça a espera por `Page.javascriptDialogClosed`
  - Isolamento multi-aba: forwarders carimbam `session_id`; gate `tests/dialog_multitab_gate.rs`; `tab_switch` com enable de domínios best-effort sob diálogo modal aberto
  - Select: `input`+`change` nativos para `pick` / `select-option` (helper de dispatch compartilhado)
  - Run: `wait_timeout_ms` público nos passos wait; scrape com `format`/`formats` (texto compacto sem monstro HTML quando só text)
  - Grab: `--format png|jpeg|webp` apenas — encode AVIF removido
  - Lighthouse: fixtures unitárias com LHR capturado (forma 13.4.1); e2e mock permanece SKIP (nunca alegar PASS de parser a partir do mock)
  - A ponta do inventário na 0.1.6 era 65 nomes de agente via `commands --json`, depois que `submit` e `storage` se juntaram a `select-option` e `pick`
  - Descubra o conjunto completo de chaves com `config list-keys --json` (não é contagem fixa de 16)
  - Residual intencional: GAP-022 ~53 multi-versões de dependência; GAP-023/024 wishlist PRD sem paridade completa
- `0.1.8`:
  - Anti-detecção: `--no-stealth`, `--stealth-profile auto|chrome-linux|chrome-win|chrome-mac`, `--stealth-seed <SEED>`; XDG `stealth` (padrão true), `stealth_profile`, `stealth_seed`
  - Modo de janela: XDG `browser_mode` (`auto|headed|headless`; `auto` resolve para headless e o `doctor` reporta o modo efetivo); `--no-xvfb` pula o display virtual privado no Linux
  - Proxy de saída: `--proxy <URL>` (`http`, `https`, `socks5`) e `--proxy-bypass <HOSTS>` valem para o Chrome **e** para o motor HTTP; XDG `proxy_url`, `proxy_bypass`, `proxy_username`, `proxy_password`, `cdp_proxy_bypass_loopback` (padrão true)
  - Fingerprint HTTP/2: XDG `http2_enabled` (padrão true), `http2_initial_stream_window_size` (6291456), `http2_initial_connection_window_size` (15663105), `http2_max_header_list_size` (262144), `http2_max_frame_size` (16384), `http2_adaptive_window` (padrão false, porque desligado mantém o fingerprint constante)
  - Cinemática humana de input: `--input-profile human|direct` (padrão `human`) e `--input-seed <SEED>`; XDG `input_profile`, `input_move_steps` (24), `input_move_gap_ms` (12), `input_click_dwell_ms` (65), `input_key_dwell_ms` (45), `input_type_delay_ms` (95), `input_scroll_tick_px` (100), `input_scroll_max_ticks` (40), `input_target_jitter_px` (3), `input_scroll_settle_rounds` (3)
  - Warmup de sessão: `--warmup` e `--warmup-url <URL>`
  - Asserções sobre o payload: `--expect <EXPR>` com `key=value`, `key!=value` ou `key~substring`, repetível e conjugado por AND; `--expect-exit-code` sai com 65 quando alguma asserção falha, desligado por padrão porque mudar o exit code por conteúdo de dado quebraria chamadores em silêncio
  - `config unset <KEY>` restaura uma chave ao default embutido
  - Chaves avulsas novas: `robots_user_agent`, `scrape_no_cache`, `monitor_diff_max_bytes`
  - A superfície de configuração cresce de 176 para **204** chaves (`config list-keys --json`)
  - A ponta do inventário permaneceu em 69 nomes de agente via `commands --json`: a 0.1.8 acrescentou flags e chaves, nunca um comando
- `0.1.9`:
  - Comandos novos `sitemap` e `feed`, o primeiro crescimento de inventário desde a 0.1.7. Nenhum dos dois acrescenta capacidade: `sitemap <url>` é `map --sitemap-only` e `feed <url>` é `scrape --formats feed --engine http`. Eles existem por DESCOBRIBILIDADE, porque numa CLI voltada a agente uma capacidade alcançável só por saber que uma flag de um verbo com outro nome a carrega é, na prática, inalcançável
  - `sitemap <url>` aceita `--limit`, `--select`, `--include-path`, `--exclude-path`, `--search`, `--sort`, `--dedup-key`, `--include-subdomains`, `--ignore-query-params`. Não existe `--depth`: um sitemap é uma lista DECLARADA, não uma fronteira, então não há grafo de links a limitar
  - `feed <url>` aceita `--select`, `--header`, `--no-cache`. As flags que moldam HTML estão ausentes em vez de ignoradas, porque `ScrapeFormat::Feed` parseia o corpo BRUTO e a redução por seletor destruiria um documento XML ou JSON; o Chrome não é oferecido porque renderizar um feed produz o visualizador XML do navegador
  - `doctor --fingerprint` acrescenta `stealth_installed`, `stealth_seed_active`, `measurement_scope_matches_host`, `measurement_scope` (`linux-headless-xvfb`), `unmeasured_os`, `stealth_profile_source`, `fonts_method` e os derivados `gpu_source` / `fonts_source` / `audio_source`; novo mismatch de coerência `stealth_not_installed`
  - `--stealth-profile list` imprime os quatro tokens sem lançar o Chrome; `commands --json` também emite `stealth_profiles`, `stealth_seed_fields` e `stealth_seed_does_not_vary`
  - `--min-delay-ms` define o piso de cortesia por origem a cada invocação; a espera efetiva é o MÁXIMO entre a flag, o piso XDG `scrape_min_delay_ms` e o `Crawl-delay` do site
  - `--max-items` é aceito como alias de `--limit-rows`; ele limita o que é EMITIDO, enquanto o `--limit` de um comando limita o que é BUSCADO
  - `crawl --include-regex` / `--exclude-regex` / `--sitemap-only`; `--webhook-url` em `crawl` e `batch-scrape`; `map --include-subdomains` e `map --ignore-query-params`; `search --include-domains`, `--exclude-domains`, `--country`, `--search-lang`, `--time-filter`
  - `map` restringe os resultados ao host semente por padrão; `--include-subdomains` amplia, e não existe mais forma de fazer o `map` devolver hosts externos arbitrários
  - `parse --format` deriva formatos de scrape de um arquivo parseado; `heap take --url` navega antes de capturar; `sheet-write --force`; `--paths-file` em treze ações de `image` / `video` / `audio`; `mitm capture-url --capture-hosts`
  - `cookie clear` exige `--all`, e `mitm block` exige `--host` ou `--path`: verbo irreversível toma o escopo do argv, nunca da ausência de uma flag
  - `--timeout` é limitado a 86400 segundos; `--schema-json` passa pela mesma jaula de sistema de arquivos do `run --script`
  - Todo verbo com efeito colateral publica `target_resolved` e `target_source` (`argv` / `step` / `xdg` / `ambient`), o contrato de Explicit Target Designation, verificado por `tests/etd_gate.rs`
  - Chaves XDG novas `screen` (`WxH`), `platform_child_poll_ms`, `extension_attach_poll_iters`, `user_data_dir` (perfil persistente do Chrome, opt-in, ausente por padrão, e deixá-la ausente é o que mantém o residual-zero verdadeiro), `input_typo_permille` (`0`) e `capture_preserved_rings` (`3`); dezoito chaves que eram aceitas e ignoradas em runtime estão ligadas
  - Superfície de configuração: **217** chaves via `config list-keys --json` (o número 204 pertence ao parágrafo da 0.1.8 acima)
  - A ponta do inventário é **71** nomes de agente via `commands --json`; a superfície de topo do clap é 69, porque `select-option` e `pick` seguem sendo nomes de multi-step sem verbo autônomo
