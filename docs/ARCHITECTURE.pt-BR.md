[English](ARCHITECTURE.md) | [Português Brasileiro](ARCHITECTURE.pt-BR.md)

# Arquitetura — browser-automation-cli

- Automação Chrome CDP one-shot para agentes de IA
- Ciclo de vida sempre: BORN → EXECUTE → FINALIZE → DIE (um processo; sem daemon)

## Camadas

- Binary thin — `src/main.rs` — panic hook, `run_from_args`, exit code
- Lib entry — `src/lib.rs` — `run` / `run_from_args`, hold de telemetria, lifecycle
- Superfície CLI — `src/cli/` — Clap derive (`Parser` / `Subcommand`); help = UX do agente
- Dispatch — `src/commands/` — handlers PRD (`mod.rs` match + `meta` + `run`)
- Session — `src/browser/` — sessão Chrome one-shot, actions, hooks do residual ledger
- Native CDP — `src/native/` — client chromiumoxide, snapshot, heap, cookies, …
- Contract I/O — `src/output.rs`, `src/envelope.rs`, `src/json_util.rs` — envelopes stdout; BrokenPipe → 141
- Lifecycle — `src/lifecycle/` — cancel token, orquestração BORN/FINALIZE, SIGINT/SIGTERM
- Residual disco/processo — `src/residual/` — marker + GC Singleton Chromium tmp; `ResidualDiskReport`
- Tracing local — `src/tracing_local/` — dual sink tracing (stderr + JSON rotativo opcional)
- Config XDG — `src/xdg/`, `src/config.rs` — settings de produto: só flags + XDG `config`
- i18n — `src/i18n/`, `locales/*.ftl` — `--lang` + XDG `lang` → negotiate → OnceLock; só sugestões humanas
- Platform — `src/platform/` — PATH `which_bin`, console UTF-8/VT, HostEnvironment, sandbox do browser
- Windows jobs — `src/win_job.rs` — Job Object para kill residual de processo (stubs fora do Windows)

## Lei de produto residual (processo + disco)

- Residual-zero cobre árvores Chrome vivas e higiene de disco após DIE
- Residual de processo — PID Chrome no ledger (Unix SIGTERM → grace → SIGKILL; Windows Job Object kill-on-close)
- Residual de marker — perfis temp owned da CLI sob `browser-automation-cli-chrome-*`
- Residual Singleton Chromium tmp — `/tmp/org.chromium.Chromium.*` e `/tmp/.org.chromium.Chromium.*` owned, só Singleton (ou vazios), mesmo uid, sem processo vivo segurando o path
- Nunca matar nem apagar árvores Chrome Flatpak do host (ex.: prefixos temp `com.google.Chrome.*`)
- GC cross-run só por shape Singleton + uid + age + sem holder vivo

### Papel de `src/residual/`

- Constantes públicas de prefixo de marker e de Chromium tmp (anti-hardcode)
- Descoberta de side-channels da janela de invocação (atribuição pid/profile)
- GC stale cross-run: `scavenge_stale_singleton_orphans` com age floor 60s (`STALE_MIN_AGE_SECS`)
- Checks de processo vivo via um único índice de cmdline `/proc` (sem rescans O(N×P))
- Relatório máquina: `ResidualDiskReport` / `residual_disk_report()` para doctor e agentes

### Scavenge dual BORN e FINALIZE

- BORN (`Lifecycle::new`) — `scavenge_stale_singleton_orphans` apaga orphans Singleton-only cross-run com age > 60s
- FINALIZE (`Lifecycle::finalize`) — kill/wipe residual do ledger; redescobre side-channels da invocação; `scavenge_owned_chromium_tmp_orphans`; segunda `scavenge_stale_singleton_orphans`
- Drop — safety net síncrono no mesmo path de finalize idempotente
- Dual scavenge no FINALIZE = orphans da janela de invocação mais GC Singleton stale, para o one-shot não deixar lixo de disco para o próximo processo

### Superfície residual do doctor

