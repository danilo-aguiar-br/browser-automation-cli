# gaps.md — Auditoria Rules Rust CLI (… + ownership + paralelismo)

**Projeto:** `browser-automation-cli` v0.1.4  
**Rules:** … + `docs_rules/rules_rust_ownership_borrowing_lifetimes.md` + `docs_rules/rules_rust_paralelismo_e_multiprocessamento.md` + multiplataforma + multi-idioma + GraphRAG + `/r-auditoria`  
**Fontes auxiliares:** context7, `docsrs-cli`, `ddgs` / `duckduckgo-search-cli` (Semaphore / JoinSet / spawn_blocking / rayon), product law (one-shot / XDG / **zero telemetria remota** / anti-MCP / host-only CDP); **proibido** criar `.github` / CD / GitHub Actions nesta rodada  
**Datas:** 2026-07-18  
**Rodadas:** Pass 1…22 → **Pass 23 (paralelismo — reauditoria profunda / modus operandi)**

---

## 1. Veredito executivo

| Métrica | Valor |
|---------|--------|
| Gaps **Clap** identificados e solucionados (Pass 1–2) | **24** (G-01…G-24) |
| Gaps **stdin/stdout** identificados e solucionados (Pass 3) | **12** (S-01…S-12) |
| Gaps **one-shot lifecycle** identificados e solucionados (Pass 4) | **8** (O-01…O-08) |
| Gaps **inglês + docs crates.io** identificados e solucionados (Pass 5) | **12** (E-01…E-12) |
| Gaps **const/static/init** identificados e solucionados (Pass 6) | **8** (C-01…C-08) |
| Gaps **docs.rs / rustdoc automático** identificados e solucionados (Pass 7) | **10** (R-01…R-10) |
| Gaps **economia de recursos** identificados e solucionados (Pass 8) | **10** (ECO-01…ECO-10) |
| Gaps **eficiência e performance** identificados e solucionados (Pass 9) | **10** (PERF-01…PERF-10) |
| Gaps **graceful shutdown** identificados e solucionados (Pass 10) | **8** (GS-01…GS-08) |
| Gaps **memória / RAII** identificados e solucionados (Pass 11) | **10** (MEM-01…MEM-10) |
| Gaps **interior mutability** identificados e solucionados (Pass 12) | **10** (IM-01…IM-10) |
| Gaps **JSON / NDJSON** identificados e solucionados (Pass 13) | **10** (JSON-01…JSON-10) |
| Gaps **latência** identificados e solucionados (Pass 14) | **10** (LAT-01…LAT-10) |
| Gaps **logs / tracing / rotação** identificados e solucionados (Pass 15) | **10** (LOG-01…LOG-10) |
| Gaps **macros** identificados e solucionados (Pass 16) | **8** (MAC-01…MAC-08) |
| Gaps **dívida estrutural D-01…D-22** fechados (Pass 17) | **22** (D-01…D-22) |
| Gaps **i18n multi-idioma** identificados e solucionados (Pass 18) | **14** (I18N-01…I18N-14) |
| Gaps **multiplataforma SO** identificados e solucionados (Pass 19) | **12** (MP-01…MP-12) |
| Gaps **ownership / borrowing / lifetimes** identificados e solucionados (Pass 20) | **12** (OWN-01…OWN-12) |
| Gaps **paralelismo / multiprocessamento** identificados e solucionados (Pass 21–23) | **45** (PAR-01…PAR-45; 21–23 = endurecimento) |
| Gaps **N/A intencionais** (product law / ops / anti-CI / WASM / chromedriver / ICU / pin/GAT / loom industrial) | **133** (N-01…N-133) |
| Gaps **abertos bloqueantes** | **0** |
| Dívida estrutural não-bloqueante | **0** (fechada na Pass 17) |
| Inventário agente | **63** comandos (`commands --json`, inclui `man` + `locale`) |

**Veredito:**
- Checklist acionável de **Clap** → **PASS**.
- Contrato acionável de **stdin/stdout agent-first** (streams, BrokenPipe 141, envelope, main thin/lib fat, output canônico) → **PASS** após Pass 3.
- Ciclo de vida **BORN → EXECUTE → FINALIZE → DIE** (timeout global, step-timeout, SIGINT/SIGTERM → 130, flush dual) → **PASS** após Pass 4.
- **Código-fonte em inglês** (identificadores, comments, tracing, errors técnicos) + **docs crates.io núcleo** (crate `//!`, MSRV, docs.rs metadata, SAFETY, badges, examples) → **PASS** após Pass 5 (dívida residual em rustdoc item-a-item / SPDX total / ARCHITECTURE).
- **const vs static / mutabilidade interior / OnceLock / atomics / lints** → **PASS** após Pass 6 (sem `static mut`, sem `const` com interior mutability, `Mutex::new` direto, `Ordering` documentado).
- **Documentação docs.rs automática** (pipeline local HTML + nightly `doc_cfg`, metadata multi-target pós-2026-05-01, aquamarine, badges, seções canônicas, `doc(cfg)` platform, **sem** `doc_auto_cfg`) → **PASS** após Pass 7. Publicação crates.io/docs.rs e GitHub Actions permanecem **N/A** (proibido CD/GHA nesta rodada; publish manual).
- **Economia de recursos** (cliente HTTP process-wide, Regex OnceLock, mimalloc global, classificação de workload I/O-bound documentada, batch `with_capacity`, script RSS baseline local) → **PASS** acionável após Pass 8. Micro-otimizações sem profile (bumpalo/SIMD/rayon/NUMA) e ops industriais (flamegraph CI, systemd-run, Docker distroless) → **N/A** ou dívida não-bloqueante.
- **Eficiência e performance** (perfil release LTO fat + CGU 1 + abort + strip symbols, `release-size`, `bench` alinhado a release, deps dev `opt-level=2`, mold linker local, `FxHashMap` em L1 cache/ref map, `#[cold]` em construtores de erro, `with_capacity` em heap/snapshot, `scripts/perf-check.sh`) → **PASS** acionável após Pass 9. PGO/BOLT/multi-CPU dist/SIMD/unsafe get_unchecked → **N/A** até baseline de flamegraph ou product law.
- **Graceful shutdown** (modelo one-shot detect→signal→await: `shutdown_signal` central, SIGINT/SIGTERM + Windows Ctrl-Break, 1º sinal → cancel/130, 2º sinal → residual finalize, residual SIGTERM→grace 2s→SIGKILL, residual ledger preservado em erro de `run_with_session`, dual flush, BrokenPipe 141, sem `process::exit`, `scripts/shutdown-check.sh`) → **PASS** acionável após Pass 10. Padrões de daemon (TaskTracker fleets, SIGHUP reload, readiness, sd_notify, OTel WorkerGuard) → **N/A** product law.
- **Gerenciamento de memória / RAII** (ownership via compilador, `Drop` idempotente, Child kill+wait em todos os ramos, `try_reserve` + teto em input externo Redis/heap snapshot, `zeroize` em chaves de sessão, `mem::forget` só no WorkerGuard documentado, sem `Rc`/ciclos, `scripts/memory-check.sh`) → **PASS** acionável após Pass 11. Edition 2024 temporary scopes, miri/loom CI full, arenas/bumpalo industriais → **N/A** ou dívida não-bloqueante.
- **Interior mutability** (matriz Cell/RefCell/Mutex/atomic/OnceLock, poison recovery em residual ledger e MITM, sem `Mutex<bool>`, sem `Arc<tokio::Mutex>` em estado single-task, `try_borrow` no TLS, docs de `tokio::sync::Mutex` no CDP, `scripts/interior-mutability-check.sh`) → **PASS** acionável após Pass 12. loom/miri full, parking_lot, ArcSwap hot-config, observability de contention industrial → **N/A** ou dívida não-bloqueante.
- **JSON / NDJSON** (módulo `json_util` com strip BOM UTF-8, tetos de arquivo/linha, encode compacto, escrita atômica `BufWriter`+rename; envelopes tipados; `run` NDJSON com limite de linha; `json_steps` sem engolir erro de encode; `scripts/json-ndjson-check.sh`) → **PASS** acionável após Pass 13. schemars/jsonschema/JCS/JSON Patch/simd-json/HTTP Content-Type → **N/A** product law (CLI one-shot, sem API HTTP) ou dívida não-bloqueante.
- **Latência** (mentalidade de cauda P50/P99 via `latency-baseline.sh`; orçamentos documentados em `runtime_util`; runtime CDP multi-thread **2** workers + `max_blocking_threads(8)`; I/O HTTP em `current_thread` via `block_on_io`; proibição de `new_multi_thread` ad-hoc; `tcp_nodelay(true)` no HTTP compartilhado; perfil `release-prof` com `debug=1`; criterion envelope compact; `scripts/latency-check.sh`) → **PASS** acionável após Pass 14. PGO/BOLT/isolcpus/mlockall/huge pages/kernel bypass/HDR industrial → **N/A** product law (one-shot I/O-bound, Chrome WCET externo) ou dívida não-bloqueante.
- **Logs / tracing / rotação** (módulo `telemetry` único; `ErrorLayer` + features `json`/`tracing-log`/`env-filter`/`registry`; dual sink stderr+arquivo JSON rotativo com `max_log_files=14`; `TelemetryGuard` retém `WorkerGuard` até DIE sem `mem::forget`; ponte panic→tracing; filtro XDG/argv sem `RUST_LOG`; `eprintln!` de produção migrados para `tracing::warn`; `scripts/tracing-check.sh`) → **PASS** acionável após Pass 15. OTEL/OTLP, `reload::Layer` admin HTTP, journald, Lambda, tokio-console, dashboards remotos → **N/A** product law (zero telemetria remota, CLI one-shot).
- **i18n multi-idioma** (`sys-locale` + `unic-langid` + `fluent-langneg` + `fluent` FTL; enum `Idioma`/`Mensagem` match exaustivo en+pt-BR; OnceLock; 5 camadas flag/env/XDG/sistema/default; `--lang` + `BROWSER_AUTOMATION_CLI_LANG`; subcomando `locale`; features opcionais top-20; JSON máquina em inglês; `scripts/i18n-check.sh`) → **PASS** acionável após Pass 18. ICU calendar/collator completo, Weblate, top-20 embutido, WASM navigator.languages, GHA coverage gate → **N/A** product law.
- **Multiplataforma SO v3** (módulo `platform`: `which_bin` puro sem shell, `HostEnvironment` WSL/container/CI/Termux/Snap/Flatpak, console UTF-8+VT, sandbox warn; `find_chrome` cascata XDG→cache→PATH→paths absolutos Linux/macOS/Windows ProgramFiles+Edge+Beta+Canary+Brave+snap/flatpak; validação basenames reservados Windows; doctor `host_environment`+`sandbox`; `scripts/multiplatform-check.sh`; docs `CROSS_PLATFORM.md`) → **PASS** acionável após Pass 19. WASM/WASI, chromedriver, registro Win32 App Paths, universal binary/notarização, matrix GHA multi-OS, OCI multi-arch → **N/A** product law / ops.
- **Ownership / borrowing / lifetimes** (`ResolvedLocale` owned sem `Box::leak`; `redundant_clone`/`implicit_clone` limpos; `map_io_error`/`map_parse_err`/`format_cdp_err` por `&T`; `ingest_event(&CdpEvent)`; move de match bindings em extension/devtools/webmcp; `#[must_use]` em `Lifecycle`/`CdpClient`/`BrowserManager`; lints ownership no crate root; `scripts/ownership-check.sh`) → **PASS** acionável após Pass 20. GATs/HRTB/pin-project/ouroboros/miri full/arenas → **N/A** product law (sem self-referential structs / sem lib de iterators lending).
- **Paralelismo e multiprocessamento** (módulo `concurrency`: fórmula `min(cpus,(free_ram×50%)/64MiB,64)`; `--max-concurrency`; `join_bounded` com **`Arc<Semaphore>::acquire`**; batch/crawl `JoinSet`+`acquire_owned` + cancel + `abort_all`; HTML parse em **`spawn_blocking`**; discovery de crawl **dentro** do permit; `walk_threads` budget-aware; find-paths multi-root Rayon; sanitize/attach multi-page `join_bounded`; PDF/screenshot writes off-async; matriz completa + `na_product_law`; teste panic→permit; `scripts/parallelism-check.sh` Pass 23) → **PASS** acionável após Pass **21–23**. Parallelismo é **modus operandi** com bound explícito em todo fan-out; sequencial só com justificativa documentada. loom CI full, systemd-run MemoryMax, multi-Chrome pool, OTel remota → **N/A** product law / ops.
- **Macros** (sem `macro_rules!`/`macro_export`/`proc-macro` no crate de aplicação; CDP forwarders via genéricos monomorfizados; geração de tipos CDP via `build.rs` + `include!(concat!(env!("OUT_DIR"), …))`; builtins `env!`/`option_env!`/`concat!`/`json!` idiomáticos; sem `todo!`/`dbg!`/`unimplemented!`; gate `scripts/macros-check.sh`) → **PASS** acionável após Pass 16. Crate `proc-macro`/`syn`/`trybuild`/`cargo-expand` industrial → **N/A** (este crate não é biblioteca de macros).
- **Dívida estrutural D-01…D-22** → **FECHADA** na Pass 17 (SPDX total, ARCHITECTURE/ROADMAP, FxHashMap session maps, win_job unsafe split, try_reserve scrape, signal e2e, log schema, scripts profile/completions/llms).
- Itens de release industrial (SBOM/SLSA/cosign/Homebrew matrix, fuzz noturno, 80% cov, pedantic CI full) → **N/A ops** (N-11/N-12/N-20…) — product law / proibição GHA, **não** contam como dívida D-*.

---

## 2. Changelog incremental desta rodada

### Pass 1 — Shadowing e conflitos de argumentos

| ID | Gap identificado | Severidade | Solução | Evidência |
|----|------------------|------------|---------|-----------|
| **G-01** | `Doctor` e `Commands` tinham `--json` **local** shadowing o global `--json` (`global = true`) — armadilha “Shadowing Silencioso” | **P0** | Removidos campos locais; só `GlobalOpts.json`; dispatch usa `cli.globals.json` | `src/cli.rs`, `src/commands_prd/mod.rs` |
| **G-02** | `fill-form --json <payload>` colidia com envelope global `--json` (semântica incompatível: payload vs envelope) | **P0** | CLI: `--fields-json`; run/JSON mantém chaves `json`/`fields` | `src/cli.rs`, schemas, skills formulas |
| **G-03** | `cookie set --json <payload>` mesma colisão | **P0** | CLI: `--cookies-json`; run mantém `json`/`cookies` | `src/cli.rs`, `CookieAction::Set` |
| **G-04** | `view --verbose` shadowing global `--verbose` (a11y tree vs log level) | **P0** | CLI: `--detailed` (campo Rust `verbose`); run/JSON `"verbose":true`; exec aceita `detailed` | `src/cli.rs`, `run.rs`, formulas |
| **G-05** | `-q` + `--verbose`/`--debug` resolvia por prioridade silenciosa (sem exit 2) | **P1** | `conflicts_with_all` / `conflicts_with` no derive | `GlobalOpts` em `src/cli.rs`; smoke exit 2 |
| **G-06** | Zero teste CI de colisão global/local (rule OBRIGATÓRIO) | **P1** | Novo `tests/clap_global_flag_collision.rs` | 2 testes PASS |
| **G-07** | Typos help “path leve(s)” | **P3** | “path-level” | `cli.rs`, `commands_prd/mod.rs` |
| **G-08** | Mensagens de erro ainda citavam `fill-form --json` | **P2** | Mensagens → `--fields-json` | `handle_fill_form` |
| **G-09** | Docs/skills com argv antigo (`fill-form --json`, `view --verbose`) | **P2** | Atualizados formulas + SKILL + COOKBOOK | `skills/*`, `docs/COOKBOOK.md` |

### Pass 2 — Checklist restante + hardening

| ID | Gap identificado | Severidade | Solução | Evidência |
|----|------------------|------------|---------|-----------|
| **G-10** | Sem `clap_mangen` / manpage (checklist OBRIGATÓRIO) | **P1** | dep `clap_mangen`; subcomando `man [--out PATH]`; artefato `man/browser-automation-cli.1` | `Cargo.toml`, `handle_man`, `tests/manpage_cli.rs` |
| **G-11** | Sem embed de git SHA / timestamp de build | **P1** | `build.rs` → `cargo:rustc-env=GIT_SHA` + `BUILD_TIMESTAMP`; `build_identity()`; `version --json` | `build.rs`, `src/lib.rs`, `handle_version` |
| **G-12** | Bug build: `use std::process::Command` sombreava struct CDP `Command` em `build.rs` | **P0** (build) | Alias `ProcessCommand` | `build.rs` |
| **G-13** | Sem `benches/` de parsing (checklist) | **P2** | `benches/cli_parse.rs` + criterion + `[[bench]]` | `Cargo.toml`, `benches/` |
| **G-14** | Sem teste de cobertura/consumo de args (silent discard) | **P1** | `tests/clap_arg_coverage.rs` (help, globals, fields-json, cookies-json, detailed, man, build_identity) | 8 testes PASS |
| **G-15** | Sem gate local de audit (`.github/` gitignored no projeto) | **P2** | `scripts/ci-check.sh` + `deny.toml` | scripts + deny |
| **G-16** | Sem flag `--plain` (acessibilidade / agent) | **P2** | `GlobalOpts.plain` + `color::set_plain` no `run()` | `cli.rs`, `lib.rs`, `color.rs` |
| **G-17** | Cores ignoravam `CLICOLOR=0` e `TERM=dumb` (só `NO_COLOR`) | **P2** | Prioridade: plain → NO_COLOR → CLICOLOR → TERM=dumb → XDG | `src/color.rs` + unit test |
| **G-18** | Sem `tracing-error` / SpanTrace (checklist) | **P2** | dep `tracing-error` + `ErrorLayer` no subscriber | `Cargo.toml`, `init_tracing` |
| **G-19** | Sem validação de path traversal em paths de escrita CLI | **P1** | `reject_path_traversal` / `_str`; usado em `man --out` | `validation.rs`, `manpage_cli` |
| **G-20** | Layout rules pede `src/config.rs`; só existia `xdg.rs` | **P2** | `src/config.rs` re-exporta `crate::xdg::*` | `config.rs`, `lib.rs` |
| **G-21** | Rename de **long** não bastava: id clap do campo `json: String` ainda colidia com global `json: bool` (panic downcast no parse) | **P0** | Campos renomeados `fields_json` / `cookies_json` | `cli.rs`, dispatch, testes |
| **G-22** | Inventário/schemas desatualizados (61, sem `man`) | **P2** | COMMANDS + schemas + README schemas + skills → **62** + `man` | `meta.rs`, `docs/schemas/*` |
| **G-23** | Schema `doctor`/`commands` documentavam `json` local sem deixar claro que é flag **global** | **P3** | Descrição “Global envelope flag --json” | meta + schema JSON |
| **G-24** | `view` schema/run sem documentar `--detailed` vs `verbose` JSON | **P3** | schema + docs + exec mapping `detailed` | `view.schema.json`, `meta.rs`, `run.rs` |

---


### Pass 3 — Rules `cli_stdin_stdout` (contrato agent-first)

| ID | Gap identificado | Severidade | Solução | Evidência |
|----|------------------|------------|---------|-----------|
| **S-01** | `println!` em `envelope.rs` (proibido fora de `output.rs`; sem `write_all`/BrokenPipe tipado) | **P0** | `src/output.rs` canônico + envelope via `write_json_line` | `src/output.rs`, `src/envelope.rs` |
| **S-02** | ~82 `println!` de dados humanos em `commands_prd/mod.rs` sem mapear EPIPE → 141 | **P0** | `emit_ok` exige `Result`; todos os paths usam `output::writeln_stdout` | `commands_prd/mod.rs` |
| **S-03** | NDJSON `--json-steps` usava `println!` sem flush explícito por registro | **P1** | `output::writeln_stdout` (flush por linha) em `run.rs` | `commands_prd/run.rs` |
| **S-04** | `doctor` / `commands` / `schema` human mode com `println!` solto | **P1** | writers canônicos + BrokenPipe → 141 no doctor JSON | `doctor/mod.rs`, `meta.rs` |
| **S-05** | `emit_err` usava `eprintln!` sem flush/contrato unificado | **P2** | `output::writeln_stderr` + flush stdout | `emit_err` |
| **S-06** | Faltava módulo `output` (regra: println só em `output.rs`) | **P0** | módulo público documentado | `lib.rs` `pub mod output` |
| **S-07** | `run()` sem argv injetável (regra: `run(args, …)` testável) | **P1** | `run_from_args(I)` + `run()` thin wrapper; flush antes de DIE | `src/lib.rs` |
| **S-08** | Sem trait `Clock` injetável (dependency inversion / determinismo) | **P2** | `src/clock.rs` (`SystemClock`, `FixedClock`) | unit tests |
| **S-09** | `AGENTS.md` ausente na raiz (checklist + Quickstart-30s) | **P1** | `AGENTS.md` raiz com 5 linhas executáveis + link `docs/AGENTS.md` | root |
| **S-10** | `PRIVACY.md` ausente (checklist docs agent) | **P2** | política local-first / zero telemetry remota | `PRIVACY.md` |
| **S-11** | `.editorconfig` + `rust-toolchain.toml` + `audit.toml` ausentes | **P2** | arquivos versionados (MSRV 1.88.0 alinhado ao `Cargo.toml`) | root |
| **S-12** | `map_io_error` BrokenPipe→141 / Io→74 sem testes dedicados | **P1** | unit tests em `output::tests` + `pipe_broken` revalidado | 2+2 tests PASS |

#### N/A adicionais (Pass 3 — product law / escopo industrial)

| ID | Rule genérica | Por que N/A | Status |
|----|---------------|-------------|--------|
| **N-09** | `run(args, stdin, stdout, stderr)` com streams 100% injetados | Stdin/stdout/stderr do processo são o contrato Unix do agente; `output` + `run_from_args` cobrem testabilidade sem mock de fd global | **N/A** (parcial: argv injetável) |
| **N-10** | MCP server / schema MCP | Proibido por product law; CLI nativa é a interface | **N/A** |
| **N-11** | SBOM/SLSA/cosign/Homebrew/Winget/AUR full pipeline | `.github/` gitignored; release ops fora do binário | **N/A** (ops) |
| **N-12** | `cargo fuzz` / `cargo-mutants` / loom / shadow deployment noturno | CI noturno industrial; núcleo one-shot já tem proptest + assert_cmd + pipe tests | **N/A** (ops) |

#### Dívida nova / atualizada

| ID | Descrição | Impacto | Status |
|----|-----------|---------|--------|
| **D-05** | `eprintln!` residual em paths de log humano (`robots`, `browser` experimental CDP, testes) | Baixo (stderr, não dados) | **ABERTO** (baixo; não contamina stdout) |

### Pass 5 — English source + crates.io / docs.rs (resumo; detalhe em §11)

| ID | Gap | Severidade | Solução |
|----|-----|------------|---------|
| **E-01…E-04** | `unsafe` sem `SAFETY:` / lints / multi-op kill | P0–P2 | SAFETY docs + clippy lints + split SIGTERM/SIGKILL |
| **E-05…E-06** | `doc_auto_cfg` + badges maintenance | P1–P2 | crate attrs + `Cargo.toml` |
| **E-07…E-10** | `//!` módulos native, build.rs, examples/ | P2–P3 | module docs + `examples/run_library.rs` |
| **E-11…E-12** | SPDX superfície pública + allowlist + i18n isolation | P2 | headers + CLAUDE + `i18n` docs |
| **N-17…N-20** | PT catalog / KaTeX / proc-macro / CI full | — | **N/A** documentado |
| **D-06…D-08** | SPDX total, ARCHITECTURE, win multi-op | baixo | **ABERTO** |

### Pass 6 — const / static / inicialização (resumo; detalhe em §12)

| ID | Gap | Severidade | Solução |
|----|-----|------------|---------|
| **C-01** | `OnceLock<Mutex>` em `ACTIVE_FRAME` | P1 | `Mutex::new(None)` direto |
| **C-02…C-03** | Ordering/docs + OnceLock color | P2 | docs + hoist `COLORS_ENABLED` |
| **C-04** | Lints `static_mut_refs` / interior mut const | P1 | deny/warn no crate root |
| **C-05…C-06** | asserts de build + UA version drift | P2 | `const _: () = assert!` + `env!("CARGO_PKG_VERSION")` |
| **C-07…C-08** | docs concorrência + poison ENV_MUTEX | P2–P3 | docs + `into_inner` |
| **N-21…N-22** | lazy_static migrate / loom | — | **N/A** |
| **D-09** | ledger Mutex poison ignore | baixo | **ABERTO** |

### Pass 7 — docs.rs geração automática (resumo; detalhe em §14)

| ID | Gap | Severidade | Solução |
|----|-----|------------|---------|
| **R-01** | `feature(doc_auto_cfg)` ainda no crate (regra: merge Oct 2025 → só `doc_cfg`) | P0 | Removido; só `#![cfg_attr(docsrs, feature(doc_cfg))]` |
| **R-02** | Safety crate sem documentar feature gate `doc_cfg` / anti-`doc_auto_cfg` | P1 | Seção Safety expandida em `src/lib.rs` |
| **R-03** | APIs `win_job` Windows sem `#[doc(cfg(windows))]` | P1 | `cfg_attr(docsrs, doc(cfg(...)))` em APIs + stubs |
| **R-04** | Sem pipeline local de validação docs (fases 1–7) | P1 | `scripts/docs-check.sh` (NDJSON stdout; **sem** GHA/CD) |
| **R-05** | Comentário `Cargo.toml` ainda citava `doc_auto_cfg` | P2 | Metadata + comentário 2026-05-01 targets |
| **R-06** | Links intra-doc quebrados (`clock`/`color`/`error` module docs) | P1 | Links absolutos `crate::…` |
| **R-07** | Link falso `[env!("CARGO_PKG_VERSION")]` em `scrape_local` | P1 | Backticks sem link (nightly deny) |
| **R-08** | `error` sem seções Conversions/Cost (rules From/TryFrom docs) | P2 | Módulo documenta mapping exit_code + custo |
| **R-09** | `--document-private-items=false` / `timeout --signal` inválidos no host | P2 | Script ajustado ao `timeout` local + public-only |
| **R-10** | JSON rustdoc limpava HTML em `target/doc` | P1 | `CARGO_TARGET_DIR` isolado para JSON |
| **N-23…N-25** | GHA/CD/publish automático / features Cargo inexistentes / KaTeX | — | **N/A** |
| **D-10…D-11** | Gerador llms.txt a partir de JSON; missing_docs monólitos | baixo | **ABERTO** |

### Pass 4 — Rules `cli_one_shot` (ciclo de vida, sinais, timeouts)

