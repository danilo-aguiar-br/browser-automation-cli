# gaps.md — Auditoria profunda `browser-automation-cli` (registro histórico + fechamento v0.1.4)

> **Estado atual v0.1.4: GAP-001…GAP-025 Closed**  
> Este arquivo é o **registro histórico da auditoria pré-fix** (identificação 2026-07-17 + addendum e2e 2026-07-18) e o **mapa de fechamento** da implementação v0.1.4.  
> Seções de detalhe (problema/consequências/causa) usam tempo presente de auditoria; **status operacional atual de todos os IDs é Closed** (tabela §0).  
> Não leia prosa histórica de “quebra”/“falta” como bug aberto no produto v0.1.4.

**Data da auditoria:** 2026-07-17  
**Addendum e2e real (FlowAIper /ciclos):** 2026-07-18  
**Binário auditado (auditoria base):** `target/release/browser-automation-cli` (compilação local `cargo build --release`, exit 0)  
**Escopo:** inventário de comandos, paridade com tool-ref DevTools (52 tools / 10 categorias), pilares PRD, lifecycle one-shot, e2e real, rules Rust (GraphRAG + `docs_rules`), docs (`context7`, `docs-rs`, `duckduckgo-search-cli`).  
**Escopo addendum 2026-07-18:** feedback e2e agent-first em app real (login + `/ciclos` + mutação HTMX) — `wait`, `run` stdout, `console dump`, `schema` UX, forms HIG/popover, wait pós-navegação, assert console.  
**Restrições da auditoria base (2026-07-17):** **somente identificação** — **proibido corrigir código** naquela fase.  
**Publicação:** proibido GitHub / crates.io nesta rodada.

---

## 0. Resumo executivo

| Métrica | Resultado |
|--------|-----------|
| Compilação local release | **OK** (`Finished release` ~3 min) |
| Superfície CLI top-level | **59** comandos |
| Mapa tool-ref ↔ CLI (`commands` JSON) | **53** entradas de mapa |
| E2E oficial `scripts/e2e_all_52_tools.sh` | **TOTAL=53 PASS=53 FAIL=0 SKIP=0** + residual markers=0 |
| `cargo test --release --tests` | **OK** — lib **234 passed**; integration (parity/envelope/residual/robots/…) **0 failed** (`TEST_EXIT:0`) |
| `cargo clippy --release --all-targets -- -W clippy::all` | **OK** — **0 warnings / 0 errors** (Finished release ~2m) |
| Doctor | Chrome/launch/ffmpeg **pass**; lighthouse **ausente** (info); redis sqlite |
| Gaps reais encontrados nesta auditoria (2026-07-17) | **18 IDs** (bugs + gaps + warnings + melhorias) |
| Gaps addendum e2e FlowAIper /ciclos (2026-07-18) | **+7 IDs** (GAP-019 … GAP-025) |
| Total inventário gaps (base + addendum) | **25 IDs** |
| Correções aplicadas na auditoria base | **0** (proibido naquela fase) |

### Status de fechamento v0.1.4 (2026-07-18 — implementação)

| ID | Status | Evidência |
|----|--------|-----------|
| GAP-001 | **Closed** | `print-pdf` em `RUN_DISPATCHED_CMDS` + braço run; smoke data: URL |
| GAP-002 | **Closed** | `Cli::try_parse` + envelope JSON com `--json` |
| GAP-003 | **Closed** | `BeforeUnloadAction` accept\|dismiss em goto/reload |
| GAP-004 | **Closed** | `isolated_context` Option\<String\> nomeado |
| GAP-005 | **Closed** | reload ignore-cache; goto options documentados |
| GAP-006 | **Closed** | dialog `--if-present` soft path |
| GAP-007 | **Closed** | `INTENTIONAL_RUN_EXCLUDE` + reason |
| GAP-008 | **Closed** | doctor/lighthouse XDG honesty; e2e mock|real label |
| GAP-009 | **Closed** | scrape multi `--format` / alias formats |
| GAP-010 | **Closed** | batch/crawl `--engine browser` |
| GAP-011 | **Closed** | `mitm capture-url` one-shot + Chrome proxy |
| GAP-012 | **Closed** | view empty refuse unless `--allow-empty` |
| GAP-013 | **Closed** | print-pdf blank refuse |
| GAP-014 | **Closed** | schema dual assert kinds documentados |
| GAP-015 | **Closed** | extract --llm seletor DOM → XDG LLM |
| GAP-016 | **Closed** | sem `metrics-recording-only` no launch |
| GAP-017 | **Closed** | `tests/parity_run_inventory.rs` |
| GAP-018 | **Closed** | aliases clap format/formats limit |
| GAP-019 | **Closed** | wait multi-selector OR + matched_selector; smoke `#kb, .missing` |
| GAP-020 | **Closed** | `--json` steps[] + `--json-steps` stream |
| GAP-021 | **Closed** | console dump `[]` JSON array (nunca 0 bytes) |
| GAP-022 | **Closed** | `schema run` e `schema --cmd run` |
| GAP-023 | **Closed** | `pick` / `select-option` role=option |
| GAP-024 | **Closed** | wait `url` / `url_contains` / `navigation` |
| GAP-025 | **Closed** | assert `console_empty` / `console_no_match` |

**Gates:** `cargo test --release --tests` OK; `cargo clippy --release --all-targets -W clippy::all` OK (0 errors); e2e_all_52_tools residual markers=0; binário `browser-automation-cli` 0.1.4.



**Leitura crítica (histórica, pré-fix):** a matriz e o e2e oficial marcavam a superfície DevTools como Behavior-Closed e passavam 53/53. A auditoria independente **confirmou a maior parte do caminho feliz** e **registrou** bugs/gaps fora do gate e2e de então (principalmente `run`/`exec` incompleto, contrato agent-first em erros clap, parciais semânticos tool-ref, pilares HTTP/MITM/LLM, dependência externa lighthouse). **Status atual: Closed (GAP-001…018).**

**Addendum 2026-07-18 (e2e real FlowAIper /ciclos) — registro histórico da auditoria pré-fix; status atual Closed:** o caminho feliz de login + board + `eval`/`file_path` funcionava; a auditoria descreveu cegueira agent-first em seletor composto no `wait` (GAP-019), stdout mínimo do `run` (GAP-020), `console dump` vazio inválido (GAP-021), `schema` sem posicional (GAP-022), forms HIG/badge/popover (GAP-023), wait pós-login flake (GAP-024), assert de console ausente (GAP-025). **Todos fechados em v0.1.4** (tabela §0). Ver §10.

**Tese de causa raiz sistêmica (histórica; mitigada em v0.1.4):**  
o produto evoluiu por **ondas de paridade** clap + e2e multi-step. A auditoria pediu contrato único entre: (1) clap top-level, (2) dispatcher NDJSON de `run`/`exec`, (3) `schema --cmd` / posicional, (4) tool-ref DevTools, (5) pilares PRD, (6) observabilidade agent-first. **Mitigações v0.1.4** incluem `tests/parity_run_inventory.rs`, `print-pdf` em run, `--json-steps`, wait multi-selector/url, pick/select-option, assert console_*, schema posicional, mitm capture-url, scrape multi-format, batch/crawl `--engine browser`, dialog `--if-present`, view `--allow-empty`, console dump `[]`.

---

## 1. Método e evidências

### 1.1 Fontes obrigatórias consultadas
- GraphRAG (`graphrag.sqlite`): corpus rules-rust + memórias de paridade/e2e v0.1.3
- Rules em disco: `docs_rules/rules_rust_*` (CLI one-shot, stdin/stdout, clap, XDG, chromiumoxide, CDP, erros, tracing, etc.)
- PRD: `docs_prd/prd_browser-automation-cli.md` (§5C paridade DevTools, pilares scrape/MITM/workflow, zero telemetria)
- Matriz: `docs_prd/parity_devtools_matrix.md`
- Base de conhecimento DevTools: `base_conhecimento_chrome-devtools-mcp-main/docs/tool-reference.md` (52 tools)
- Docs: `context7` (chromiumoxide), `docs-rs` (chromiumoxide 0.9.1), `duckduckgo-search-cli` (CDP/printToPDF)