- Check id: `residual_disk` (path-light; sem launch de Chrome só para o relatório)
- Campo JSON de topo do doctor: `residual` (`ResidualDiskReport`)
- Campos (os dez; uma lista mais curta aqui discordava da struct)
- `scanned_roots` — as raízes realmente percorridas; um zero sem elas é infalsificável
- `cli_marker_dirs` — contagem de `browser-automation-cli-chrome-*` sob as raízes escaneadas
- `chromium_tmp_singleton_orphans` — Chromium tmp Singleton-only com aparência de orphan
- `scavenge_safe_candidates` — paths que o GC stale apagaria agora (age ≥ 60s, owned, sem holder vivo)
- `live_cli_marker_processes` — contagem legada por processo; agentes NÃO DEVEM exigir zero
- `sibling_live_processes` — invocações concorrentes; informativo, nunca reprova
- `orphan_marker_dirs` — dir marker acima do piso de idade cujo owner pid está morto
- `foreign_root_orphans` — PERFIS marker fora das raízes escaneadas, ainda segurados
- `ghost_marker_processes` — browser CLI vivo cujo dir de perfil marker sumiu
- `process_table_unavailable` — enumeração falhou, então todo wipe é recusado
- Status: `fail` em `orphan_marker_dirs` ou `ghost_marker_processes`; `warn` em dirs marker ou orphans Singleton; senão `pass`
- Uma invocação irmã viva é saudável e nunca reprova a checagem

### Como um processo é identificado como browser

- A identidade vem do caminho do executável reportado pelo kernel, nunca do argv
- O argv é escrito pelo próprio processo; o `sysinfo` documenta `cmd[0]` como não confiável para isso
- O predicado é dividido por consequência, porque o mesmo erro custa coisas opostas
- Veredito e reaping são ESTRITOS — executável desconhecido nunca é tratado como browser
- Proteção de wipe é PERMISSIVA — o que puder estar segurando um perfil o mantém vivo
- Ponto cego conhecido: wrappers de sandbox reportam `bwrap` (Flatpak) ou `snap` como raiz da árvore
- As contagens estritas sub-reportam essas raízes, e sub-reportar não reprova host saudável nem sinaliza processo inocente
- Gates locais do mantenedor (só scripts locais do mantenedor): `scripts/residual-check.sh`, `scripts/residual-stress.sh`

## i18n (sugestões humanas)

- Precedência: `--lang` → XDG `lang` → locale do SO (`sys-locale` + `fluent-langneg`) → default `en`
- Packs MVP: `en` + `pt-BR` (`Idioma` / `Mensagem` match exaustivo + paridade FTL)
- JSON máquina `error.message` e tracing ficam em inglês (contrato de agente)
- Packs opcionais: features `i18n-cjk` / `i18n-rtl` / `i18n-europe` / `i18n-full` (scaffold)
- Diagnóstico: subcomando `locale` (+ `--json`)
- Man page: subcomando `man` (roff via clap_mangen; sem Chrome)
- Settings de produto (incluindo idioma) usam só flags + XDG
- Não inventar nem promover variáveis de ambiente de produto para config durável

## Mapa de módulos (`commands`)

- `mod.rs` — match `dispatch` em `Commands` + handlers browser/session  
- `meta/` — inventário `commands` / `schema` para agentes (**69** nomes via `commands --json`; schema em dir SRP)
- `run/` — engine multi-passo `run` / `exec` (passos NDJSON)

### Diálogo multi-aba e settle (v0.1.6)