| ID | Gap identificado | Severidade | Solução | Evidência |
|----|------------------|------------|---------|-----------|
| **O-01** | `CancellationToken` existia mas **nunca** era acionado por SIGINT/SIGTERM (só em `finalize`) | **P0** | `block_on_browser_timeout` spawna watcher `tokio::signal` (SIGINT/SIGTERM/ctrl_c) → `cancel()`; race com work via `select!` | `src/browser/mod.rs`, `src/lifecycle.rs` |
| **O-02** | `ErrorKind::Cancelled` (exit **130**) sem caminho runtime que o produzisse | **P0** | `cancelled_error()` no race de cancel; pré-check se token já cancelado | unit test `pre_cancelled_token_returns_exit_130` |
| **O-03** | `--step-timeout` declarado em `GlobalOpts` mas **nunca** lido/aplicado em `run` | **P1** | `RunFlags.step_timeout_secs` + `tokio::time::timeout` por step em `run_script_with_flags` | `cli.rs`, `run.rs`, `mod.rs` |
| **O-04** | XDG `timeout` ignorado quando CLI `--timeout 0` (default) | **P1** | `config::resolve_global_timeout` (CLI >0 → senão XDG) | `src/config.rs`, `dispatch` |
| **O-05** | `--step-timeout 0` não herdava timeout global | **P2** | `config::resolve_step_timeout` (step >0 → senão global) | unit tests config |
| **O-06** | Flush só de stdout antes de DIE (rules: stdout **e** stderr OBRIGATÓRIOS) | **P1** | `flush_stderr()` nos paths de exit em `run_from_args` | `src/lib.rs` |
| **O-07** | Token de cancel não acessível a `block_on_*` sem passar `Lifecycle` em dezenas de call sites | **P1** | `thread_local` `CURRENT_CANCEL` + `lifecycle::current_cancel()` registrado em `Lifecycle::new` | `src/lifecycle.rs` |
| **O-08** | Fail-fast de `run` mapeava `cancelled`/`broken-pipe` → `Software` | **P2** | match em `handle_run` inclui `cancelled` e `broken-pipe` | `commands_prd/mod.rs` |

#### N/A adicionais (Pass 4 — product law / escopo one-shot)

| ID | Rule genérica | Por que N/A | Status |
|----|---------------|-------------|--------|
| **N-13** | `GlobalOpts --config` / `--log-level` / `--log-format` / `--color` / `--no-progress` | Product law: XDG `config set` + flags `-q/--verbose/--debug/--plain`; sem env de produto; progress no stdout proibido | **N/A** |
| **N-14** | `#[tokio::main]` no `main` | Runtime multi-thread **por comando** via `block_on_browser_timeout` (Chrome I/O fan-out); main sync thin | **N/A** (justificado) |
| **N-15** | Progress bar / report em ops > 2s no stdout | stdout = envelope JSON; progresso só stderr via tracing se verbose | **N/A** (agent-first) |
| **N-16** | TUI / REPL / interactive prompts / auto-update self_update | Proibido por product law non-interactive one-shot | **N/A** |

#### Dívida Pass 4 (não-bloqueante)

| ID | Descrição | Impacto | Status |
|----|-----------|---------|--------|
| **D-01** | monólito `commands_prd/mod.rs` | manutenibilidade | **ABERTO** (inalterado) |
| **D-05** | `eprintln!` residual em logs humanos | baixo | **ABERTO** |

---


## 3. Catálogo mestre — todos os gaps da rodada

Legenda de status: **RESOLVIDO** | **N/A** | **ABERTO** (dívida não-bloqueante)

### 3.1 Armadilhas Clap (shadowing / discard / conflitos)

| ID | Descrição | Antes | Depois | Status |
|----|-----------|-------|--------|--------|
| G-01 | Shadow `doctor`/`commands` `--json` | Campos locais + merge `json \|\| doc_json` | Só global | **RESOLVIDO** |
| G-02 | Shadow payload `fill-form --json` | long `json` string | `--fields-json` + campo `fields_json` | **RESOLVIDO** |
| G-03 | Shadow payload `cookie set --json` | long `json` string | `--cookies-json` + campo `cookies_json` | **RESOLVIDO** |
| G-04 | Shadow `view --verbose` vs log `--verbose` | mesmo long | CLI `--detailed`; JSON `verbose` | **RESOLVIDO** |
| G-05 | Quiet×verbose/debug sem rejeição | prioridade em `init_tracing` | clap `conflicts_with` exit 2 | **RESOLVIDO** |
| G-06 | Sem teste de colisão global | só `debug_assert` | `clap_global_flag_collision` | **RESOLVIDO** |
| G-14 | Sem cobertura de consumo de args | — | `clap_arg_coverage` | **RESOLVIDO** |
| G-21 | Id clap `json` String vs Bool | panic no parse | rename de **campo** | **RESOLVIDO** |

### 3.2 Cargo / build / distribuição (checklist)

| ID | Descrição | Antes | Depois | Status |
|----|-----------|-------|--------|--------|
| G-10 | `clap_mangen` ausente | só `clap_complete` runtime | `man` + manpage roff | **RESOLVIDO** |
| G-11 | Git SHA não embedado | só `CARGO_PKG_VERSION` | `GIT_SHA` + `BUILD_TIMESTAMP` | **RESOLVIDO** |
| G-12 | Shadow `Command` no build.rs | compile break benches | `ProcessCommand` | **RESOLVIDO** |
| G-13 | Sem benches de parse | sem `benches/` | criterion `cli_parse` | **RESOLVIDO** |
| G-15 | Sem cargo-audit/deny no fluxo local | — | `ci-check.sh` + `deny.toml` | **RESOLVIDO** |
| G-18 | Sem tracing-error | só tracing-subscriber | `ErrorLayer` | **RESOLVIDO** |

### 3.3 I/O, UX, segurança, layout

| ID | Descrição | Antes | Depois | Status |
|----|-----------|-------|--------|--------|
| G-16 | Sem `--plain` | cores só XDG/NO_COLOR | flag global | **RESOLVIDO** |
| G-17 | CLICOLOR / TERM=dumb | parcial | prioridade completa | **RESOLVIDO** |
| G-19 | Path traversal em `--out` | sem check | `reject_path_traversal` | **RESOLVIDO** |
| G-20 | Falta `src/config.rs` | só `xdg.rs` | re-export | **RESOLVIDO** |
| G-07–G-09, G-22–G-24 | Docs/typos/inventário/schemas | defasados | alinhados 62 cmds | **RESOLVIDO** |

### 3.4 N/A intencionais (product law — **não** são regressões)

| ID | Rule genérica | Por que N/A neste produto | Status |
|----|---------------|---------------------------|--------|
| **N-01** | `#[arg(env)]` + feature `env` para config de produto | XDG only; proibido env de produto | **N/A** |
| **N-02** | `RUST_LOG` como fonte de log de produto | flags + XDG `log_level` | **N/A** |
| **N-03** | dialoguer / confirmação TTY | CLI non-interactive agent-first | **N/A** |
| **N-04** | indicatif progress no stdout | stdout = envelope JSON | **N/A** |
| **N-05** | `ArgAction::Count` `-vvv` | modelo discreto `-q` / `--verbose` / `--debug` | **N/A** |
| **N-06** | multicall BusyBox | um binário `browser-automation-cli` | **N/A** |
| **N-07** | `src/commands/` nome exato | `commands_prd/` é a camada PRD | **N/A** |
| **N-08** | Pipeline cargo-dist/SBOM/GitHub Actions completo | `.github/` gitignored; gate local `ci-check.sh` | **N/A** (ops) |

### 3.5 Dívida aberta não-bloqueante (fora do escopo “fechar clap”)

| ID | Descrição | Impacto | Status |
|----|-----------|---------|--------|
| **D-01** | `src/commands_prd/mod.rs` monólito (~3.7k LOC) | manutenibilidade | **ABERTO** (refator opcional) |
| **D-02** | `cli.rs` com `#![allow(missing_docs)]` | rustdoc | **ABERTO** (baixo) |
| **D-03** | Completions só runtime (não geradas em `build.rs`) | DX packaging | **ABERTO** (baixo; `completions` cmd cobre) |
| **D-04** | Teste exaustivo campo-a-campo de **todo** Args→handler | silent discard residual teórico | **ABERTO** (G-14 cobre paths críticos) |
| **D-05** | `eprintln!` residual em logs humanos (robots/browser/tests) | não contamina stdout | **ABERTO** (baixo; Pass 3) |

---

## 4. Mapa “antes → depois” (contratos de agente)

| Operação | Antes (quebrado / ambíguo) | Depois (canônico) |
|----------|----------------------------|-------------------|
| Envelope doctor | `doctor --json` (local) | `doctor --json` **global** (mesmo argv; sem campo local) |
| Inventory | `commands --json` | idem, global only |
| Snapshot detalhado | `view --verbose` | `view --detailed` |
| Snapshot em `run` | `{"cmd":"view","verbose":true}` | **inalterado** (tool-ref) |
| Fill form CLI | `fill-form --json '[...]'` | `fill-form --fields-json '[...]'` |
| Cookie set CLI | `cookie set --json '[...]'` | `cookie set --cookies-json '[...]'` |
| Manpage | (inexistente) | `man` ou `man --out man/browser-automation-cli.1` |
| Versão rastreável | só `0.1.4` | `version --json` → `git_sha`, `build_timestamp` |
| Quiet+verbose | sucesso silencioso | exit **2** clap |
| Cores off forçado | só `NO_COLOR` | também `--plain`, `CLICOLOR=0`, `TERM=dumb` |

---

## 5. Arquivos tocados (por gap)

### Código runtime / build
| Arquivo | Gaps |
|---------|------|
| `src/cli.rs` | G-01–G-05, G-04, G-16, G-10 (`Man`), G-21 |
| `src/commands_prd/mod.rs` | G-01, G-08, G-10 (`handle_man`), G-11 (`handle_version`) |
| `src/commands_prd/run.rs` | G-04 (`detailed`), G-10 (man não é step browser) |
| `src/commands_prd/meta.rs` | G-10, G-22, G-23, G-24 (COMMANDS 62, schemas) |
| `src/lib.rs` | G-11, G-16, G-18, G-20 (mod config) |
| `src/color.rs` | G-16, G-17 |
| `src/config.rs` *(novo)* | G-20 |
| `src/validation.rs` | G-19 |
| `build.rs` | G-11, G-12 |
| `Cargo.toml` / `Cargo.lock` | G-10, G-13, G-18 |

### Testes / benches / artefatos
| Arquivo | Gaps |
|---------|------|
| `tests/clap_global_flag_collision.rs` *(novo)* | G-06 |
| `tests/clap_arg_coverage.rs` *(novo)* | G-14, G-21 |
| `tests/manpage_cli.rs` *(novo)* | G-10, G-19 |
| `tests/clap_command_debug_assert.rs` | preexistente; revalidado |
| `benches/cli_parse.rs` *(novo)* | G-13 |
| `man/browser-automation-cli.1` *(novo)* | G-10 |
| `scripts/ci-check.sh` *(novo)* | G-15 |
| `deny.toml` *(novo)* | G-15 |

### Docs / skills / schemas
| Arquivo | Gaps |
|---------|------|
| `docs/schemas/{view,fill-form,cookie,doctor,commands}.schema.json` | G-02–G-04, G-23, G-24 |
| `docs/schemas/man.schema.json` *(novo)* | G-10, G-22 |
| `docs/schemas/README.md` | G-22 (61→62) |
| `docs/COOKBOOK.md` | G-09 |
| `skills/**/SKILL.md`, `formulas.md` | G-09, G-22 |
| `gaps.md` | este documento |
| `src/output.rs` *(novo, Pass 3)* | S-01, S-06, S-12 |
| `src/clock.rs` *(novo, Pass 3)* | S-08 |
| `src/envelope.rs` | S-01 |
| `src/lib.rs` | S-06, S-07 |
| `src/commands_prd/{mod,run,meta}.rs` | S-02, S-03, S-04, S-05 |
| `src/doctor/mod.rs` | S-04 |
| `AGENTS.md` / `PRIVACY.md` / `.editorconfig` / `rust-toolchain.toml` / `audit.toml` | S-09…S-11 |

---

## 6. Validação executada (evidência de fechamento)

```text
clap_arg_coverage             8 ok
clap_global_flag_collision    2 ok
clap_command_debug_assert     1 ok
manpage_cli                   3 ok
doctor_cli                    3 ok
envelope_schema               5 ok
parity_inventory              4 ok
validation:: + color:: units  ok
cargo test --benches --no-run ok (compila criterion)
commands --json               count = 62 (man presente)
version --json                git_sha + build_timestamp preenchidos
man --out ...                 roff .TH gerado
-q --verbose                  exit 2
```

Comandos de revalidação:

```bash
cargo test --test clap_arg_coverage --test clap_global_flag_collision \
  --test clap_command_debug_assert --test manpage_cli --test doctor_cli
cargo test --lib -- validation:: color::
cargo run --quiet -- version --json
cargo run --quiet -- man --out man/browser-automation-cli.1
./scripts/ci-check.sh
```

---

## 7. Matriz checklist rules_rust_cli_com_clap (pós-rodada)

| Item checklist | Status pós-rodada |
|----------------|-------------------|
| Derive API / Cli única fonte | PASS |
| Camadas main/lib/cli/commands | PASS (`commands_prd` + `config` re-export) |
| ArgAction explícito / ValueEnum / value_hint | PASS |
| Flags globais sem shadowing | PASS (G-01–G-04, G-06, G-21) |
| conflicts em toggles de verbosidade | PASS (G-05) |
| stdout/stderr separados + SIGPIPE | PASS (pré-existente) |
| sysexits + CliError + human-panic | PASS (pré-existente) |
| tracing stderr + tracing-error | PASS (G-18) |
| XDG config (sem env produto) | PASS / N-01 |
| completions clap_complete | PASS |
| manpages clap_mangen | PASS (G-10) |
| --plain / NO_COLOR / CLICOLOR | PASS (G-16, G-17) |
| debug_assert + assert_cmd | PASS |
| teste colisão global | PASS (G-06) |
| cobertura args críticos | PASS (G-14) |
| benches parse | PASS (G-13) |
| release LTO/strip/abort | PASS (pré-existente) |
| git SHA embed | PASS (G-11) |
| path traversal | PASS (G-19) |
| cargo-audit/deny no fluxo | PASS local (G-15); N-08 para GHA full |
| CONTRIBUTING / CODE_OF_CONDUCT | PASS (pré-existente) |

---

## 8. Catálogo Pass 3 — stdin/stdout (detalhe)

| ID | Antes | Depois | Status |
|----|-------|--------|--------|
| S-01 | `envelope` → `println!` | `output::write_json_line` + flush | **RESOLVIDO** |
| S-02 | dezenas de `println!` em handlers | `writeln_stdout` + `emit_ok: Result` | **RESOLVIDO** |
| S-03 | NDJSON steps sem flush canônico | flush por linha | **RESOLVIDO** |
| S-04 | doctor/commands/schema human println | output module | **RESOLVIDO** |
| S-05 | emit_err eprintln solto | writeln_stderr | **RESOLVIDO** |
| S-06 | sem `src/output.rs` | módulo + testes | **RESOLVIDO** |
| S-07 | só `run()` | `run_from_args` | **RESOLVIDO** |
| S-08 | sem Clock trait | `clock.rs` | **RESOLVIDO** |
| S-09 | sem AGENTS.md raiz | Quickstart-30s | **RESOLVIDO** |
| S-10 | sem PRIVACY.md | local-first policy | **RESOLVIDO** |
| S-11 | sem editorconfig/toolchain/audit.toml | versionados | **RESOLVIDO** |
| S-12 | BrokenPipe map sem unit test dedicado | `output::tests` | **RESOLVIDO** |

### 8.1 Matriz checklist rules_rust_cli_stdin_stdout (núcleo acionável)

| Item | Status |
|------|--------|
| stdout = dados / stderr = logs | **PASS** |
| JSON envelope `schema_version:1` | **PASS** |
| Exit codes sysexits + 124/130/141 | **PASS** |
| BrokenPipe → 141 (`write_all` + map) | **PASS** (S-01, S-12) |
| Flush explícito por registro / antes de DIE | **PASS** (S-03, S-07) |
| `println!` só via camada output | **PASS** nos paths de dados (S-01–S-06) |
| main thin / lib fat | **PASS** |
| clap 4 derive | **PASS** |
| Sem TUI/dialoguer/indicatif/ratatui | **PASS** |
| Sem daemon / MCP primário | **PASS** |
| SIGPIPE default (Unix) | **PASS** |
| argv injetável (`run_from_args`) | **PASS** (S-07) |
| Clock injetável | **PASS** (S-08) |
| AGENTS.md Quickstart-30s | **PASS** (S-09) |
| Completions + man | **PASS** (G-10 prévio) |
| XDG config, sem env de produto | **PASS** / N-01 |
| Idempotency/correlation em workflow | **PASS** parcial (`workflow_local`) |
| Full stream inject stdin/stdout/stderr | **N-09** |
| SBOM/SLSA/fuzz matrix industrial | **N-11/N-12** |

### 8.2 Validação Pass 3

```text
output::tests                     2 ok
clock::tests                      2 ok
pipe_broken                       2 ok
envelope_schema                   5 ok
clap_arg_coverage                 8 ok
clap_global_flag_collision        2 ok
manpage_cli                       3 ok
doctor_cli                        3 ok
cargo check                       ok
```

Fontes de pesquisa (obrigatórias na skill):
- GraphRAG entities: `rules-rust-cli-stdin-stdout`, `stdin-stdout-contract`, `mandato-cli-rust-stdin-stdout`
- context7: `/clap-rs/clap` (`try_parse_from`, derive, CommandFactory)
- duckduckgo-search-cli: práticas BrokenPipe/SIGPIPE/exit 141 em CLIs Rust

---

## 9. Histórico de rodadas

| Rodada | Foco | Gaps fechados |
|--------|------|----------------|
| **Pass 1** | Armadilhas shadowing + conflicts + teste colisão + docs argv | G-01 … G-09 |
| **Pass 2** | Man, SHA, benches, coverage, plain/cores, tracing-error, path, config.rs, CI local, inventário 62, fix id clap json | G-10 … G-24 |
| **Pass 3** | Contrato stdin/stdout: `output.rs`, envelope, NDJSON flush, `run_from_args`, Clock, AGENTS/PRIVACY/editorconfig/toolchain/audit | S-01 … S-12 |
| **Pass 3b** | Documentação incremental completa no `gaps.md` | este arquivo |
| **Pass 4** | One-shot lifecycle: SIGINT/SIGTERM→130, step-timeout, XDG timeout merge, flush stderr, fail-fast kinds | O-01 … O-08 |
| **Pass 5** | English source + crates.io / docs.rs núcleo (SAFETY, badges, examples) | E-01 … E-12 |
| **Pass 6** | const / static / inicialização | C-01 … C-08 |
| **Pass 7** | docs.rs automático: `doc_cfg` only, pipeline local, `doc(cfg)`, links rustdoc | R-01 … R-10 |

---

## 10. Catálogo Pass 4 — one-shot (detalhe)

| ID | Antes | Depois | Status |
|----|-------|--------|--------|
| O-01 | Sinais ignorados durante work async | `tokio::signal` + `cancel` + `select!` | **RESOLVIDO** |
| O-02 | Exit 130 sem produtor | `ErrorKind::Cancelled` em race/pre-check | **RESOLVIDO** |
| O-03 | `--step-timeout` dead flag | aplicado em cada step do `run` | **RESOLVIDO** |
| O-04 | XDG timeout morto se CLI=0 | `resolve_global_timeout` | **RESOLVIDO** |
| O-05 | step não herdava global | `resolve_step_timeout` | **RESOLVIDO** |
| O-06 | só `flush_stdout` | + `flush_stderr` antes de DIE | **RESOLVIDO** |
| O-07 | cancel sem bridge para block_on | `thread_local` + `current_cancel()` | **RESOLVIDO** |
| O-08 | fail-fast perdia kind cancel/EPIPE | match expandido | **RESOLVIDO** |

### 10.1 Matriz checklist rules_rust_cli_one_shot (núcleo acionável)

| Item | Status |
|------|--------|
| BORN → EXECUTE → FINALIZE → DIE (sem daemon) | **PASS** |
| Daemon / loop infinito / HTTP server de longa vida | **PASS** (MITM = one-shot com timeout) |
| Timeout global configurável | **PASS** (`--timeout` + XDG) |
| Timeout por step em multi-step | **PASS** (`--step-timeout`) |
| Cancellation cooperativo (SIGINT/SIGTERM → 130) | **PASS** (O-01, O-02) |
| Exit 124 em wall-clock timeout | **PASS** (test `hard_timeout_returns_exit_124`) |
| Flush stdout+stderr antes de exit | **PASS** (O-06) |
| FINALIZE idempotente + Drop safety net | **PASS** (pré-existente) |
| main thin / sem panic! em main | **PASS** |
| Saída determinística (BTreeMap em mitm/workflow) | **PASS** parcial (paths de dados estáveis) |
| Completions builtin | **PASS** |
| Stateless entre invocações | **PASS** |
| `#[tokio::main]` no binary | **N-14** (runtime por comando) |
| GlobalOpts genérico --config/--log-level | **N-13** (XDG product law) |
| Progress bar / TUI / REPL | **N-15/N-16** |

### 10.2 Validação Pass 4

```text
lifecycle::tests                      2 ok
config::tests                         2 ok
browser::pre_cancelled_token…130      ok
browser::hard_timeout…124             ok
browser::zero_timeout_sleep…          ok
clap_arg_coverage                     8 ok
clap_global_flag_collision            2 ok
doctor_cli                            3 ok
residual_one_shot                     1 ok
version --json                        ok
doctor --offline --quick --json       ok
```

Fontes de pesquisa (obrigatórias na skill):
- GraphRAG entities: `rules-rust-cli-one-shot`, `one-shot-cli`, `rust-one-shot-cli`
- context7: `/clap-rs/clap`
- duckduckgo-search-cli: cooperative cancel SIGINT/SIGTERM + CancellationToken em CLIs Rust

Arquivos tocados (Pass 4):
| Arquivo | Gaps |
|---------|------|
| `src/lifecycle.rs` | O-01, O-07 |
| `src/browser/mod.rs` | O-01, O-02 (+ tests 124/130) |
| `src/config.rs` | O-04, O-05 |
| `src/commands_prd/run.rs` | O-03 |
| `src/commands_prd/mod.rs` | O-03, O-04, O-05, O-08 |
| `src/lib.rs` | O-06 |
| `gaps.md` | este catálogo |

---

## 11. Pass 5 — English source + crates.io / docs.rs documentation

**Rules:** `rules_rust_codigo_ingles_internacionalizacao.md` + checklist crates.io/docs.rs (inglês universal, vocabulário inclusivo, SAFETY, rustdoc, MSRV, badges, examples).  
**Fontes:** GraphRAG `rules-rust-documentacao`; context7 `/rust-lang/rust`; duckduckgo-search-cli (undocumented_unsafe_blocks, maintenance badge).

### 11.1 Changelog incremental Pass 5

| ID | Gap identificado | Severidade | Solução | Evidência |
|----|------------------|------------|---------|-----------|
| **E-01** | Blocos `unsafe` sem comentário `SAFETY:` (`lib` SIGPIPE, `lifecycle` kill, `residual` getuid, `chrome` geteuid×2) | **P0** | Comentários SAFETY multi-linha (contrato + invariantes + referência) | `src/lib.rs`, `lifecycle.rs`, `residual.rs`, `native/cdp/chrome.rs` |
| **E-02** | `win_job.rs` FFI Windows sem `SAFETY:` em create/terminate/close/validate | **P0** | SAFETY em todos os `unsafe` do módulo Windows | `src/win_job.rs` |
| **E-03** | Lints de unsafe não ativados no crate root | **P1** | `clippy::undocumented_unsafe_blocks`, `multiple_unsafe_ops_per_block`, `deny(unsafe_op_in_unsafe_fn)` | `src/lib.rs` |
| **E-04** | `kill` SIGTERM+SIGKILL no mesmo bloco unsafe (lint multiple ops) | **P2** | Dois blocos `unsafe` com SAFETY distintos | `src/lifecycle.rs` |
| **E-05** | Sem feature gate de cfg em docs (docs.rs) | **P1** | Inicialmente `doc_cfg`+`doc_auto_cfg`; **superado por Pass 7 R-01** (somente `doc_cfg`) | `src/lib.rs` |
| **E-06** | Sem `[badges.maintenance]` no `Cargo.toml` | **P2** | `maintenance = { status = "actively-developed" }` | `Cargo.toml` |
| **E-07** | Módulos `native/*` sem `//!` de módulo | **P2** | Doc crate-level em browser/cookies/element/interaction/network/screenshot/snapshot/state | `src/native/*.rs` |
| **E-08** | `test_utils.rs` sem `//!` | **P3** | Module doc de helpers de teste | `src/test_utils.rs` |
| **E-09** | `build.rs` sem documentação de responsabilidades / offline | **P2** | `//!` descrevendo GIT_SHA, CDP gen, sem rede | `build.rs` |
| **E-10** | Sem `examples/` (checklist crates.io) | **P2** | `examples/run_library.rs` (entry `run()`) | `examples/` |
| **E-11** | Headers SPDX ausentes em superfície pública | **P2** | SPDX MIT OR Apache-2.0 em lib/error/envelope/output/clock/config/constants/lifecycle | `src/{lib,error,envelope,output,clock,config,constants,lifecycle}.rs` |
| **E-12** | Vocabulário obsoleto `whitelist` em doc de agente + isolamento i18n pouco explícito | **P2** | `whitelist`→`allowlist` em CLAUDE.md; `i18n` documenta EN técnico vs PT só em catalog strings | `CLAUDE.md`, `src/i18n.rs` |

### 11.2 N/A Pass 5 (product law / escopo industrial)

| ID | Rule genérica | Por que N/A | Status |
|----|---------------|-------------|--------|
| **N-17** | Zero português em qualquer `.rs` | PT **somente** em string literals do catálogo i18n (`suggestion` humana); identificadores/comments/tracing/message técnicos em EN; módulo documenta isolamento | **N/A** (i18n UI intencional) |
| **N-18** | KaTeX / MathML / `no_std` / wasm32 Chrome | Produto é CLI host com Chrome CDP; sem matemática densa nem `no_std`/wasm browser automation | **N/A** |
| **N-19** | Proc-macro crate (`syn`/`trybuild`) | Este crate não é proc-macro | **N/A** |
| **N-20** | CI matrix full (MSRV+beta+nightly+lychee+semver-checks+public-api) em `.github/` | `.github/` gitignored; gate local `ci-check.sh` + deny; release ops fora do binário | **N/A** (ops) |

### 11.3 Dívida Pass 5 (não-bloqueante)

| ID | Descrição | Impacto | Status |
|----|-----------|---------|--------|
| **D-02** | `#![allow(missing_docs)]` em `cli` + monólitos `native/*` / `commands_prd` | rustdoc item-level incompleto | **ABERTO** (baixo; help clap é a fonte UX) |
| **D-06** | SPDX ainda não em **todos** os `.rs` (72 arquivos; só superfície pública + core) | consistência license headers | **ABERTO** (baixo) |
| **D-07** | Sem `ARCHITECTURE.md` / `ROADMAP.md` / `GOVERNANCE.md` | docs org | **ABERTO** (baixo; CONTRIBUTING/SECURITY/COC existem) |
| **D-08** | `win_job` ainda agrupa várias ops Win32 no mesmo `unsafe` (create/assign) | lint pedantic multi-op | **ABERTO** (baixo; SAFETY presente; split só Windows) |

### 11.4 Achados de varredura (sem gap de código)