### 1.2 Compilação e testes executados
```text
cargo build --release                         → exit 0
bash scripts/e2e_all_52_tools.sh              → TOTAL=53 PASS=53 FAIL=0 (+ residual PASS)
browser-automation-cli --json doctor          → ok=true (lighthouse missing info)
auditoria manual: meta, path-light, scrape/map/search/crawl, mitm, workflow, qr,
  parse, sheet-write, find-paths, sg-scan, config XDG, multi-step run, gates de categoria
cargo test --release --tests                  → TEST_EXIT:0
  - lib: 234 passed; 0 failed
  - integration: parity_toolref_schema (11), envelope, residual_one_shot, robots_http,
    doctor_cli, cold_start, pipe_broken, proptest_parsers, … all ok
```

**Nota (histórica pré-fix; status atual Closed):** na auditoria base, a suíte unit/integration **ainda não** falhava em GAP-001/002/011 por ausência de inventário `run⊇top-level` e de envelope em erros clap / MITM composto. **Em v0.1.4** existe `tests/parity_run_inventory.rs` (e demais gates de fechamento) — não trate a frase histórica como estado atual.

### 1.3 Diagrama de Ishikawa (efeito auditado)

```
        Código                    Configuração                 Dados
           │                           │                         │
   ┌───────┴────────┐         ┌────────┴────────┐       ┌───────┴────────┐
   │run dispatcher  │         │lighthouse_path  │       │assert dual     │
   │sem print-pdf   │         │ausente no XDG   │       │surface run vs  │
   │clap≠JSON error │         │category flags   │       │clap subcmd     │
   │beforeunload    │         │experimental*    │       │wait text array │
   │bool only       │         │                 │       │em run scripts  │
   └────────────────┘         └─────────────────┘       └────────────────┘
                    \                │                 /
                     ─────────────────────────────────
                      GAPS / BUGS / FALHAS PARCIAIS
                     NA CLI browser-automation-cli
                     ─────────────────────────────────
                    /                │                 \
   ┌────────────────┐         ┌──────┴──────┐       ┌───┴──────────────┐
   │chromiumoxide   │         │Chrome host  │       │e2e mock-lh only  │
   │event schema    │         │metrics flag │       │sem inventário    │
   │drift (CDP)     │         │crashpad     │       │run⊇top-level     │
   └────────────────┘         └─────────────┘       └──────────────────┘
      Dependências              Infraestrutura            Processo
```

### 1.4 FTA (árvore de falha resumida)

```
[EVENTO TOPO: Agente não completa fluxo real one-shot com paridade total]
                              │
                         ┌────┴────┐
                         │   OR    │
                         └────┬────┘
          ┌───────────────────┼───────────────────┐
          │                   │                   │
 [run desconhece cmd]  [erro clap não-JSON]  [dep. externa ausente]
          │                   │                   │
     print-pdf etc.      --json + argv inválido   lighthouse real
          │                   │                   │
     AND: falta gate     AND: clap before        AND: e2e só mock
     inventário run      handler envelope
```

---

## 2. O que está saudável (contexto — não são gaps)

Estes itens **passaram** e2e real + smoke independente e **não** entram como bug aberto:

1. **Compilação release** e binário `browser-automation-cli` (auditoria base 0.1.3; produto atual 0.1.4).
2. **E2E 52/53 tools DevTools** (incluindo heap graph, screencast, extension, webmcp, 3p, residual DIE markers=0).
3. **Lifecycle one-shot residual** no gate oficial: `markers=0 live_marker_procs=0`.
4. **Meta agent-first parcial:** `version`, `commands`, `schema --cmd`, `doctor`, `config path|show|list-keys|set` via XDG (sem `.env` runtime).
5. **Input/nav/emulation/perf/net/console/snapshot/screenshot** no multi-step `run`.
6. **Path-light:** `qr encode/decode`, `parse`, `sheet-write`, `find-paths`, `sg-scan`, `completions`, `workflow run` offline.
7. **HTTP pillars:** `scrape` (browser/http), `batch-scrape`, `crawl --limit`, `map`, `search`, `monitor check`.
8. **MITM:** `init-ca`, `status`, `start --seconds` em `127.0.0.1` only.
9. **Gates de categoria** funcionam para extension / webmcp / 3p / screencast / click-at / heap deep.

---

## 3. Inventário de problemas (obrigatório)

Formato de cada item:

- **Problema**
- **Consequências**
- **Causa raiz (5 Porquês + validação)**
- **Solução (proposta — não implementada)**
- **Benefícios da solução**
- **Como resolver (plano técnico)**
- **Severidade / tipo / evidência**

---

### GAP-001 — BUG: `print-pdf` existe no top-level e no `schema`, mas é desconhecido em `run`/`exec`

| Campo | Conteúdo |
|-------|----------|
| **Problema** | Comando top-level `print-pdf` (CDP `Page.printToPDF`) funciona one-shot, mas no script NDJSON `run` retorna `unknown script cmd: print-pdf`. O handler existe em `src/commands_prd/mod.rs` (`handle_print_pdf`), porém **não há braço** em `src/commands_prd/run.rs` `match cmd`. |
| **Consequências** | Agente multi-passo não consegue PDF no mesmo processo após `goto/view/grab`. Quebra fluxo PRD “multi-step no browser”. Fail-fast aborta o restante do script. |
| **Causa raiz (5 Porquês)** | 1) Por quê falhou? → dispatcher devolve unknown cmd. 2) Por quê? → `match cmd` em `run.rs` não lista `print-pdf`. 3) Por quê? → comando foi adicionado só no clap/top-level. 4) Por quê passou review/e2e? → e2e 52 tools **não inclui** print-pdf (não é tool-ref DevTools). 5) **Raiz:** ausência de inventário exaustivo `top-level Commands ⊆ run dispatcher ∪ intentional_exclude` com teste de gate. |
| **Validação reversa** | Sem inventário → print-pdf só top-level → run falha → agente multi-step quebra. ✓ |
| **Solução** | Adicionar `print-pdf`/`print_pdf` no dispatcher; espelhar flags `path`/`url`; incluir no texto de `Supported:`; teste e2e + unit do inventário. |
| **Benefícios** | Paridade top-level↔run; PDF em scripts one-shot; menos falhas de agente. |
| **Como resolver** | 1) Braço em `run.rs` chamando a mesma lógica de `handle_print_pdf`/session. 2) `argv_to_step` para `exec print-pdf`. 3) Teste `parity_run_inventory` que falha se Commands browser-side faltar no match. 4) Atualizar `schema`/`docs`. |
| **Severidade** | **Alta** (bug funcional) |
| **Evidência** | `run --script` com `{"cmd":"print-pdf"}` → EC=2, message `unknown script cmd: print-pdf`. Single-shot `print-pdf --path` → ok. |

---

### GAP-002 — BUG/UX agent-first: erros do clap **não** saem no envelope JSON mesmo com `--json`

