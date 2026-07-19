[English](ARCHITECTURE.md) | [Português Brasileiro](ARCHITECTURE.pt-BR.md)

# Arquitetura — browser-automation-cli

- Automação Chrome CDP one-shot para agentes de IA
- Ciclo de vida sempre: BORN → EXECUTE → FINALIZE → DIE (um processo; sem daemon)

## Camadas

- Binary thin — `src/main.rs` — panic hook, `run_from_args`, exit code
- Lib entry — `src/lib.rs` — `run` / `run_from_args`, hold de telemetria, lifecycle
- Superfície CLI — `src/cli.rs` — Clap derive (`Parser` / `Subcommand`); help = UX do agente
- Dispatch — `src/commands_prd/` — handlers PRD (`mod.rs` match + `meta` + `run`)
- Session — `src/browser/` — sessão Chrome one-shot, actions, hooks do residual ledger
- Native CDP — `src/native/` — client chromiumoxide, snapshot, heap, cookies, …
- Contract I/O — `src/output.rs`, `src/envelope.rs`, `src/json_util.rs` — envelopes stdout; BrokenPipe → 141
- Lifecycle — `src/lifecycle.rs` — cancel token, orquestração BORN/FINALIZE, SIGINT/SIGTERM
- Residual disco/processo — `src/residual.rs` — marker + GC Singleton Chromium tmp; `ResidualDiskReport`
- Telemetria — `src/telemetry.rs` — dual sink tracing (stderr + JSON rotativo opcional)
- Config XDG — `src/xdg.rs`, `src/config.rs` — settings de produto: só flags + XDG `config`
- i18n — `src/i18n/`, `locales/*.ftl` — `--lang` + XDG `lang` → negotiate → OnceLock; só sugestões humanas
- Platform — `src/platform.rs` — PATH `which_bin`, console UTF-8/VT, HostEnvironment, sandbox do browser
- Windows jobs — `src/win_job.rs` — Job Object para kill residual de processo (stubs fora do Windows)

## Lei de produto residual (processo + disco)

- Residual-zero cobre árvores Chrome vivas e higiene de disco após DIE
- Residual de processo — PID Chrome no ledger (Unix SIGTERM → grace → SIGKILL; Windows Job Object kill-on-close)
- Residual de marker — perfis temp owned da CLI sob `browser-automation-cli-chrome-*`
- Residual Singleton Chromium tmp — `/tmp/org.chromium.Chromium.*` e `/tmp/.org.chromium.Chromium.*` owned, só Singleton (ou vazios), mesmo uid, sem processo vivo segurando o path
- Nunca matar nem apagar árvores Chrome Flatpak do host (ex.: prefixos temp `com.google.Chrome.*`)
- GC cross-run só por shape Singleton + uid + age + sem holder vivo

### Papel de `src/residual.rs`

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
- `cli_marker_dirs` — contagem de `browser-automation-cli-chrome-*` sob temp
- `chromium_tmp_singleton_orphans` — Chromium tmp Singleton-only com aparência de orphan
- `scavenge_safe_candidates` — paths que o GC stale apagaria agora (age ≥ 60s, owned, sem holder vivo)
- `live_cli_marker_processes` — processos vivos cuja cmdline contém o prefixo marker chrome da CLI
- Status: `fail` se há processos marker vivos; `warn` se restam dirs marker ou orphans Singleton; senão `pass`
- Gates locais do mantenedor (sem exigência de CI/GHA): `scripts/residual-check.sh`, `scripts/residual-stress.sh`

## i18n (sugestões humanas)

- Precedência: `--lang` → XDG `lang` → locale do SO (`sys-locale` + `fluent-langneg`) → default `en`
- Packs MVP: `en` + `pt-BR` (`Idioma` / `Mensagem` match exaustivo + paridade FTL)
- JSON máquina `error.message` e tracing ficam em inglês (contrato de agente)
- Packs opcionais: features `i18n-cjk` / `i18n-rtl` / `i18n-europe` / `i18n-full` (scaffold)
- Diagnóstico: subcomando `locale` (+ `--json`)
- Man page: subcomando `man` (roff via clap_mangen; sem Chrome)
- Settings de produto (incluindo idioma) usam só flags + XDG
- Não inventar nem promover variáveis de ambiente de produto para config durável

## Mapa de módulos (`commands_prd`)

- `mod.rs` — match de `dispatch` em `Commands` + handlers de browser/session
- `meta.rs` — inventário `commands` / `schema` para agentes (63 nomes via `commands --json`)
- `run.rs` — engine multi-passo `run` / `exec` (passos NDJSON)
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
- sem GitHub Actions / CD no repositório (gates locais sob `scripts/*-check.sh`)
- Chrome CDP só no host (sem alvo de automação WASM)

## Docs relacionados

- `docs/COOKBOOK.pt-BR.md` — receitas para agentes
- `docs/TESTING.pt-BR.md` — como rodar gates
- `docs/CROSS_PLATFORM.pt-BR.md` — matriz de SO, paths de browser, sandboxes
- `docs/HOW_TO_USE.pt-BR.md` — inventário completo dos 63 comandos
- `gaps.md` — catálogo `/r-auditoria` (RES-01…12 fechados no Pass 27)
- `PRIVACY.md` — tratamento de dados só local