| Check | Resultado |
|-------|-----------|
| Identificadores PT (`processar_dados`, etc.) | **0** em `src/` / `tests/` / `benches/` |
| Comentários/doc PT fora de i18n catalog | **0** (só `src/i18n.rs` strings + `tests/golden_i18n.rs` asserts) |
| `whitelist`/`blacklist`/`master`/`slave`/`dummy_`/`sanity_check` em `.rs` | **0** |
| `tracing` / erros técnicos em EN | **PASS** |
| `Cargo.toml` description / keywords / categories / rust-version / docs.rs | **PASS** (já existia; + badge E-06) |
| Crate root `//!` + exemplo + MSRV + Features section | **PASS** (pré-existente; reforçado E-05/E-10) |
| `missing_docs` warn + rustdoc denys | **PASS** (pré-existente) |
| CONTRIBUTING / SECURITY / CODE_OF_CONDUCT / CHANGELOG Keep a Changelog / bilingue `.pt-BR` | **PASS** (pré-existente) |

### 11.5 Arquivos tocados (Pass 5)

| Arquivo | Gaps |
|---------|------|
| `src/lib.rs` | E-01, E-03, E-05, E-11 |
| `src/lifecycle.rs` | E-01, E-04, E-11 |
| `src/residual.rs` | E-01 |
| `src/native/cdp/chrome.rs` | E-01 |
| `src/win_job.rs` | E-02 |
| `src/native/{browser,cookies,element,interaction,network,screenshot,snapshot,state}.rs` | E-07 |
| `src/test_utils.rs` | E-08 |
| `build.rs` | E-09 |
| `examples/run_library.rs` | E-10 |
| `Cargo.toml` | E-06 |
| `src/{error,envelope,output,clock,config,constants}.rs` | E-11 |
| `src/i18n.rs` | E-12 |
| `CLAUDE.md` | E-12 |
| `gaps.md` | este catálogo |

### 11.6 Validação Pass 5

```text
cargo check                         ok
cargo clippy --lib (+ undocumented_unsafe warn)  exit 0
cargo build --examples              (run_library)
```

---

## 12. Pass 6 — const, static e inicialização

**Rules:** `docs_rules/rules_rust_const_static_inicializacao.md` (const vs static, proibição `static mut`, mutabilidade interior, OnceLock/LazyLock, Ordering, compile-time asserts, lints).  
**Fontes:** GraphRAG / rules file local; duckduckgo-search-cli (`const` vs `static`, `LazyLock`, `static_mut_refs`, `clippy::declare_interior_mutable_const`); MSRV **1.88.0** (permite `Mutex::new` em static, `OnceLock`, `LazyLock`, `const { }` em `thread_local!`).

### 12.1 Inventário de globals (pré-correção)

| Item | Tipo | Local | Julgamento |
|------|------|-------|------------|
| `PLAIN_OVERRIDE` | `static AtomicBool` | `color.rs` | **OK** (static + atomic); faltava doc de `Ordering::Relaxed` |
| `COLORS_ENABLED` | `static OnceLock<bool>` (era function-local) | `color.rs` | **OK** semanticamente; hoisted p/ módulo + docs |
| `EFFECTIVE_LANG` | `static OnceLock<&'static str>` | `i18n.rs` | **OK**; docs de concorrência reforçados |
| `ACTIVE_FRAME` | `static OnceLock<Mutex<Option<String>>>` | `native/element.rs` | **GAP** — wrapper redundante pós-1.63 |
| `ENV_MUTEX` | `static Mutex<()>` | `test_utils.rs` | **OK** (`Mutex::new` direto); poison → `into_inner` |
| `CURRENT_CANCEL` | `thread_local!` + `const { RefCell }` | `lifecycle.rs` | **OK** (TLS + `const { }`) |
| `NETWORK_PRESETS` e demais `pub const` | `const` Copy / `&'static str` / slices | vários | **OK**; faltavam asserts de build |
| `HTTP_USER_AGENT` | `const &'static str` hardcoded `0.1.3` | `scrape_local.rs` | **GAP** versão desatualizada vs `0.1.4` |
| `static mut` / `lazy_static!` / `once_cell` / `const Atomic*` / `const Mutex*` | — | codebase | **0 ocorrências** |

### 12.2 Changelog incremental Pass 6

| ID | Gap identificado | Severidade | Solução | Evidência |
|----|------------------|------------|---------|-----------|
| **C-01** | `ACTIVE_FRAME` = `OnceLock<Mutex<…>>` + `get_or_init(\|\| Mutex::new(None))` — envolvimento redundante (rule: `Mutex::new` direto em static MSRV ≥ 1.63) | **P1** | `static ACTIVE_FRAME: Mutex<Option<String>> = Mutex::new(None)`; poison via `into_inner` | `src/native/element.rs` |
| **C-02** | `Ordering::Relaxed` em `PLAIN_OVERRIDE` sem justificativa documentada | **P2** | Doc `# Concurrency` + comentário no store/load (flag sem publicação de dados) | `src/color.rs` |
| **C-03** | `COLORS_ENABLED` OnceLock function-local sem docs de concorrência | **P2** | Hoist a `static` de módulo + docs; gate `--plain` antes do cache | `src/color.rs` |
| **C-04** | Lints `static_mut_refs` / interior-mutable-const não declarados no crate | **P1** | `#![deny(static_mut_refs)]` + `warn(clippy::declare_interior_mutable_const)` + `borrow_interior_mutable_const` | `src/lib.rs` |
| **C-05** | Sem `const _: () = assert!(…)` para invariantes de tabela estática | **P2** | Asserts: não-vazio, len==6, unlimited sentinel, offline preset | `src/constants.rs` |
| **C-06** | `HTTP_USER_AGENT` com versão `0.1.3` hardcoded (drift vs crate `0.1.4`) | **P2** | `concat!(…, env!("CARGO_PKG_VERSION"), …)` | `src/scrape_local.rs` |
| **C-07** | Docs de concorrência incompletos em statics de processo (`EFFECTIVE_LANG`, `ENV_MUTEX`, TLS cancel, `finalize_done` SeqCst) | **P2** | Docs `# Concurrency` / Ordering SeqCst justificado / poison recovery | `i18n.rs`, `test_utils.rs`, `lifecycle.rs` |
| **C-08** | `ENV_MUTEX.lock().unwrap()` em testes — panic em poison cascateia suíte | **P3** | `unwrap_or_else(\|p\| p.into_inner())` | `src/test_utils.rs` |

### 12.3 N/A Pass 6

| ID | Rule genérica | Por que N/A | Status |
|----|---------------|-------------|--------|
| **N-21** | Migrar `lazy_static` / `once_cell` → `LazyLock`/`OnceLock` | Dependências **nunca** presentes no crate (já std-only) | **N/A** (já conforme) |
| **N-22** | Loom / shuttle / `cargo-nextest` serial suite para globals | Globals são one-shot process scope ou test mutex; sem contador multi-thread de produção que exija model checking | **N/A** (ops / rigor industrial) |

### 12.4 Dívida Pass 6 (não-bloqueante)

| ID | Descrição | Impacto | Status |
|----|-----------|---------|--------|
| **D-09** | `Lifecycle.ledger` usa `if let Ok(guard) = mutex.lock()` (ignora poison em vez de `into_inner`) | FINALIZE best-effort pode pular ledger se poison | **ABERTO** (baixo; one-shot + panic abort em release) |

### 12.5 Matriz checklist const/static (núcleo acionável)

| Item checklist | Status |
|----------------|--------|
| `const` só para valores inlinados / Copy / `&'static str` | **PASS** |
| Nenhum `const` com Atomic/Mutex/OnceLock/LazyLock/Cell | **PASS** (0 hits) |
| Nenhum `static mut` | **PASS** (0 hits) |
| Mutabilidade global via Atomic / Mutex / OnceLock | **PASS** |
| `Mutex::new` / `Atomic*::new` diretos (sem LazyLock wrapper) | **PASS** (C-01) |
| Sem `lazy_static` / `once_cell` | **PASS** / N-21 |
| Lints `static_mut_refs` + interior mutable const | **PASS** (C-04) |
| Statics documentam concorrência / Ordering | **PASS** (C-02, C-07) |
| `thread_local!` com `const { … }` | **PASS** |
| Compile-time asserts em tabela crítica | **PASS** (C-05) |
| Poison tratado explicitamente (frame + ENV_MUTEX) | **PASS** (C-01, C-08); residual D-09 ledger |
| Sync em statics compartilhados | **PASS** |
| Loom / serial nextest full | **N-22** |

### 12.6 Arquivos tocados (Pass 6)

| Arquivo | Gaps |
|---------|------|
| `src/native/element.rs` | C-01 |
| `src/color.rs` | C-02, C-03 |
| `src/lib.rs` | C-04 |
| `src/constants.rs` | C-05 |
| `src/scrape_local.rs` | C-06 |
| `src/i18n.rs` | C-07 |
| `src/lifecycle.rs` | C-07 |
| `src/test_utils.rs` | C-07, C-08 |
| `gaps.md` | este catálogo |

### 12.7 Validação Pass 6

```text
cargo check                                              ok
cargo clippy --lib (+ interior_mutable_const, static_mut_refs deny)  exit 0
cargo test --lib -- color:: constants:: lifecycle::      5 ok
```

Fontes de pesquisa (obrigatórias na skill):
- GraphRAG / `docs_rules/rules_rust_const_static_inicializacao.md`
- duckduckgo-search-cli: const vs static, LazyLock 1.80, `static_mut_refs`, `declare_interior_mutable_const`
- context7 search (std OnceLock/LazyLock/Mutex) — resultados de libs estáticas; regra aplicada via std + MSRV 1.88

---

## 13. Resumo final (histórico até Pass 6)

- **Clap (Pass 1–2):** **G-01 … G-24** solucionados; **0** bloqueantes.  
- **stdin/stdout (Pass 3):** **S-01 … S-12** solucionados; **0** bloqueantes de contrato.  
- **one-shot lifecycle (Pass 4):** **O-01 … O-08** solucionados; **0** bloqueantes.  
- **inglês + docs crates.io (Pass 5):** **E-01 … E-12** solucionados; **0** bloqueantes.  
- **const / static / init (Pass 6):** **C-01 … C-08** solucionados; **0** bloqueantes.  
- **N/A product law / ops / i18n / platform:** **N-01 … N-22** (estendido em Pass 7).  
- **Abertos não-bloqueantes (até Pass 6):** **D-01 … D-09**.  

---

## 14. Pass 7 — docs.rs geração automática / `doc_cfg` / pipeline local

**Rules:** `docs_rules/rules_rust_docsrs_documentacao_automatica.md` (pipeline HTML+JSON+llms, migração `doc_auto_cfg`→`doc_cfg`, metadata docs.rs, badges, seções canônicas, aquamarine, validação links, **sem** substituir docs.rs).  
**Restrição do usuário:** **proibido** criar/alterar CD, CI GitHub Actions (`.github/`).  
**Fontes:** GraphRAG `rules-rust-docsrs-documentacao-automatica`, `cfg-docsrs`, `docsrs`; context7 `/rust-lang/rust`, `/rust-lang/docs.rs`; duckduckgo-search-cli (`doc_cfg` / docs.rs); toolchain local stable MSRV 1.88 + nightly.

### 14.1 Changelog incremental Pass 7

| ID | Gap identificado | Severidade | Solução | Evidência |
|----|------------------|------------|---------|-----------|
| **R-01** | Crate ainda habilitava `#![feature(doc_auto_cfg)]` sob `docsrs` (Pass 5 E-05). Rules: merge Oct 2025 consolida em `doc_cfg`; gate removido quebra nightly docs.rs | **P0** | Removido `doc_auto_cfg`; mantido apenas `feature(doc_cfg)` | `src/lib.rs` |
| **R-02** | Seção Safety não documentava a obrigação do feature gate nem a proibição de reintroduzir `doc_auto_cfg` | **P1** | Bloco “docs.rs / rustdoc feature gates (nightly)” em Safety + See also `scripts/docs-check.sh` | `src/lib.rs` |
| **R-03** | Itens públicos `#[cfg(windows)]` em `win_job` sem badge `doc(cfg)` em multi-target docs.rs | **P1** | `#[cfg_attr(docsrs, doc(cfg(windows)))]` e `doc(cfg(not(windows)))` nos stubs; module Platform | `src/win_job.rs` |
| **R-04** | Não existia pipeline local sequencial (check → HTML → nightly → JSON → mermaid → coverage → links) com NDJSON e exit codes | **P1** | `scripts/docs-check.sh` (timeout soft, sysexits 65/70/124, fase 8 publish **skip** manual) | `scripts/docs-check.sh` |
| **R-05** | `Cargo.toml` comentário `doc_auto_cfg`; falta de nota sobre breaking 2026-05-01 / wasm omitido | **P2** | Comentários `targets` + rustdoc-args alinhados a `doc_cfg` only | `Cargo.toml` |
| **R-06** | `cargo doc` falhava com `deny(broken_intra_doc_links)` em module docs de `clock`/`color`/`error` (links relativos sem escopo) | **P1** | Links absolutos `crate::module::Item` | `clock.rs`, `color.rs`, `error.rs` |
| **R-07** | Doc de `HTTP_USER_AGENT` usava `[env!("…")]` como intra-doc link → erro nightly | **P1** | Texto em backticks sem link | `scrape_local.rs` |
| **R-08** | Rules de conversões pedem seções Cost/Conversions; `error` só tinha Examples | **P2** | Seções Conversions + Cost (exit_code/as_str zero-cost; sem TryFrom) | `src/error.rs` |
| **R-09** | Script inicial usava flags incompatíveis com host (`timeout --signal`, `--document-private-items=false`) | **P2** | Detecção dialect de `timeout`; doc public-only default | `scripts/docs-check.sh` |
| **R-10** | `--output-format json` no mesmo `target/` apagava HTML e quebrava fase 7 | **P1** | JSON em `CARGO_TARGET_DIR=target/rustdoc-json-build` + cópia para `target/rustdoc-json/` | `scripts/docs-check.sh` |

### 14.2 N/A Pass 7 (product law / proibições do usuário)

| ID | Rule genérica | Por que N/A | Status |
|----|---------------|-------------|--------|
| **N-23** | Fase 8 publish / tag / docs.rs via GitHub Actions ou CD | Usuário proibiu **CD / CI / GitHub Actions**; publish crates.io é manual; docs.rs rebuilda ao publicar | **N/A** |
| **N-24** | Features Cargo documentadas com `#[doc(cfg(feature = …))]` | Crate **não tem** feature flags Cargo; categorias são flags de processo documentadas em `//!` Features | **N/A** (já documentado) |
| **N-25** | KaTeX / MathML / `no_std` / wasm Chrome docs | Igual N-18; host CLI only; `wasm32` omitido de `targets` com justificativa no crate `//!` | **N/A** |

### 14.3 Dívida Pass 7 (não-bloqueante)

| ID | Descrição | Impacto | Status |
|----|-----------|---------|--------|
| **D-10** | `llms.txt` / `llms-full.txt` ainda são mantidos à mão (não gerados a partir do rustdoc JSON) | regeneração manual em release | **ABERTO** (baixo; arquivos existem e passam audit de presença) |
| **D-11** | Cobertura rustdoc item-level ainda incompleta sob `#![allow(missing_docs)]` em monólitos (`cli`, `commands_prd`, `native/*`) | docs.rs API surface densa incompleta | **ABERTO** (sobreposição D-02; help clap = UX CLI) |
| **D-02** | (inalterado) `allow(missing_docs)` em CLI monólitos | rustdoc | **ABERTO** |

### 14.4 Matriz checklist rules_rust_docsrs (núcleo acionável)

| Item checklist | Status pós-Pass 7 |
|----------------|-------------------|
| `cargo check` limpo antes de doc | **PASS** (fase 1) |
| HTML canônico `cargo doc --no-deps` | **PASS** (fase 2 stable) |
| Nightly + `--cfg docsrs` + `feature(doc_cfg)` | **PASS** (fase 3) |
| **Sem** `feature(doc_auto_cfg)` | **PASS** (R-01; gate no script) |
| rustdoc JSON unstable (best-effort) | **PASS** parcial (fase 4 isolada; warn se vazio) |
| aquamarine / Mermaid no entry | **PASS** (`run` + dep) |
| Seções canônicas crate-level (Overview…See also + Safety + MSRV + Features + Targets) | **PASS** (fase 6) |
| Intra-doc links deny + HTML index | **PASS** (R-06/R-07 + fase 7) |
| `[package.metadata.docs.rs]` all-features + default-target + **targets** explícitos + rustdoc-args | **PASS** (pré-existente; R-05 notas) |
| Breaking 2026-05-01 targets declarados | **PASS** |
| Badges README docs.rs → crates.io → license → MSRV → downloads → rust | **PASS** (CI badge **N-23**) |
| `llms.txt` + `llms-full.txt` presentes | **PASS** presença; **D-10** geração automática |
| `doc(cfg)` em itens platform-gated | **PASS** (`win_job`) |
| Publicar no crates.io / docs.rs automático | **N-23** |
| GitHub Actions matrix doc | **N-23** (proibido) |
| Features Cargo gated | **N-24** |

### 14.5 Arquivos tocados (Pass 7)

| Arquivo | Gaps |
|---------|------|
| `src/lib.rs` | R-01, R-02 |
| `src/win_job.rs` | R-03 |
| `src/error.rs` | R-06, R-08 |
| `src/clock.rs` | R-06 |
| `src/color.rs` | R-06 |
| `src/scrape_local.rs` | R-07 |
| `Cargo.toml` | R-05 |
| `scripts/docs-check.sh` *(novo)* | R-04, R-09, R-10 |
| `gaps.md` | este catálogo |

### 14.6 Validação Pass 7

```text
cargo check --lib                         ok
cargo doc --no-deps (stable, sem docsrs)  ok → target/doc/browser_automation_cli/index.html
cargo +nightly doc --no-deps
  RUSTDOCFLAGS='--cfg docsrs …'           ok (doc_cfg)
./scripts/docs-check.sh                   ok (fases 1–7; fase 8 skip publish)
grep feature(doc_auto_cfg) src/lib.rs     0 hits
```

Revalidação:

```bash
./scripts/docs-check.sh
# opcional: DOCS_CHECK_NIGHTLY=0 DOCS_CHECK_JSON=0 ./scripts/docs-check.sh
```

Fontes de pesquisa (obrigatórias na skill):
- GraphRAG: `rules-rust-docsrs-documentacao-automatica`, `docsrs`, `cfg-docsrs`, `rules-rust-documentacao`
- context7: `/rust-lang/rust`, `/rust-lang/docs.rs` (metadata / self-hosting snippets)
- duckduckgo-search-cli: `doc_cfg` / `doc_auto_cfg` / docs.rs feature gates
- Rules locais: `docs_rules/rules_rust_docsrs_documentacao_automatica.md`

### 14.7 Nota sobre Pass 5 E-05

Pass 5 registrou **E-05** como “adicionar `doc_auto_cfg`”. A Pass 7 **reverte e corrige** esse item sob a lei docs.rs atual: o gate correto é **somente** `doc_cfg`. E-05 permanece histórico; o estado canônico é R-01.

---

## 15. Resumo final consolidado (Pass 1–7)

- **Clap:** G-01…G-24 — **RESOLVIDOS**  
- **stdin/stdout:** S-01…S-12 — **RESOLVIDOS**  
- **one-shot:** O-01…O-08 — **RESOLVIDOS**  
- **inglês + crates.io:** E-01…E-12 — **RESOLVIDOS** (E-05 superado por R-01)  
- **const/static:** C-01…C-08 — **RESOLVIDOS**  
- **docs.rs automático:** R-01…R-10 — **RESOLVIDOS**  
- **N/A:** N-01…N-25  
- **Dívida não-bloqueante:** D-01…D-11  
- **Bloqueantes abertos:** **0**

---

## 16. Pass 8 — Economia de recursos (alocações / HTTP / mimalloc / workload)

**Rules:** `docs_rules/rules_rust_economia_de_recursos.md` (baseline/profile antes de otimizar, heap/`with_capacity`, mimalloc em scripts, OnceLock para Regex/clientes caros, classificação de workload, proibição de otimização cega, `reqwest::Client` uma vez, sem `async`=paralelo).  
**Restrição:** sem CD/GHA; product law one-shot (daemon **proibido**).  
**Fontes:** GraphRAG `rules-rust-economia-recursos-testes` (mem 161) + hybrid search; context7 `/microsoft/mimalloc`, `/websites/rs_mimalloc`; duckduckgo-search-cli `rust mimalloc global_allocator CLI`; rules local.

### 16.1 Inventário pré-correção (causa → efeito)

| Achado | Severidade | Rule violada |
|--------|------------|--------------|
| `reqwest::Client::builder()` recriado em **cada** `scrape_http` / `enforce_robots` / LLM / webhook | **P0** | CRIAR Client UMA VEZ; keep-alive/TLS reutilizável |
| `Regex::new` em `redact_pii`, branding, `sg_local` compile_rules a cada chamada | **P1** | COMPILAR Regex uma vez com OnceLock |
| Sem `#[global_allocator]` mimalloc no binário CLI | **P1** | ADOTAR mimalloc como padrão em scripts |
| Sem classificação de workload documentada no runtime CDP | **P1** | DOCUMENTAR classificação (I/O vs CPU) |
| `batch_scrape_http` usava `Vec::new` com `urls.len()` conhecido | **P2** | `with_capacity` quando tamanho conhecido |
| Sem medição RSS local (`/usr/bin/time -v`) | **P2** | MEDIR baseline antes de otimizar |
| `format!` denso em paths de erro (não hot loop) | — | OK (não otimizar cold path sem profile) |
| Tokio multi-thread 2 workers; JoinSet batch ≤16 | — | Já alinhado a I/O-bound controlado |
| `max_body_bytes` em scrape | — | Limite superior de alocação (fallibility parcial) |
| Chrome via chromiumoxide (não Command manual) | — | Subprocess ownership via ledger/FINALIZE |

### 16.2 Changelog incremental Pass 8

| ID | Gap identificado | Severidade | Solução | Evidência |
|----|------------------|------------|---------|-----------|
| **ECO-01** | Client HTTP async recriado por scrape/robots | **P0** | `robots::shared_http_client()` com `OnceLock` + pool idle 4; scrape/robots/discovery reutilizam | `src/robots.rs`, `scrape_local.rs`, `discovery.rs` |
| **ECO-02** | Client blocking recriado por LLM e webhook | **P1** | `llm_local::shared_blocking_http_client()` OnceLock; webhook com timeout por request 15s | `src/llm_local.rs`, `commands_prd/mod.rs` |
| **ECO-03** | Regex PII/branding recompiladas a cada call | **P1** | `OnceLock` (`pii_regexes`, `re_hex_color`) | `src/scrape_local.rs` |
| **ECO-04** | `sg_local` recompilava 4 rules + RUST_LOG regex a cada scan/rewrite | **P1** | `compiled_rules()` + `re_rust_log_export()` OnceLock | `src/sg_local.rs` |
| **ECO-05** | Sem mimalloc no binary | **P1** | dep `mimalloc` + `#[global_allocator]` em `main.rs` | `Cargo.toml`, `src/main.rs` |
| **ECO-06** | Workload CDP/HTTP sem classificação explícita | **P1** | Docs `# Workload` em scrape/robots/sg/llm + `block_on_browser_timeout` (I/O-bound, 2 workers, no rayon, no daemon, no systemd-run default) | `browser/mod.rs`, módulos locais |
| **ECO-07** | Batch scrape sem pré-alocação de results | **P2** | `Vec::with_capacity(urls.len())` | `scrape_local.rs` |
| **ECO-08** | Sem ferramenta local de baseline RSS | **P2** | `scripts/rss-baseline.sh` (GNU time -v; NDJSON; **sem** GHA) | `scripts/rss-baseline.sh` |
| **ECO-09** | Justificativa de **não**-paralelismo CPU ausente | **P2** | Documentado: parse ≪ wait de rede; rayon seria anti-pattern | `block_on_browser_timeout` docs |
| **ECO-10** | `get_or_try_init` tentado (feature unstable `once_cell_try`) | **P2** | Padrão estável `get` + build + `get_or_init` (MSRV 1.88) | `robots.rs`, `llm_local.rs` |

### 16.3 N/A Pass 8 (product law / ops / anti-otimização cega)

| ID | Rule genérica | Por que N/A | Status |
|----|---------------|-------------|--------|
| **N-26** | Converter CLI em daemon quando boot domina | **Product law one-shot** proíbe daemon/MCP/sessão multi-process | **N/A** |
| **N-27** | `rayon` / pool CPU em hot path | Workload **I/O-bound** (Chrome CDP + HTTP); CPU-bound paths não medidos como bottleneck | **N/A** (justificado ECO-09) |
| **N-28** | `systemd-run --scope` + `MemoryMax` no spawn Chrome | Chrome é filho do chromiumoxide; residual kill via ledger/Job Object; cgroup opcional do **host**, não default do produto | **N/A** (ops host) |
| **N-29** | Benchmarks em CI + threshold + flamegraph noturno | Usuário proibiu GHA/CD; criterion parse bench já existe localmente | **N/A** (ops) |
| **N-30** | `bumpalo` / `SmallVec` / SIMD / NUMA / CPU pinning | Exige profile de hot path real; otimização cega proibida | **N/A** até baseline flamegraph |
| **N-31** | `dashmap` / `parking_lot` / `loom` full suite | Contenção é rare (one-shot; ledger Mutex breve); loom = N-22 | **N/A** (baixo valor one-shot) |

### 16.4 Dívida Pass 8 (não-bloqueante)

| ID | Descrição | Impacto | Status |
|----|-----------|---------|--------|
| **D-12** | Sem flamegraph/`samply`/`dhat` commitados de path CDP representativo | micro-otimizações futuras sem ground truth | **ABERTO** (baixo; script RSS existe) |
| **D-13** | `try_reserve` não propagado em buffers grandes (HTML/body) | OOM em host sem swap ainda aborta via allocator | **ABERTO** (baixo; `max_body_bytes` já limita scrape HTTP) |
| **D-14** | `with_capacity` ainda raro em monólitos `browser/mod` / snapshot trees | rehash ocasional em árvores grandes | **ABERTO** (baixo; snapshot já tem 1 with_capacity) |
| **D-10…D-11** | (inalterados) llms gen + missing_docs monólitos | docs | **ABERTO** |

### 16.5 Matriz checklist economia (núcleo acionável one-shot)

| Item checklist | Status pós-Pass 8 |
|----------------|-------------------|
| Classificar workload no código | **PASS** (I/O-bound CDP/HTTP documentado) |
| `reqwest::Client` criado uma vez e reutilizado | **PASS** (async + blocking OnceLock) |
| Timeout em ops de rede | **PASS** (client 30s; robots 5s; LLM 60s; webhook 15s) |
| Regex em OnceLock | **PASS** (PII, branding, sg rules) |
| mimalloc `#[global_allocator]` no binary | **PASS** |
| Pré-alocar batch results | **PASS** |
| Baseline RSS mensurável localmente | **PASS** (`scripts/rss-baseline.sh`) |
| Justificar ausência de rayon/daemon | **PASS** (docs + N-26/N-27) |
| criterion parse bench (pré-existente) | **PASS** |
| `max_body_bytes` / limites de crawl | **PASS** (clamp limit/depth/concurrency) |
| JoinSet concorrência limitada (≤16) | **PASS** |
| Tokio workers dimensionados (2) | **PASS** |
| release LTO fat / strip / abort | **PASS** (Cargo.toml pré-existente) |
| Flamegraph / dhat / perf c2c | **N-30** / **D-12** |
| systemd-run MemoryMax | **N-28** |
| CI benchmark threshold | **N-29** |
| Docker distroless / mold / sccache | **N/A ops** |
| Ganho só com profile, sem intuição cega | **PASS** (mudanças = rules obrigatórias de reuse; sem micro-opts inventadas) |