| Campo | Conteúdo |
|-------|----------|
| **Problema** | Com `--json` e argv inválido (`schema goto`, `goto` sem URL, flags erradas), a CLI imprime texto humano do clap em **stderr** e **stdout vazio** (sem `{"ok":false,...}`). Exit code costuma ser 2 (correto), mas o contrato agent-first quebra. |
| **Consequências** | Parsers de agente esperam envelope; falham com parse error. Mensagens i18n/suggestion do produto não aparecem. Diferente dos erros de domínio (`CliError`) que já usam envelope. |
| **Causa raiz (5 Porquês)** | 1) Por quê stdout vazio? → clap aborta no parse. 2) Por quê? → `Cli::parse` default error handler. 3) Por quê não interceptado? → não há `try_parse` + mapeamento para `CliError`/envelope. 4) Por quê testes não pegam? → e2e usa argv válido. 5) **Raiz:** contrato “JSON sempre no stdout para agentes” não é enforced no caminho pré-dispatch. |
| **Solução** | `Cli::try_parse_from` → em erro, emitir envelope `kind=usage` + `exit_code=2` no stdout quando `--json` estiver presente (ou sempre para máquina). |
| **Benefícios** | Agentes robustos; um único schema de erro; alinhamento `rules_rust_cli_stdin_stdout` + `tratamento_de_erros`. |
| **Como resolver** | Refatorar `main`/`lib::run` para try_parse; detectar `--json` cedo nos argv; golden tests `devtools_envelope_behavior` para clap errors. |
| **Severidade** | **Alta** (contrato agent-first) |
| **Evidência** | `browser-automation-cli --json schema goto` → stderr clap, stdout vazio, EC=2. |

---

### GAP-003 — GAP semântico tool-ref: `handleBeforeUnload` é só booleano “accept”, sem `dismiss`

| Campo | Conteúdo |
|-------|----------|
| **Problema** | Tool-ref `navigate_page.handleBeforeUnload` é enum `accept|dismiss`. CLI expõe `--handle-before-unload` booleano que **só aceita**. Não há caminho para dismiss. |
| **Consequências** | Fluxos que precisam cancelar beforeunload não são expressáveis. Paridade parcial escondida sob status “Closed”. |
| **Causa raiz** | Implementação mínima “auto-accept” para destravar navegação; tool-ref enum não foi modelado como tipo de domínio. |
| **Solução** | Flag tipada `--handle-before-unload accept|dismiss|off` (ou dois modos) + repasse CDP coerente em goto/reload/run. |
| **Benefícios** | Paridade real tool-ref; controle fino de diálogos de saída. |
| **Como resolver** | Enum clap `ValueEnum`; wire em session navigate; teste com page que registra beforeunload. |
| **Severidade** | **Média** |
| **Evidência** | `goto --help` texto “Accept beforeunload dialogs automatically”; tool-ref enum accept/dismiss. |

---

### GAP-004 — GAP semântico tool-ref: `isolatedContext` é **string nome** no tool-ref, booleano na CLI

| Campo | Conteúdo |
|-------|----------|
| **Problema** | Tool-ref: `isolatedContext` (string) cria contexto nomeado e permite **compartilhar** cookies/storage entre pages do mesmo nome. CLI: `--isolated-context` flag booleana (cria contexto isolado anônimo). |
| **Consequências** | Impossível reutilizar o mesmo contexto nomeado entre tabs/steps como no tool-ref. Testes de multi-conta/isolamento nomeado ficam incompletos. |
| **Causa raiz** | Mapeamento simplificado flag on/off em vez de newtype de nome de contexto. |
| **Solução** | `--isolated-context <NAME>` opcional; mapa nome→BrowserContextId na sessão `run`. |
| **Benefícios** | Paridade tool-ref; cenários multi-contexto reais. |
| **Como resolver** | Alterar clap; session state `HashMap<String, ContextId>`; page new/select; e2e com dois nomes. |
| **Severidade** | **Média** |
| **Evidência** | tool-ref new_page.isolatedContext string; CLI `page new --isolated-context` flag. |

---

### GAP-005 — GAP: `goto` sem `--ignore-cache` (reload tem; navigate_page tool-ref documenta ignoreCache)

| Campo | Conteúdo |
|-------|----------|
| **Problema** | `reload --ignore-cache` existe. `goto` **não** expõe ignore-cache. Tool-ref lista `ignoreCache` no navigate_page (aplicável sobretudo a reload, mas o contrato unificado espera o parâmetro no navigate). |
| **Consequências** | Agente que traduz tool-ref 1:1 para `goto` com ignoreCache falha ou silencia o parâmetro. |
| **Causa raiz** | Split CLI goto/back/forward/reload sem espelhar todos os params opcionais no braço url. |
| **Solução** | Aceitar `--ignore-cache` em goto (no-op documentado ou force revalidation) e em steps `run`; ou documentar estritamente “só reload” no schema + rejeitar com erro claro se passado em goto. |
| **Benefícios** | Contrato previsível (aceitar ou rejeitar explicitamente). |
| **Como resolver** | Preferível: rejeição tipada em schema/run se cmd=goto+ignore_cache; aceitar só em reload. Teste de schema. |
| **Severidade** | **Baixa/Média** (contrato) |
| **Evidência** | `goto --help` sem ignore-cache; `reload --help` com `--ignore-cache`. |

---

### GAP-006 — BUG de robustez: `dialog` sem diálogo aberto aborta `run` (fail-fast EC=70)

| Campo | Conteúdo |
|-------|----------|
| **Problema** | `{"cmd":"dialog","action":"accept"}` sem JS dialog ativo → CDP `-32602 No dialog is showing` → fail-fast encerra o script. |
| **Consequências** | Scripts defensivos “sempre dismiss” quebram. Diferente de tool-ref handle_dialog que assume diálogo pendente, mas em orquestrações reais o estado é incerto. |
| **Causa raiz** | Erro CDP propagado como browser fatal; sem modo `--if-present` / soft-ok. |
| **Solução** | Opção `if_present:true` ou action `accept-if-any`; default documentado; envelope com `dialog_shown:false` ok. |
| **Benefícios** | Scripts resilientes; menos flakiness e2e. |
| **Como resolver** | Tratar -32602 como usage/soft quando flag; teste fixture com e sem alert. |
| **Severidade** | **Média** |
| **Evidência** | Auditoria `run` rest script step5 dialog → EC=70. |

---

### GAP-007 — GAP de superfície: `extension install|uninstall` **propositadamente** fora de `run`

| Campo | Conteúdo |
|-------|----------|
| **Problema** | Em `run`, `extension action=install` retorna `unsupported extension action in run` (só list/reload/trigger). Install/uninstall são top-level one-shot (relançam Chrome com `--load-extension`). |
| **Consequências** | Agente não monta um único script “install → trigger → assert → uninstall”. Precisa 2–3 processos + XDG/artefatos. |
| **Causa raiz** | One-shot + flags de launch Chrome: install exige relaunch; run assume browser já vivo. Limitação arquitetural documentada, mas **ainda é gap de DX** vs tool-ref contínuo. |
| **Solução** | (A) Documentar receita multi-process no skill/COOKBOOK; (B) ou `run` meta-step que relança sessão com extensions (complexo). |
| **Benefícios** | Expectativa correta do agente; menos tentativas inválidas. |
| **Como resolver** | Expandir `schema`/suggestion; skill com receita; opcionalmente journal workflow ligando processos. |
| **Severidade** | **Média** (gap arquitetural, não regressão) |
| **Evidência** | run extension install → usage unsupported; e2e top-level install PASS. |

---

### GAP-008 — GAP operacional: `lighthouse` real ausente no ambiente; e2e só valida **mock**

| Campo | Conteúdo |
|-------|----------|
| **Problema** | `doctor`: lighthouse not on PATH/XDG. `lighthouse https://example.com` → EC=69 `unavailable`. E2E oficial passa com `scripts/mock-lighthouse.sh`. |
| **Consequências** | Scores Lighthouse reais não existem out-of-the-box. Matriz “Closed” depende de binário externo não empacotado. |
| **Causa raiz** | Decisão de não embutir Node/npm; path XDG obrigatório; CI/e2e usa mock para não acoplar. |
| **Solução** | (1) Documentar install + `config set lighthouse_path`; (2) opcional bundle de runner sem npm se PRD exigir; (3) e2e condicional `REAL_LIGHTHOUSE=1`. |
| **Benefícios** | Transparência de DoD; menos falsa sensação de auditoria real. |
| **Como resolver** | HOW_TO + doctor suggestion já existe; adicionar gate docs; teste opcional real. |
| **Severidade** | **Média** (dep. externa) |
| **Evidência** | doctor lighthouse info; CLI EC=69; e2e `source=mock`. |