- **`dialog_map_key`:** helper puro mapeia diálogos JS abertos pela identidade de sessão CDP. O `session_id` do evento vence; browser-scoped `None` cai no id da página ativa.
- **Forwarders de página carimbam `Page::session_id`:** assim `Page.javascriptDialogOpening` / `Closed` de abas não ativas não colidem com a entrada do mapa da aba ativa. Isolamento multi-aba via `Page::session_id` / `dialog_map_key`.
- **`dialog_settled`:** após accept/dismiss, a sessão espera até XDG `dialog_settle_ms` por `javascriptDialogClosed` e devolve um booleano compacto (agent-first; consumidores não inventam wait pós-settle). GAP-054.
- **`dialog_settle_ms`:** chave de config XDG apenas (`config set dialog_settle_ms`); nunca env de produto.
- **Orçamento de domain enable em `tab_switch`:** ao trocar de aba sob diálogo modal de página, o enable de domínio é best-effort sob `TAB_SWITCH_DOMAIN_ENABLE_BUDGET_MS` para o caminho de switch não travar.

### Wait / scrape / select em run (v0.1.6)

- **`wait_timeout_ms`:** chave pública nos passos wait de run (GAP-053); o parser a honra (não descarte silencioso).
- **Scrape `format`/`formats` em run:** sem monstro HTML quando só texto é pedido (GAP-057).
- **Select nativo:** `pick` / `select-option` despacham `input` e depois `change`, reportam `via: native_select` (GAP-055).
- **Encode do `grab`:** só **png|jpeg|webp**; AVIF removido (breaking).
- Inventário **69** inclui `submit` + `storage` + `image` + `video` + `audio` + `record`; superfície clap de produto é **67** (`pick` / `select-option` são nomes multi-passo de inventário/run).

### Parse puro de LHR lighthouse (v0.1.6)

- **`scores_from_lhr`:** função pura extrai scores de categorias do JSON Lighthouse Result (auditorias 0–1 ou null). Fixtures unit: `scripts/fixtures/lighthouse/minimal_lhr.json` e `chrome_captured_lhr.json` real sanitizado. Caminho mock e2e permanece SKIP (não é alegação de PASS do parser). GAP-021 parcial.
- **GAP-022 residual:** ~53 dups multi-versão aceitos (poda barata esgotada).
- **GAP-023/024:** divergências intencionais de PRD em `parity_intentional_divergences.json`.
- Lei residual-zero de disco da 0.1.5 ainda corrente.
- Config de produto: só flags + XDG (nunca env de produto).

## Operações de saída de agente (`agent_ops`)
- `src/agent_ops/` aplica oito operações universais sobre `data` antes do stdout
- Uma única implementação cobre os 69 comandos, inclusive os que ninguém ligou localmente
- Quatro das flags globais são `--fields`, `--filter-rows`, `--limit-rows`, `--sort-rows`
- As outras quatro são `--dedupe-by`, `--count-only`, `--truncate-content`, `--max-output-bytes`
- `--select`, `--filter`, `--limit` e `--sort` não são flags globais
- Elas existem só como flags por comando em `scrape`, `crawl`, `map` e `search`
- `batch-scrape` e os verbos info de mídia também declaram esse conjunto por comando
- O sufixo `-rows` impede o clap de entregar um valor às duas projeções
- Ordem das operações: select, resolver linhas, filter, sort, dedupe, limit
- Depois truncagem de strings, colapso do count-only e o teto de bytes
- `select` roda primeiro porque também desambigua `data` com dois arrays
- O envelope ganha o campo `agent_ops` somente quando alguma flag realmente rodou
- Envelope intocado mantém a forma anterior exata para os consumidores existentes
- Flag que rodou e resolveu limpo também omite o campo, então rodar uma é necessário e não suficiente
- Membros são todos opcionais: `total`, `matched`, `truncated`, `omitted_rows`, `unresolved_paths`
- `unresolved_paths` é uma lista de entradas `{flag, path}`
- Ela nomeia o caminho pedido que nenhuma linha carrega de fato
- Sem ela um caminho digitado errado devolve exit 0 e parece sucesso
- `src/agent_ops/path.rs` tem `project()` devolvendo `(Value, Vec<String>)`
- O segundo elemento é o conjunto de caminhos que não resolveram
- `src/agent_ops/filter.rs` tem a sonda pura `rows_with_key()`
- Ela roda antes de dedupe e sort, que apagariam a evidência
- Operação de linha sobre `data` sem lista única falha como `Usage`