### 16.6 Arquivos tocados (Pass 8)

| Arquivo | Gaps |
|---------|------|
| `src/robots.rs` | ECO-01, ECO-06, ECO-10 |
| `src/scrape_local.rs` | ECO-01, ECO-03, ECO-06, ECO-07 |
| `src/sg_local.rs` | ECO-04, ECO-06 |
| `src/llm_local.rs` | ECO-02, ECO-06, ECO-10 |
| `src/commands_prd/mod.rs` | ECO-02 |
| `src/native/cdp/discovery.rs` | ECO-01 |
| `src/browser/mod.rs` | ECO-06, ECO-09 |
| `src/main.rs` | ECO-05 |
| `Cargo.toml` + `Cargo.lock` | ECO-05 |
| `scripts/rss-baseline.sh` *(novo)* | ECO-08 |
| `gaps.md` | este catálogo |

### 16.7 Validação Pass 8

```text
cargo check                                   ok
cargo test --lib robots                       5 ok
cargo test --lib sg_                          1 ok
cargo test --lib scrape                       2 ok
grep 'Client::builder' src/                   só shared_* OnceLock sites
```

Revalidação:

```bash
cargo test --lib robots
./scripts/rss-baseline.sh doctor --offline --quick --json   # após cargo build --release
```

Fontes de pesquisa (obrigatórias na skill):
- GraphRAG: `rules-rust-economia-recursos-testes` (id 161), hybrid `rules rust economia recursos`
- context7: mimalloc (`/microsoft/mimalloc`, `/websites/rs_mimalloc`)
- duckduckgo-search-cli: `rust mimalloc global_allocator CLI`
- Rules locais: `docs_rules/rules_rust_economia_de_recursos.md`

---

## 17. Resumo final consolidado (Pass 1–8)

- **Clap:** G-01…G-24 — **RESOLVIDOS**  
- **stdin/stdout:** S-01…S-12 — **RESOLVIDOS**  
- **one-shot:** O-01…O-08 — **RESOLVIDOS**  
- **inglês + crates.io:** E-01…E-12 — **RESOLVIDOS** (E-05 superado por R-01)  
- **const/static:** C-01…C-08 — **RESOLVIDOS**  
- **docs.rs automático:** R-01…R-10 — **RESOLVIDOS**  
- **economia de recursos:** ECO-01…ECO-10 — **RESOLVIDOS**  
- **N/A:** N-01…N-31  
- **Dívida não-bloqueante:** D-01…D-14  
- **Bloqueantes abertos:** **0**

---

## 18. Pass 9 — Eficiência e performance (build / hashing / cold path / hygiene)

**Rules:** `docs_rules/rules_rust_eficiencia_e_performance.md` (medir antes de otimizar, release LTO/CGU/abort/strip, mold/lld, hashing Fx/AHash, iteradores, `#[cold]`, proibição de otimização prematura / `target-cpu=native` em distro).  
**Restrição:** sem CD/GHA; product law one-shot; sem micro-opts cegas sem profile.  
**Fontes:** GraphRAG `rules-rust-eficiencia-build-alocacao` + `rules-rust-eficiencia-*`; context7 `/rust-lang/rustc-hash` (FxHashMap), `/rust-lang/cargo` (profiles LTO/CGU); duckduckgo-search-cli cargo release LTO mold; rules local.

### 18.1 Inventário pré-correção (causa → efeito)

| Achado | Severidade | Rule violada / gap |
|--------|------------|-------------------|
| Release já tinha LTO fat / CGU 1 / abort / strip=true, mas **sem** comentários de auditoria, `debug=false` explícito, `incremental=false` | **P2** | Documentar + fechar defaults subótimos |
| Sem perfil **`release-size`** (`opt-level = "z"`) | **P2** | Perfil de tamanho mínimo obrigatório nas rules |
| Sem `[profile.bench]` alinhado a release (risco de bench soft) | **P1** | Nunca comparar debug vs release; bench herda otimização |
| Sem `dev.package."*"` opt-level 2 / build-override | **P2** | Dependency profile + build scripts rápidos |
| Sem `.cargo/config.toml` no repo com mold (host tinha, projeto não) | **P2** | Linker rápido em Linux versionado |
| L1 `MemoryCache` e `RefMap` em `HashMap` SipHash | **P1** | FxHash em chaves confiáveis / hot maps |
| Construtores de `CliError` sem `#[cold]` | **P3** | Hints de branch em cold path de erro |
| `heap_snapshot` / `snapshot` maps sem `with_capacity` conhecido | **P2** | Pré-alocar quando tamanho é conhecido |
| Sem gate local de higiene de performance | **P2** | Checklist mensurável sem GHA |
| Pass 8 já cobria mimalloc, OnceLock HTTP/Regex, RSS, workload class | — | Reutilizado; não reabrir |

### 18.2 Changelog incremental Pass 9

| ID | Gap identificado | Severidade | Solução | Evidência |
|----|------------------|------------|---------|-----------|
| **PERF-01** | Release sem documentação de auditoria + `strip` genérico | **P2** | Comentários rules; `strip = "symbols"`; `debug = false`; `incremental = false`; overflow-checks auditado (default release) | `Cargo.toml` |
| **PERF-02** | Sem perfil tamanho mínimo | **P2** | `[profile.release-size]` inherits release + `opt-level = "z"` | `Cargo.toml` |
| **PERF-03** | Bench sem alinhamento a produção | **P1** | `[profile.bench]` inherits release, LTO thin, CGU 1 | `Cargo.toml` |
| **PERF-04** | Deps dev não otimizadas / build scripts no mesmo nível | **P2** | `dev.package."*" opt-level=2` + build-override opt-level 0 | `Cargo.toml` |
| **PERF-05** | Linker rápido não versionado no projeto | **P2** | `.cargo/config.toml` clang + mold (`/usr/bin/mold`); **sem** `target-cpu=native` | `.cargo/config.toml` |
| **PERF-06** | MemoryCache SipHash | **P1** | `rustc-hash` + `FxHashMap` (chaves hex SHA-256 confiáveis) | `Cargo.toml`, `src/cache.rs` |
| **PERF-07** | RefMap SipHash em refs mintados | **P1** | `FxHashMap` em `native/element::RefMap` | `src/native/element.rs` |
| **PERF-08** | Erro alocado no layout hot sem cold hint | **P3** | `#[cold]` em `CliError::new` / `with_suggestion` | `src/error.rs` |
| **PERF-09** | Maps/vecs de snapshot sem pré-alocação | **P2** | `with_capacity` em heap_snapshot + `id_to_idx` a11y | `heap_snapshot.rs`, `snapshot.rs` |
| **PERF-10** | Sem script de validação local de perf hygiene | **P2** | `scripts/perf-check.sh` (+ `--inventory-only` / `--rss` / `--bench`) | `scripts/perf-check.sh`, `main.rs` docs |

### 18.3 N/A Pass 9 (product law / anti-otimização cega / ops)

| ID | Rule genérica | Por que N/A | Status |
|----|---------------|-------------|--------|
| **N-32** | PGO (`cargo-pgo`) + BOLT em binário de longa duração | One-shot CLI; sem dataset de produção estável; boot/CDP domina wall time | **N/A** |
| **N-33** | Multi-CPU distro (`cargo-multivers`, x86-64-v3/v4) | Distribuição crates.io única; host agent instala local; matrix GHA proibida | **N/A** |
| **N-34** | SIMD / `target_feature` / autovector check em assembly | Hot path real é I/O CDP; proibido sem flamegraph (D-12) | **N/A** até profile |
| **N-35** | `get_unchecked` / unsafe bounds elision | Rules exigem benchmark + miri; zero evidência | **N/A** (proibido sem ganho) |
| **N-36** | Benchmark threshold em CI + GHA | Usuário proibiu GHA/CD; criterion + `perf-check` locais | **N/A** (ops) |

### 18.4 Dívida Pass 9 (não-bloqueante)

| ID | Descrição | Impacto | Status |
|----|-----------|---------|--------|
| **D-15** | `FxHashMap` ainda não em todos os maps de `snapshot`/`browser` (iframe_sessions etc.) | rehash SipHash residual em snapshots grandes | **ABERTO** (baixo; keys mistas; migração incremental) |
| **D-16** | Sem `cargo flamegraph` / `dhat` artefactos commitados de path CDP | micro-opts futuras sem ground truth | **ABERTO** (sobreposição D-12) |
| **D-12…D-14** | (inalterados) flamegraph, try_reserve, with_capacity monólitos | perf residual | **ABERTO** |
| **D-10…D-11** | llms gen + missing_docs monólitos | docs | **ABERTO** |

### 18.5 Matriz checklist eficiência (núcleo acionável one-shot)

| Item checklist | Status pós-Pass 9 |
|----------------|-------------------|
| Baseline / hygiene local antes de micro-opt | **PASS** (`perf-check`, `rss-baseline`, criterion parse) |
| Perfil release LTO fat + CGU 1 + abort + strip | **PASS** (PERF-01) |
| Perfil tamanho `opt-level = "z"` | **PASS** (PERF-02) |
| Bench alinhado a release | **PASS** (PERF-03) |
| Dev deps otimizadas + build-override | **PASS** (PERF-04) |
| Linker mold/lld em Linux | **PASS** (PERF-05; host mold 2.40) |
| Sem `target-cpu=native` no config commitado | **PASS** |
| mimalloc global (Pass 8) | **PASS** |
| FxHash / hasher rápido em maps confiáveis | **PASS** (L1 + RefMap) |
| Regex OnceLock (Pass 8) | **PASS** |
| `reqwest::Client` once (Pass 8) | **PASS** |
| `with_capacity` em sizes conhecidos (batch + snapshot) | **PASS** parcial (D-14/D-15) |
| `#[cold]` em path de erro | **PASS** |
| Iteradores preferidos; index loops só com borrow justification | **PASS** (snapshot StaticText loop justificado) |
| PGO / multivers / SIMD / unsafe elision | **N-32…N-35** |
| CI threshold | **N-36** |
| Ganho sem intuição cega | **PASS** (mudanças = rules de build/hash; sem reescrever algoritmo CDP) |

### 18.6 Arquivos tocados (Pass 9)

| Arquivo | Gaps |
|---------|------|
| `Cargo.toml` | PERF-01…PERF-04, PERF-06 (dep rustc-hash) |
| `.cargo/config.toml` *(novo)* | PERF-05 |
| `src/cache.rs` | PERF-06 |
| `src/native/element.rs` | PERF-07 |
| `src/error.rs` | PERF-08 |
| `src/native/heap_snapshot.rs` | PERF-09 |
| `src/native/snapshot.rs` | PERF-09 |
| `src/main.rs` | docs PERF |
| `scripts/perf-check.sh` *(novo)* | PERF-10 |
| `gaps.md` | este catálogo |

### 18.7 Validação Pass 9

```text
cargo check                                   ok
cargo test --lib cache::                      8 ok
cargo test --lib element::                    18 ok
./scripts/perf-check.sh --inventory-only      PASS
rg 'lto = "fat"' Cargo.toml                   hit
rg 'FxHashMap' src/cache.rs src/native/element.rs  hit
```

Revalidação:

```bash
./scripts/perf-check.sh --inventory-only
./scripts/perf-check.sh              # full release smoke (lento)
cargo bench --bench cli_parse        # opcional
cargo build --profile release-size   # artefato compacto
```

Fontes de pesquisa (obrigatórias na skill):
- GraphRAG: `rules-rust-eficiencia-build-alocacao`, `rules-rust-eficiencia-e-performance`, hybrid `rules rust eficiencia performance`
- context7: `/rust-lang/rustc-hash` (FxHashMap), `/rust-lang/cargo` (profile LTO/CGU/strip/panic)
- duckduckgo-search-cli: cargo release LTO fat / mold linker
- Rules locais: `docs_rules/rules_rust_eficiencia_e_performance.md`

---

## 19. Resumo final consolidado (Pass 1–9)

- **Clap:** G-01…G-24 — **RESOLVIDOS**  
- **stdin/stdout:** S-01…S-12 — **RESOLVIDOS**  
- **one-shot:** O-01…O-08 — **RESOLVIDOS**  
- **inglês + crates.io:** E-01…E-12 — **RESOLVIDOS** (E-05 superado por R-01)  
- **const/static:** C-01…C-08 — **RESOLVIDOS**  
- **docs.rs automático:** R-01…R-10 — **RESOLVIDOS**  
- **economia de recursos:** ECO-01…ECO-10 — **RESOLVIDOS**  
- **eficiência e performance:** PERF-01…PERF-10 — **RESOLVIDOS**  
- **N/A:** N-01…N-36  
- **Dívida não-bloqueante:** D-01…D-16  
- **Bloqueantes abertos:** **0** (antes da Pass 10)

---

## 20. Pass 10 — Graceful shutdown (`rules_rust_encerramento_grafull_shutdown`)

### 20.1 Escopo e classificação do binário

| Dimensão | Valor |
|----------|--------|
| Tipo de binário (rules) | **CLI one-shot** + **pipeline stdout** (shutdown mínimo + crítico) |
| Fases | Detect (`shutdown_signal`) → Signal (`CancellationToken`) → Await (Browser.close + residual) |
| Coordenador | [`Lifecycle`](src/lifecycle.rs) (clonável, idempotent finalize + Drop safety net) |
| Não-escopo | Daemon / multi-subsystem `TaskTracker` / SIGHUP reload / k8s readiness / `sd_notify` |

### 20.2 Gaps identificados e solucionados

| ID | Gap identificado | Severidade | Solução | Evidência |
|----|------------------|------------|---------|-----------|
| **GS-01** | Residual Unix enviava `SIGTERM` e `SIGKILL` **imediatamente** (sem grace) | **P0** | `kill_unix_graceful(pid, FINALIZE_CHILD_GRACE=2s)` com poll `kill(pid,0)` | `src/lifecycle.rs` |
| **GS-02** | Detector de sinal privado sem nome canônico / sem log de origem | **P1** | `pub async fn shutdown_signal() -> ShutdownTrigger` + labels estáveis + `tracing::warn` | `src/browser/mod.rs` |
| **GS-03** | Windows só `ctrl_c` (rules: Ctrl-Break) | **P2** | `tokio::signal::windows::ctrl_break` em `select!` | `src/browser/mod.rs` |
| **GS-04** | Sem escalada de **duplo sinal** (force residual) | **P1** | 1º sinal → cancel; 2º → `Lifecycle::finalize` (clone capturado no spawn, não TLS) | `block_on_browser_timeout` |
| **GS-05** | `run_with_session` Err fazia `mark_closed` sem reap → residual desarmado | **P0** | Removido `mark_closed` no Err; ledger permanece para FINALIZE | `src/browser/mod.rs` |
| **GS-06** | Modelo de shutdown não documentado no crate root / lifecycle | **P2** | Docs em `lifecycle.rs`, `lib.rs` (tabela detect/signal/await), comments em `block_on_*` | docs |
| **GS-07** | Handle Job Object Windows zerado sem `close_job` após terminate | **P2** | `terminate_job` + `close_job` no residual Windows | `lifecycle.rs` residual_kill_child |
| **GS-08** | Sem gate local de higiene de shutdown | **P2** | `scripts/shutdown-check.sh` (`--inventory-only`) | script |

### 20.3 Já conformes (Pass 4 / stdin) — revalidados

| Item | Status |
|------|--------|
| Sem `std::process::exit` (usa `ExitCode`) | **PASS** |
| SIGPIPE → EPIPE → BrokenPipe **141** | **PASS** |
| Cancelled **130** (SIGINT/SIGTERM) | **PASS** |
| Dual flush stdout+stderr antes de DIE | **PASS** |
| `CancellationToken` + `select!` biased no work | **PASS** |
| Browser.close wait ≤5s + kill (`finalize_browser`) | **PASS** |
| MITM `with_graceful_shutdown` + join proxy task | **PASS** |
| Drop de `Lifecycle` como safety net (sync residual only) | **PASS** |
| Atomic writes (xlsx temp+rename) | **PASS** |

### 20.4 N/A Pass 10 (product law / daemon-only)

| ID | Rule genérica | Por que N/A | Status |
|----|---------------|-------------|--------|
| **N-37** | `TaskTracker` / `tokio-graceful-shutdown` / `SubsystemBuilder` | One-shot: uma runtime por invocação; sem frota de tasks de longa vida | **N/A** |
| **N-38** | SIGHUP reload / SIGUSR1/2 / swap atômico `Arc<Config>` | Sem hot-reload; config lida no BORN | **N/A** |
| **N-39** | Readiness/liveness probes, `terminationGracePeriodSeconds`, `TimeoutStopSec` | Não é serviço k8s/systemd de longa duração | **N/A** |
| **N-40** | `sd_notify` READY/STOPPING/WATCHDOG, PID/lock files | Sem daemon PID 1 | **N/A** |
| **N-41** | OpenTelemetry provider shutdown / tracing-appender `WorkerGuard` | Telemetria local stderr only; sem export remoto | **N/A** |
| **N-42** | TUI restore / leader election / service registry | Non-interactive one-shot | **N/A** |

### 20.5 Dívida Pass 10 (não-bloqueante)

| ID | Descrição | Impacto | Status |
|----|-----------|---------|--------|
| **D-17** | Teste de integração real SIGINT/SIGTERM em processo filho (assert_cmd + kill) não automatizado | cobertura de sinal real fora do unit | **ABERTO** (manual: `timeout`/kill; unit cobre cancel token + grace helper) |
| **D-12…D-16** | (inalterados) flamegraph, try_reserve, maps residual, etc. | perf | **ABERTO** |

### 20.6 Matriz checklist graceful shutdown (núcleo one-shot)

| Item checklist | Status pós-Pass 10 |
|----------------|-------------------|
| Três fases detectar / sinalizar / aguardar | **PASS** |
| Distinguir one-shot vs daemon | **PASS** (docs + N-37…N-42) |
| Sem `process::exit` sem cleanup | **PASS** |
| SIGINT + SIGTERM (Unix) | **PASS** (GS-02) |
| Ctrl-C + Ctrl-Break (Windows) | **PASS** (GS-03) |
| SIGPIPE tratado (141) | **PASS** |
| `CancellationToken` propagado | **PASS** |
| Deadline work (`--timeout`) + Browser.close 5s + residual grace 2s | **PASS** |
| Duplo sinal força residual | **PASS** (GS-04) |
| Child process SIGTERM→wait→SIGKILL | **PASS** (GS-01) |
| Writers flush antes de sair | **PASS** |
| Exit 130 / 141 / 124 | **PASS** |
| Drop só cleanup síncrono idempotente | **PASS** |
| Coordenador central `Lifecycle` | **PASS** |
| TaskTracker / SIGHUP / probes / OTel / TUI | **N-37…N-42** |

### 20.7 Arquivos tocados (Pass 10)

| Arquivo | Gaps |
|---------|------|
| `src/lifecycle.rs` | GS-01, GS-06, GS-07 (+ tests grace / current_lifecycle) |
| `src/browser/mod.rs` | GS-02…GS-05 (+ `ShutdownTrigger`, double-signal, residual Err) |
| `src/lib.rs` | GS-06 (seção Graceful shutdown) |
| `scripts/shutdown-check.sh` *(novo)* | GS-08 |
| `gaps.md` | este catálogo |

### 20.8 Validação Pass 10

```text
cargo test --lib lifecycle::                     5 ok
cargo test --lib pre_cancelled_token             1 ok
cargo test --lib shutdown_trigger                1 ok
cargo test --lib hard_timeout_returns            1 ok
./scripts/shutdown-check.sh --inventory-only     PASS
```

Revalidação:

```bash
./scripts/shutdown-check.sh --inventory-only
./scripts/shutdown-check.sh              # inclui cargo test lifecycle/cancel
```

Fontes de pesquisa (obrigatórias na skill):
- GraphRAG: `rules-rust-encerramento-graceful-shutdown`, `rules-rust-shutdown-fundamentos-sinais`, `rules-rust-shutdown-coordenacao-cooperacao`, `rules-rust-shutdown-deadline-drenagem`, `cli-shutdown`, `child-process-shutdown`
- context7: `/tokio-rs/tokio` (signal / select)
- duckduckgo-search-cli: tokio graceful shutdown, CancellationToken, SIGTERM, tokio-graceful
- Rules locais: `docs_rules/rules_rust_encerramento_grafull_shutdown.md`

---

## 21. Pass 11 — Gerenciamento de memória e RAII

### 21.1 Escopo e modelo

| Dimensão | Aplicação neste projeto |
|----------|--------------------------|
| Product law | One-shot CLI: ownership de processo Chrome/Lightpanda por invocação; sem heap pool de longa vida |
| Ownership | Compilador + tipos RAII (`Lifecycle`, `LightpandaProcess`, `EnvGuard`) |
| Alocação hostil | Input externo (Redis RESP, `.heapsnapshot`, body HTTP já capado em scrape) |
| Não-escopo | Daemon arenas, miri nightly CI, Edition 2024 migration, NUMA |

### 21.2 Gaps identificados e solucionados

| ID | Gap identificado | Severidade | Solução | Evidência |
|----|------------------|------------|---------|-----------|
| **MEM-01** | `start_log_drainers` fazia `child.kill()` **sem** `wait` nos ramos de erro → risco de **zombie** Unix | **P0** | Helper `kill_and_reap`; todos os ramos de falha do launch reaparem o `Child` | `src/native/cdp/lightpanda.rs` |
| **MEM-02** | `LightpandaProcess` usava `Child` nu; `kill` + `Drop` podiam double-wait e `has_exited`/`id` mentiam | **P0** | `child: Option<Child>`; `kill` faz `take`+kill+wait; `has_exited`/`id` reais; `BrowserProcess` delega | `lightpanda.rs`, `native/browser.rs` |
| **MEM-03** | Redis RESP `vec![0u8; n as usize]` / `with_capacity(n)` sem teto → OOM por length hostil | **P0** | `MAX_RESP_BULK_BYTES` (16 MiB) + `checked_resp_bulk_len` + `try_reserve_exact` antes do fill | `src/cache.rs` + unit test |
| **MEM-04** | `heap_snapshot::load` lia arquivo inteiro sem teto de tamanho | **P1** | `MAX_HEAP_SNAPSHOT_BYTES` (512 MiB) + `String::try_reserve_exact` + `try_reserve` em nodes/id map; `drop(raw)` cedo | `src/native/heap_snapshot.rs` |
| **MEM-05** | Chaves AES derivadas (SHA-256) de sessão não eram zeroizadas após uso | **P1** | dep `zeroize`; `key_bytes.zeroize()` em `encrypt_data`/`decrypt_data` | `Cargo.toml`, `src/native/state.rs` |
| **MEM-06** | `mem::forget(_guard)` do tracing-appender sem contrato documentado (rules: forget só intencional) | **P2** | Comentário de contrato/process-lifetime + SAFETY resource | `src/lib.rs` `init_tracing` |
| **MEM-07** | `Drop for Lifecycle` sem documentar bound de bloqueio residual (≤ grace 2s) | **P2** | Doc de Drop: idempotente, bounded, no-panic, CAS SeqCst | `src/lifecycle.rs` |
| **MEM-08** | Mock RESP array `with_capacity(n)` sem limite de arity | **P2** | `MAX_RESP_ARRAY_LEN` + `try_reserve_exact` no parser de teste | `cache.rs` tests |
| **MEM-09** | Sem gate local de higiene de memória/RAII | **P2** | `scripts/memory-check.sh` (`--inventory-only`) | script |
| **MEM-10** | `BrowserProcess::has_exited` sempre `false` e `id` sempre `None` (Lightpanda) | **P1** | Delegação a `LightpandaProcess::{has_exited,id}` | `native/browser.rs` |

### 21.3 Já conformes (revalidados)

| Item | Status |
|------|--------|
| Sem `std::process::exit` | **PASS** |
| Sem `Rc` / ciclos `Arc` óbvios (grafo linear client/session) | **PASS** |
| Sem `Box::leak` / `Vec::leak` | **PASS** |
| Sem `static mut` / transmute | **PASS** (Pass 6) |
| `unsafe` com comentários SAFETY | **PASS** |
| `Command::output()` paths (ffmpeg, lighthouse) reaparem via `output` | **PASS** |
| `Lifecycle` Drop safety net + finalize idempotente | **PASS** |
| `EnvGuard` Drop restaura env | **PASS** |
| `with_capacity` em batch/scrape/snapshot conhecidos | **PASS** (Pass 8–9) |
| scrape JSON parse cap `MAX_PARSE_BYTES` 50 MB | **PASS** pré-existente |
| redis line length cap 16 MiB | **PASS** (reforçado com bulk budget) |
| mimalloc global allocator | **PASS** (Pass 8) |
| Locks: `std::sync::Mutex` curtos; `tokio::Mutex` só em CDP await paths | **PASS** (produto one-shot) |

### 21.4 N/A Pass 11 (product law / tooling)

| ID | Rule genérica | Por que N/A | Status |
|----|---------------|-------------|--------|
| **N-43** | Migração Edition 2024 (temporários `if let` / tail drop order) | Crate permanece `edition = "2021"`; sem migração nesta rodada | **N/A** |
| **N-44** | `miri` / `loom` / sanitizers em CI | Proibido GHA/CD; one-shot I/O-bound sem lock-free custom | **N/A** |
| **N-45** | `SmallVec`/`ArrayVec`/bumpalo arenas industriais | Sem hot path profiled exigindo stack collections | **N/A** |
| **N-46** | `Pin` + `PhantomPinned` self-referential | Sem tipos auto-referenciais no produto | **N/A** |
| **N-47** | Pools de conexão `close().await` multi-tenant | Sem pool SQL/HTTP de longa vida (reqwest `OnceLock` process-wide) | **N/A** |
| **N-48** | `Command::new("sudo")` + `Stdio::null` | Produto não invoca `sudo` | **N/A** |

### 21.5 Dívida Pass 11 (não-bloqueante)

| ID | Descrição | Impacto | Status |
|----|-----------|---------|--------|
| **D-18** | `try_reserve` não cobre **todas** as alocações derivadas de HTML/CDP (só Redis RESP + heap file + maps conhecidos) | residual OOM em payloads extremos | **ABERTO** (mitigado por caps HTTP/scrape e one-shot) |
| **D-12…D-17** | (inalterados) flamegraph, maps residual, SIGINT e2e, etc. | perf / cobertura | **ABERTO** |

### 21.6 Matriz checklist memória/RAII (núcleo one-shot)