---

### GAP-009 — GAP PRD scrape: um único `--format` por invocação (sem multi-format nativo)

| Campo | Conteúdo |
|-------|----------|
| **Problema** | `scrape --format` aceita um valor (`text|markdown|html|links|...`). Não há `--formats markdown,links` multi-valor em uma resposta. |
| **Consequências** | Agente que precisa markdown+links+metadata faz N invocações (custo Chrome se engine=browser). |
| **Causa raiz** | Modelo de envelope single-format; multi-format do pilar de scrape local não foi exposto como lista. |
| **Solução** | Aceitar `--format` repetível ou CSV; envelope com chaves por formato. |
| **Benefícios** | Menos cold-starts; paridade com pipelines de scrape multi-artefato. |
| **Como resolver** | clap `Vec<Format>`; pipeline scrape_local preenche campos; golden envelope. |
| **Severidade** | **Média** (pilar scrape) |
| **Evidência** | `--formats` rejeitado; `--format markdown` ok; links em segunda chamada ok. |

---

### GAP-010 — GAP: `batch-scrape` / `crawl` forçados em engine HTTP (sem browser CDP)

| Campo | Conteúdo |
|-------|----------|
| **Problema** | `scrape` tem `--engine http|browser`. `batch-scrape` e `crawl` são HTTP-only (reqwest). SPAs/JS-heavy retornam HTML incompleto. |
| **Consequências** | Paridade incompleta do pilar de extração em massa para sites dinâmicos. |
| **Causa raiz** | Throughput/JoinSet HTTP priorizado; browser multi-URL é caro no one-shot. |
| **Solução** | `--engine browser` com concurrency 1..N e `run` interno; ou documentar limitação + receita `run` NDJSON. |
| **Benefícios** | Extração JS-complete em lote quando necessário. |
| **Como resolver** | Feature flag; pool de pages; timeouts por URL; testes example.com + fixture file://. |
| **Severidade** | **Média** |
| **Evidência** | help batch-scrape “HTTP engine”; crawl pages engine http. |

---

### GAP-011 — GAP: MITM one-shot **não** acopla Chrome proxy automaticamente

| Campo | Conteúdo |
|-------|----------|
| **Problema** | `mitm start --seconds N` sobe proxy 127.0.0.1 e encerra. Nota diz para configurar Chrome `--proxy-server=...` **durante a janela**. Não há comando único `mitm capture --url ...` que lance Chrome com proxy + CA. |
| **Consequências** | Captura HAR/API discovery exige orquestração externa (dois processos + timing). Fácil obter `capture_count=0`. |
| **Causa raiz** | Separação estrita one-shot MITM vs one-shot browser; falta “modo composto” no mesmo processo. |
| **Solução** | Comando composto `mitm capture-url` (sobe proxy, lança Chrome com proxy+CA trust one-shot, navega, exporta HAR, DIE). |
| **Benefícios** | Aceite PRD §5E utilizável por agente sem shell externo. |
| **Como resolver** | Lifecycle único; trust CA no profile temp; timeout; e2e com fixture HTTPS local. |
| **Severidade** | **Alta** para o pilar MITM prático |
| **Evidência** | mitm start 2s → capture_count=0; nota de proxy manual. |

---

### GAP-012 — GAP UX: `view` one-shot em página vazia retorna `ok:true` (about:blank)

| Campo | Conteúdo |
|-------|----------|
| **Problema** | `view` sem navegação prévia lança Chrome, snapshot vazio `tree=(empty page)`, **ok:true**. |
| **Consequências** | Agente interpreta sucesso e tenta `press @e1` em refs inexistentes. |
| **Causa raiz** | Snapshot vazio é operação CDP válida; não há política “require content/url”. |
| **Solução** | Warning no envelope `empty:true`; ou exit usage se `ref_count=0` e url about:blank sem `--allow-empty`. |
| **Benefícios** | Menos cascatas de erro; melhor agent UX. |
| **Como resolver** | Política em handler view; flag escape; teste. |
| **Severidade** | **Baixa/Média** |
| **Evidência** | `view` → ok true, ref_count 0, url about:blank. |

---

### GAP-013 — GAP: `print-pdf` one-shot sem `--url` imprime página em branco com **ok:true**

| Campo | Conteúdo |
|-------|----------|
| **Problema** | `print-pdf --path out.pdf` sem URL gera PDF de about:blank (bytes>0) sucesso. |
| **Consequências** | Artefato “válido” mas inútil; agente não detecta erro de fluxo. |
| **Causa raiz** | Mesma sessão default blank; ausência de guard de URL/conteúdo. |
| **Solução** | Exigir `--url` **ou** estado de página não-blank; senão usage error. |
| **Benefícios** | Artefatos confiáveis. |
| **Como resolver** | Validação pré-print; teste. |
| **Severidade** | **Média** |
| **Evidência** | print-pdf solo → ok, pdf 848 bytes blank. |

---

### GAP-014 — GAP dual-surface: `assert` clap (subcomandos) ≠ `assert` em `run` (kind/fields)

| Campo | Conteúdo |
|-------|----------|
| **Problema** | Top-level: `assert url|text|console ...`. Em `run`: `{"cmd":"assert","kind":"text","expect":"..."}`. Dois dialetos. |
| **Consequências** | Agente mistura sintaxes; `schema --cmd assert` pode não documentar o dialeto NDJSON. |
| **Causa raiz** | clap ergonomics vs JSON steps sem codegen único. |
| **Solução** | Schema dual explícito; `exec` argv mapper; exemplos skill. |
| **Benefícios** | Menos erros de uso. |
| **Como resolver** | Documentar ambos em schema; testes argv_to_step. |
| **Severidade** | **Baixa/Média** |
| **Evidência** | `assert --help` subcommands; run usa kind. |

---

### GAP-015 — GAP residual LLM: `extract --llm` depende de chave XDG e URL/arquivo

| Campo | Conteúdo |
|-------|----------|
| **Problema** | `extract --llm` exige `config set openrouter_api_key` (XDG) e target http(s)/file — não seletor DOM sozinho. |
| **Consequências** | Pilar “extract LLM” residual; agente com seletor falha com usage. |
| **Causa raiz** | Opt-in rede LLM; separação scrape HTTP vs DOM. |
| **Solução** | (1) Se target seletor + --llm, serializar textContent e mandar ao LLM; (2) docs claros. |
| **Benefícios** | Extract LLM no mesmo fluxo browser. |
| **Como resolver** | Branch handler; sem hardcode de endpoint além de XDG llm_base_url. |
| **Severidade** | **Baixa** (opt-in) |
| **Evidência** | extract body --llm → usage must be http(s) or file. |

---

### GAP-016 — WARNING: Chrome launch ainda passa `--metrics-recording-only`

| Campo | Conteúdo |
|-------|----------|
| **Problema** | `src/native/cdp/chrome.rs` inclui `--metrics-recording-only` junto com `--disable-background-networking`. PRD zero telemetria pede anti-metrics agressivo; “metrics-recording-only” **limita** mas **não elimina** o subsistema de metrics do Chromium. Crashpad handlers ainda aparecem no process tree do host. |
| **Consequências** | Risco residual de telemetria/crash reports do browser embutido; auditoria de privacidade pode reprovar. |
| **Causa raiz** | Flags herdadas de receitas headless “anti-ruído” sem matriz PRD 5F fechada linha-a-linha. |
| **Solução** | Revisar flags vs PRD 5F: preferir disable metrics/crash reporter onde suportado; documentar limites do Chromium host. |
| **Benefícios** | Alinhamento privacidade por default. |
| **Como resolver** | Diff de flags; teste doctor/privacy; strings/process asserts em e2e residual. |
| **Severidade** | **Média** (compliance) |
| **Evidência** | chrome.rs linha com `--metrics-recording-only`; process list com chrome_crashpad_handler. |