## Colheita de metadados do documento (`html_meta`)
- `src/scrape_local/html_meta.rs` sustenta o formato de scrape `metadata`
- `collect_metadata()` lê o documento já parseado por `build_scrape_payload`
- A cobertura extra custa uma passada de seletor por campo e nenhuma dependência nova
- Famílias colhidas: Open Graph, Dublin Core, `article:` e Twitter card
- Também colhidos: URL canônica, favicon, charset declarado e `html_lang`
- Nomes `<meta>` simples cobrem keywords, author, language, robots, viewport, generator e theme-color
- Prefixos qualificados viram chaves `prefix_name`, sem os dois pontos
- Campos ausentes são omitidos em vez de emitidos como null (stdout CLEAN)
- O favicon tenta `icon`, depois `shortcut icon`, depois `apple-touch-icon`
- `html_lang` vem do atributo `<html lang>`, não da meta tag `language`
- `meta_property()` usa match literal de seletor para prefixos qualificados
- O helper compartilhado adiciona um fallback implícito `og:` a toda busca
- Esse fallback faria `dc:title` responder silenciosamente com `og:title`
- O match literal impede a colheita de reportar campo que a página nunca declarou

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

Nota: `pick` e `select-option` são nomes multi-passo de inventário usados em scripts `run`; a contagem de subcomandos clap de produto é **67**.

- Superfície grande de handlers permanece em `mod.rs` de propósito (tabela match única para parity de agente)
- Prefira extrair famílias novas de comando para módulos irmãos em vez de crescer helpers não relacionados
- Lista completa de nomes: `docs/HOW_TO_USE.pt-BR.md` e `browser-automation-cli commands --json`

## Macros / codegen

- Sem crate pública `macro_rules!` / `proc-macro`
- Stubs de protocolo CDP: `build.rs` + `include!(concat!(env!("OUT_DIR"), "/cdp_generated.rs"))`
- Forwarders de evento: funções genéricas (`spawn_cdp_event_forwarder`), não macros

## Descoberta de browser (multiplataforma)

- Ordem: XDG `chrome_path` → cache de browsers do produto → nomes no `$PATH` → layouts absolutos conhecidos (Linux `/usr`/`/opt`/snap/flatpak, macOS `/Applications`, Windows `%ProgramFiles%` / LocalAppData incluindo Edge/Beta/Canary/Brave) → caches home Puppeteer/Playwright
- Sem env de produto `CHROME_PATH` (lei do produto: só flags + XDG)
- Paths Snap/Flatpak emitem warn via `tracing` e campo `sandbox` do doctor
- Containers/root recebem Chrome `--no-sandbox` + `--disable-dev-shm-usage`
- Probe de host: `doctor --json` → `host_environment` (wsl/container/ci/termux/snap/flatpak)

## Lei de produto (não negociável)

- stdout = só envelopes JSON (agent-first)
- stderr = diagnósticos / tracing
- zero telemetria remota / sem servidor MCP
- residual zero após DIE: processo Chrome + markers CLI + Chromium Singleton tmp (processo e disco)
- nunca matar residual Chrome Flatpak do host
- settings de produto: só flags + XDG (sem catálogos de env de produto)
- sem pipelines remotas de orquestração de release no repositório (gates locais sob `scripts/*-check.sh`)
- Chrome CDP só no host (sem alvo de automação WASM)

## Docs relacionados

- `docs/COOKBOOK.pt-BR.md` — receitas para agentes
- `docs/TESTING.pt-BR.md` — como rodar gates
- `docs/CROSS_PLATFORM.pt-BR.md` — matriz de SO, paths de browser, sandboxes
- `docs/HOW_TO_USE.pt-BR.md` — inventário completo dos **69** comandos
- `gaps.md` — Status v0.1.6 residual DoD + catálogo histórico da auditoria 0.1.5
- `PRIVACY.md` — tratamento de dados só local