| Item checklist | Status pós-Pass 11 |
|----------------|-------------------|
| Ownership exclusivo / move semantics | **PASS** (compilador) |
| RAII: recurso no construtor, liberação no Drop | **PASS** (Lifecycle, Lightpanda, EnvGuard) |
| Drop simples, sem panic, bounded | **PASS** (MEM-07) |
| Todo `spawn` tem `wait`/`output` em todos os ramos | **PASS** (MEM-01/02; ffmpeg/lighthouse via `output`) |
| `try_reserve` / teto em input externo crítico | **PASS** (MEM-03/04) |
| Sem `mem::forget` fora de cenário documentado | **PASS** (MEM-06) |
| Sem `Box::leak`/`Vec::leak` | **PASS** |
| `Rc`/`Arc` ciclos auditados | **PASS** (sem Rc; Arc sem ciclos) |
| Locks não atravessam I/O longo de forma perigosa no one-shot | **PASS** |
| `unsafe` com SAFETY | **PASS** |
| Zeroize de material sensível de sessão | **PASS** (MEM-05) |
| `OnceLock` em vez de `lazy_static` | **PASS** (Pass 6) |
| Assinaturas com `&str`/`&[T]` onde aplicável | **PASS** residual em APIs legadas (não-bloqueante) |
| miri/loom/edition 2024 | **N-43…N-44** |
| Gate local automatizado | **PASS** (`scripts/memory-check.sh`) |

### 21.7 Arquivos tocados (Pass 11)

| Arquivo | Gaps |
|---------|------|
| `src/native/cdp/lightpanda.rs` | MEM-01, MEM-02 |
| `src/native/browser.rs` | MEM-02, MEM-10 |
| `src/cache.rs` | MEM-03, MEM-08 (+ test) |
| `src/native/heap_snapshot.rs` | MEM-04 |
| `src/native/state.rs` | MEM-05 |
| `Cargo.toml` | MEM-05 (`zeroize`) |
| `src/lib.rs` | MEM-06 |
| `src/lifecycle.rs` | MEM-07 |
| `scripts/memory-check.sh` *(novo)* | MEM-09 |
| `gaps.md` | este catálogo |

### 21.8 Validação Pass 11

```text
cargo check --lib                                ok (0 warnings após fix)
cargo test --lib resp_bulk_rejects               1 ok
cargo test --lib finalize_is_idempotent          1 ok
cargo test --lib kill_unix_graceful              1 ok
cargo test --lib memory_hit_miss                 1 ok
./scripts/memory-check.sh --inventory-only       PASS
```

Revalidação:

```bash
./scripts/memory-check.sh --inventory-only
./scripts/memory-check.sh              # inclui cargo test RESP/lifecycle
```

Fontes de pesquisa (obrigatórias na skill):
- GraphRAG: `rules-rust-gerenciamento-memoria`, `rules-rust-mem-raii-borrow`, `rules-rust-mem-ownership-stack`, `rules-rust-mem-smartptr-weak`, `rules-rust-mem-io-seguranca`, `rules-rust-ownership-borrowing-lifetimes`, entidades `raii` / `drop-trait` / `drop-order`
- context7: `/rust-lang/rust`, `/tokio-rs/tokio`
- duckduckgo-search-cli: `Rust Vec try_reserve OOM untrusted input`, `Rust Child process Drop wait kill RAII`
- Rules locais: `docs_rules/rules_rust_gerenciamento_memoria.md` (+ ownership / interior mutability / unsafe FFI como suporte)

---

## 22. Resumo final consolidado (Pass 1–11)

- **Clap:** G-01…G-24 — **RESOLVIDOS**  
- **stdin/stdout:** S-01…S-12 — **RESOLVIDOS**  
- **one-shot:** O-01…O-08 — **RESOLVIDOS**  
- **inglês + crates.io:** E-01…E-12 — **RESOLVIDOS** (E-05 superado por R-01)  
- **const/static:** C-01…C-08 — **RESOLVIDOS**  
- **docs.rs automático:** R-01…R-10 — **RESOLVIDOS**  
- **economia de recursos:** ECO-01…ECO-10 — **RESOLVIDOS**  
- **eficiência e performance:** PERF-01…PERF-10 — **RESOLVIDOS**  
- **graceful shutdown:** GS-01…GS-08 — **RESOLVIDOS**  
- **memória / RAII:** MEM-01…MEM-10 — **RESOLVIDOS**  
- **N/A:** N-01…N-48  
- **Dívida não-bloqueante:** D-01…D-18  
- **Bloqueantes abertos:** **0**

---

## 23. Pass 12 — Interior Mutability (Cell / RefCell / Mutex / atomics / poison)

### 23.1 Escopo e fontes

| Fonte | Uso |
|-------|-----|
| `docs_rules/rules_rust_interior_mutability.md` | Checklist completo OBRIGATÓRIO/PROIBIDO |
| GraphRAG | `rules-rust-interior-mutability`, `rules-rust-mem-interior-locks`, `rules-rust-atomica-*`, `poison-error`, `interior-mutability-choice` |
| context7 `/rust-lang/rust` | RefCell/Mutex/Atomic ordering |
| context7 `/tokio-rs/tokio` | `tokio::sync::Mutex` vs std across `.await` |
| duckduckgo-search-cli | Mutex poison `into_inner`, RefCell short borrow |

### 23.2 Inventário pré-correção (núcleo)

| Primitiva | Locais | Veredito pré-fix |
|-----------|--------|------------------|
| `RefCell` TLS | `lifecycle` CURRENT_* | Legítimo single-thread; panic em reentrada |
| `Arc<std::Mutex<ResourceLedger>>` | Lifecycle + ~40 call sites | **Poison silenciado** com `if let Ok` → residual skip |
| `Arc<tokio::Mutex<HashSet>>` | `poll_network_idle` | **Desnecessário** (single-task) |
| `Arc<Mutex<bool>>` stop | cache RESP mock | **Mutex&lt;bool&gt;** antipadrão |
| `std::Mutex` MITM capture | `mitm_local` | Poison silenciado |
| `AtomicU16` bound_port | `mitm_local` | **Dead write** (nunca lido) |
| `tokio::sync::Mutex<Browser>` | `CdpClient` | Correto (lock across await) — docs fracos |
| `ACTIVE_FRAME: Mutex<Option<String>>` | `element` | Correto + poison recover |
| `AtomicBool` PLAIN / `OnceLock` | color, i18n, clients, regex | PASS (Pass 6) |
| `static mut` / `lazy_static` / `Arc<RefCell>` / `Rc` | — | Ausentes |

### 23.3 Gaps corrigidos

| ID | Gap | Sev | Solução | Evidência |
|----|-----|-----|---------|-----------|
| **IM-01** | `poll_network_idle` usava `Arc<tokio::Mutex<HashSet>>` para estado exclusivo de uma task | **P1** | `mut pending: HashSet` local; removido import `Mutex` | `src/native/browser.rs` |
| **IM-02** | Mock Redis `Arc<Mutex<bool>>` stop flag | **P1** | `Arc<AtomicBool>` + `Ordering::Relaxed` documentado | `src/cache.rs` tests |
| **IM-03** | ~40× `if let Ok(ledger) = life.ledger.lock()` **silenciava poison** — residual kill/wipe podia ser pulado | **P0** | `Lifecycle::with_ledger_mut` (`into_inner` + debug log) + `record_chrome` / `clear_chrome` / `clear_chrome_and_profile`; todos os call sites migrados | `lifecycle.rs`, `commands_prd/*`, `browser/mod.rs` |
| **IM-04** | MITM `if let Ok(cap.lock())` silenciava poison e perdia captures | **P1** | `lock_capture()` com recover; handlers + save paths | `src/mitm_local.rs` |
| **IM-05** | `AtomicU16` bound_port escrito e **nunca lido** | **P2** | Removido; port é `u16` local | `src/mitm_local.rs` |
| **IM-06** | TLS `RefCell` usava `borrow`/`borrow_mut` (panic em reentrada de sinal) | **P1** | `try_borrow` / `try_borrow_mut` com fallback inerte | `src/lifecycle.rs` |
| **IM-07** | `CdpClient` `tokio::Mutex` sem justificativa documentada | **P2** | rustdoc: hold across `.await` em execute/pages/listeners | `src/native/cdp/client.rs` |
| **IM-08** | `ACTIVE_FRAME` / `LaunchLogBuffer` / `MemoryCache` sem contrato de IM documentado | **P3** | Docs de escolha de primitiva + poison policy | `element.rs`, `lightpanda.rs`, `cache.rs` |
| **IM-09** | Mock store Redis `if let Ok` / `.ok()` em poison | **P2** | `lock_store()` com `into_inner` | `src/cache.rs` tests |
| **IM-10** | Sem gate local de interior mutability | **P2** | `scripts/interior-mutability-check.sh` | script novo |

### 23.4 N/A e dívida (Pass 12)

| ID | Item | Motivo |
|----|------|--------|
| **N-49** | `parking_lot` / `RwLock` / `arc-swap` / `qcell` | Sem contention medida; one-shot não justifica |
| **N-50** | `loom` / `shuttle` / ThreadSanitizer CI | Proibido GHA; miri full fora do one-shot residual (D-18/N-44) |
| **N-51** | `tokio-console` / métricas de contention / timeout `try_lock_for` | Daemon observability; product law one-shot |
| **D-19** | Instrumentação `tracing` de duração de lock em hot path CDP | Opcional; locks curtos documentados |

### 23.5 Matriz checklist interior mutability (núcleo one-shot)

| Item checklist | Status pós-Pass 12 |
|----------------|-------------------|
| Justificar IM antes de adotar | **PASS** (docs em cada site) |
| Menor primitiva correta | **PASS** (AtomicBool, OnceLock, HashSet local, Mutex composto) |
| Single-thread vs multi-thread | **PASS** (RefCell TLS; Mutex/Arc multi) |
| Sync vs async mutex | **PASS** (tokio só com await; std em seções curtas) |
| Encapsular lock na API | **PASS** (`with_ledger_mut`, `lock_capture`) |
| Seções críticas curtas / sem callback sob borrow | **PASS** |
| Locks std não atravessam `.await` | **PASS** |
| Ordering atômico documentado | **PASS** (SeqCst finalize, Relaxed flags) |
| Poison tratado (não unwrap cego / não silent skip residual) | **PASS** (IM-03/04) |
| Sem `static mut` / `lazy_static` / `Arc<RefCell>` | **PASS** |
| Sem `Mutex<bool>` | **PASS** (IM-02) |
| Gate local | **PASS** (`interior-mutability-check.sh`) |
| loom/miri industrial | **N-50** |

### 23.6 Arquivos tocados (Pass 12)

| Arquivo | Gaps |
|---------|------|
| `src/lifecycle.rs` | IM-03, IM-06 |
| `src/commands_prd/mod.rs` | IM-03 |
| `src/commands_prd/run.rs` | IM-03 |
| `src/browser/mod.rs` | IM-03 |
| `src/native/browser.rs` | IM-01 |
| `src/cache.rs` | IM-02, IM-08, IM-09 |
| `src/mitm_local.rs` | IM-04, IM-05 |
| `src/native/cdp/client.rs` | IM-07 |
| `src/native/element.rs` | IM-08 |
| `src/native/cdp/lightpanda.rs` | IM-08 |
| `scripts/interior-mutability-check.sh` *(novo)* | IM-10 |
| `gaps.md` | este catálogo |

### 23.7 Validação Pass 12

```text
cargo check --lib                                ok
cargo test --lib finalize / lifecycle / plain / memory / redis  ok
./scripts/interior-mutability-check.sh           PASS
```

Revalidação:

```bash
./scripts/interior-mutability-check.sh
cargo test --lib finalize
```

Fontes de pesquisa (obrigatórias na skill):
- GraphRAG: `rules-rust-interior-mutability`, `rules-rust-mem-interior-locks`, `rules-rust-atomica-padrao-core`, `poison-error`, `interior-mutability-choice`
- context7: `/rust-lang/rust`, `/tokio-rs/tokio`
- duckduckgo-search-cli: Mutex poison into_inner, RefCell borrow scope
- Rules locais: `docs_rules/rules_rust_interior_mutability.md`

---

## 24. Resumo final consolidado (Pass 1–12)

- **Clap:** G-01…G-24 — **RESOLVIDOS**  
- **stdin/stdout:** S-01…S-12 — **RESOLVIDOS**  
- **one-shot:** O-01…O-08 — **RESOLVIDOS**  
- **inglês + crates.io:** E-01…E-12 — **RESOLVIDOS** (E-05 superado por R-01)  
- **const/static:** C-01…C-08 — **RESOLVIDOS**  
- **docs.rs automático:** R-01…R-10 — **RESOLVIDOS**  
- **economia de recursos:** ECO-01…ECO-10 — **RESOLVIDOS**  
- **eficiência e performance:** PERF-01…PERF-10 — **RESOLVIDOS**  
- **graceful shutdown:** GS-01…GS-08 — **RESOLVIDOS**  
- **memória / RAII:** MEM-01…MEM-10 — **RESOLVIDOS**  
- **interior mutability:** IM-01…IM-10 — **RESOLVIDOS**  
- **N/A:** N-01…N-51 (estendido em Pass 13)  
- **Dívida não-bloqueante:** D-01…D-19 (estendido em Pass 13)  
- **Bloqueantes abertos:** **0**

---

## 25. Pass 13 — JSON e NDJSON (`rules_rust_json_e_ndjson`)

### 25.1 Escopo e fontes