---

### GAP-017 — WARNING: inventário `run` Supported incompleto vs top-level

| Campo | Conteúdo |
|-------|----------|
| **Problema** | Mensagem `Supported: goto wait ... webmcp-exec` omite vários top-level browser-side (ex.: **print-pdf**, monitor, cookie ok está, etc.). Não há teste que compare `Commands` enum ⊆ dispatcher. |
| **Consequências** | Novos comandos repetirão GAP-001. Agente não descobre cobertura real de `run`. |
| **Causa raiz** | Manutenção manual de três listas (clap, match, suggestion string). |
| **Solução** | Única source of truth (macro/enum exhaustivo) + teste `run_dispatch_covers_browser_commands`. |
| **Benefícios** | Impede regressão estrutural. |
| **Como resolver** | Enum `RunCmd` shared; compile-time exhaustiveness; CI test. |
| **Severidade** | **Alta** (processo/qualidade) — causa raiz de GAP-001 |
| **Evidência** | suggestion string em run.rs; print-pdf ausente. |

---

### GAP-018 — GAP DX: flags/nomes inconsistentes com expectativas de agente (`--formats` vs `--format`, `--max-pages` vs `--limit`)

| Campo | Conteúdo |
|-------|----------|
| **Problema** | Auditoria humana errou flags (`--formats`, `--max-pages`, `--url` em scrape, `--out` em qr). CLI é consistente **internamente**, mas **schema/help** não são a primeira superfície que o agente “chuta”. |
| **Consequências** | Muitas falhas usage EC=2 em automação; atrito. |
| **Causa raiz** | Naming clap local sem aliases de compatibilidade. |
| **Solução** | Aliases clap (`alias = "formats"`, `alias = "max-pages"`) + `schema` rico + skill formulas. |
| **Benefícios** | Menos tentativas; melhor discovery. |
| **Como resolver** | `#[arg(alias=...)]`; testes de parse alias. |
| **Severidade** | **Baixa** (DX) — mas volume alto de fricção |
| **Evidência** | múltiplos `unexpected argument` na auditoria antes de ler `--help`. |

---


> **Registro histórico da auditoria pré-fix (GAP-019…GAP-025); status atual Closed** — as seções abaixo preservam o texto de identificação; não interpretá-las como bugs abertos no produto v0.1.4.

### GAP-019 — BUG/UX: `wait` com seletor CSS composto (vírgula / OR) falha ou não é confiável

| Campo | Conteúdo |
|-------|----------|
| **Problema** | Em e2e real FlowAIper `/ciclos`, `wait` com `selector: "#kb, .kwrap, .ciclos-actions"` retornou `wait condition not met before deadline` apesar de `#kb` existir no HTML. Mitigação: seletor **único** `#kb` + timeout maior. |
| **Consequências** | Fail-fast aborta o `run`; passos `eval`/`console`/submit não executam; falso negativo em páginas válidas. |
| **Causa raiz (5 Porquês)** | 1) Wait falhou no prazo. 2) Seletor multiplo com vírgula. 3) Path de wait pode tratar a string de forma rígida / não equivalente a `querySelector` com lista OR. 4) Sem validação prévia nem erro claro “multi-selector não suportado”. 5) **Raiz:** contrato de `wait.selector` não documenta nem implementa OR de forma estável; gate e2e só usa seletor simples. |
| **Solução** | (A) Suportar seletor composto via `querySelector` nativo do browser; **ou** (B) rejeitar com erro usage claro se multi-selector não for suportado. Documentar no `schema --cmd wait`. |
| **Benefícios** | Menos flake; scripts de agente mais robustos; RCA mais rápida. |
| **Como resolver** | Unificar wait selector no page CDP; teste com `"#a, #b"` em fixture; schema description + e2e. |
| **Severidade** | **Alta** (na auditoria: quebrava pipeline real; **Closed em v0.1.4**) |
| **Evidência** | Feedback e2e 2026-07-18 — step 7 `wait` fail-fast; `#kb` presente no HTML. |

---

### GAP-020 — GAP agent-first: `run` stdout quase vazio (só resumo `ok run steps=N`)

| Campo | Conteúdo |
|-------|----------|
| **Problema** | `run` com 20 steps termina com sucesso, mas stdout efetivo é só a linha de sucesso. Resultados de `eval` / `console` **não** aparecem no stream padrão sem `file_path` ou verbose útil. |
| **Consequências** | Agente cego: não vê `guardOk`, scripts, erros de console; precisa reexecutar só para extrair dados; atrasa RCA e loops e2e. |
| **Causa raiz** | Modo quiet / resumo mínimo no fim do `run`; falta NDJSON de resultado por passo no stdout (ou flag `--json-steps`). |
| **Solução** | Emitir NDJSON por passo (`step`, `cmd`, `ok`, `result`) quando `--json`; ou flag explícita `--json-steps` / `--emit-step-results`. |
| **Benefícios** | Agent-first completo; um único `run` basta para RCA. |
| **Como resolver** | Em `run.rs`, após cada `execute_step`, serializar envelope parcial para stdout (ou buffer + stream); manter quiet como opt-in. |
| **Severidade** | **Alta** (contrato agent-first; **Closed em v0.1.4** via `--json` steps + `--json-steps`) |
| **Evidência** | Feedback e2e 2026-07-18 — `ok run steps=20` sem payload de steps. |

---

### GAP-021 — BUG: `console dump` grava arquivo **vazio** (0 bytes) em lista vazia

| Campo | Conteúdo |
|-------|----------|
| **Problema** | `{"cmd":"console","action":"dump","path":"..."}` cria arquivo com **0 bytes**; `json.load` falha (`Expecting value`). |
| **Consequências** | Impossível provar “console limpo” via dump; força confiar em `console_count` embutido em `eval` + `file_path`. |
| **Causa raiz** | `dump` sem mensagens não escreve `[]` JSON válido — arquivo vazio em vez de array vazio serializado. |
| **Solução** | Sempre serializar JSON válido (`[]` se vazio; array de entradas se houver). |
| **Benefícios** | Contrato estável para assert/arquivo; parsers de agente não quebram. |
| **Como resolver** | Em `console_dump`, `serde_json::to_vec_pretty(&entries)` mesmo se `entries.is_empty()`; teste unitário empty dump. |
| **Severidade** | **Alta** (bug funcional + JSON inválido; **Closed em v0.1.4** — dump `[]`) |
| **Evidência** | Feedback e2e 2026-07-18 — `/tmp/e2e_htmx/console.json` 0 bytes. |

---

### GAP-022 — GAP DX: `schema` não aceita comando posicional (`schema run` falha)

| Campo | Conteúdo |
|-------|----------|
| **Problema** | `browser-automation-cli schema run` → erro clap. Forma correta: `schema --cmd run`. |
| **Consequências** | Descoberta de schema falha no primeiro try do agente; tempo perdido redescobrindo a flag. |
| **Causa raiz** | API assimétrica: subcomando `Schema { cmd: String }` exige `--cmd`, enquanto a UX mental é `schema <cmd>`. |
| **Solução** | Aceitar posicional **além** de `--cmd` (`#[arg(value_name="CMD")]` ou dual). |
| **Benefícios** | Discovery agent-first no primeiro try; alinha com `commands` mental model. |
| **Como resolver** | clap: posicional opcional + long `--cmd`; se ambos, preferir posicional; teste de parse. |
| **Severidade** | **Média** (DX / discovery) |
| **Evidência** | Feedback e2e 2026-07-18 — `schema run` erro; só `--cmd` funciona. |