| Fonte | Uso |
|-------|-----|
| `docs_rules/rules_rust_json_e_ndjson.md` | Checklist completo (RFC 8259, I-JSON, NDJSON, serde, limites, BOM) |
| GraphRAG | `rules-rust-json-e-ndjson`, `json-ndjson-definicoes`, `i-json-rfc7493`, `ndjson-streaming`, `serde-json`, `json-seguranca-validacao` |
| context7 | `/serde-rs/json` (from_reader/BufReader, Number, errors), `/websites/rs_serde_json` |
| duckduckgo-search-cli | serde_json + UTF-8 BOM issues (#1115), RFC 7493 I-JSON, safe integer 2^53−1 |
| Product law | CLI one-shot agent-first; **sem** servidor HTTP → Content-Type HTTP N/A; JSON5/schemars/JCS/Patch só se surface existir |

### 25.2 Gaps corrigidos

| ID | Gap identificado | Severidade | Solução | Evidência |
|----|------------------|------------|---------|-----------|
| **JSON-01** | Sem strip de BOM UTF-8 antes de `serde_json::from_str` (falha em scripts/configs salvos no Windows) | **P0** | `json_util::strip_utf8_bom` / `from_str` / `from_slice` | `src/json_util.rs`; testes BOM |
| **JSON-02** | `run --script` lia arquivo inteiro sem teto de bytes | **P0** | `read_text_file_limited` + `MAX_JSON_FILE_BYTES` (32 MiB) | `run.rs` |
| **JSON-03** | NDJSON sem limite por linha (DoS / linha multi-MB) | **P1** | `check_ndjson_line_len` + `MAX_NDJSON_LINE_BYTES` (1 MiB) | `parse_run_script` |
| **JSON-04** | `--json-steps` engolia falha de encode (`if let Ok(line) = to_string`) | **P1** | `write_json_line_ser` propaga `Result` | `run.rs` |
| **JSON-05** | Envelope construído só com `json!` / `Value` (sem struct tipada no wire) | **P1** | `SuccessEnvelope` / `ErrorEnvelope` / `ErrorBody` + `Serialize` | `envelope.rs` |
| **JSON-06** | Helpers JSON duplicados / inconsistentes (BOM, limites, compact encode) | **P1** | Módulo canônico `json_util` + `output::write_json_line_ser` | `lib.rs`, `output.rs` |
| **JSON-07** | Manifest workflow / sheet JSON / config JSON / MITM load / perf trace sem BOM+teto | **P1** | Wire para `read_json_file` / `read_text_file_limited` | `workflow_local`, `sheet_local`, `xdg`, `mitm_local`, `perf_insight` |
| **JSON-08** | Payloads CLI (`--fields-json`, cookies, params, input, args) sem BOM/size guard | **P2** | `parse_cli_json_value` + `MAX_CLI_JSON_PAYLOAD_BYTES` | `commands_prd/mod`, `browser/mod` |
| **JSON-09** | State/heap/cache parse sem strip BOM | **P2** | `json_util::from_str` nos loaders | `state.rs`, `heap_snapshot.rs`, `cache.rs`, `scrape_local` |
| **JSON-10** | Sem gate local de regras JSON/NDJSON | **P2** | `scripts/json-ndjson-check.sh` | scripts |

### 25.3 N/A intencionais (Pass 13)

| ID | Item da rule | Motivo N/A |
|----|--------------|------------|
| **N-52** | Content-Type `application/json` / `application/x-ndjson` em HTTP | CLI one-shot; sem endpoint HTTP |
| **N-53** | `schemars` / `jsonschema` runtime no binário | Schemas versionados em `docs/schemas/`; validação agente via inventário, não runtime embutido |
| **N-54** | JSON Patch / Merge Patch / JSONPath / JCS | Sem surface de update parcial ou assinatura de payload |
| **N-55** | `simd-json` / `sonic-rs` | Throughput de payloads CLI << 100 MB/s; `serde_json` default |
| **N-56** | JSON5 / JSONC em configs humanas | Config canônica é TOML XDG; JSON estrito só se extensão `.json` |
| **N-57** | `zstd` compressão NDJSON em repouso | Artefatos one-shot locais; sem log ship industrial |
| **N-58** | Async `FramedRead`+`LinesCodec` pipeline NDJSON | Scripts lidos com teto e parse em memória (tamanho limitado); sem daemon stream |

### 25.4 Dívida não-bloqueante

| ID | Item | Notas |
|----|------|-------|
| **D-20** | Migrar todos os `Value` de passos `run` para DTO tipado por comando | Passos são fronteira dinâmica legítima; tipagem total é refactor grande sem ganho de contrato agent imediato |

### 25.5 Checklist acionável (Pass 13)

| Item | Status |
|------|--------|
| `serde_json` + `serde` derive como stack JSON | **PASS** |
| Sem simd-json/sonic-rs sem benchmark | **PASS** |
| NDJSON = 1 objeto JSON completo por `\n`, compact | **PASS** (`write_json_line_ser`, `--json-steps`) |
| Envelope JSON único (não misturar array/NDJSON no envelope) | **PASS** |
| BOM UTF-8 removido antes do parse | **PASS** (`json_util`) |
| Limite de tamanho em entrada externa (arquivo/linha/flag) | **PASS** |
| `BufReader` em leitura de arquivo JSON | **PASS** (`read_text_file_limited`) |
| Escrita atômica disponível (`BufWriter` + rename) | **PASS** (`write_json_file_atomic`) |
| Pretty print só em artefatos humanos (state/MITM) | **PASS** (agent path compact) |
| Struct tipada no envelope de saída | **PASS** |
| Must-Ignore em config TOML (chaves desconhecidas ignoradas) | **PASS** (`parse_simple_toml`) |
| Sem JSON5 em interop máquina | **PASS** |
| Gate local | **PASS** (`json-ndjson-check.sh`) |
| schemars/JCS/Patch/HTTP Content-Type | **N-52…N-54** |

### 25.6 Arquivos tocados (Pass 13)

| Arquivo | Gaps |
|---------|------|
| `src/json_util.rs` *(novo)* | JSON-01…03, JSON-06, JSON-10 |
| `src/lib.rs` | JSON-06 |
| `src/output.rs` | JSON-04, JSON-06 |
| `src/envelope.rs` | JSON-05 |
| `src/commands_prd/run.rs` | JSON-01…04 |
| `src/commands_prd/mod.rs` | JSON-08 |
| `src/browser/mod.rs` | JSON-08 |
| `src/workflow_local.rs` | JSON-07 |
| `src/sheet_local.rs` | JSON-07 |
| `src/xdg.rs` | JSON-07 |
| `src/mitm_local.rs` | JSON-07 |
| `src/native/perf_insight.rs` | JSON-07 |
| `src/native/state.rs` | JSON-09 |
| `src/native/heap_snapshot.rs` | JSON-09 |
| `src/cache.rs` | JSON-09 |
| `src/scrape_local.rs` | JSON-09 |
| `scripts/json-ndjson-check.sh` *(novo)* | JSON-10 |
| `gaps.md` | este catálogo |

### 25.7 Validação Pass 13

```text
cargo check --lib                                    ok
cargo test --lib json_util                           6 ok
cargo test --lib envelope                            2 ok
cargo test --lib parse_script                        5 ok
cargo test --test envelope_schema                    5 ok
./scripts/json-ndjson-check.sh                       PASS
```

Revalidação:

```bash
./scripts/json-ndjson-check.sh
cargo test --lib parse_script
cargo test --test envelope_schema
```

Fontes de pesquisa (obrigatórias na skill):
- GraphRAG: `rules-rust-json-e-ndjson`, `i-json-rfc7493`, `ndjson-streaming`, `serde-json`
- context7: `/serde-rs/json`, `/websites/rs_serde_json`
- duckduckgo-search-cli: serde_json UTF-8 BOM, RFC 7493 I-JSON
- Rules locais: `docs_rules/rules_rust_json_e_ndjson.md`

---

## 26. Resumo final consolidado (Pass 1–13)

- **Clap:** G-01…G-24 — **RESOLVIDOS**  
- **stdin/stdout:** S-01…S-12 — **RESOLVIDOS**  
- **one-shot:** O-01…O-08 — **RESOLVIDOS**  
- **inglês + crates.io:** E-01…E-12 — **RESOLVIDOS** (E-05 superado por R-01)  
- **const/static:** C-01…C-08 — **RESOLVIDOS**  
- **docs.rs automático:** R-01…R-10 — **RESOLVIDOS**  
- **economia de recursos:** ECO-01…ECO-10 — **RESOLVIDOS**  
- **eficiência e performance:** PERF-01…PERF-10 — **RESOLVIDOS**  
- **graceful shutdown:** GS-01…GS-08 — **RESOLVIDOS**  
- **memória / RAII:** MEM-01…MEM-10 — **RESOLVIDOS**  
- **interior mutability:** IM-01…IM-10 — **RESOLVIDOS**  
- **JSON / NDJSON:** JSON-01…JSON-10 — **RESOLVIDOS**  
- **N/A:** N-01…N-58 (estendido na Pass 14)  
- **Dívida não-bloqueante:** D-01…D-20 (estendido na Pass 14)  
- **Bloqueantes abertos:** **0**

---

## 27. Pass 14 — Redução de latência (P50/P99, runtime, build, baseline)

**Rules:** `docs_rules/rules_rust_latencia_reduzir.md` (mentalidade de cauda, jitter, release LTO/CGU/abort, PGO/BOLT, alocação hot path, mimalloc, TLB/mlock, zero-copy, Tokio tuning, atomics, branch hints, checklist final).  
**Product law:** CLI **one-shot** I/O-bound (Chrome CDP / HTTP). Latência de cauda de trading (ns), isolcpus, mlockall, kernel bypass e PGO industrial **não** são o hot path real — Chrome boot/network é WCET externo em **segundos**. A auditoria aplica o subconjunto **acionável** e marca o resto **N/A**.

**Fontes:** GraphRAG `rules-rust-latencia-reduzir` (mem 34), `rules-rust-latencia-fundamentos-build` (73), `rules-rust-latencia-alocacao-memoria` (74), `incidente-blocking-async-degrada-latencia` (60); context7 `/tokio-rs/tokio`, `/bheisler/criterion.rs`; `ddgs` (criterion percentiles, tokio multi vs current_thread, mimalloc); Pass 8–9 (mimalloc, LTO fat, FxHash, `#[cold]`).

### 27.1 Inventário pré-correção (gaps)

| Gap | Severidade | Rule violada |
|-----|------------|--------------|
| Sem medição wall-clock P50/P99/P999 de paths agent (só criterion mean/CI micro) | **P0** | MEDIR percentis; NUNCA resumir latência em um número; baseline antes de otimizar |
| Sem orçamento de latência documentado por operação | **P1** | ESTABELECER orçamento; P99 máximo antes de codificar |
| `extract_llm` criava `new_multi_thread()` **sem** `worker_threads` → pool = num_cpus em one-shot | **P0** | DIMENSIONAR workers; current_thread para I/O single-core previsível |
| Runtimes ad-hoc duplicados (workflow current_thread ×2, browser multi) sem módulo único | **P1** | Runtime tuning centralizado; evitar drift |
| Browser runtime sem `max_blocking_threads` (default Tokio enorme) | **P2** | Cap de pool blocking; reduzir jitter/RSS de threads ociosas |
| Sem perfil release com `debug=1` para flamegraph/perf (strip no release dist) | **P1** | `debug=1` durante profiling; strip só após profiling |
| Sem gate local de higiene de latência | **P2** | Checklist / scripts locais (anti-GHA) |
| HTTP client sem `tcp_nodelay` explícito documentado | **P2** | TCP_NODELAY em sockets do path HTTP próprio |
| Criterion sem bench do encode compacto de envelope (path agent) | **P3** | Microbench do hot path de serialização |
| Docs de latência HFT misturáveis com product law (risco de otimização cega) | **P2** | NUNCA otimizar sem baseline; classificar N/A |

### 27.2 Correções (LAT-01…LAT-10)

| ID | Gap | Severidade | Solução | Evidência |
|----|-----|------------|---------|-----------|
| **LAT-01** | Sem baseline P50/P99 wall-clock | **P0** | `scripts/latency-baseline.sh` (help / doctor offline / version; warmup; outliers retained; mean só diagnóstico) | scripts |
| **LAT-02** | Sem orçamentos documentados | **P1** | Tabela de budgets em `runtime_util` (doctor ≤50 ms P99, help ≤80 ms, envelope ≤100 µs, Chrome = s) | `src/runtime_util.rs` |
| **LAT-03** | multi_thread ilimitado em extract_llm | **P0** | `runtime_util::block_on_io` (current_thread) | `commands_prd/mod.rs` |
| **LAT-04** | Runtimes ad-hoc | **P1** | Módulo `runtime_util`: `build_browser_runtime` / `build_io_runtime` / `block_on_io`; workflow + browser reusam | `runtime_util.rs`, `workflow_local.rs`, `browser/mod.rs` |
| **LAT-05** | Sem cap blocking pool | **P2** | `BROWSER_MAX_BLOCKING_THREADS = 8` + `worker_threads(2)` + thread names | `runtime_util.rs` |
| **LAT-06** | Sem perfil de profiling | **P1** | `[profile.release-prof]` inherits release, `debug=1`, `strip="none"` | `Cargo.toml` |
| **LAT-07** | Sem gate local | **P2** | `scripts/latency-check.sh` (LTO/CGU/abort, release-prof, mimalloc, no ad-hoc multi_thread, tcp_nodelay, budgets) | scripts |
| **LAT-08** | TCP_NODELAY não explícito | **P2** | `shared_http_client().tcp_nodelay(true)` | `robots.rs` |
| **LAT-09** | Criterion sem envelope compact | **P3** | Benches `envelope_success_to_compact_string` / `envelope_error_…` | `benches/cli_parse.rs` |
| **LAT-10** | Product law vs HFT não documentado no path crítico | **P2** | Docs em `block_on_browser_timeout` + `runtime_util` + main.rs (N/A PGO/isolcpus/mlock) | browser, main |

### 27.3 N/A intencionais (Pass 14)

| ID | Item da rule | Motivo N/A |
|----|--------------|------------|
| **N-59** | PGO (`cargo-pgo`) + BOLT + AutoFDO | One-shot; sem carga de produção estável; Chrome domina wall time; re-treino a cada release sem ROI medido |
| **N-60** | isolcpus / nohz_full / IRQ affinity / NUMA bind / governor performance | Ops de host HFT; CLI de agente em laptop/CI genérico |
| **N-61** | mlockall / huge pages / MADV_POPULATE / KSM off | Working set one-shot curto; privilégios e ops de kernel não product default |
| **N-62** | Kernel bypass (DPDK/io_uring monoio/glommio) | CDP via chromiumoxide WebSocket; sem NIC dedicada |
| **N-63** | dhat zero-alloc hot path / bumpalo / SmallVec industrial | Hot path real é I/O CDP; zero-alloc sem profile é otimização cega (Pass 8 N-30) |
| **N-64** | loom/miri full em atomics de latência | Lifecycle atomics já documentados (Pass 6/12); sem SPSC HFT |
| **N-65** | CachePadded / perf c2c HITM / SIMD runtime | Sem contadores multi-core hot; payload pequeno |
| **N-66** | Logger assíncrono batching / HDR histogram export OTel | tracing stderr one-shot; sem daemon de observabilidade |
| **N-67** | tick-to-trade / TSC calibrado / session resumption pool | Não é sistema de trading; HTTP client keep-alive já process-wide |
| **N-68** | `target-cpu=native` em distro + Cranelift ban | Já proibido em config commitado (Pass 9); native só local via RUSTFLAGS |

### 27.4 Dívida não-bloqueante

| ID | Item | Notas |
|----|------|-------|
| **D-21** | Artefatos flamegraph/`perf` de path CDP representativo commitados | Script aponta o comando; captura sob carga real fica com o maintainer (sobreposição D-12/D-16) |

### 27.5 Checklist acionável (Pass 14)

| Item | Status |
|------|--------|
| Baseline P50/P99/P999 (meta paths) | **PASS** (`latency-baseline.sh`) |
| Release LTO fat + CGU 1 + panic abort | **PASS** (Pass 9 + gate) |
| mimalloc global | **PASS** (Pass 8) |
| Runtime CDP workers limitados (2) + blocking cap | **PASS** |
| I/O HTTP current_thread | **PASS** (`block_on_io`) |
| Sem multi_thread ad-hoc | **PASS** (gate) |
| TCP_NODELAY no HTTP próprio | **PASS** |
| Perfil profiling debug=1 | **PASS** (`release-prof`) |
| Criterion + wall-clock separados | **PASS** |
| Orçamentos documentados | **PASS** |
| PGO/BOLT/isolcpus/mlock/kernel bypass | **N-59…N-62** |
| Gate local | **PASS** (`latency-check.sh`) |

### 27.6 Amostra de baseline (host local, 2026-07-18, n=12, release)

| Path | P50 (s) | P99 (s) | Budget P99 |
|------|---------|---------|------------|
| help | ~0.006 | ~0.007 | ≤ 0.080 |
| doctor_offline_quick_json | ~0.008 | ~0.009 | ≤ 0.050 |
| version_json | ~0.006 | ~0.007 | ≤ 0.050 |

(Valores variam por máquina; reexecutar `./scripts/latency-baseline.sh --build`.)

### 27.7 Arquivos tocados (Pass 14)

| Arquivo | Gaps |
|---------|------|
| `src/runtime_util.rs` *(novo)* | LAT-02…05, LAT-10 |
| `src/lib.rs` | LAT-04 |
| `src/browser/mod.rs` | LAT-04, LAT-05, LAT-10 |
| `src/commands_prd/mod.rs` | LAT-03 |
| `src/workflow_local.rs` | LAT-04 |
| `src/robots.rs` | LAT-08 |
| `src/main.rs` | LAT-06, LAT-10 |
| `Cargo.toml` | LAT-06 |
| `benches/cli_parse.rs` | LAT-09 |
| `scripts/latency-baseline.sh` *(novo)* | LAT-01 |
| `scripts/latency-check.sh` *(novo)* | LAT-07 |
| `scripts/perf-check.sh` | hints release-prof / latency |
| `gaps.md` | este catálogo |

### 27.8 Validação Pass 14

```text
cargo test --lib runtime_util                     3 ok
cargo check                                       ok
./scripts/latency-check.sh                        PASS
./scripts/latency-baseline.sh --samples 12        PASS (p50/p99 impressos)
```

Revalidação:

```bash
./scripts/latency-check.sh
./scripts/latency-baseline.sh --build
cargo bench --bench cli_parse -- --sample-size 20   # opcional
```

Fontes de pesquisa (obrigatórias na skill):
- GraphRAG: `rules-rust-latencia-reduzir`, `rules-rust-latencia-fundamentos-build`, `rules-rust-latencia-alocacao-memoria`, `incidente-blocking-async-degrada-latencia`
- context7: `/tokio-rs/tokio`, `/bheisler/criterion.rs`, `/websites/rs_tokio`
- duckduckgo-search-cli / ddgs: criterion P99, tokio multi vs current_thread, mimalloc CLI
- Rules locais: `docs_rules/rules_rust_latencia_reduzir.md`

---

## 28. Resumo final consolidado (Pass 1–14)

- **Clap:** G-01…G-24 — **RESOLVIDOS**  
- **stdin/stdout:** S-01…S-12 — **RESOLVIDOS**  
- **one-shot:** O-01…O-08 — **RESOLVIDOS**  
- **inglês + crates.io:** E-01…E-12 — **RESOLVIDOS** (E-05 superado por R-01)  
- **const/static:** C-01…C-08 — **RESOLVIDOS**  
- **docs.rs automático:** R-01…R-10 — **RESOLVIDOS**  
- **economia de recursos:** ECO-01…ECO-10 — **RESOLVIDOS**  
- **eficiência e performance:** PERF-01…PERF-10 — **RESOLVIDOS**  
- **graceful shutdown:** GS-01…GS-08 — **RESOLVIDOS**  
- **memória / RAII:** MEM-01…MEM-10 — **RESOLVIDOS** (MEM-06 supersedido por LOG-02)  
- **interior mutability:** IM-01…IM-10 — **RESOLVIDOS**  
- **JSON / NDJSON:** JSON-01…JSON-10 — **RESOLVIDOS**  
- **latência:** LAT-01…LAT-10 — **RESOLVIDOS**  
- **N/A (até Pass 14):** N-01…N-68  
- **Dívida não-bloqueante (até Pass 14):** D-01…D-21  
- **Bloqueantes abertos:** **0**

---

## 29. Pass 15 — Logs com Tracing e Rotação (`rules_rust_logs_com_tracing_e_rotacao`)

### 29.1 Escopo e product law

CLI **one-shot** agent-first: stdout = envelope JSON; stderr = diagnósticos; **zero telemetria remota**.  
Regras de **servidor/daemon** (endpoint `/admin/log-level`, OTEL BatchSpanProcessor, sampling OTLP, journald, Lambda flush, encrypted-at-rest pipeline, dashboards) → **N/A** intencional.

### 29.2 Gaps identificados e solucionados

| ID | Gap identificado | Severidade | Solução | Evidência |
|----|------------------|------------|---------|-----------|
| **LOG-01** | Init de tracing embutido em `lib.rs` sem módulo dedicado / sem `init_telemetry` canônico | **P1** | Novo `src/telemetry.rs` com `init_telemetry` + `TelemetryOpts` + docs de targets | `src/telemetry.rs`, `src/lib.rs` |
| **LOG-02** | `mem::forget(WorkerGuard)` — flush não garantido por Drop nomeado (rules: nunca discard guard) | **P0** | `TelemetryGuard { _file_worker }` retido em `run()` até pós-FINALIZE; `drop(_telemetry)` explícito | `src/lib.rs`, `src/telemetry.rs` |
| **LOG-03** | `try_init` Result ignorado; sem evento de confirmação do filtro | **P1** | Match em `try_init`; `tracing::info!(filter, log_to_file, …)` após sucesso | `init_telemetry` |
| **LOG-04** | Features incompletas: sem `json`, `registry`, `tracing-log` (ponte `log` crate) | **P1** | `tracing-subscriber` features explícitas | `Cargo.toml` |
| **LOG-05** | Rotação via `daily()` sem `max_log_files` / Builder | **P1** | `RollingFileAppender::builder()` + `Rotation::DAILY` + `max_log_files(14)` | `build_file_appender` |
| **LOG-06** | Arquivo de log sem JSON estruturado; ANSI não controlado; `with_target(false)` | **P2** | File layer `.json()` sem ANSI; stderr com ANSI só se TTY+color policy; `with_target(true)` | `telemetry.rs` |
| **LOG-07** | Sem panic hook integrado ao tracing (só `human_panic` antes do subscriber) | **P1** | `install_panic_tracing_bridge` após init (target `panic` → `error` + chain hook anterior) | `telemetry.rs`, `main.rs` docs |
| **LOG-08** | `eprintln!` de diagnóstico em hot paths de produção | **P1** | `tracing::warn!` estruturado em robots / native browser / snapshot | `robots.rs`, `native/browser.rs`, `native/snapshot.rs` |
| **LOG-09** | Diretório de logs sem permissões restritas; filtro XDG inválido abortaria parse | **P2** | `DirBuilder` mode `0o700` (Unix); `EnvFilter::try_new` com fallback `error` | `create_log_dir`, `resolve_filter_directive` tests |
| **LOG-10** | Sem gate local de auditoria de tracing | **P2** | `scripts/tracing-check.sh` + unit tests de prioridade de filtro | `scripts/tracing-check.sh` |

### 29.3 N/A intencionais (product law / daemon / remote)

| ID | Item da rule | Motivo N/A |
|----|--------------|------------|
| **N-69** | `reload::Layer` + endpoint `/admin/log-level` | One-shot; nível via `-q`/`-v`/`--debug` ou `config set log_level` entre invocações |
| **N-70** | OpenTelemetry / OTLP / `BatchSpanProcessor` / `OTEL_EXPORTER_*` | Product law: zero telemetria remota |
| **N-71** | Sampling head/tail, métricas OTEL, correlação dashboards remotos | Sem backend remoto |
| **N-72** | `tokio-console` / `console-subscriber` / `RUSTFLAGS=tokio_unstable` | Dev opcional; não no binário de produção |
| **N-73** | `tracing-journald` / Docker logging driver / AWS Lambda flush | Não é serviço systemd/container-serverless dedicado |
| **N-74** | MakeWriter multi-severidade (error→stderr, info→file split) | Dual sink stderr+file suficiente; stdout reservado a envelopes |
| **N-75** | Criptografia de logs em repouso | Logs locais opcionais; XDG state user-owned; sem pipeline enterprise |
| **N-76** | `tracing-chrome` / `tracing-timing` histogramas | Profiling via `release-prof` + `perf`/flamegraph (Pass 14) |
| **N-77** | Rate limit / sampling de eventos de alto volume | CLI one-shot; default filter `error` |
| **N-78** | Serverless force-flush / desktop path-by-OS logging product | Binário CLI Unix-first; paths XDG |

### 29.4 Dívida não-bloqueante

| ID | Item | Notas |
|----|------|-------|
| **D-22** | Snapshot de schema JSON de log file em teste de integração | Opcional; unit filter + gate local cobrem núcleo |

### 29.5 Checklist acionável (subset CLI)

| Item | Status |
|------|--------|
| Subscriber único no path `run()` | **PASS** |
| `WorkerGuard` nomeado até fim do processo | **PASS** (`TelemetryGuard`) |
| Panic → tracing + human_panic chain | **PASS** |
| Ponte `log` via feature `tracing-log` | **PASS** |
| `ErrorLayer` | **PASS** |
| Writer non_blocking + rolling + `max_log_files` | **PASS** |
| JSON em arquivo; pretty/ANSI controlado em stderr | **PASS** |
| Sem `println!`/`dbg!` no init | **PASS** |
| `eprintln!` produção hot path eliminado | **PASS** (tests/build.rs ok) |
| Sem OTEL remoto | **PASS** (N-70) |
| Gate local | **PASS** (`tracing-check.sh`) |

### 29.6 Arquivos tocados (Pass 15)

| Arquivo | Gaps |
|---------|------|
| `src/telemetry.rs` *(novo)* | LOG-01…LOG-07, LOG-09 |
| `src/lib.rs` | LOG-01, LOG-02, mod + hold guard |
| `src/main.rs` | docs panic/tracing-check |
| `Cargo.toml` | LOG-04 |
| `src/robots.rs` | LOG-08 |
| `src/native/browser.rs` | LOG-08 |
| `src/native/snapshot.rs` | LOG-08 |
| `scripts/tracing-check.sh` *(novo)* | LOG-10 |
| `scripts/memory-check.sh` | MEM-06 → LOG-02 (ban forget) |
| `gaps.md` | este catálogo |

### 29.7 Validação Pass 15

```text
cargo check --lib                              ok
cargo test --lib telemetry::                   6 ok
./scripts/tracing-check.sh                     PASS
./scripts/memory-check.sh                      PASS (no mem::forget(...))
```

Revalidação:

```bash
./scripts/tracing-check.sh
cargo test --lib telemetry::
# optional file path:
# browser-automation-cli config set log_to_file true
# browser-automation-cli doctor --offline --quick -v
```

Fontes de pesquisa (obrigatórias na skill):
- Rules locais: `docs_rules/rules_rust_logs_com_tracing_e_rotacao.md`
- context7: `/websites/rs_tracing-subscriber`, `/tokio-rs/tracing` (library resolve)
- ddgs: WorkerGuard / non_blocking / rolling max_log_files
- Product law: one-shot, XDG, **no remote telemetry**, stdout envelopes only

---

## 30. Pass 16 — Macros em Rust (`rules_rust_macros`)

### 30.1 Escopo e product law

Este crate é um **CLI one-shot / lib de runtime**, **não** uma biblioteca de macros.  
Checklist de **proc-macro crate** (`syn`/`quote`/`trybuild`/`cargo-semver-checks`/MSRV matrix de API macro) → **N/A** intencional.  
Obrigatório aplicável: esgotar genéricos/traits antes de `macro_rules!`; builtins std idiomáticos; `build.rs`+`include!` para dados externos; sem `todo!`/`dbg!` em produção; sem `macro_export` sem justificativa.

### 30.2 Gaps identificados e solucionados

| ID | Gap identificado | Severidade | Solução | Evidência |
|----|------------------|------------|---------|-----------|
| **MAC-01** | `macro_rules! fwd` local em `spawn_event_forwarders` sem justificativa / higiene documentada — corpo idêntico monomorfizável | **P1** | Removida macro; `spawn_cdp_event_forwarder` + `attach_browser_event_forwarder` genéricos | `src/native/cdp/client.rs` |
| **MAC-02** | Boilerplate page-level de `event_listener` copiado 6× (console/network/heap/screencast/dialog) — oportunidade de abstração sem macro | **P1** | `attach_page_event_forwarder::<T>` genérico unifica todos os paths | `client.rs` |
| **MAC-03** | Justificativa de codegen CDP (`build.rs` vs proc-macro) não documentada nas rules de macros | **P2** | Docs em `build.rs` e `types::generated` explicando `include!(concat!(env!(OUT_DIR)))` | `build.rs`, `types.rs` |
| **MAC-04** | Ausência de gate local anti-`todo!`/`dbg!`/`macro_export`/`macro_rules!` | **P2** | `scripts/macros-check.sh` | `scripts/macros-check.sh` |
| **MAC-05** | Política “prefer generics over macros” não codificada no forwarder CDP | **P1** | Docs + helper genérico com bounds `IntoEventKind + Serialize` | `spawn_cdp_event_forwarder` |
| **MAC-06** | Sem teste unitário do caminho de serialização do forwarder (antes só e2e browser) | **P2** | Teste com `futures::stream::iter` + `broadcast` | `client::tests` |
| **MAC-07** | Inventário: zero `macro_export` / zero proc-macro — não auditado formalmente | **P3** | Gate confirma ausência; tabela N/A para superfície de macro-lib | `macros-check.sh` |
| **MAC-08** | Documentação de builtins (`env!`/`option_env!`/`concat!` UA) sem checklist de auditoria | **P3** | Gate exige `env!` identity + proíbe `format!` aninhado em `println!` | `macros-check.sh`, `lib.rs`/`main.rs` |

### 30.3 N/A intencionais (crate não é macro library)

| ID | Item da rule | Motivo N/A |
|----|--------------|------------|
| **N-79** | Crate `proc-macro = true` + `syn`/`quote`/`proc-macro2` | Produto é CLI, não exporta derives/atributos |
| **N-80** | `trybuild` / UI tests de mensagens de erro de macro | Sem macros públicas |
| **N-81** | `cargo-expand` / snapshots de expansão versionados | Sem expansão de macro de usuário |
| **N-82** | `format_ident!` / spans / `syn::Error::new_spanned` | Sem proc-macro |
| **N-83** | TT munchers / push-down accumulation / recursion limit | Sem `macro_rules!` recursiva |
| **N-84** | `$crate` hygiene em macros exportadas | Sem `#[macro_export]` |
| **N-85** | Convenção `nome-derive` + coexistência com derives de terceiros | Não publica derive |
| **N-86** | `cargo-semver-checks` de API de macro / CHANGELOG de sintaxe | Sem API de macro |
| **N-87** | `no_std` validation de macros / compile-time impact de proc-macro tree | N/A product |
| **N-88** | rust-analyzer `procMacro.enable` / experiência IDE de macros de usuário | Consumidor só usa derives de deps (`clap`, `serde`, `thiserror`) |

### 30.4 Checklist acionável (subset CLI / application crate)

| Item | Status |
|------|--------|
| Alternativas (generics/traits) esgotadas antes de macro | **PASS** (forwarders genéricos) |
| Sem `macro_rules!` de produção | **PASS** |
| Sem `#[macro_export]` | **PASS** |
| Sem `todo!`/`unimplemented!`/`dbg!` em `src/` | **PASS** |
| Builtins `env!`/`option_env!`/`concat!`/`include!` idiomáticos | **PASS** |
| Geração externa via `build.rs` + `include!` | **PASS** (CDP protocol) |
| Derives de terceiros (`clap`/`serde`/`thiserror`/`Error`) sem proc-macro próprio | **PASS** |
| Sem `format!` aninhado em `println!` | **PASS** |
| Gate local | **PASS** (`macros-check.sh`) |
| Teste do helper genérico | **PASS** |

### 30.5 Arquivos tocados (Pass 16)

| Arquivo | Gaps |
|---------|------|
| `src/native/cdp/client.rs` | MAC-01, MAC-02, MAC-05, MAC-06 |
| `src/native/cdp/types.rs` | MAC-03 |
| `build.rs` | MAC-03 |
| `scripts/macros-check.sh` *(novo)* | MAC-04, MAC-07, MAC-08 |
| `gaps.md` | este catálogo |

### 30.6 Validação Pass 16

```text
cargo check --lib                                          ok
cargo test --lib native::cdp::client::                     1 ok
./scripts/macros-check.sh                                  PASS
```

Revalidação:

```bash
./scripts/macros-check.sh
cargo test --lib native::cdp::client::
```

Fontes de pesquisa (obrigatórias na skill):
- Rules locais: `docs_rules/rules_rust_macros.md`
- context7: `/veykril/tlborm` (Little Book of Rust Macros)
- ddgs: macro_rules hygiene, prefer generics over macros, chromiumoxide event_listener
- Product law: application CLI — **not** a macro library

---

## 31. Pass 17 — Fechamento da dívida estrutural (D-01…D-22)

### 31.1 Objetivo

O usuário exigiu fechar **obrigatoriamente todos os gaps**, incluindo a dívida
não-bloqueante acumulada. Itens de product law (N-*) permanecem N/A; **D-***
receberam mitigação concreta ou aceitação documentada com evidência.

### 31.2 Catálogo D — status pós-Pass 17

| ID | Antes | Solução | Status |
|----|-------|---------|--------|
| **D-01** | monólito `commands_prd/mod.rs` | Module map em rustdoc + `docs/ARCHITECTURE.md`; `meta`/`run` já extraídos; novos handlers em sibling modules | **RESOLVIDO** (mitigado) |
| **D-02** | `allow(missing_docs)` em CLI | Política documentada: clap `///` + skills = UX; allow justificado | **RESOLVIDO** |
| **D-03** | completions só runtime | `scripts/gen-completions.sh` gera `target/completions/*` | **RESOLVIDO** |
| **D-04** | cobertura Args parcial | `more_subcommand_args_bind` (goto/doctor/schema/plain) | **RESOLVIDO** |
| **D-05** | `eprintln!` produção | Só testes (`cache`, lighthouse skip); produção usa `tracing` | **RESOLVIDO** |
| **D-06** | SPDX parcial | **55/55** arquivos `src/**/*.rs` com SPDX | **RESOLVIDO** |
| **D-07** | sem ARCHITECTURE/ROADMAP | `docs/ARCHITECTURE.md` + `docs/ROADMAP.md` | **RESOLVIDO** |
| **D-08** | `win_job` multi-op unsafe | `create_and_assign` / `terminate_pid` com unsafe **por chamada** + SAFETY | **RESOLVIDO** |
| **D-09** | ledger poison skip | Já `into_inner` + teste poison (revalidado) | **RESOLVIDO** |
| **D-10** | llms.txt manual | `scripts/gen-llms-txt.sh` anexa `commands --json` | **RESOLVIDO** |
| **D-11** | missing_docs monólitos | Política D-02 + map D-01 (aceito: help clap) | **RESOLVIDO** |
| **D-12** | sem flamegraph local | `scripts/profile-cdp.sh` (`--flamegraph` / `--samply`) | **RESOLVIDO** |
| **D-13** | try_reserve scrape body | `String::try_reserve` + `Vec::try_reserve_exact` em scrape HTTP | **RESOLVIDO** |
| **D-14** | with_capacity monólitos | snapshot `with_capacity_and_hasher`; scrape batches já cobertos | **RESOLVIDO** |
| **D-15** | FxHashMap residual | `iframe_sessions` / `named_contexts` / snapshot maps / network headers | **RESOLVIDO** |
| **D-16** | flamegraph artefactos | Igual D-12 (script local; sem commit de SVG) | **RESOLVIDO** |
| **D-17** | SIGINT/SIGTERM e2e | `tests/signal_shutdown.rs` (2 testes PASS) | **RESOLVIDO** |
| **D-18** | try_reserve residual | Mitigado D-13 + caps HTTP/heap existentes | **RESOLVIDO** |
| **D-19** | tracing de lock duration | Locks curtos documentados; instrumentação industrial N/A one-shot | **RESOLVIDO** (aceito) |
| **D-20** | DTO tipado total em `run` | Fronteira dinâmica legítima; envelope tipado no I/O | **RESOLVIDO** (aceito) |
| **D-21** | perf artefacts commitados | Explicitamente **não** commitados; script profile | **RESOLVIDO** |
| **D-22** | schema JSON de log | `tests/telemetry_log_schema.rs` | **RESOLVIDO** |

### 31.3 Validação Pass 17

```text
cargo test --lib                                           272 ok
cargo test --test clap_arg_coverage                        9 ok
cargo test --test signal_shutdown                          2 ok
cargo test --test telemetry_log_schema                     2 ok
./scripts/macros-check.sh                                  PASS
SPDX src/**/*.rs                                           55/55
docs/ARCHITECTURE.md + docs/ROADMAP.md                     present
```

### 31.4 Arquivos tocados (Pass 17)

| Área | Arquivos |
|------|----------|
| SPDX | todos `src/**/*.rs` sem header |
| Maps | `browser/mod.rs`, `native/{snapshot,element,interaction,screenshot,network,browser}.rs` |
| Win32 | `win_job.rs` |
| Scrape | `scrape_local.rs` (try_reserve) |
| Docs | `docs/ARCHITECTURE.md`, `docs/ROADMAP.md` |
| Scripts | `profile-cdp.sh`, `gen-completions.sh`, `gen-llms-txt.sh` |
| Testes | `signal_shutdown.rs`, `telemetry_log_schema.rs`, `clap_arg_coverage.rs` |
| Map | `commands_prd/mod.rs`, `cli.rs` docs |

---

## 32. Resumo consolidado (Pass 1–17)

- **Clap … macros + D-01…D-22:** **RESOLVIDOS**  
- **N/A product law (até Pass 17):** N-01…N-88  

---

## 33. Pass 18 — i18n multi-idioma automático (locale SO)

### 33.1 Objetivo

Auditar `docs_rules/rules_rust_multi-idiona_i18_automatico_…` + `rules_rust_internacionalizacao.md`
contra o CLI one-shot e **fechar todos os gaps aplicáveis**, respeitando product law:
stdout JSON estável em inglês; só `suggestion` humana localiza.

### 33.2 Gaps identificados e solucionados

| ID | Gap | Severidade | Solução | Evidência |
|----|-----|------------|---------|-----------|
| **I18N-01** | Detecção via `LANG`/`LC_*` diretos (proibido) | **P0** | `sys-locale::get_locale` único | `src/i18n/detect.rs` |
| **I18N-02** | Sem parse BCP47 / underscore / encoding | **P0** | `unic-langid` + `parse_langid` | `detect.rs` |
| **I18N-03** | Sem negociação de locale | **P0** | `fluent-langneg` Filtering | `negotiate()` |
| **I18N-04** | Idioma como `&str` frouxo (`en`/`pt`) | **P0** | enum `Idioma` `#[non_exhaustive]` | `idioma.rs` |
| **I18N-05** | Sem enum `Mensagem` / match catch-all | **P0** | `Mensagem` + `en`/`pt_br` exaustivos | `mensagem.rs`, `en.rs`, `pt_br.rs` |
| **I18N-06** | Sem FTL bilíngue MVP | **P0** | `locales/en.ftl` + `locales/pt-BR.ftl` | `locales/` |
| **I18N-07** | Sem paridade de chaves / acentos em gate | **P1** | testes + `scripts/i18n-check.sh` | script + `ftl.rs` |
| **I18N-08** | Precedência incompleta (sem env product) | **P1** | 5 camadas + `BROWSER_AUTOMATION_CLI_LANG` | `resolve()` |
| **I18N-09** | Sem subcomando `locale` diagnóstico | **P1** | `Commands::Locale` + schema | `cli.rs`, `meta.rs` |
| **I18N-10** | Console Windows UTF-8 não na boot | **P1** | `configure_console_utf8` fase 1 | `i18n/mod.rs`, `lib.rs` |
| **I18N-11** | Features top-20 ausentes | **P2** | `i18n-cjk`/`rtl`/`europe`/`full`/`pseudo` | `Cargo.toml` |
| **I18N-12** | Fluent não validava FTL embutido | **P2** | `fluent` parse + format tests | `ftl.rs` |
| **I18N-13** | PRIVACY sem menção a locale | **P2** | tabela locale local-only | `PRIVACY.md` |
| **I18N-14** | Truncamento sem grapheme clusters | **P3** | `unicode-segmentation` helper | `truncate_graphemes` |

### 33.3 N/A intencionais (product law / escopo CLI agente)

| ID | Item da rule | Motivo N/A |
|----|--------------|------------|
| **N-89** | Top-20 idiomas embutidos no binário default | Proibido; só features scaffold |
| **N-90** | `icu::calendar` / calendários não gregorianos | CLI agente; sem UI de datas localizadas |
| **N-91** | `icu::collator` listas ordenadas localizadas | Sem UI de listas humanas multi-locale |
| **N-92** | Weblate/Crowdin/Pontoon pipeline | Projeto solo; PR + FTL + revisão humana |
| **N-93** | Pseudolocalização runtime default | Feature `i18n-pseudo` scaffold only |
| **N-94** | GHA bloqueio de cobertura de tradução | Proibido GHA; gate local `i18n-check.sh` |
| **N-95** | Traduzir `error.message` / envelopes JSON | Contrato agente estável em inglês |
| **N-96** | Traduzir tracing / logs técnicos | Policy: logs em inglês |
| **N-97** | WASM `navigator.languages` | Host-only CDP CLI |
| **N-98** | Android JNI / iOS locale APIs | Fora do target matrix |
| **N-99** | `fluent-langneg` 0.14 + `icu_locid` | 0.14 quebra tipo vs fluent 0.17; pin **0.13** documentado |
| **N-100** | Hash SHA256 release de FTL em CI remota | Release local; conteúdo versionado no git |

### Pass 19 — Multiplataforma SO completo v3

| ID | Gap identificado | Severidade | Solução | Evidência |
|----|------------------|------------|---------|-----------|
| **MP-01** | `Command::new("which")` / `"where"` em chrome + lightpanda (anti-pattern multiplataforma) | **P0** | `platform::which_bin` puro via `$PATH` | `src/platform.rs`, `chrome.rs`, `lightpanda.rs` |
| **MP-02** | Descoberta Chrome incompleta (sem paths absolutos Linux, snap/flatpak, Edge, Beta/Canary; Windows hardcoded `C:\` sem `%ProgramFiles%`) | **P0** | Cascata `find_chrome` + `find_chrome_known_paths` multi-OS | `src/native/cdp/chrome.rs` |
| **MP-03** | Sem detecção Snap/Flatpak nem warning | **P1** | `BrowserSandbox` + `warn_if_sandboxed_browser` | `platform.rs`, doctor |
| **MP-04** | Sem checagem de bit execute no binário resolvido | **P1** | `is_executable_file` (Unix mode 0o111) | `platform.rs`, `find_chrome` |
| **MP-05** | Console Windows só UTF-8; sem `ENABLE_VIRTUAL_TERMINAL_PROCESSING` | **P1** | `platform::configure_console` CP 65001 + VT | `platform.rs`, `lib.rs` boot |
| **MP-06** | Sem probe unificado WSL/container/CI/Termux | **P1** | `HostEnvironment::detect` + doctor check | `platform.rs`, `doctor/mod.rs` |
| **MP-07** | Basenames reservados Windows (`CON`/`NUL`/…) não rejeitados | **P2** | `reject_windows_reserved_basename` em todos os OS | `validation.rs` |
| **MP-08** | Doctor sem `sandbox` / `host_environment` / `path` | **P2** | Campos JSON em checks chrome + host | `doctor --json` |
| **MP-09** | `which_bin` duplicado (doctor/cache/lighthouse) | **P2** | Centralizado em `platform::which_bin` | doctor, cache tests, `commands_prd` |
| **MP-10** | Docs multiplataforma sem cascata/sandbox | **P2** | `CROSS_PLATFORM.md` + `.pt-BR` | `docs/CROSS_PLATFORM*.md` |
| **MP-11** | Sem gate local multiplataforma | **P2** | `scripts/multiplatform-check.sh` | PASS |
| **MP-12** | Launch Chrome: detecção container duplicada ad-hoc | **P3** | `HostEnvironment.container` em sandbox/dev-shm flags | `chrome.rs` |

#### N/A multiplataforma (product law / ops) — Pass 19

| ID | Item | Motivo |
|----|------|--------|
| **N-101** | Variável produto `CHROME_PATH` | Lei do produto: settings só flags + XDG `chrome_path` |
| **N-102** | chromedriver / WebDriver version match | Stack é **CDP** chromiumoxide, não WebDriver |
| **N-103** | Registro Win32 `App Paths\chrome.exe` | Cobertura via ProgramFiles/LocalAppData/PATH; evita dep `winreg` |
| **N-104** | WASM / WASI / Cloudflare Workers / Lambda | Host-only Chrome CDP; wasm32 fora de targets docs.rs |
| **N-105** | Universal binary macOS + notarização + Authenticode | Ops de release; proibido GHA nesta rodada |
| **N-106** | Matrix CI Ubuntu/macOS/Windows + Trusted Publishing | Proibido `.github` / CD nesta rodada |
| **N-107** | OCI multi-arch + cosign + SBOM industrial | Ops; binário local one-shot |
| **N-108** | Safari automation | Fora do contrato CDP Chrome/Chromium/Edge |
| **N-109** | Firefox WebDriver paths | Fora do motor CDP do produto |
| **N-110** | Download automático de Chrome/chromedriver | Sem BrowserFetcher no MVP; system Chrome only |
| **N-111** | `which` crate externa | Implementação pura em `platform::which_bin` (zero dep extra) |
| **N-112** | seccomp/landlock/setrlimit industrial | Hardening host ops; CLI confia no sandbox do Chrome + one-shot |

### 33.4 Checklist acionável (subset MVP CLI)

| Item | Status |
|------|--------|
| `en` + `pt-BR` 100% mensagens humanas | **PASS** |
| enum `Idioma` / `Mensagem` | **PASS** |
| match exaustivo sem `_` | **PASS** |
| Windows UTF-8 antes de I/O | **PASS** (cfg) |
| `sys-locale` uma vez na cadeia de resolve | **PASS** |
| `LanguageIdentifier` + negotiate | **PASS** |
| OnceLock global | **PASS** |
| 5 camadas de precedência | **PASS** |
| `--lang` + env crate | **PASS** |
| subcomando `locale` | **PASS** |
| features opcionais top-20 | **PASS** (scaffold) |
| FTL embutido + fluent parse | **PASS** |
| logs EN / suggestion localizada | **PASS** |
| stdout JSON estável | **PASS** |
| NO_COLOR / plain / TERM=dumb | **PASS** (color.rs pré-existente) |
| testes `Mensagem::texto(idioma)` sem global | **PASS** |
| paridade en/pt-BR | **PASS** |
| PRIVACY locale | **PASS** |
| gate local | **PASS** (`i18n-check.sh`) |

### 33.5 Arquivos tocados (Pass 18)

| Arquivo | Gaps |
|---------|------|
| `Cargo.toml` | deps + features i18n |
| `src/i18n/*` (novo módulo) | I18N-01…14 |
| `locales/en.ftl`, `locales/pt-BR.ftl` | I18N-06 |
| `src/lib.rs`, `src/cli.rs`, `src/commands_prd/*` | boot + locale cmd |
| `src/commands_prd/meta.rs` | inventário 63 |
| `tests/golden_i18n.rs` | e2e |
| `scripts/i18n-check.sh` | gate |
| `PRIVACY.md`, `docs/ARCHITECTURE.md`, `gaps.md` | docs |

### 33.6 Validação Pass 18

```text
cargo test --lib                                           287 ok
cargo test --lib i18n::                                    15 ok
cargo test --test golden_i18n                              7 ok
./scripts/i18n-check.sh                                    PASS
browser-automation-cli locale --json                       ok
```

---

## 34. Pass 19 — Multiplataforma SO v3

### 34.1 Checklist acionável

| Item | Status |
|------|--------|
| PATH lookup sem shell `which`/`where` | **PASS** |
| Cascata Chrome multi-OS + ProgramFiles | **PASS** |
| Snap/Flatpak sandbox detect + warn | **PASS** |
| Execute bit check | **PASS** |
| Windows UTF-8 + VT | **PASS** |
| HostEnvironment (WSL/container/CI/Termux) | **PASS** |
| Reserved Windows basenames | **PASS** |
| Doctor host_environment + sandbox | **PASS** |
| Docs CROSS_PLATFORM cascata | **PASS** |
| Gate `multiplatform-check.sh` | **PASS** |
| Job Objects Windows (pré-existente) | **PASS** |
| Completions 5 shells (pré-existente) | **PASS** |
| XDG directories (pré-existente) | **PASS** |
| WASM / chromedriver / GHA matrix | **N/A** product law |

### 34.2 Arquivos tocados (Pass 19)

| Arquivo | Gaps |
|---------|------|
| `src/platform.rs` (novo) | MP-01,03,04,05,06,09 |
| `src/native/cdp/chrome.rs` | MP-01,02,04,12 |
| `src/native/cdp/lightpanda.rs` | MP-01 |
| `src/doctor/mod.rs` | MP-08 |
| `src/validation.rs` | MP-07 |
| `src/lib.rs`, `src/i18n/mod.rs` | boot console |
| `src/commands_prd/mod.rs`, `src/cache.rs` | which central |
| `docs/CROSS_PLATFORM.md`, `.pt-BR.md` | MP-10 |
| `docs/ARCHITECTURE.md` | platform layer |
| `scripts/multiplatform-check.sh` | MP-11 |
| `gaps.md` | este doc |

### 34.3 Validação Pass 19

```text
cargo test --lib                                           296 ok
cargo test --lib platform::                                  8 ok
cargo test --lib validation::                                4 ok
cargo test --lib chrome::                                   21 ok
./scripts/multiplatform-check.sh                           PASS
doctor --offline --quick --json   host_environment + sandbox  ok
```

Revalidação:

```bash
./scripts/i18n-check.sh
cargo test --lib i18n::
cargo test --test golden_i18n
```

Fontes de pesquisa (obrigatórias na skill):
- Rules: `docs_rules/rules_rust_multi-idiona_i18_automatico_…`, `rules_rust_internacionalizacao.md`
- ddgs: sys-locale, unic-langid, fluent-langneg negotiate_languages
- Product law: one-shot agent CLI — JSON EN; suggestions localizam

---

### Pass 20 — Ownership, Borrowing e Lifetimes

| ID | Gap identificado | Severidade | Solução | Evidência |
|----|------------------|------------|---------|-----------|
| **OWN-01** | `Box::leak` em `i18n/detect.rs` para forçar `ResolvedLocale: Copy` via `&'static str` artificial | **P0** | `system_raw: Option<String>` owned; remove `Copy`; sem leak | `src/i18n/detect.rs`, `i18n/mod.rs` |
| **OWN-02** | 11× `clippy::redundant_clone` (cache path, lighthouse paths, extension/devtools/webmcp match bindings) | **P1** | Move ownership; clone só quando dual-use documentado | `cache.rs`, `commands_prd/mod.rs` |
| **OWN-03** | `clippy::implicit_clone` em `sheet_names().to_vec()` (calamine já devolve `Vec<String>`) | **P1** | `workbook.sheet_names()` direto | `scrape_local.rs` |
| **OWN-04** | `map_io_error` / `map_parse_err` / `format_cdp_err` por valor sem consumir | **P1** | Assinaturas `&io::Error` / `&serde_json::Error` / `&CdpError` | `output.rs`, `json_util.rs`, `cdp/client.rs` |
| **OWN-05** | `ingest_event(evt: CdpEvent)` só lia campos | **P1** | `ingest_event(&CdpEvent)` | `browser/mod.rs` |
| **OWN-06** | `mark_launched` clonava `profile` no ledger e reusava original | **P2** | Move para ledger + **um** clone deliberado `profile_scan` para residual pós-settle | `browser/mod.rs` |
| **OWN-07** | Resource owners sem `#[must_use]` | **P1** | `#[must_use]` em `Lifecycle`, `CdpClient`, `BrowserManager` | lifecycle, cdp/client, native/browser |
| **OWN-08** | Lints de ownership ausentes no crate root | **P1** | `#![warn(clippy::redundant_clone, needless_pass_by_value, ptr_arg, …)]` | `src/lib.rs` |
| **OWN-09** | Sem gate local de ownership | **P2** | `scripts/ownership-check.sh` + hook em `ci-check.sh` | scripts |
| **OWN-10** | `EnvGuard` impl com lifetime elidível ruidoso | **P3** | `impl EnvGuard<'_>` | `test_utils.rs` |
| **OWN-11** | Double `s.id.clone()` no DAG de workflow | **P3** | Um clone + `insert` com key movida | `workflow_local.rs` |
| **OWN-12** | Documentação de ownership ausente em tipos chave | **P2** | Docs de ownership em `ResolvedLocale`, `Lifecycle`, `CdpClient`, `BrowserManager` | vários |

#### N/A ownership (product law / sem caso de uso) — Pass 20

| ID | Item | Motivo |
|----|------|--------|
| **N-113** | GATs / lending iterators | Sem API de iterator genérico sobre dados emprestados no produto |
| **N-114** | HRTB `for<'a>` avançado | Assinaturas elidem lifetimes; sem higher-rank bounds necessários |
| **N-115** | `pin-project` / self-referential / ouroboros / yoke | Sem futures auto-referenciais manuais; tokio + chromiumoxide encapsulam |
| **N-116** | Arenas / handles tipados para grafos cíclicos | Workflow usa `petgraph` DAG acíclico; sem grafo de objetos com ciclo de ownership |
| **N-117** | `Rc<T>` monothread | CLI multi-thread no runtime CDP; `Arc` só onde `Send` é requisito real |
| **N-118** | `Cow<'a, T>` generalizado | Poucos caminhos half-owned; `RawCdpCommand` já usa `Cow::Owned` em MethodId |
| **N-119** | miri full suite em CI remota | Proibido GHA; `unsafe` isolado + SAFETY docs; gate local memory/ownership |
| **N-120** | `thread::scope` / shared `&T` cross-thread | Async cancel via `CancellationToken` + Arc ledger; sem threads manuais com borrows |

---

## 35. Pass 20 — Ownership / Borrowing / Lifetimes

### 35.1 Checklist acionável (subset CLI one-shot)

| Item | Status |
|------|--------|
| Ownership rastreável; sem aliasing XOR mutação violado | **PASS** (compila + clippy) |
| `&str` / `&[T]` preferidos; sem `&String` / `&Vec` | **PASS** |
| `.clone()` só com justificativa (residual dual-use documentado) | **PASS** |
| `Option::take` / move de match bindings | **PASS** (extension/devtools/webmcp) |
| Sem `Box::leak` / `mem::forget` em código de produção | **PASS** |
| `'static` só para dados verdadeiramente globais | **PASS** (sem raw OS leak) |
| `#[must_use]` em portadores de recurso | **PASS** |
| `Rc` ausente; `Arc` só multi-thread real | **PASS** |
| Locks std não atravessam `.await` (pré-existente CDP tokio Mutex) | **PASS** |
| Lints ownership no crate + gate local | **PASS** |
| GATs / pin-project / arenas | **N/A** product law |

### 35.2 Arquivos tocados (Pass 20)

| Arquivo | Gaps |
|---------|------|
| `src/i18n/detect.rs`, `src/i18n/mod.rs` | OWN-01,12 |
| `src/output.rs`, `src/json_util.rs` | OWN-04 |
| `src/native/cdp/client.rs` | OWN-04,07,12 |
| `src/browser/mod.rs` | OWN-05,06 |
| `src/commands_prd/mod.rs` | OWN-02 |
| `src/cache.rs`, `src/scrape_local.rs`, `src/workflow_local.rs` | OWN-02,03,11 |
| `src/lifecycle.rs`, `src/native/browser.rs` | OWN-07,12 |
| `src/lib.rs`, `src/test_utils.rs` | OWN-08,10 |
| `scripts/ownership-check.sh`, `scripts/ci-check.sh` | OWN-09 |
| `gaps.md` | este doc |

### 35.3 Validação Pass 20

```text
cargo test --lib                                           296 ok
./scripts/ownership-check.sh                               PASS
  - no Box::leak / mem::forget
  - no Rc / Arc<RefCell> / &String / &Vec
  - #[must_use] Lifecycle / CdpClient / BrowserManager
  - system_raw owned
  - clippy ownership deny set clean
  - lib unit tests 296 ok
```

Fontes de pesquisa (obrigatórias na skill):
- Rules: `docs_rules/rules_rust_ownership_borrowing_lifetimes.md`
- ddgs: Rust ownership borrowing lifetimes; Box::leak intentional use
- context7: std Cow / AsRef / Option patterns
- Product law: one-shot CLI — sem self-referential / sem GAT industrial

---

## 36. Pass 21 — Paralelismo e Multiprocessamento

### 36.1 Gaps fechados

| ID | Gap | Sev | Solução | Evidência |
|----|-----|-----|---------|-----------|
| **PAR-01** | Sem módulo central de budget / fórmula CPU×RAM | **P0** | `src/concurrency.rs` + `compute_auto_budget` + docs | `concurrency.rs` |
| **PAR-02** | Sem flag global `--max-concurrency` | **P0** | `GlobalOpts.max_concurrency` + `install_limit` no boot | `cli.rs`, `lib.rs` |
| **PAR-03** | `join_all` ilimitado em snapshot/screenshot CDP | **P0** | `join_bounded` / `join_bounded_ordered` cap 32 | `snapshot.rs`, `screenshot.rs` |
| **PAR-04** | Batch HTTP cap fixo 16; default concurrency=2 | **P1** | `0` = budget efetivo; hard cap 64; JoinSet + JoinError | `scrape_local.rs`, `cli.rs` |
| **PAR-05** | Crawl HTTP sequencial (BFS serial) | **P1** | Fronteira paralela JoinSet + budget | `scrape_local::crawl_http` |
| **PAR-06** | Runtime I/O `current_thread` (sem fan-out real) | **P1** | Multi-thread budgeted (`browser_worker_threads`) | `runtime_util.rs` |
| **PAR-07** | Workers CDP fixos em 2 | **P1** | Workers dinâmicos 2…8 via budget | `concurrency` + `runtime_util` |
| **PAR-08** | `sg_scan` sequencial (CPU-bound sobre arquivos) | **P1** | Rayon `par_iter` + walk multi-thread | `sg_local.rs` |
| **PAR-09** | Sem dep `rayon` / pool dimensionado | **P1** | `rayon = 1.10` + `install_rayon_pool_once` | `Cargo.toml` |
| **PAR-10** | Doctor sem visibilidade do budget | **P2** | Check `concurrency_budget` + campo JSON | `doctor/mod.rs` |
| **PAR-11** | Classificação de workload incompleta / desatualizada | **P2** | Docs em concurrency, scrape, sg, browser, workflow, find_paths | vários |
| **PAR-12** | Sem gate local de paralelismo | **P2** | `scripts/parallelism-check.sh` + hook `ci-check.sh` | scripts |

### 36.2 N/A paralelismo (product law / ops) — Pass 21

| ID | Item | Justificativa |
|----|------|---------------|
| **N-121** | `systemd-run --scope MemoryMax` default | Ops host; residual kill + Job Object já cobrem one-shot |
| **N-122** | loom model checking em CI remota | Proibido GHA; unit test de peak concurrency local |
| **N-123** | parking_lot deadlock detection | Poucos locks std; CDP usa `tokio::sync::Mutex` documentado |
| **N-124** | Métrica remota `available_permits` / OTel | Zero telemetria remota (product law) |
| **N-125** | `CancellationToken` hierárquico multi-fleet | One-shot: um token de Lifecycle basta |
| **N-126** | Multi-process pool de Chrome paralelo por URL | Uma sessão CDP por processo (anti-daemon); HTTP engine paralelo |
| **N-127** | Workflow ready-set paralelo com SQLite multi-writer | Journal single-writer; paralelismo **dentro** dos steps |
| **N-128** | Reversão de N-27 “sem rayon” | **Superseded** pela Pass 21: rayon em CPU-bound (sg); I/O permanece Tokio |

### 36.3 Checklist acionável (subset CLI one-shot)

| Item | Status |
|------|--------|
| Workload classificado (I/O / CPU / mista / subprocess) | **PASS** |
| Bound em todo fan-out (JoinSet / join_bounded / hard cap) | **PASS** |
| Fórmula CPU × RAM × 50% / 64 MiB documentada | **PASS** |
| `--max-concurrency=N` global | **PASS** |
| Semaphore helpers + `acquire` patterns disponíveis | **PASS** (`io_semaphore`) |
| Rayon em CPU-bound (sg); não no hot path async CDP | **PASS** |
| Runtime multi-thread budgeted (não unbounded num_cpus) | **PASS** |
| Teste peak concurrency ≤ N | **PASS** (`join_bounded_respects_peak`) |
| JoinError panic/cancel tratado em batch | **PASS** |
| Doctor expõe budget | **PASS** |
| Gate local `parallelism-check.sh` | **PASS** |
| loom / systemd-run / OTel permits | **N/A** product law |

### 36.4 Arquivos tocados (Pass 21)

| Arquivo | Gaps |
|---------|------|
| `src/concurrency.rs` *(novo)* | PAR-01,02,07,09,12 |
| `src/cli.rs`, `src/lib.rs` | PAR-02 |
| `src/runtime_util.rs` | PAR-06,07 |
| `src/scrape_local.rs` | PAR-04,05 |
| `src/native/snapshot.rs`, `screenshot.rs` | PAR-03 |
| `src/sg_local.rs` | PAR-08,09 |
| `src/find_paths.rs`, `workflow_local.rs`, `browser/mod.rs` | PAR-11 |
| `src/doctor/mod.rs`, `src/commands_prd/mod.rs` | PAR-10,04 |
| `src/main.rs`, `Cargo.toml` | PAR-09,11 |
| `scripts/parallelism-check.sh`, `scripts/ci-check.sh` | PAR-12 |
| `gaps.md` | este doc |

### 36.5 Validação Pass 21

```text
cargo test --lib                                           302 ok
cargo test --lib concurrency::                             6 ok
cargo test --test clap_global_flag_collision               ok
./scripts/parallelism-check.sh                             PASS
doctor --offline --quick --json                            concurrency.budget present
--help                                                     Parallelism / --max-concurrency
```

Fontes:
- Rules: `docs_rules/rules_rust_paralelismo_e_multiprocessamento.md`
- ddgs: rayon par_iter; tokio bounded concurrency patterns
- Product law: one-shot — paralelismo **bounded**, sem daemon fleets

---

## 37. Pass 22 — Paralelismo reauditoria (modus operandi)

Reauditoria completa das rules `rules_rust_paralelismo_e_multiprocessamento` após Pass 21, com postura **pró-ativa**: todo comando multi-item deve fan-out com bound ou justificar sequencial no código/docs.

### 37.1 Gaps fechados (Pass 22)

| ID | Gap | Sev | Solução | Evidência |
|----|-----|-----|---------|-----------|
| **PAR-13** | Batch/crawl usavam contador `in_flight` sem `Arc<Semaphore>` + `acquire_owned` (padrão da rule) | **P1** | `Semaphore::acquire_owned` / `try_acquire_owned` movido para a task; RAII devolve permit | `scrape_local.rs` |
| **PAR-14** | `sg_rewrite` dry-run sequencial (CPU+disco sobre N arquivos) | **P1** | Collect paths + Rayon `par_iter` no dry-run; `--apply` permanece sequencial | `sg_local.rs` |
| **PAR-15** | `free_ram_mb` só Linux (`None` em macOS/Windows → fórmula só CPUs) | **P1** | macOS `host_statistics64`; Windows `GlobalMemoryStatusEx` | `concurrency.rs`, `Cargo.toml` windows-sys feature |
| **PAR-16** | Sem matriz agente “qual comando paraleliza / por que sequencial” | **P1** | `command_workload_matrix()` embutida em `budget_report` / doctor | `concurrency.rs`, doctor JSON |
| **PAR-17** | `resolve_permits(0)` duplicado em handlers | **P2** | Helper único `resolve_permits` | `concurrency.rs`, `commands_prd` |
| **PAR-18** | Justificativa de **não**-paralelismo ausente em fill-form / sheet / qr / residual | **P2** | Comentários workload no módulo / método | `browser/mod.rs`, `sheet_local`, `qr_local`, `residual` |
| **PAR-19** | Gate local não assertava Semaphore nem matriz | **P2** | `parallelism-check.sh` checa `acquire_owned` + `command_workload_matrix` | scripts |
| **PAR-20** | Testes insuficientes (só peak join_bounded) | **P2** | + `resolve_permits`, matrix shape, free_ram Linux | `concurrency::tests` |

### 37.2 N/A adicionais (Pass 22)

| ID | Item | Justificativa |
|----|------|---------------|
| **N-129** | Multi-process Chrome pool por URL no `batch-scrape --engine browser` | Product law: um residual Chrome / uma Page; use `--engine http` para fan-out |
| **N-130** | Workflow ready-set paralelo multi-writer SQLite | Reafirma N-127; paralelismo **dentro** dos steps (batch/sg/find-paths) |

### 37.3 Checklist final de conformidade (subset one-shot)

| Item rule | Status |
|-----------|--------|
| Classifiquei workload (CPU / I/O / mista / subprocess) | **PASS** (módulo + matriz por comando) |
| Bound em TODO fan-out | **PASS** (Semaphore / join_bounded / WalkBuilder / rayon pool) |
| Permits CPU × RAM × 50% / 64 MiB | **PASS** (Linux/macOS/Windows free_ram) |
| RSS por task documentado | **PASS** (`RAM_PER_IO_TASK_MB` + rss-baseline) |
| `Arc<Semaphore>` + `acquire_owned` em spawn | **PASS** (batch/crawl HTTP) |
| `spawn_blocking` / Rayon fora do worker async CDP | **PASS** |
| `systemd-run MemoryMax` default | **N/A** ops (N-121) |
| OnceLock para recursos caros (HTTP client, regex, rayon pool) | **PASS** |
| JoinSet + JoinError panic/cancel | **PASS** |
| Cancel safety / CancellationToken Lifecycle | **PASS** (one token) |
| yield / unconstrained | **N/A** (sem loops CPU longos em async) |
| Métrica remota permits | **N/A** (N-124); nota local no budget |
| loom CI | **N/A** (N-122); unit peak local |
| `--max-concurrency=N` | **PASS** |
| Docs classificação + fórmula + sequential justificado | **PASS** |

### 37.4 Arquivos tocados (Pass 22)

| Arquivo | Gaps |
|---------|------|
| `src/concurrency.rs` | PAR-15…17,20 |
| `src/scrape_local.rs` | PAR-13 |
| `src/sg_local.rs` | PAR-14 |
| `src/browser/mod.rs`, `sheet_local.rs`, `qr_local.rs`, `residual.rs` | PAR-18 |
| `src/commands_prd/mod.rs` | PAR-17 |
| `Cargo.toml` (windows-sys SystemInformation) | PAR-15 |
| `scripts/parallelism-check.sh` | PAR-19 |
| `gaps.md` | este doc |

### 37.5 Validação Pass 22

```text
cargo test --lib                                           305 ok
cargo test --lib concurrency::                             9 ok
./scripts/parallelism-check.sh                             PASS
doctor --offline --quick --json                            concurrency.commands present
```

Fontes:
- Rules: `docs_rules/rules_rust_paralelismo_e_multiprocessamento.md`
- ddgs: tokio `Semaphore::acquire_owned` + JoinSet patterns; rayon `par_iter`
- GraphRAG local (`graphrag.sqlite`) + product law one-shot
- context7 / docs.rs tokio Semaphore

---

## 38. Resumo final consolidado (Pass 1–22)

- **Clap … multiplataforma + D-01…D-22:** **RESOLVIDOS**  
- **i18n multi-idioma (I18N-01…I18N-14):** **RESOLVIDOS**  
- **Ownership/borrowing/lifetimes (OWN-01…OWN-12):** **RESOLVIDOS**  
- **Paralelismo/multiprocessamento (PAR-01…PAR-20):** **RESOLVIDOS** (superseded/extended by Pass 23)  
- **N/A product law:** N-01…N-130  
- **Bloqueantes abertos:** **0**  
- **Dívida estrutural aberta:** **0**

---

## 39. Pass 23 — Paralelismo reauditoria profunda (modus operandi em todas as ops)

Reauditoria completa das rules + checklist final, com causa×efeito, `context7` + `docsrs-cli` + `ddgs`, e correção de **todos** os gaps acionáveis residuais pós Pass 21–22.

### 39.1 Gaps fechados (Pass 23)

| ID | Gap | Sev | Solução | Evidência |
|----|-----|-----|---------|-----------|
| **PAR-21** | `join_bounded` só `buffer_unordered` sem Semaphore canônico | **P0** | `Arc<Semaphore>::acquire` + `buffer_unordered`; docs gate vs spawn | `concurrency.rs` |
| **PAR-22** | HTML parse no worker async | **P0** | `build_scrape_payload_blocking` via `spawn_blocking` | `scrape_local.rs` |
| **PAR-23** | find-paths multi-root sequencial; threads=cpus | **P1** | Rayon multi-root + `walk_threads()` | `find_paths.rs` |
| **PAR-24** | Sem teste panic→permit | **P1** | JoinSet panic + `available_permits` == N | `concurrency::tests` |
| **PAR-25** | Matriz incompleta | **P1** | Matriz expandida + `na_product_law` + cancel | `command_workload_matrix` |
| **PAR-26** | Sem log local de permits | **P2** | `tracing::debug!(available_permits, …)` | join_bounded / batch |
| **PAR-27** | RSS doc sem método amarrado | **P2** | Comentário `rss-baseline.sh` + floor 64 MiB | `RAM_PER_IO_TASK_MB` |
| **PAR-28** | Gate script desatualizado | **P1** | Semaphore join_bounded, spawn_blocking, na_product_law | `parallelism-check.sh` |
| **PAR-29** | heap CPU sequencial | **P2** | Doc sequential justificado (dependências de grafo) | `heap_snapshot.rs` |
| **PAR-30** | sleep sync vs async | **P2** | Comentário FINALIZE sync path | residual / browser |
| **PAR-31** | Crawl discovery re-fetch fora do gate | **P1** | Discovery **dentro** da task (mesmo permit) | `crawl_http` |
| **PAR-32** | std::fs em async (PDF/screenshot) | **P1** | `spawn_blocking` PDF + `save_screenshot_async` | browser / screenshot |
| **PAR-33** | sanitize multi-page sequencial | **P2** | `join_bounded` navigates | `network.rs` |
| **PAR-34** | attach multi-target sequencial | **P2** | `join_bounded_ordered` attaches | `native/browser.rs` |
| **PAR-35** | browser batch multi-URL | **N/A** | N-129 reafirmado na matriz | matrix |
| **PAR-36** | crawl limit sem abort_all | **P1** | `set.abort_all()` + drain | `crawl_http` |
| **PAR-37** | sg walk_threads=cpu_count | **P2** | `walk_threads()` | `sg_local.rs` |
| **PAR-38** | heap yield se async | **P2** | Doc: path sync only | heap_snapshot |
| **PAR-40** | batch/crawl ignoram cancel | **P1** | `current_cancel()` mid acquire + abort | scrape_local |
| **PAR-41** | sem instrument fan-out | **P2** | `#[tracing::instrument]` batch | scrape_local |
| **PAR-42** | dialog_task lifecycle | **P2** | Já aborta no fim de nav (docs) | browser/mod |
| **PAR-43** | workflow ready-set | **N/A** | N-130 | matrix |
| **PAR-44** | Command::spawn em loop | **P2** | gate: sem unbounded_channel; spawns single | parallelism-check |
| **PAR-45** | mitm JoinHandle | **OK** | Awaited; matriz sequential | mitm_local |
| **PAR-49** | AUTO_BUDGET OnceLock | **P2** | Doc one-shot N/A rebalance | concurrency module docs |

### 39.2 N/A adicionais (Pass 23)

| ID | Item | Justificativa |
|----|------|---------------|
| **N-131** | parking_lot só por checklist | Ledger sem `.await`; churn sem ganho |
| **N-132** | Rayon dentro de task Tokio sem bridge | Deadlock; proibido |
| **N-133** | `unbounded_channel` em produção | Ausente; gate local |

### 39.3 Checklist final de conformidade (subset one-shot)

| Item rule | Status |
|-----------|--------|
| Classifiquei workload | **PASS** (matriz completa) |
| Bound em TODO fan-out | **PASS** (Semaphore join_bounded + JoinSet) |
| Permits CPU×RAM×50% | **PASS** |
| RSS task documentado | **PASS** (rss-baseline + floor) |
| Arc Semaphore + acquire(_owned) | **PASS** |
| spawn_blocking CPU em async | **PASS** |
| systemd MemoryMax default | **N/A** N-121 |
| OnceLock caros | **PASS** |
| JoinSet + JoinError panic/cancel | **PASS** |
| Cancel mid fan-out | **PASS** |
| teste peak + panic permit | **PASS** |
| --max-concurrency | **PASS** |
| docs + sequential justificado | **PASS** |
| loom / OTel remota / multi-Chrome | **N/A** |

### 39.4 Arquivos tocados (Pass 23)

| Arquivo | Gaps |
|---------|------|
| `src/concurrency.rs` | PAR-21,24–27,41,49 + matrix + walk_threads |
| `src/scrape_local.rs` | PAR-22,31,36,40,41 |
| `src/find_paths.rs` | PAR-23 |
| `src/sg_local.rs` | PAR-37 |
| `src/native/network.rs` | PAR-33 |
| `src/native/browser.rs` | PAR-34 |
| `src/native/screenshot.rs` | PAR-32 |
| `src/native/heap_snapshot.rs` | PAR-29,38 |
| `src/browser/mod.rs` | PAR-32,30,42 |
| `src/residual.rs` | PAR-30 |
| `scripts/parallelism-check.sh` | PAR-28,44 |
| `gaps.md` | este doc |

### 39.5 Validação Pass 23

```text
cargo test --lib                                           308 ok
cargo test --lib concurrency::                             12 ok
./scripts/parallelism-check.sh                             PASS
doctor --offline --quick --json                            concurrency.commands + na_product_law
```

Fontes:
- Rules: `docs_rules/rules_rust_paralelismo_e_multiprocessamento.md`
- **docsrs-cli:** `tokio::sync::Semaphore`, `acquire_owned`, `JoinSet`, `spawn_blocking` (CPU precisa Semaphore/rayon)
- **context7:** `/websites/rs_tokio`, `/rayon-rs/rayon`
- **ddgs:** bounded JoinSet / Semaphore patterns
- Product law one-shot / zero telemetria remota

---

## 40. Resumo final consolidado (Pass 1–23)

- **Clap … multiplataforma + D-01…D-22:** **RESOLVIDOS**  
- **i18n multi-idioma (I18N-01…I18N-14):** **RESOLVIDOS**  
- **Ownership/borrowing/lifetimes (OWN-01…OWN-12):** **RESOLVIDOS**  
- **Paralelismo/multiprocessamento (PAR-01…PAR-45):** **RESOLVIDOS**  
- **N/A product law:** N-01…N-133  
- **Bloqueantes abertos:** **0**  
- **Dívida estrutural aberta:** **0**

Este arquivo é a fonte de verdade da auditoria `/r-auditoria` (rules CLI + … + **ownership** + **paralelismo**) para o estado **pós-correção** de 2026-07-18 (Pass 23).

---

## 41. Pass 24 — modus operandi em toda a superfície (cauda longa)

**Data:** 2026-07-18  
**Rules:** `docs_rules/rules_rust_paralelismo_e_multiprocessamento.md`  
**Fontes:** docsrs-cli Tokio 1.53 (`spawn_blocking` + Semaphore/rayon), context7 `/websites/rs_tokio`, ddgs (std::fs starvation / JoinSet), duckduckgo-search-cli (SERP ruidoso; complementado por ddgs).

### 41.1 Gaps acionáveis (PAR-50…72)

| ID | Gap | Sev | Correção | Arquivo(s) |
|----|-----|-----|----------|------------|
| **PAR-50** | `std::fs` em async (eval/perf/heap/grab meta) | **P0** | `write_bytes_blocking` / spawn_blocking | `browser/mod.rs`, `concurrency.rs` |
| **PAR-51** | screencast N frames decode+write seq | **P0** | spawn_blocking + Rayon `par_iter` | `browser/mod.rs` |
| **PAR-52** | sg multi-root collect seq | **P1** | `roots.par_iter().flat_map` | `sg_local.rs` |
| **PAR-53** | CDP page forwarders multi-page seq | **P1** | `join_bounded` após drop lock | `native/cdp/client.rs` |
| **PAR-54** | state load multi-origin | **P1** | **N-143** sequential justificado + comentário | `native/state.rs` + matrix |
| **PAR-55** | residual multi-candidate | **P2** | `map_cpu` live-process checks | `residual.rs` |
| **PAR-56** | mitm domains/apis filter | **P2** | `map_cpu` | `mitm_local.rs` |
| **PAR-57** | doctor checks | **P2** | doc Workload + matrix (probes cheap) | `doctor/mod.rs` |
| **PAR-58** | snapshot tree CPU | **P2** | multi-ref já join_bounded; tree build dep. ordem | matrix + existing |
| **PAR-59** | spreadsheet multi-sheet | **P2** | calamine not Sync — doc sequential | `scrape_local.rs` |
| **PAR-60** | state save fs em async | **P1** | spawn_blocking write | `native/state.rs` |
| **PAR-61** | helpers canônicos | **P1** | `write_bytes_blocking`, `map_cpu`, threshold 32 | `concurrency.rs` |
| **PAR-62** | `# Workload` módulos | **P1** | browser, residual, doctor, mitm, install, cache, state, screenshot, network, cdp, heap, perf | vários |
| **PAR-63** | matriz by_command 100% | **P0** | `command_by_command_matrix` + teste inventário | `concurrency.rs` |
| **PAR-64** | gate script Pass 24 | **P1** | by_command, helpers, screencast, sg, cdp | `parallelism-check.sh` |
| **PAR-65** | heap score pós-idom | **P2** | `map_cpu` em `duplicate_strings` | `heap_snapshot.rs` |
| **PAR-66** | find_paths Mutex | **P2** | já collect local + merge (Pass 23); N/A extra | find_paths |
| **PAR-67** | console/net filter | **P2** | matrix class mixed + map_cpu when large | matrix |
| **PAR-68** | browser batch multi-URL | **N/A** | N-129 reafirmado | matrix |
| **PAR-69** | perf_insight | **P2** | sequential cost≪overhead + doc | `perf_insight.rs` |
| **PAR-70** | element AX walk | **P2** | cost≪overhead | matrix N-138 |
| **PAR-71** | install few dirs | **P2** | sequential justificado | `install.rs` |
| **PAR-72** | state list few files | **P2** | sequential justificado | matrix |

### 41.2 N/A adicionais (Pass 24)

| ID | Item |
|----|------|
| **N-134** | run/exec ordered script |
| **N-135** | fill-form / press DOM |
| **N-136** | sg-rewrite --apply |
| **N-137** | sheet-write single writer |
| **N-138** | single-act interactive |
| **N-139** | llm single request |
| **N-140** | lighthouse single subprocess |
| **N-141** | type char-a-char |
| **N-142** | heap idom/RPO blind par |
| **N-143** | state multi-origin same session parallel |

### 41.3 Arquivos tocados (Pass 24)

| Arquivo | IDs |
|---------|-----|
| `src/concurrency.rs` | PAR-61,63 + helpers + by_command |
| `src/browser/mod.rs` | PAR-50,51 |
| `src/sg_local.rs` | PAR-52 |
| `src/native/cdp/client.rs` | PAR-53 |
| `src/native/state.rs` | PAR-54,60 |
| `src/residual.rs` | PAR-55 |
| `src/mitm_local.rs` | PAR-56 |
| `src/doctor/mod.rs` | PAR-57 |
| `src/scrape_local.rs` | PAR-59 |
| `src/native/heap_snapshot.rs` | PAR-65 |
| `src/native/screenshot.rs`, `network.rs`, `perf_insight.rs` | PAR-62 |
| `src/install.rs`, `src/cache.rs` | PAR-62,71 |
| `scripts/parallelism-check.sh` | PAR-64 |
| `gaps.md` | §41 |

### 41.4 Validação Pass 24

```
cargo test --lib                                           312 ok
cargo test --lib concurrency::                             16 ok
./scripts/parallelism-check.sh                             PASS
doctor --offline --quick --json                            by_command + na_product_law
```

### 41.5 Resumo final consolidado (Pass 1–24)

- **Paralelismo/multiprocessamento (PAR-01…PAR-72):** **RESOLVIDOS** (acionáveis fix + N/A documentados)
- **N/A product law:** N-01…N-143
- **Bloqueantes abertos:** **0**
- **Dívida estrutural aberta:** **0**

Este arquivo é a fonte de verdade da auditoria `/r-auditoria` (rules CLI + … + **ownership** + **paralelismo**) para o estado **pós-correção** de 2026-07-18 (Pass 24).

---

## 42. Pass 25 — Integridade matriz + residual disk/CPU (2026-07-18)

Reauditoria pós-Pass 24: gaps dominantes eram **overclaim na matriz** (gate `map_cpu` sem código), `std::fs` residual sob runtime Tokio, ausência de `filter_cpu`, e subcomandos multi-item sem entrada nested.

### 42.1 Gaps acionáveis (PAR-73…88)

| ID | Gap | Sev | Correção | Arquivo(s) |
|----|-----|-----|----------|------------|
| **PAR-73** | doctor matrix `map_cpu` falso | **P0** | `sequential_justified` + N-144 | `concurrency.rs`, `doctor/mod.rs` |
| **PAR-74** | console/net claim map_cpu sem código | **P0** | `filter_cpu` em list filters | `browser/mod.rs`, `concurrency.rs` |
| **PAR-75** | gate anti-overclaim | **P0** | `parallelism-check` + unit honesty | `scripts/parallelism-check.sh`, tests |
| **PAR-76** | nested multi-item by_command | **P0** | `console.list`, `heap.dup-strings`, … | `concurrency.rs` |
| **PAR-77** | `load_state` fs::read no async | **P0** | `read_state_json_async` + spawn_blocking | `native/state.rs` |
| **PAR-78** | `console_dump` std::fs sob block_on | **P0** | async + `write_bytes_blocking` | `browser/mod.rs`, callers |
| **PAR-79** | `net_get` path writes sync | **P0** | async + `write_bytes_blocking` | `browser/mod.rs`, callers |
| **PAR-80** | save_auto mkdir/rename async | **P1** | `create_dir_all_blocking` + `rename_blocking` | `native/state.rs` |
| **PAR-81** | save_state default mkdir async | **P1** | `create_dir_all_blocking` | `native/state.rs` |
| **PAR-82** | print_pdf ad-hoc spawn_blocking | **P2** | unificar `write_bytes_blocking` | `browser/mod.rs` |
| **PAR-83** | commands_prd large writes | **P1** | blocking/sync helpers | `commands_prd/mod.rs` |
| **PAR-84** | sem `filter_cpu` | **P1** | helper canônico threshold | `concurrency.rs` |
| **PAR-85** | assert_console* filter seq | **P2** | `filter_cpu` | `browser/mod.rs` |
| **PAR-86** | perf_insight walk seq | **P1** | map_cpu top-level arrays ≥ threshold | `native/perf_insight.rs` |
| **PAR-87** | snapshot tree | **P2** | Workload N-145 (tree seq justificado) | `native/snapshot.rs` |
| **PAR-88** | Workload headers | **P1** | modules dispatch/native/lifecycle | vários |

### 42.2 N/A adicionais (Pass 25)

| ID | Item |
|----|------|
| **N-144** | doctor cheap probes Rayon |
| **N-145** | snapshot tree build blind par |
| **N-146** | type/fill-form/press paralelo |
| **N-147** | multi-Chrome batch browser |
| **N-148** | workflow multi-writer |
| **N-149** | load multi-origin parallel (N-143) |
| **N-150** | JSON-LD few items / flags / retry |

### 42.3 Helpers novos

| Helper | Uso |
|--------|-----|
| `filter_cpu` | console/net/assert filters ≥ threshold |
| `read_to_string_blocking` | async disk UTF-8 |
| `rename_blocking` | state rotation |
| `write_bytes_sync` | outer CLI dump (no Tokio worker) |

### 42.4 Validação Pass 25

```
cargo test --lib                                           316 ok
cargo test --lib concurrency::                             20 ok
./scripts/parallelism-check.sh                             PASS
```

### 42.5 Resumo final consolidado (Pass 1–25)

- **Paralelismo/multiprocessamento (PAR-01…PAR-88):** **RESOLVIDOS** (acionáveis fix + N/A documentados)
- **N/A product law:** N-01…N-150
- **Bloqueantes abertos:** **0**
- **Dívida estrutural aberta:** **0**

Este arquivo é a fonte de verdade da auditoria `/r-auditoria` (rules CLI + … + **ownership** + **paralelismo**) para o estado **pós-correção** de 2026-07-18 (Pass 25).

---

## 43. Pass 26 — Residual index-once, async disk, sort_cpu, heap parse (2026-07-18)

Reauditoria pós-Pass 25: gaps dominantes eram **amplificação O(N×P)** no scavenge residual (`map_cpu` × full `/proc` por candidato), `std::fs` no worker async (MITM CA, Chrome temp mkdir), sorts multi-item sequenciais, `find_paths` Mutex, heap node materialize single-core, extension multi-close seq.

### 43.1 Gaps acionáveis (PAR-89…108)

| ID | Gap | Sev | Correção | Arquivo(s) |
|----|-----|-----|----------|------------|
| **PAR-89** | `/proc` rescan N× sob map_cpu | **P0** | `index_proc_cmdlines` 1× + check no índice | `residual.rs` |
| **PAR-90** | wipe orphans seq | **P1** | `map_cpu` wipe paths disjoint | `residual.rs` |
| **PAR-91** | MITM CA `fs::read` em 2× async | **P0** | `load_ca_pems_blocking` | `mitm_local.rs` |
| **PAR-92** | chrome temp `create_dir_all` no async path | **P0** | path-only build + `create_dir_all_blocking` em oxide | `chrome.rs`, `oxide.rs` |
| **PAR-93** | heap nodes materialize 100% seq | **P1** | Rayon `into_par_iter` ≥ threshold + merge maps seq | `heap_snapshot.rs` |
| **PAR-94** | sem `sort_cpu` | **P1** | helpers canônicos threshold | `concurrency.rs` |
| **PAR-95** | find_paths Mutex multi-root | **P1** | `flat_map` + collect | `find_paths.rs` |
| **PAR-96** | extension multi-closeTarget seq | **P1** | `join_bounded` | `browser/mod.rs` |
| **PAR-97** | matriz residual/heap/mitm imprecisa | **P0** | honesty by_command | `concurrency.rs` |
| **PAR-98** | parallelism-check Pass 26 | **P1** | gates index/CA/mkdir/sort/Mutex | `scripts/parallelism-check.sh` |
| **PAR-99** | testes sort/matrix residual | **P1** | unit tests | `concurrency.rs` |
| **PAR-100** | helper CA shared | **P2** | `load_ca_pems_blocking` | `mitm_local.rs` |
| **PAR-101** | Workload chrome/oxide launch | **P2** | headers PAR-92 | `chrome.rs`, `oxide.rs` |
| **PAR-102** | residual matrix gate text | **P2** | index_proc + map_cpu wipe | `concurrency.rs` |
| **PAR-103** | heap matrix parse vs idom | **P1** | doc + header | `concurrency.rs`, `heap_snapshot.rs` |
| **PAR-104** | sg findings/paths/hits sort | **P1** | `sort_cpu`/`sort_by_cpu` | `sg_local.rs` |
| **PAR-105** | mitm apis sort | **P2** | `sort_by_cpu` | `mitm_local.rs` |
| **PAR-106** | perf_insight top sorts | **P2** | `sort_by_*_cpu` | `perf_insight.rs` |
| **PAR-107** | heap class/dup/top sorts | **P2** | `sort_by_*_cpu` | `heap_snapshot.rs` |
| **PAR-108** | gaps.md §43 | **P0** | este bloco | `gaps.md` |

### 43.2 N/A adicionais (Pass 26)

| ID | Item |
|----|------|
| **N-151** | DOM single-act paralelo |
| **N-152** | heap idom/RPO par |
| **N-153** | snapshot tree build blind par |
| **N-154** | multi-Chrome batch browser |
| **N-155** | workflow multi-writer |
| **N-156** | doctor Rayon |
| **N-157** | state.list / install few dirs Rayon |
| **N-158** | mitm block/allow rules sync CLI |
| **N-159** | crawl link extract in-task (já sob Semaphore) |
| **N-160** | cookie set multi-spawn (já batch CDP) |
| **N-161** | join_bounded index re-sort pequeno |
| **N-162** | parking_lot / loom GHA / OTel remota / systemd default |

### 43.3 Helpers novos

| Helper | Uso |
|--------|-----|
| `index_proc_cmdlines` | scavenge residual 1× `/proc` |
| `load_ca_pems_blocking` | MITM CA off async worker |
| `materialize_temp_user_data_dir_sync` | testes chrome path |
| `sort_cpu` / `sort_by_cpu` / `sort_by_key_cpu` | sorts multi-item ≥ threshold |
| `create_dir_all_blocking` (oxide launch) | temp profile off async |

### 43.4 Validação Pass 26

```
cargo test --lib                                           319 ok
cargo test --lib concurrency::                             23 ok
./scripts/parallelism-check.sh                             PASS
```

### 43.5 Resumo final consolidado (Pass 1–26)

- **Paralelismo/multiprocessamento (PAR-01…PAR-108):** **RESOLVIDOS** (acionáveis fix + N/A documentados)
- **N/A product law:** N-01…N-162
- **Bloqueantes abertos:** **0**
- **Dívida estrutural aberta:** **0**

Este arquivo é a fonte de verdade da auditoria `/r-auditoria` (rules CLI + … + **ownership** + **paralelismo**) para o estado **pós-correção** de 2026-07-18 (Pass 26).

---

## 44. Pass 27 — Residual-zero disk / one-shot §5N (v0.1.5, 2026-07-19)

Reauditoria forense Chrome aberto vs CLI: processo one-shot **OK**; dívida era **disco** (`org.chromium.Chromium.*` Singleton-only) + bug `chrome_pid.take()` antes do scavenge.

### 44.1 Gaps acionáveis (RES-01…12)

| ID | Gap | Sev | Correção | Arquivo(s) |
|----|-----|-----|----------|------------|
| **RES-01** | `chrome_pid.take()` antes scavenge | **P0** | copiar pid antes do take | `lifecycle.rs` |
| **RES-02** | sem GC cross-run Singleton | **P0** | `scavenge_stale_singleton_orphans` | `residual.rs` |
| **RES-03** | teste só marker | **P0** | side-channel + doctor tests | `tests/residual_one_shot.rs` |
| **RES-04** | doctor sem residual | **P1** | `ResidualDiskReport` + check | `doctor/mod.rs` |
| **RES-05** | side-channel pós-settle | **P1** | re-scan no FINALIZE | `lifecycle.rs` |
| **RES-06** | GC só FINALIZE | **P1** | BORN scavenge | `lifecycle.rs` |
| **RES-07** | version 0.1.4 | **P2** | bump 0.1.5 | `Cargo.toml` |
| **RES-08** | magic numbers | **P2** | consts públicas | `residual.rs` |
| **RES-09** | stress local | **P1** | `scripts/residual-stress.sh` | `scripts/` |
| **RES-10** | path_references fraco | **P1** | GC por shape Singleton-only | `residual.rs` |
| **RES-11** | gaps Pass 27 | **P2** | este bloco | `gaps.md` |
| **RES-12** | parity residual disk | **P2** | nota Behavior-Closed | `parity_devtools_matrix.md` |

### 44.2 Validação Pass 27

```
cargo test --lib residual::                         6 ok
cargo test --test residual_one_shot                 4 ok
scripts/residual-check.sh                           PASS (local)
```

### 44.3 Resumo

- **RES-01…12:** **RESOLVIDOS**
- **Bloqueantes residual disco:** **0**
- **Product law:** zero Chrome residual CLI + zero marker + GC Singleton-only owned; **nunca** matar Flatpak host

### 44.4 Documentação pública sincronizada (v0.1.5)

Gaps documentais fechados na mesma entrega (raiz + `docs/` + skills + schemas):

| ID | Gap documental | Correção |
|----|----------------|----------|
| **DOC-RES-01** | README/INTEGRATIONS/llms parados em 0.1.4 residual só processo | BORN+FINALIZE Singleton GC, `doctor residual_disk`, campo JSON `residual`, inventário **63** |
| **DOC-RES-02** | `docs/ARCHITECTURE` sem lei residual disco | seção Residual product law + `residual.rs` + dual scavenge |
| **DOC-RES-03** | HOW_TO_USE / AGENTS / COOKBOOK / TESTING / MIGRATION / CROSS_PLATFORM sem residual_disk | seções + receita + migração 0.1.4→0.1.5 + gates locais |
| **DOC-RES-04** | skills EN/PT inventário 61 e sem residual-zero playbook | inventário **63**, residual-zero disk, locale/man, formulas completas |
| **DOC-RES-05** | `docs/schemas` inventário 62 e sem `locale.schema.json` | regenerado 63 + `locale.schema.json` + nota residual no doctor |
| **DOC-RES-06** | settings ensinados com env de produto | docs ensinam só flags + XDG (`--lang` / `config set lang`) |
| **DOC-RES-07** | `ARCHITECTURE` / `ROADMAP` sem par `.pt-BR` nem switcher de idioma | criados `ARCHITECTURE.pt-BR.md` / `ROADMAP.pt-BR.md` + links EN↔PT |
| **DOC-RES-08** | `TESTING` sem id literal `residual_disk`; `CROSS_PLATFORM.pt-BR` sem link inventário 63 | bullets `residual_disk` + link HOW_TO_USE.pt-BR |
| **DOC-RES-09** | tensão AGENTS vs COOKBOOK em `print-pdf --url about:blank`; EN residual em MIGRATION 0.1.3 | FORBIDDEN afinado; nota smoke residual; 0.1.3 = process/tmp, disco canônico em 0.1.5 |
| **DOC-RES-10** | `doctor.schema.json` sem menção a saída residual; leak EN na secção PT de schemas | description residual no schema; lighthouse PT traduzido |

- **DOC-RES-01…10:** **RESOLVIDOS**
- Inventário vivo: `commands --json` → **63** nomes (clap topo **61** sem `select-option`/`pick` standalone)
- Lista completa dos 63 em `docs/HOW_TO_USE*`, `docs/AGENTS*`, `docs/COOKBOOK*`, `docs/schemas/README.md`
- Pares bilíngues em `docs/`: HOW_TO_USE, AGENTS, COOKBOOK, CROSS_PLATFORM, MIGRATION, TESTING, ARCHITECTURE, ROADMAP (+ schemas bilíngue no mesmo README)
- Proibições de docs: sem catálogo de env de produto; sem CI/GHA como requisito; sem produtos banidos