---

### GAP-023 — GAP: sem helper para form HIG / badge / popover / custom select

| Campo | Conteúdo |
|-------|----------|
| **Problema** | Modal `#dlgNovo` abriu; `eval` setou `input/select` (`tipo=anomalia`); submit `#btnCriar` → toast “Tipo é obrigatório”; dialog permaneceu aberto; **nenhum POST** na network do browser. |
| **Consequências** | E2e de mutação HTMX real (create card) falhou no browser; validação create só via `curl` + `HX-Request`. Fluxo UI completo não fecha na CLI. |
| **Causa raiz** | UI não é form nativo puro: tipo/prioridade usam **badge + popover + hidden/sync**. Setar `.value` no DOM não passa na validação client-side. `write`/`press` não “escolhe” opção de popover HIG. |
| **Solução** | Helpers: `press` em `role=option` / popover; `write`/`select-option` em custom select; ou `click` + `wait` recipe documentada no schema. |
| **Benefícios** | E2e UI real em SPAs/HIG sem reimplementar app em `eval`. |
| **Como resolver** | Comando ou step `select-option` / `pick` com role/name; documentar padrão badge→option; fixture HTMX no e2e. |
| **Severidade** | **Alta** (na auditoria bloqueava mutação UI real; **Closed em v0.1.4** via `pick`/`select-option`) |
| **Evidência** | Feedback e2e 2026-07-18 — toast “Tipo é obrigatório”; zero POST HTMX no browser. |
| **Nota** | Bug de produto (`htmx classList null`) é **fora do CLI**; CLI serviu para provar scripts + guard. |

---

### GAP-024 — GAP robustez: primeiro `wait` pós-login / pós-submit frágil (sem wait URL/navigation)

| Campo | Conteúdo |
|-------|----------|
| **Problema** | Após `press` submit no login, `wait ms=1500` + `goto /ciclos` — em um run a board falhou; em outros ok. |
| **Consequências** | Flake e2e: cookie/navegação ainda não estáveis. |
| **Causa raiz** | Sem `wait` por URL (`/`) ou seletor pós-auth antes do `goto`; `press` submit não espera navigation complete de forma explícita no script. |
| **Solução** | Suportar e documentar `wait url` / `wait navigation` (e/ou auto-wait pós-press em form submit). |
| **Benefícios** | Menos flake em fluxos login→board. |
| **Como resolver** | Step `wait` com `url`/`url_contains`/`navigation`; schema; e2e login fixture. |
| **Severidade** | **Média** (flake) |
| **Evidência** | Feedback e2e 2026-07-18 — wait pós-login intermitente. |

---

### GAP-025 — GAP: falta assert de console (`console_empty` / `no_match TypeError`)

| Campo | Conteúdo |
|-------|----------|
| **Problema** | Pedido de melhoria: `assert kind=console_empty` / `assert no_match TypeError` para provar console limpo no pipeline. |
| **Consequências** | Agente depende de `eval` + `console_count` ad-hoc ou dump (hoje bugado se vazio). |
| **Causa raiz** | Superfície `assert` em `run` cobre url/text etc., mas não kinds de console de forma first-class. |
| **Solução** | `assert kind=console_empty` e `assert kind=console_no_match pattern=…` (ou `no_match` genérico em console). |
| **Benefícios** | Gate e2e declarativo; menos `eval` boilerplate. |
| **Como resolver** | Estender `AssertKind` + handler run; schema; teste com console limpo e com TypeError. |
| **Severidade** | **Média** (DX / qualidade de assert) |
| **Evidência** | Feedback e2e 2026-07-18 — pedidos de melhoria §7 item 7. |

---
## 4. Mapa de paridade DevTools (auditoria vs matriz)

| Categoria tool-ref | Resultado e2e oficial | Gaps remanescentes (esta auditoria) |
|--------------------|----------------------|-------------------------------------|
| Input (10) | PASS | dialog fail-fast (GAP-006); extension install fora de run (GAP-007) |
| Navigation (6 + split CLI) | PASS | beforeunload enum (GAP-003); isolatedContext nome (GAP-004); ignoreCache goto (GAP-005) |
| Emulation (2) | PASS | — |
| Performance (3) | PASS | — |
| Network (2) | PASS | requer `--capture-network` (esperado) |
| Debugging (8) | PASS | lighthouse real (GAP-008); view empty ok (GAP-012) |
| Memory (12) | PASS | deep ops exigem `--category-memory` (esperado) |
| Extensions (5) | PASS top-level | install/uninstall ≠ run (GAP-007) |
| Third-party (2) | PASS | gate `--category-third-party` |
| Web surface (2) | PASS | gate `--category-webmcp` |
| Extra CLI print-pdf | n/a e2e | **Closed** em v0.1.4 (GAP-001) |

**Conclusão paridade §5C (histórica + atual):** caminho e2e das 52 tools **fechado no gate**. Itens listados na auditoria como semântica fina / DX / deps / run-extra foram **Closed em v0.1.4** (tabela §0).

---

## 5. Análise de causa raiz consolidada (Fase 3)

### 5.1 Cadeia principal (sistêmica)

| Nível | Pergunta | Resposta baseada em dados |
|-------|----------|---------------------------|
| Sintoma | Agente/auditor encontra bugs apesar de e2e 53/53 PASS | print-pdf em run; clap non-JSON; MITM vazio; lighthouse real ausente |
| Por quê 1 | Por que e2e não pegou? | Gate cobre tool-ref 52 + residual, **não** todo top-level nem clap error path |
| Por quê 2 | Por que o gate é estreito? | DoD histórico = paridade tool-ref Behavior-Closed |
| Por quê 3 | Por que top-level diverge de run? | Dois dispatchers manuais (clap vs match) |
| Por quê 4 | Por que manuais? | Sem enum único / teste de inventário cruzado |
| **Causa raiz** | **Falta de single source of truth + gates de inventário** entre clap, run, schema e tool-ref/PRD pillars | |

**Validação reversa:** inventário único → print-pdf entra no run → e2e inventário falha se omitir → clap errors envelope testados → gaps estruturais não reaparecem. ✓

### 5.2 Cadeias bifurcadas

**Cadeia A — Agent envelope**  
clap default errors → não passa por `CliError` → stdout sem JSON → agentes quebram (GAP-002).

**Cadeia B — Pilar MITM**  
proxy one-shot isolado → Chrome sem proxy automático → capture_count=0 → aceite prático falha (GAP-011).

**Cadeia C — Lighthouse**  
binário externo + mock e2e → Closed na matriz sem scores reais no host (GAP-008).

**Cadeia D — Privacidade**  
flags Chrome legadas → metrics-recording-only/crashpad residual (GAP-016).

**Cadeia E — Observabilidade agent-first em app real (addendum 2026-07-18; Closed v0.1.4)**  
Registro histórico: `run` resumo mínimo + dump console vazio + wait seletor composto + sem wait URL + sem helpers HIG → agente abortava/ficava cego no e2e FlowAIper (GAP-019…025). Mitigado em v0.1.4.

---

## 6. Plano de ação (contra-medidas) — **executado em v0.1.4**

> Implementação hard-close 2026-07-18. Status Closed na tabela acima.

### 6.1 To-do priorizado

| ID | Contra-medida | Ataca | Prioridade |
|----|---------------|-------|------------|
| T1 | Criar inventário exaustivo `Commands` → `run`/`exec`/`schema` com teste CI | Raiz sistêmica, GAP-001/017 | P0 |
| T2 | Implementar `print-pdf` no dispatcher run/exec + e2e | GAP-001 | P0 |
| T3 | `try_parse` + envelope JSON para erros clap quando `--json` | GAP-002 | P0 |
| T4 | Comando composto MITM+Chrome one-shot ou receita workflow | GAP-011 | P1 |
| T5 | Enum beforeunload accept\|dismiss; isolated context nomeado | GAP-003/004 | P1 |
| T6 | Guards empty view / blank print-pdf | GAP-012/013 | P1 |
| T7 | dialog `--if-present` | GAP-006 | P2 |
| T8 | scrape multi-format; batch/crawl engine browser opcional | GAP-009/010 | P2 |
| T9 | Aliases de flags + schema richer | GAP-018 | P2 |
| T10 | Matriz flags Chrome vs PRD zero-metrics; e2e privacy | GAP-016 | P1 |
| T11 | Docs/skill lighthouse real + optional e2e | GAP-008 | P2 |
| T12 | extract --llm a partir de seletor DOM | GAP-015 | P3 |
| T13 | `wait` multi-selector: suporte real **ou** erro usage claro + schema | GAP-019 | P0 |
| T14 | `run` NDJSON por passo (`--json` / `--json-steps`) | GAP-020 | P0 |
| T15 | `console dump` sempre JSON válido (`[]` se vazio) | GAP-021 | P0 |
| T16 | `schema <cmd>` posicional além de `--cmd` | GAP-022 | P1 |
| T17 | Helpers form HIG: select-option / role=option / popover pick | GAP-023 | P1 |
| T18 | `wait url` / `wait navigation` pós-submit | GAP-024 | P1 |
| T19 | `assert` console_empty / console_no_match | GAP-025 | P2 |

### 6.2 Critérios de aceite do plano (cumpridos em v0.1.4)

1. `cargo test` inventário: **zero** comando browser-side top-level sem braço run **ou** entry em `intentional_run_exclude` documentada.
2. Qualquer argv inválido com `--json` → stdout envelope `ok:false`.
3. `print-pdf` em NDJSON produz PDF não-vazio após goto fixture.
4. MITM: um comando captura ≥1 exchange contra fixture local.
5. doctor/privacy: flags alinhadas ao PRD 5F; residual DIE continua 0.
6. e2e 52 tools continua PASS=100%.
7. `wait` com seletor `"#a, #b"` resolve **ou** falha com usage claro (nunca falso negativo silencioso).
8. `run --json` emite resultado por passo (ou `--json-steps`); agente lê `eval`/`console` sem `file_path` obrigatório.
9. `console dump` com zero mensagens → arquivo `[]` (JSON parseável).
10. `schema run` e `schema --cmd run` ambos OK.
11. Documentado path para custom select / popover; wait URL pós-login reduz flake.

---

## 7. Tabela rápida severidade × esforço

| Gap | Tipo | Severidade | Esforço est. |
|-----|------|------------|--------------|
| GAP-001 print-pdf run | Bug | Alta | S |
| GAP-002 clap JSON | Bug contrato | Alta | M |
| GAP-017 inventário | Processo/raiz | Alta | M |
| GAP-011 MITM composto | Gap pilar | Alta | L |
| GAP-003 beforeunload | Gap semântico | Média | S |
| GAP-004 isolated name | Gap semântico | Média | M |
| GAP-006 dialog soft | Robustez | Média | S |
| GAP-007 ext run | Arquitetura | Média | M/L |
| GAP-008 lighthouse | Dep externa | Média | S (docs) / L (bundle) |
| GAP-009 multi-format | Gap scrape | Média | M |
| GAP-010 batch browser | Gap scrape | Média | L |
| GAP-012/013 empty ok | UX | Média | S |
| GAP-016 metrics flags | Privacy | Média | M |
| GAP-005 ignore-cache goto | Contrato | Baixa | S |
| GAP-014 assert dual | DX | Baixa | S |
| GAP-015 extract llm | Residual | Baixa | M |
| GAP-018 aliases | DX | Baixa | S |
| GAP-019 wait multi-selector | Bug/UX | Alta | S/M |
| GAP-020 run stdout steps | Gap agent-first | Alta | M |
| GAP-021 console dump empty | Bug JSON | Alta | S |
| GAP-022 schema posicional | DX | Média | S |
| GAP-023 form HIG/popover | Gap UI | Alta | M/L |
| GAP-024 wait navigation/url | Robustez/flake | Média | S/M |
| GAP-025 assert console | DX assert | Média | S |

---

## 8. Comandos de reprodução (auditoria)

```bash
# build local (sem publish)
cargo build --release

# e2e oficial DevTools
BIN=./target/release/browser-automation-cli bash scripts/e2e_all_52_tools.sh
# → TOTAL=53 PASS=53 FAIL=0

# GAP-001 (repro histórico pré-fix; v0.1.4: print-pdf conhecido em run)
printf '%s\n' '{"cmd":"goto","url":"about:blank"}' '{"cmd":"print-pdf","path":"/tmp/x.pdf"}' > /tmp/pp.ndjson
./target/release/browser-automation-cli --json --quiet run --script /tmp/pp.ndjson
# histórico → unknown script cmd: print-pdf | atual → ok / blank-guard conforme flags

# GAP-002 (repro histórico; v0.1.4: envelope JSON em erros clap com --json)
./target/release/browser-automation-cli --json schema goto
# histórico → stderr clap, stdout vazio, EC=2 | atual → envelope ok:false

# GAP-008
./target/release/browser-automation-cli --json lighthouse https://example.com
# → EC=69 unavailable (se lighthouse_path ausente)

# GAP-011
./target/release/browser-automation-cli --json mitm start --seconds 2
# → capture_count=0 sem Chrome proxy paralelo

# Residual DIE (gate)
# e2e residual_one_shot → markers=0 live_marker_procs=0

# GAP-019 (wait multi-selector) — fixture local com #kb
# printf '%s\n' '{"cmd":"goto","url":"http://127.0.0.1:3000/ciclos"}' \
#   '{"cmd":"wait","selector":"#kb, .kwrap, .ciclos-actions","timeout_ms":5000}' | \
#   $BIN --json run --script /dev/stdin
# → histórico: wait condition not met | atual Closed (OR multi-selector)

# GAP-021 (Closed)
# {"cmd":"console","action":"dump","path":"/tmp/console.json"} após clear
# → arquivo DEVE ser "[]" (v0.1.4: array JSON válido; histórico: 0 bytes)

# GAP-022 (Closed)
# $BIN schema run          # v0.1.4 aceita posicional
# $BIN schema --cmd run    # também ok
```

---

## 9. Declarações finais

1. **Compilação local obrigatória:** cumprida (`cargo build --release` OK).  
2. **Sem GitHub / crates.io:** cumprido.  
3. **Sem correção de bugs na fase de identificação (2026-07-17):** cumprido (apenas `gaps.md`).  
4. **`gaps.md` recriado do zero** na auditoria base; **melhorado de forma incremental** em 2026-07-18 com addendum e2e.  
5. **Proibições de branding/telemetria de produto:** respeitadas neste documento.  
6. **XDG:** config/doctor/list-keys validados; sem `.env` runtime de produto na CLI (credenciais de app sob teste podem vir de `.env` do **app**, não da CLI).  
7. **Paridade DevTools tool-ref:** e2e 53/53 PASS; gaps restantes são **além do gate** ou **semântica fina/pilares**.  
8. **Addendum e2e FlowAIper /ciclos (2026-07-18):** inventário estendido a **GAP-019…GAP-025** (observabilidade agent-first + wait/schema/form HIG).

---

## 10. Addendum — Feedback e2e FlowAIper /ciclos (2026-07-18)

> **Registro histórico da auditoria pré-fix; status atual Closed (GAP-019…GAP-025).**  
> Fonte: feedback de uso real da CLI em app local (`http://127.0.0.1:3000`) — login + `/ciclos` + mutação HTMX.  
> Mapeamento formal: **Problema 1→GAP-019**, **2→GAP-020**, **3→GAP-021**, **4→GAP-022**, **5→GAP-023**, **6→GAP-024**, pedido assert console→**GAP-025**.

# Feedback e2e — browser-automation-cli (FlowAIper /ciclos)

Data: 2026-07-18  
CLI: `browser-automation-cli` (Chrome CDP, one-shot `run` NDJSON)  
Alvo: `http://127.0.0.1:3000` — login + `/ciclos` + mutação HTMX

---

## O que eu fiz

- Login com credenciais de `.env` (`mcp_firefox_login_*`)
- `run` com script NDJSON (`goto` → `write` → `press` → `wait` → `eval` → `console`)
- Flags: `--capture-console`, `--capture-network`, `--timeout`, `--step-timeout`
- `eval` com `file_path` para persistir resultado JSON
- `console action=clear|list|dump`
- Validação paralela via `curl` autenticado (scripts HTML + POST HTMX)

---

## Problema 1 — `wait` com seletor composto falha

### O que aconteceu
- `wait` com `selector: "#kb, .kwrap, .ciclos-actions"`
- Erro: `run fail-fast at step 7 cmd=wait: wait condition not met before deadline`

### Consequências
- Pipeline abortou (fail-fast)
- Passos seguintes (`eval`, `console`, submit) não rodaram
- Falso negativo: página `/ciclos` existia e `#kb` estava no HTML

### Causa raiz
- Seletor CSS multiplo (vírgula / OR) no `wait` não resolveu como esperado no prazo
- Ou: `wait` trata a string de forma rígida / não equivalente a `querySelector` com lista
- Mitigação que funcionou: seletor **único** `#kb` + timeout maior

**→ GAP-019**

---

## Problema 2 — Output do `run` quase vazio no stdout

### O que aconteceu
- `run` terminou `ok run steps=20`
- stdout efetivo: só a linha de sucesso
- Resultados de `eval` / `console` não aparecem no stream padrão (sem `--verbose` útil o bastante)

### Consequências
- Agente não vê `guardOk`, scripts, erros de console sem `file_path`
- Precisa reexecutar o script só para extrair dados
- Atrasa RCA e e2e em loop agente

### Causa raiz
- Modo quiet / resumo mínimo no fim do `run`
- Falta de NDJSON de resultado por passo no stdout (ou flag `--json` por passo)
- `eval` só fica auditável de forma confiável com `file_path`

**→ GAP-020**

---

## Problema 3 — `console dump` gerou arquivo vazio

### O que aconteceu
- `{"cmd":"console","action":"dump","path":"/tmp/e2e_htmx/console.json"}`
- Arquivo criado com **0 bytes**
- `json.load` → `Expecting value`

### Consequências
- Impossível provar “console limpo” via dump
- Tive que confiar em `console_count: 0` embutido no JSON do `eval` (`file_path`)

### Causa raiz
- `dump` sem mensagens não escreve `[]` válido (arquivo vazio em vez de JSON vazio)
- Ou path/action não serializa lista vazia de forma estável

**→ GAP-021**

---

## Problema 4 — `schema` não aceita comando posicional

### O que aconteceu
- `browser-automation-cli schema run` → erro
- Forma correta: `schema --cmd run`

### Consequências
- Descoberta de schema falha no primeiro try do agente
- Tempo perdido redescobrindo a flag

### Causa raiz
- API assimétrica: subcomando `schema` exige `--cmd`, enquanto a UX mental é `schema <cmd>`

**→ GAP-022**

---

## Problema 5 — Preenchimento de form “badge/HTMX” via `eval` + `press` submit

### O que aconteceu
- Modal abriu (`#dlgNovo`)
- `eval` setou `input/select` (`tipo=anomalia`, etc.)
- Submit `#btnCriar` → toast **“Tipo é obrigatório”**
- Dialog permaneceu aberto; **nenhum POST** na network capturada pelo browser

### Consequências
- E2e de mutação HTMX real (create card) falhou no browser
- Tive que validar create via `curl` + headers `HX-Request` (200 + `HX-Retarget`)
- Bug de produto (classList) ficou coberto no guard, mas fluxo UI completo não fechou no CLI

### Causa raiz
- UI não é form nativo puro: tipo/prioridade usam **badge + popover + hidden/sync**
- Setar `.value` no DOM não passa na validação client-side do app
- `write`/`press` não “escolhe” opção de popover HIG; falta helper de **select custom / popover option**
- Network do `eval` file_path parece snapshot parcial (mutations às vezes não listadas no mesmo blob)

**→ GAP-023**

---

## Problema 6 — Primeiro `wait` pós-login frágil

### O que aconteceu
- Após `press` submit no login, `wait ms=1500` + `goto /ciclos`
- Em um run, wait da board falhou; em outros, ok

### Consequências
- Flake: às vezes login ainda não navegou / cookie ainda não estável

### Causa raiz
- Sem `wait` por URL (`/`) ou seletor pós-auth antes do `goto`
- `press` submit não espera navigation complete de forma explícita no script

**→ GAP-024**

---

## O que funcionou bem

- `doctor` — Chrome/Xvfb ok
- Login `write` email/password + submit
- `goto` + `wait selector=#kb` estável
- `eval` + `file_path` — JSON com `result`, `console_count`, `network`
- `--capture-console` → `console_count: 0` útil no payload do `eval`
- Scripts estáticos visíveis na network (htmx, app, 4 islands)
- Guard HTMX forçado via `CustomEvent('htmx:beforeSwap')` no `eval` — `guardOk: true`

---

## Pedidos de melhoria (dev browser-automation-cli)

1. **`wait`**: documentar se multi-selector (vírgula) é suportado; se não, erro claro → **GAP-019**
2. **`run` stdout**: NDJSON por passo (`step`, `cmd`, `ok`, `result`) ou `--json-steps` → **GAP-020**
3. **`console dump`**: sempre escrever JSON válido (`[]` se vazio) → **GAP-021**
4. **`schema <cmd>`**: aceitar comando posicional além de `--cmd` → **GAP-022**
5. **Helpers form HIG**: `press` em `role=option` / popover; ou `write` em custom select → **GAP-023**
6. **`wait url` / `wait navigation`**: após submit de login → **GAP-024**
7. **Assert console**: `assert kind=console_empty` / `assert no_match TypeError` → **GAP-025**

---

## Resumo 1 linha

| Problema | Consequência | Causa raiz | Gap |
|----------|--------------|------------|-----|
| Wait multi-selector | Fail-fast, e2e morto | Seletor OR não confiável no `wait` | GAP-019 |
| Stdout só “ok steps” | Agente cego sem `file_path` | Resumo mínimo do `run` | GAP-020 |
| Console dump vazio | JSON inválido | Dump não serializa lista vazia | GAP-021 |
| Schema posicional | Descoberta falha | Só `--cmd` | GAP-022 |
| Submit modal badge | Validação “Tipo obrigatório” | Valor DOM ≠ UI popover | GAP-023 |
| Wait pós-login | Flake | Sem wait de navegação/URL | GAP-024 |
| Assert console | Sem gate declarativo | Falta kind console_* | GAP-025 |

---

## Nota de produto (fora do CLI)

O bug original (`htmx classList null`) **não** foi causado pelo CLI.  
Fix: rebuild Askama + guard `beforeSwap` em `app.js` / `ciclos-modal-form.js`.  
CLI serviu para provar scripts + guard; create card feliz via browser ficou bloqueado no form badge.

---

*Fim do relatório. v0.1.4 implementou GAP-001…GAP-025 (Closed). Gates: test/clippy/e2e residual 0.*
