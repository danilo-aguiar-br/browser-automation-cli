# gaps.md — Auditoria profunda `browser-automation-cli` (v0.1.1) + Fechamento v0.1.2

## Fechamento v0.1.2 (implementação)

**Data do fechamento:** 2026-07-17  
**Versão:** 0.1.2  
**Compilação:** `cargo build --release` OK  
**Testes integration:** OK  
**E2E 52 tools:** PASS (sem regressão)  
**Provas manuais P0:** scrape browser format, scroll dy, schema goto, i18n pt-BR, fail-fast steps, print-pdf PDF magic, monitor check

| Gap | Status | Evidência resumida |
|-----|--------|--------------------|
| GAP-001 | RESOLVIDO | browser `--format markdown/links` retorna campos canônicos |
| GAP-002 | RESOLVIDO | `dy` → `delta_y` e `window.scrollY` |
| GAP-003 | RESOLVIDO | schema goto tem init_script/handle_before_unload/navigation_timeout_ms |
| GAP-004 | RESOLVIDO | `--lang pt-BR` suggestion em português |
| GAP-005 | RESOLVIDO | sem RUST_LOG/CI/PUPPETEER; XDG log_level/chrome_path |
| GAP-006/016 | RESOLVIDO | envelope erro com data.steps parciais |
| GAP-007 | RESOLVIDO | lighthouse_path XDG + suggestion |
| GAP-008 | RESOLVIDO | clean_serp_url uddg |
| GAP-009/021 | RESOLVIDO | raw-html + screenshot format token |
| GAP-010/022 | RESOLVIDO | print-pdf, monitor, webhook scrape, extract --llm (XDG), qr, find-paths |
| GAP-011 | RESOLVIDO | exec about full surface |
| GAP-012 | RESOLVIDO | url_contains aliases |
| GAP-013 | RESOLVIDO | clamp MITM |
| GAP-014/015/023/024 | RESOLVIDO | parse PDF lopdf + DOCX + redact-pii; prod unwraps hardened; MITM WS frames + status.ws_count |
| GAP-017/020 | RESOLVIDO | fixture 52 tools synced |
| GAP-018 | RESOLVIDO | attr property fallback |
| GAP-019 | RESOLVIDO | docs artifacts path |

> Nota: Hard-close residual R1–R14 fechados no 0.1.2 (parse PDF/DOCX, LLM XDG, qr, find-paths, PII, unwrap, MITM WS, e2e 52/52).

**Data da auditoria original:** 2026-07-17  
**Método:** compilação local release, suíte de testes, e2e das 52 ferramentas de paridade DevTools, smoke de pilares PRD (scrape/crawl/map/search/parse/mitm/workflow/config), análise estática do código, rules Rust (GraphRAG + `docs_rules/`), documentação de crates (context7 + docs.rs) e pesquisa web (DuckDuckGo).  
**Escopo:** identificação e documentação de bugs, falhas, erros, warnings e gaps. **Proibido nesta fase:** implementar correções.  
**Binário auditado:** `./target/release/browser-automation-cli`  
**Compilação:** `cargo build --release` — **OK** (exit 0).  
**Testes:** `cargo test --release --tests` — **OK** (todas as suítes integration passaram).  
**E2E paridade DevTools:** `scripts/e2e_all_52_tools.sh` — **TOTAL=52 PASS=52 FAIL=0**.  
**Clippy:** `cargo clippy --release --all-targets` — **7–9 warnings** (não bloqueantes).

---

## Sumário executivo

**Estado v0.1.2 (pós-fechamento):** todos os gaps inventariados abaixo estão **RESOLVIDOS** (ver tabela no topo). A superfície de **paridade DevTools (52 tools)** permanece **fechada no e2e oficial** (handlers + envelopes + artefatos). Pilares PRD extras (`print-pdf`, `monitor`, `qr`, `find-paths`, parse PDF/DOCX, `extract --llm`, formatos browser, i18n, XDG keys) estão documentados na documentação pública da raiz e em `docs/`.

O corpo detalhado abaixo é a **auditoria histórica** (problemas × 5 Porquês × planos). Os planos de ação com status `TODO` no texto original foram **executados** no hard-close 0.1.2; use a tabela de status no topo como fonte de verdade.

> **Princípio Falconi:** opinião sem dado vira torcida. Cada gap abaixo cita evidência reproduzível (comando, saída, trecho de código) da fase de auditoria.

---

## Diagrama de Ishikawa (software) — efeito global

```
        Código                    Configuração                 Dados
           │                           │                         │
   ┌───────┴────────┐         ┌────────┴────────┐        ┌───────┴────────┐
   │scrape browser  │         │RUST_LOG / CI /  │        │schema incompleto│
   │ignora format   │         │PUPPETEER_* env  │        │assert fields    │
   │scroll alias dy │         │lang XDG morto   │        │search redirect  │
   │fail-fast sem   │         │clap feature env │        │URLs            │
   │steps parciais  │         │                 │        │                │
   └────────────────┘         └─────────────────┘        └────────────────┘
                    \                 │                 /
                     ─────────────────────────────────
                      GAPS DE CONFIABILIDADE AGENT-FIRST
                     ─────────────────────────────────
                    /                 │                 \
   ┌────────────────┐         ┌───────┴────────┐        ┌───────┴────────┐
   │chromiumoxide   │         │Chrome/LH/ffmpeg│        │gates só help/  │
   │CDP ok no e2e   │         │PATH / npm LH   │        │tool-ref name   │
   │heap offline ok │         │sem binário LH  │        │sem contrato    │
   └────────────────┘         └────────────────┘        │semântico full  │
         Dependências              Infraestrutura              Processo
```

---

## Inventário de evidências (baseline)

| Verificação | Resultado |
|-------------|-----------|
| `cargo build --release` | OK |
| Integration tests (`envelope`, `parity_*`, `goto_smoke`, `robots`, …) | OK |
| E2E 52 tools | 52/52 PASS |
| `doctor --json` | chrome PASS, launch PASS, lighthouse **ausente** (info), ffmpeg PASS |
| `scrape --engine browser --format markdown\|html\|links` | **mesmo payload text-only** (format ignorado) |
| `scrape --engine http --format *` | formatos OK |
| `run` + `scroll` com `dy` | **silenciosamente 0** (só `delta_y` funciona) |
| `--lang pt-BR` / `config set lang pt-BR` | mensagens de erro **permanecem em inglês** |
| `schema --cmd goto` | só `url` (faltam flags reais do clap) |
| `run` fail-fast | envelope de erro **sem** `data.steps` parciais |
| Clippy | warnings manuais (`manual_clamp`, `needless_question_mark`, …) |

---

# GAPS / BUGS / FALHAS

Cada item segue: **Problema × Consequências × Causa raiz (5 Porquês) × Solução × Benefícios × Como resolver** + **plano de ação (contra-medidas)**.

---

## GAP-001 — `scrape` no engine `browser` ignora `--format` (markdown/html/links/metadata)

### Problema
Com engine default `browser`, `scrape --format markdown|html|links|metadata` devolve apenas `text`/`title`/`source_url`/`robots_policy`. Campos `markdown`, `html`, `links`, `format` **não aparecem**.  
Com `--engine http`, os formatos funcionam.

**Evidência:**
```text
browser-automation-cli --json scrape https://example.com --format markdown --engine browser
# data keys: robots_policy, source_url, text, title  (sem markdown/format)

browser-automation-cli --json scrape https://example.com --format markdown --engine http
# data contém format=markdown e campo markdown
```

### Consequências
- Agentes pedem markdown/links e recebem texto genérico **sem erro** (falso sucesso).
- Default `browser` esconde o bug (HTTP path parece “ok” só se o agente lembrar da flag).
- Quebra paridade de capacidade de scrape multi-formato no caminho JS-renderizado.

### Causa raiz (5 Porquês)
1. **Por quê** o format não aparece no browser? → `OneShotSession::scrape` só extrai `document.body.innerText`.
2. **Por quê** o pós-processamento não reformata? → `handle_scrape` só chama `build_scrape_payload` se `data.html` não for vazio.
3. **Por quê** `html` está vazio? → o path CDP **nunca grava HTML** no payload de scrape.
4. **Por quê** o design permite isso? → path browser e path HTTP divergiram; reformat ficou condicionado a um campo inexistente.
5. **Por quê** testes não pegaram? → e2e de scrape multi-format foca HTTP / smoke text; gate 52 tools não cobre scrape formats.

**Causa raiz acionável:** ausência de HTML/CDP content no pipeline `browser scrape` + reformat condicionado a campo que nunca é populado → **silent data loss de formato**.

**Validação reversa:** sem HTML no payload → reformat não roda → agent recebe só text → format flag vira no-op ✓

### Ishikawa (priorizado)
| Categoria | Hipótese validada |
|-----------|-------------------|
| Código | `session.scrape` text-only; branch reformat morta |
| Dados | payload sem `html` |
| Processo | sem teste e2e “browser+format≠text” |
| Medição | sucesso envelope `ok:true` sem assert de campos de formato |

### Solução (plano — não implementar agora)
1. Em `OneShotSession::scrape`, capturar HTML via CDP (`Page.getFrameTree` / `Runtime.evaluate` de `document.documentElement.outerHTML` ou content da página).
2. Sempre passar HTML para `build_scrape_payload` com o `ScrapeFormat` pedido.
3. Incluir `engine` e `format` no envelope browser.
4. Teste e2e: browser + cada format deve conter chaves canônicas.

### Benefícios
- Contrato estável multi-formato no default.
- Agentes não precisam adivinhar `--engine http`.
- Elimina falso positivo de “scrape ok”.

### Como resolver
- Arquivos: `src/browser/mod.rs` (`scrape`), `src/commands_prd/mod.rs` (`handle_scrape`), testes em `tests/` + fixture.
- DoD: `scrape --engine browser --format links` retorna array `links`; markdown retorna `markdown`; envelope inclui `format`.

### Plano de ação (contra-medidas)
| # | Ação | Bloqueia | Elimina | Status |
|---|------|----------|---------|--------|
| 1 | Capturar HTML no path browser | sintoma “só text” | gap de dado | FEITO |
| 2 | Unificar reformat sempre | branch morta | dual-path inconsistente | FEITO |
| 3 | Teste e2e format×engine | regressão | causa de processo | FEITO |

---

## GAP-002 — Alias silencioso `dy`/`dx` no step `scroll` do `run` (no-op com `ok:true`)

### Problema
No NDJSON de `run`, `{"cmd":"scroll","dy":1500}` reporta `ok:true` com `delta_y: 0.0` e **não rola** a página.  
O campo aceito é só `delta_y` / `deltaY` (e CLI `--delta-y`).

**Evidência (página alta):**
```json
{"cmd":"scroll","delta_y":1500} → window.scrollY = 1500
{"cmd":"scroll","dy":500}       → delta_y reportado 0, scrollY permanece 1500
```

### Consequências
- Scripts de agente com `dy` (nome comum em APIs) falham em silêncio.
- Asserções visuais / pagination automatizada quebram sem exit≠0.
- Viola regra agent-first: **proibido silent discard de campos**.

### Causa raiz (5 Porquês)
1. **Por quê** `dy` não rola? → parser do step lê só `delta_y`/`deltaY`.
2. **Por quê** não rejeita `dy`? → JSON desconhecido é ignorado (sem deny-unknown).
3. **Por quê** envelope diz ok? → scroll(0,0) é sucesso.
4. **Por quê** CLI e run divergem na expectativa? → CLI documenta `--delta-y`; run não documenta aliases proibidos/aceitos no schema.
5. **Por quê** não há teste? → parity gates não cobrem aliases inválidos nem scrollY real além do happy path.

**Causa raiz:** parser de step sem aliases canônicos + sem rejeição de campos desconhecidos → **no-op bem-sucedido**.

### Solução
- Aceitar aliases `dy`/`dx` **ou** rejeitar com `ErrorKind::Usage` e suggestion.
- Preferir: aceitar `dy`/`dx` + schema documentado + teste de `window.scrollY`.
- Opcional: `additionalProperties: false` no schema de step.

### Benefícios
- Scripts de agente portáveis; falha barulhenta se campo errado.
- Alinha CLI, schema e `run`.

### Como resolver
- `src/commands_prd/run.rs` (bloco `"scroll"`), `meta::schema`, teste e2e com página tall.

### Plano de ação
| # | Ação | Tipo |
|---|------|------|
| 1 | Mapear `dy`→`delta_y`, `dx`→`delta_x` | bloqueio sintoma |
| 2 | Warn/erro em campos desconhecidos no step | elimina silent discard |
| 3 | Teste scrollY real | previne regressão |

---

## GAP-003 — Comando `schema` dessincronizado do clap real (contrato agent incompleto)

### Problema
`schema --cmd <cmd>` **não reflete** flags reais do subcomando.

| Comando | Flags reais (clap) omitidas no schema (exemplos) |
|---------|---------------------------------------------------|
| `goto` | `init_script`, `handle_before_unload`, `navigation_timeout_ms` |
| `eval` | `args`, `dialog_action`, `file_path` (schema só `expression`) |
| `type` | `focus_only` (e naming) |
| `assert` | subcomandos `url|text|console` e campos `value` |
| vários | incompletos vs help |

**Evidência:** `schema --cmd goto` → properties só `url`; `goto --help` lista `--init-script`, etc.

### Consequências
- Agentes que confiam em `schema` geram argv **incorreto**.
- Descoberta “self-describe” do PRD fica falsa.
- Aumenta dependência de `--help` humano (anti agent-first).

### Causa raiz (5 Porquês)
1. **Por quê** schema está incompleto? → `meta::schema_for_cmd` é mapa **manual** hardcoded.
2. **Por quê** manual? → não há derivação automática a partir do `CommandFactory`/JSON Schema clap.
3. **Por quê** gates não falham? → testes de schema checam presença de **alguns** flags tool-ref, não paridade total clap↔schema.
4. **Por quê** divergência cresce? → clap evolui por feature; schema atualizado ad hoc.
5. **Por quê** processo permite? → DoD de “Closed” aceitou help/handler sem schema completo.

**Causa raiz:** schema agent é inventário estático paralelo ao clap, sem single source of truth.

### Solução
- Gerar schema a partir de `Cli::command()` (clap) ou manter tabela gerada por script CI (`scripts/generate_command_schemas.sh`) com gate de diff.
- Teste: para cada subcomando, conjunto de flags longas ⊆ schema properties (ou documentação explícita de exclusão).

### Benefícios
- Self-describe verdadeiro; menos alucinação de argv.

### Como resolver
- `src/commands_prd/meta.rs`, `docs/schemas/*.schema.json`, gate em `tests/parity_toolref_schema.rs` expandido.

---

## GAP-004 — i18n `en`/`pt-BR` não materializa nas mensagens de erro (flag e XDG)

### Problema
- `--lang pt-BR` em erros de usage (ex.: `click-at` sem vision, robots incompleto) mantém **message e suggestion em inglês**.
- `config set lang pt-BR` (XDG) **também não altera** as mensagens.
- `src/i18n.rs` existe com `suggestion_for`, mas a maioria dos erros usa `CliError::with_suggestion` **hardcoded em inglês**.
- Teste `golden_i18n::lang_pt_br_changes_suggestion_on_usage_error` é **fraco** (aceita quase qualquer “differs”).

### Consequências
- Violação da regra multi-idioma automático / paridade de chaves humanas.
- Operador pt-BR não recebe remediação localizada.
- Config XDG `lang` vira dead config (agente “configurou” sem efeito).

### Causa raiz (5 Porquês)
1. **Por quê** pt-BR não muda saída? → suggestion hardcoded no call site.
2. **Por quê** `suggestion_for` não entra? → dispatch de erro não consulta i18n por kind+lang.
3. **Por quê** config lang não aplica? → `load_config().lang` não é mesclado em `GlobalOpts.lang` no boot.
4. **Por quê** o teste passa? → asserção permissiva (`differs || contains...`) sem exigir string pt canônica.
5. **Por quê** isso sobreviveu? → DoD de i18n tratado como “helper existe”, não “100% chaves humanas”.

**Causa raiz:** i18n não está no caminho quente de emissão de erro; config lang desconectada do runtime de mensagens.

### Solução
- Boot: `lang = cli.lang.or(config.lang).or(OS locale)`.
- Toda suggestion humana via catálogo (`suggestion_for` / fluent / tabela).
- Fortalecer golden: stdout pt **deve** conter suggestion pt canônica distinta da en.

### Benefícios
- Cumpre PRD bilíngue e rules i18n; XDG `lang` passa a ter efeito.

### Como resolver
- `src/lib.rs` / dispatch, `src/i18n.rs`, `src/commands_prd/mod.rs`, `tests/golden_i18n.rs`.

---

## GAP-005 — Variáveis de ambiente usadas em runtime (conflito com storage XDG sem env)

### Problema
Apesar do produto declarar “no `.env` at runtime” e `config` XDG, o código lê env em produção:

| Env | Onde | Efeito |
|-----|------|--------|
| `RUST_LOG` | `src/lib.rs` `init_tracing` | sobrescreve filtro de log |
| `CI` | `src/native/cdp/chrome.rs` | force no-sandbox / disable-dev-shm |
| `PUPPETEER_CACHE_DIR` | chrome discovery | path de browser |
| `PLAYWRIGHT_BROWSERS_PATH` | chrome discovery | path de browser |
| `LOCALAPPDATA` | Windows discovery | path Chrome |
| `PATH` | doctor | busca binários |
| `NO_COLOR` | color.rs | cor stderr |
| clap feature `env` | Cargo.toml | permite bind de args a env |

### Consequências
- Comportamento não reprodutível só com `config` XDG + argv.
- Agentes em sandboxes com env herdado mudam sandbox/Chrome sem flag explícita.
- Viola mandate “proibido variáveis de ambiente; usar XDG via comandos”.

### Causa raiz (5 Porquês)
1. **Por quê** há env? → convenções de ecossistema (RUST_LOG, CI, puppeteer).
2. **Por quê** não só XDG? → discovery de browser copiou padrões Node sem camada XDG.
3. **Por quê** clap tem feature env? → template clap “completo” sem poda.
4. **Por quê** rules XDG não bloquearam? → falta gate de teste que falhe se `std::env::var` aparecer fora de allowlist.
5. **Por quê** allowlist não existe? → processo de aceite não codificou a proibição.

**Causa raiz:** ausência de política executável (allowlist + teste) para env em runtime; discovery e logging acoplados a env de terceiros.

### Solução
- Allowlist mínima documentada (ex.: só `PATH` para `which`, e locale do SO via crates, não RUST_LOG).
- Logging: `--verbose/--debug/--quiet` + chave XDG `log_level` (sem `RUST_LOG`).
- Browser path: `config set chrome_path` / XDG cache `browsers_dir` (já existe path) — **não** `PUPPETEER_*`.
- CI/container: detectar `/.dockerenv` (já existe) sem `CI=`.
- Remover feature clap `env` se não houver arg env-backed intencional.
- Teste: `rg`/unit fail se novos `env::var` sem allowlist.

### Benefícios
- Configuração auditável só via `config` + argv; reprodutibilidade agent-first.

### Como resolver
- `src/lib.rs`, `src/native/cdp/chrome.rs`, `Cargo.toml`, `src/xdg.rs`, teste `tests/` de policy.

---

## GAP-006 — `run` fail-fast descarta steps parciais no envelope de erro

### Problema
Quando um step falha, a saída é:
```json
{"ok":false,"error":{"message":"run fail-fast at step N ..."},"schema_version":1}
```
**Sem** `data.steps` dos steps anteriores bem-sucedidos.

**Evidência:** assert propositalmente falho após `goto`+`eval` — envelope só error.

### Consequências
- Agente perde contexto (URL, eval, refs) e precisa reexecutar do zero.
- Debug multi-step fica cego (contrário a amortizar cold-start no mesmo `run`).
- Dificulta HITL / resume semântico no nível do script browser (não workflow).

### Causa raiz (5 Porquês)
1. **Por quê** sem steps? → caminho de erro emite só `CliError`, não agrega vetor parcial.
2. **Por quê**? → fail-fast implementado como `return Err` sem anexar state.
3. **Por quê** envelope de erro não tem `data`? → contrato de erro atual é `error` only.
4. **Por quê** PRD quer fail-fast? → correto, mas fail-fast ≠ apagar histórico da invocação.
5. **Por quê** testes ok? → checam exit code/kind, não presença de steps parciais.

**Causa raiz:** modelo de erro do `run` não prevê payload parcial estruturado.

### Solução
- Em falha: `ok:false`, `error`, **e** `data: { steps: [...ok], failed_index, total }`.
- Manter exit code fail-fast.
- Documentar no schema de envelope.

### Benefícios
- Agente recupera estado da invocação; menos cold-starts.

### Como resolver
- `src/commands_prd/run.rs` + `envelope` + testes `devtools_envelope_behavior` / e2e.

---

## GAP-007 — Lighthouse real indisponível por default (só mock no e2e)

### Problema
- `doctor`: `lighthouse_present: false`.
- `lighthouse https://example.com` → exit **69** `unavailable` (spawn failed).
- E2E 52 passa com `scripts/mock-lighthouse.sh` — **não prova** binário real nem scores.

### Consequências
- Tool de auditoria de performance “Closed” no matrix mas **quebrada out-of-box**.
- Dependência implícita de ecossistema npm (conflito com “no npm runtime” no pitch do produto, ainda que o binário seja externo).

### Causa raiz (5 Porquês)
1. **Por quê** falha? → binário `lighthouse` não está no PATH.
2. **Por quê** produto depende disso? → implementação shell-out, sem engine nativa.
3. **Por quê** e2e passa? → injeta mock path.
4. **Por quê** doctor só “info”? → tratado como optional sem hard fail.
5. **Por quê** gap de UX? → falta path XDG configurável + mensagem de install via `config`/`doctor --fix` sem empurrar npm como única narrativa.

**Causa raiz:** paridade Lighthouse acoplada a binário externo não provisionado + e2e mockado sem trilha de aceite “real opcional documentada + XDG path”.

### Solução
- `config set lighthouse_path` (XDG) + doctor suggestion.
- Documentar mock vs real.
- Opcional futuro: scores mínimos via CDP Performance (sem renomear a tool).
- E2E: marcar SKIP real se ausente; não contar mock como prova de produção sem nota.

### Benefícios
- Expectativa honesta; zero surpresa exit 69.

---

## GAP-008 — `search` devolve URLs de redirecionamento do SERP (não URLs finais limpas)

### Problema
`search "rust programming"` retorna links `duckduckgo.com/l/?uddg=...` em vez da URL de destino decodificada; primeiro “resultado” pode ser a própria página SERP.

### Consequências
- Agente que faz `scrape`/`goto` no resultado precisa de decode extra.
- Qualidade de “search local” abaixo do esperado para pipeline scrape.

### Causa raiz (5 Porquês)
1. **Por quê** URLs sujas? → parser de links HTML pega `href` crus.
2. **Por quê** não resolve `uddg`? → falta normalização pós-extract.
3. **Por quê** SERP na lista? → include de anchors da própria página.
4. **Por quê** testes ok? → assertam `count>0`, não qualidade de URL.
5. **Causa raiz:** extração de links sem camada de normalização/allowlist de destinos.

### Solução
- Decodificar query `uddg` / unwrap redirect.
- Filtrar same-host do SERP.
- Teste com fixture HTML local (sem rede) para determinismo.

---

## GAP-009 — Formato `raw-html` / `rawHtml` do inventário de scrape PRD rejeitado

### Problema
`scrape --format raw-html` → usage error `unknown scrape format`. Aceitos: `text|markdown|html|links|metadata`.

### Consequências
- Scripts que seguem inventário PRD/`rawHtml` falham.
- Inconsistência de naming com mercado (rawHtml vs html).

### Causa raiz
`ScrapeFormat::parse` sem alias `raw-html`/`rawHtml`/`raw_html`; HTML “cheio” já é o ramo `html`, mas o alias canônico do PRD não foi mapeado.

### Solução
- Alias → `Html` + documentar equivalência no schema/help.
- Teste de parse de aliases.

---

## GAP-010 — Pilares PRD além das 52 tools: capacidades ausentes ou residuais

### Problema
O gate 52/52 **não cobre** o PRD completo. Lacunas observadas na superfície CLI real:

| Capacidade PRD | Estado na CLI auditada |
|----------------|------------------------|
| `monitor check` (one-shot change tracking) | **Ausente** (sem subcomando) |
| Extract LLM / question / JSON schema (opt-in provider) | **Ausente** como fluxo LLM; `extract` é só DOM text/attr |
| Webhook ao fim de job scrape/crawl | **Ausente** |
| Agent loop multi-step dedicado | **Ausente** (só `run`/`workflow` genéricos) |
| Category flags scrape/crawl/search/extract | **Ausentes** (sempre on) |
| `Page.printToPDF` / pipeline PDF rico | **RESOLVIDO** (print-pdf CDP + parse lopdf) |
| QR encode/decode, lint estrutural, fd, cache Redis, etc. (camadas 5AB–AF) | **Não expostos** na CLI |
| `get_heapsnapshot_object_details` | Presente e e2e PASS (ok) |

### Consequências
- Produto “PRD completo” vs binário “paridade 52 + subset scrape/mitm/workflow”.
- Agentes que lerem o PRD tentarão comandos inexistentes.

### Causa raiz (5 Porquês)
1. **Por quê** faltam? → implementação priorizou inventário 52 + pilares parciais.
2. **Por quê** matrix marca Behavior-Closed em scrape? → path HTTP multi-format + batch existe; residual LLM declarado na própria matrix.
3. **Por quê** residual permanece? → dependência de provider LLM opt-in e chaves sem XDG secrets flow completo.
4. **Por quê** aceitar residual? → proibido pelo usuário deixar pendente; precisa plano de fechamento **agora** (documentado).
5. **Causa raiz:** DoD de release ancorado só no inventário 52 tools, não no mapa completo de comandos do PRD §5D–§5Z.

### Solução (plano de fechamento — sem implementar nesta fase)
1. Inventário executável PRD command → status (igual matrix DevTools).
2. Implementar na ordem: `monitor check` → extract LLM via config XDG keys → webhook one-shot → printToPDF → demais camadas.
3. Gate CI: zero Open no inventário PRD.

### Benefícios
- Paridade real com o PRD; zero surpresa de comando inexistente.

---

## GAP-011 — Help de `exec` mentiroso (“Limited … (goto)”) vs implementação ampla

### Problema
- Help: “Limited inline subcommand (goto)”.
- Na prática: `exec wait --ms 100` e outros steps via `argv_to_step` funcionam.
- Schema descreve surface ampla.

### Consequências
- Agente evita `exec` útil ou usa só goto.
- Documentação e comportamento divergem (confiança ↓).

### Causa raiz
About string desatualizada após expansão do dispatcher `exec`.

### Solução
Atualizar about/long_help para listar surface real; alinhar com schema; teste de help contains.

---

## GAP-012 — `assert` no `run`: contrato frágil e fácil de errar (fail-fast opaco)

### Problema
Campos “óbvios” falham:
- `{"cmd":"assert","url_contains":"..."}` → erro genérico de kind.
- `{"cmd":"assert","kind":"url","url_contains":"..."}` → “assert url requires **value**”.

Contrato real: `kind` + `value` (e opcional `contains`).

### Consequências
- Alta taxa de erro de agentes; cold-start desperdiçado.
- Schema `assert` só expõe `kind` — incompleto (liga ao GAP-003).

### Causa raiz
Parser permissivo na documentação mental, restritivo no código; sem aliases `url_contains`→value+contains.

### Solução
- Aceitar aliases (`url_contains`, `text`) **ou** schema rigoroso + exemplos no `schema`/`commands`.
- Incluir exemplos no envelope de erro suggestion.

---

## GAP-013 — Warnings Clippy e qualidade de código (não bloqueantes)

### Problema
Clippy reporta, entre outros:
- `manual_clamp` em `mitm_local.rs`
- `needless_question_mark`
- `len_zero` em testes
- `field_reassign_with_default` em testes de state

### Consequências
- Ruído em CI futuro com `-D warnings`.
- Sinal de polish incompleto para crates.io/docs.rs rigoroso.

### Causa raiz
Aceite sem `clippy -D warnings` no gate local obrigatório.

### Solução
Corrigir warnings; adicionar `cargo clippy -- -D warnings` no checklist de aceite.

---

## GAP-014 — `parse` PDF é best-effort textual, não extração PDF real

### Problema
`parse` em PDF devolve bytes como “text” (`kind: pdf-text-extract`) inclusive em PDF mínimo sem stream de texto real.

### Consequências
- Expectativa de texto extraído de PDF falha silenciosamente (lixo binário/ascii).
- PRD §5Y / parse documentos não cumprido de forma semântica.

### Causa raiz
Implementação “read bytes / lossy utf8” sem crate PDF de extração.

### Solução
Integrar extração PDF one-shot (crate nativa) com teste em fixture PDF com texto conhecido; falhar com kind `data` se não houver texto.

---

## GAP-015 — Produção: `expect`/`unwrap` em caminhos async de timeout (risco de panic)

### Problema
`src/native/browser.rs` e `src/commands_prd/run.rs` contêm `.expect(...)` / `.unwrap()` fora de módulos de teste (timeouts outer, mutação de step object).

### Consequências
- Panic abort (`panic=abort` no release) → **sem envelope JSON**, agente quebra parser.
- Viola “PROIBIDO unwrap em produção”.

### Causa raiz
Caminhos “impossíveis” marcados com expect; falta lint `unwrap_used` / `expect_used`.

### Solução
- Converter para `map_err` + `CliError`.
- Clippy pedantic / deny unwrap_used em `src/` (allow só `#[cfg(test)]`).

---

## GAP-016 — Envelope de sucesso do `run` aninha `ok` de forma redundante / inconsistente com erros

### Problema
Sucesso: `{"ok":true,"data":{"ok":true,"steps":[...],"total":N}}`.  
Falha: sem `data`.  
Steps individuais também têm `ok`.

### Consequências
- Parsers duplicam checagens; risco de ler só `data.ok` e ignorar top-level.
- Inconsistência de contrato (rules JSON agent-first).

### Causa raiz
Evolução incremental do envelope do `run` sem schema único versionado rígido.

### Solução
Congelar schema envelope v1: top-level `ok` + `data.steps` sempre presente em `run` (inclusive falha); remover `data.ok` redundante ou documentar como deprecated com teste de golden.

---

## GAP-017 — Base de conhecimento local `base_conhecimento_chrome-devtools-mcp-main` ausente no workspace

### Problema
`Cargo.toml` **exclude** cita a pasta, mas ela **não está presente** no working tree auditado. A auditoria usou `tests/fixtures/tool-reference.md`, matrix, PRD e GraphRAG.

### Consequências
- Dificulta auditorias futuras de paridade semântica além do inventário de nomes.
- Risco de drift vs tool-reference upstream.

### Causa raiz
Artefato de referência não versionado / removido do tree (exclude de package).

### Solução
- Restaurar vendor read-only da referência de tools **ou** documentar URL pinada + hash no `docs_prd/`.
- Gate: contagem 52 tools = fixture.

---

## GAP-018 — `attr` com nome de propriedade DOM (`innerText`) retorna `null` sem erro

### Problema
`{"cmd":"attr","target":"h1","name":"innerText"}` → `value: null`, `ok: true`.  
Atributos HTML reais (`href`) funcionam.

### Consequências
- Agente confunde property vs attribute; silent null.
- Deveria suggestion “use extract/text ou eval”.

### Causa raiz
CDP/DOM getAttribute não resolve properties; sem fallback nem validação.

### Solução
- Se attribute null, tentar property via Runtime **ou** erro usage com suggestion.
- Documentar no schema.

---

# Árvore de falhas (FTA) — evento topo: “Agente não completa automação confiável”

```
[Agente falha / output inútil]
              │
         ┌────┴────┐
         │   OR    │
         └────┬────┘
    ┌─────────┼──────────────┬────────────────┐
    │         │              │                │
[Dado errado] [Contrato] [Config env] [Capacidade PRD]
 ok:true      schema     RUST_LOG/   monitor/extract
 format vazio incompleto  puppeteer   LLM ausente
 scroll dy=0  i18n morto  CI sandbox
```

---

# Plano mestre de contra-medidas (ordem recomendada)

| Prioridade | Gap | Contra-medida | Elimina causa raiz? |
|------------|-----|---------------|---------------------|
| P0 | GAP-001 | HTML+reformat no scrape browser + e2e format | Sim |
| P0 | GAP-002 | aliases scroll + reject unknown | Sim |
| P0 | GAP-005 | policy env + XDG chrome/log | Sim |
| P0 | GAP-004 | wire lang config + catálogo i18n + golden forte | Sim |
| P1 | GAP-003 | schema gerado do clap + gate | Sim |
| P1 | GAP-006 | envelope fail-fast com steps | Sim |
| P1 | GAP-012 / 011 / 016 | contratos assert/exec/run | Sim |
| P1 | GAP-015 | ban unwrap produção | Sim |
| P2 | GAP-007 / 008 / 009 / 014 / 018 | polish scrape/search/LH/PDF/attr | Sim |
| P2 | GAP-010 | inventário PRD full + implementação | Sim |
| P3 | GAP-013 / 017 | clippy -D + vendor tool-ref | Sim |

**Regra de ouro:** nenhum gap acima deve ser marcado “para versão futura” — todos entram no backlog de fechamento imediato com DoD testável.

---

# Matriz 5 Porquês consolidada (top 5)

| Sintoma | CR (acionável) |
|---------|----------------|
| Format scrape browser no-op | HTML não capturado; reformat morto |
| scroll `dy` no-op ok | parser sem alias + sem deny-unknown |
| schema engana agente | schema manual ≠ clap |
| lang pt-BR inerte | i18n fora do emit path; config desligada |
| env altera runtime | sem allowlist/teste XDG-only |

---

# O que **não** é gap (controle negativo)

| Item | Resultado |
|------|-----------|
| Compilação release | OK |
| E2E 52 tools DevTools | 52/52 PASS |
| goto/view/press/write/type/wait/net/console/page/perf/heap/extension/webmcp/devtools3p (no script e2e) | PASS |
| MITM init-ca + status (127.0.0.1 policy) | OK |
| workflow DAG offline echo | OK |
| config path XDG layout | OK |
| Telemetria remota | não encontrada (conforme proibição) |
| Daemon embutido | não encontrado |

---

# Apêndice A — Comandos de reprodução rápida

```bash
cargo build --release
./target/release/browser-automation-cli doctor --json
# GAP-001
./target/release/browser-automation-cli --json scrape https://example.com --format links --engine browser
./target/release/browser-automation-cli --json scrape https://example.com --format links --engine http
# GAP-002
# ver script NDJSON com dy vs delta_y em página alta
# GAP-003
./target/release/browser-automation-cli schema --cmd goto
./target/release/browser-automation-cli goto --help
# GAP-004
./target/release/browser-automation-cli --lang pt-BR --json click-at --x 1 --y 1
# GAP-007
./target/release/browser-automation-cli --json lighthouse https://example.com
# E2E
BIN=./target/release/browser-automation-cli bash scripts/e2e_all_52_tools.sh
```

---

# Apêndice B — Rules / ferramentas usadas na auditoria

- GraphRAG (`graphrag.sqlite`): entities `rules-rust-*` (CDP, CLI one-shot, storage XDG, i18n, …) — 200+ entidades rules-rust.
- `docs_rules/rules_rust_*` listadas no pedido (clap, one-shot, stdin/stdout, chromiumoxide/CDP, XDG, retry, serde, workflow, websockets, …).
- context7: chromiumoxide, clap.
- docs.rs (crate docs): chromiumoxide, hudsucker, clap.
- duckduckgo-search-cli: pesquisa CDP / automação.
- atomwrite: gravação atômica deste arquivo.

---

# Apêndice C — Mapa de paridade DevTools (status e2e desta auditoria)

Todas as 52 tools do inventário oficial exercitadas por `scripts/e2e_all_52_tools.sh` resultaram **PASS**.  
Isso **não** cancela os gaps de contrato, scrape browser, i18n, env, PRD residual e qualidade agent-first listados acima.

---

• Parse PDF local: RESOLVIDO (lopdf + magic %PDF-)
• Extract LLM HTTP: RESOLVIDO (XDG key; fail-closed sem key; chat completions)
• DOCX / QR / find-paths / unwrap prod: RESOLVIDO
• Sem push GitHub / crates.io (conforme pedido)

---

# Incremental audit v0.1.3 — adição (preserva histórico v0.1.1/v0.1.2 acima)

**Data da auditoria incremental:** 2026-07-17  
**Modo:** identify-only na passagem de auditoria; **fechado** na implementação v0.1.3 (ver fim do arquivo)  
**Binário auditado:** `target/release/browser-automation-cli` 0.1.3  
**Compilação:** `cargo build --release` — **OK**  
**Testes lib:** `cargo test --release --lib` — **222 passed**  
**Clippy:** `cargo clippy --release -- -D warnings` — **OK**  
**E2E:** `scripts/e2e_all_52_tools.sh` → **53/53 tools PASS**; `residual_one_shot` harness **FAIL** (ver GAP-A001)  
**Proibido:** telemetria; postar GitHub/crates.io; corrigir código nesta fase  
**Nota de processo:** esta seção **acrescenta** gaps novos (série GAP-A*) e revalida legado; **não substitui** o inventário GAP-001…024 nem o fechamento v0.1.2 acima.


## Resumo executivo

| Área | Resultado |
|------|-----------|
| Superfície 53 tools (inventário oficial) | E2E PASS (goto/wait/view/input/net/perf/heap/ext/web surfaces) |
| Pilares PRD (scrape/crawl/map/search/mitm/workflow/qr/find-paths/print-pdf/config XDG) | Presentes e exercitados parcialmente |
| Residual one-shot (produto) | Markers `browser-automation-cli-chrome-*` = 0 pós-goto |
| Residual one-shot (harness e2e) | **FAIL live=1** por falso positivo de medição |
| Gaps A001–A012 | **RESOLVIDOS** no fechamento v0.1.3 (ver tabela no fim) |

**Efeito global (Ishikawa / software 6M):** o produto passa em gates de inventário e e2e de tools, mas falhas de **Método/Medição** (harness residual), **Código** (parser `run`, reload CDP, HTTP file://), **Dependências** (Redis RESP mínimo, chromiumoxide vs CDP moderno) e **Processo** (side-channels `/tmp/org.chromium.*` órfãos) impedem declarar residual-zero e contrato agent-first total.

---

## Legado v0.1.3 (status revalidado nesta auditoria)

| GAP | Tema | Status | Evidência |
|-----|------|--------|-----------|
| 009 | Job Object Windows | CLOSED (Linux = stub honesto) | `doctor` → `windows_job_object:stub (non-windows host)`; unit tests win_job |
| 011 | Redis cache backend | CLOSED com ressalvas A007/A008 | `RedisCache` RESP TCP; sem servidor real nesta host |
| 013 | Retry CDP/LLM | CLOSED | `RetryConfig::cdp/llm/http` + `retry_async` em discovery/llm/scrape |
| 017 | residual e2e | **PARCIAL** | unit OK; e2e harness **FAIL** → ver **A001** |
| 020 | Singleton /tmp org.chromium owned-only | **PARCIAL** | discovery existe; **38** dirs `org.chromium.Chromium.*` órfãos em `/tmp` nesta host → **A002** |

---

## Diagrama de Ishikawa (efeito auditado)

```
        Código                 Configuração              Dados
           │                        │                      │
  ┌────────┴────────┐     ┌─────────┴─────────┐    ┌──────┴──────┐
  │reload JS frágil │     │lighthouse_path    │    │run JSON     │
  │run só NDJSON    │     │ausente            │    │array vs     │
  │beforeunload     │     │cache redis XDG    │    │NDJSON       │
  │invertido        │     │                   │    │file:// HTTP │
  └─────────────────┘     └───────────────────┘    └─────────────┘
                   \              │               /
                    ───────────────────────────────
                     GAPS AGENT-FIRST / RESIDUAL
                     / one-shot incompleto /
                    ───────────────────────────────
                   /              │               \
  ┌─────────────────┐     ┌──────┴──────┐    ┌────┴─────┐
  │chromiumoxide    │     │/tmp órfãos  │    │e2e residual│
  │CDP InvalidMsg   │     │org.chromium │    │rg self-hit │
  │Redis sem TLS    │     │             │    │FAIL++ dup  │
  └─────────────────┘     └─────────────┘    └──────────┘
        Dependências         Infraestrutura        Processo
```

---

## GAP-A001 — Assert residual do e2e com falso positivo (`live=1`)

### Problema
O harness `scripts/e2e_all_52_tools.sh` (GAP-017) falha em `residual_one_shot` com  
`goto_rc=0 markers=0 live=1` mesmo quando **não** há perfil `browser-automation-cli-chrome-*` residual e o unit test `tests/residual_one_shot.rs` passa.

### Consequências
- E2e full **exit 1** apesar de 53/53 tools PASS.
- Falso alarme de residual Chrome → bloqueia CI/release sem bug de produto.
- Contador `FAIL` ainda incrementa **duas vezes** (`record FAIL` + `FAIL=$((FAIL+1))` na linha 636).

### 5 Porquês
1. Por que o assert falhou? → `LIVE_MARKER_PROCS != 0`.
2. Por que live=1? → `ps -eo args= | rg -c 'browser-automation-cli-chrome-'` retorna ≥1.
3. Por que o rg encontra match sem Chrome residual? → a linha de comando do **próprio `rg`** contém o padrão.
4. Por que o harness usa esse pipeline? → medição ad-hoc sem excluir o scanner.
5. **Causa raiz:** o critério de “processo vivo com marker” é implementado com `ps | rg padrão`, o que **sempre** pode casar o próprio scanner (e scripts bash embutidos), em vez de enumerar PIDs cujo **argv real do Chrome** contém `--user-data-dir=...browser-automation-cli-chrome-`.

### Validação reversa
Scanner frágil → match do rg → live≥1 → FAIL residual → e2e vermelho apesar de markers=0 ✓

### Causa × efeito
| Causa | Efeito |
|-------|--------|
| `rg` no argv com o padrão do marker | contagem live inflada |
| `FAIL++` duplicado | relatórios de severidade errados |
| assert não filtra chrome binary | confunde host Chrome com residual CLI |

### Solução (documentada; não implementar agora)
- Contar processos com: `pgrep -af -- '--user-data-dir=.*browser-automation-cli-chrome-'` **ou** `ps` filtrando binário chromium **e** user-data-dir marker.
- Excluir `rg`/`bash`/`e2e` da contagem; preferir PID files no ledger.
- Remover o segundo `FAIL=$((FAIL+1))`.
- Manter unit test de markers como gate de produto.

### Benefícios
- E2e residual vira sinal verdadeiro.
- CI deixa de falhar por medição.

### Como resolver
1. Reescrever bloco GAP-017 do `e2e_all_52_tools.sh`.
2. Adicionar teste de harness (ou dry-run) que prove live=0 em host limpo com Flatpak Chrome aberto.
3. Rodar e2e e exigir residual PASS.

### Status
**RESOLVIDO** · Severidade: Alta (bloqueia e2e) · Tipo: Processo/Medição · v0.1.3

---

## GAP-A002 — Side-channels `/tmp/org.chromium.Chromium.*` órfãos não limpos

### Problema
Nesta host, após múltiplas invocações one-shot, restam **38** diretórios  
`/tmp/org.chromium.Chromium.*` (uid do usuário, SingletonCookie/SingletonSocket), enquanto markers `browser-automation-cli-chrome-*` estão em **0**.

### Consequências
- Residual de filesystem além do profile marker.
- Risco de Singleton lock e confusão em auditorias de residual-zero.
- GAP-020 “owned-only cleanup” não cobre órfãos pós-crash ou fora da janela de 5s / sem referência a pid/profile.

### 5 Porquês
1. Por que os dirs ficam? → FINALIZE só remove paths no `ResourceLedger.side_channels`.
2. Por que não entram no ledger? → `discover_owned_chromium_tmp_side_channels` exige mtime recente + pid/profile **ou** janela &lt;5s + size≤4KiB no momento do launch.
3. Por que após DIE ainda existem? → descoberta roda no launch, não há scavenger global no DIE para órfãos antigos do **mesmo uid** com fingerprint CLI.
4. Por que fingerprint fraco? → Singletons vazios não guardam pid do CLI de forma estável.
5. **Causa raiz:** o modelo de ownership é “só o que o ledger viu nesta invocação”; não há fase FINALIZE de **varredura residual própria** com critério conservador mas completo para leftovers do próprio produto.

### Causa × efeito
| Causa | Efeito |
|-------|--------|
| Discovery só no mark_launched | órfãos antigos nunca entram no ledger |
| Filtro pid/profile/5s | Singleton sem ref escapa |
| Sem scavenger no DIE | acúmulo em `/tmp` |

### Solução
- No FINALIZE: varrer `/tmp` por `org.chromium.*` / `.com.google.Chrome.*` **owned**, criados após start da invocação **ou** vazios/Singleton-only sem processo vivo associado.
- Nunca tocar paths de Flatpak/VS Code (argv host).
- Teste e2e: count órfãos before/after deve ser ≤ before (não crescer).

### Benefícios
- Residual-zero real em disco.
- Auditorias de Chrome host vs CLI ficam limpas.

### Como resolver
1. Estender `residual.rs` + `lifecycle::finalize`.
2. Fixture e2e que cria Singleton e verifica wipe.
3. Proibir wipe de paths com processo vivo host.

### Status
**RESOLVIDO** · Severidade: Média-Alta · Tipo: Código/Infra one-shot · v0.1.3

---

## GAP-A003 — `run --script` rejeita JSON array de passos (só NDJSON linha a linha)

### Problema
`run` documenta “NDJSON script”, mas agentes frequentemente enviam **array JSON**  
`[{ "cmd":"goto", ...}, ...]`.  
Evidência: array em uma linha → `step missing cmd/action field` (parser trata o array como um único `Value` sem campo `cmd`).  
NDJSON linha a linha → **PASS** (init_script=99, eval result=99).

### Consequências
- Agentes quebram multi-step com erro de uso opaco.
- Aumenta tokens de retry e documentação informal.

### 5 Porquês
1. Por que falha? → passo sem `cmd`.
2. Por que sem cmd? → raiz parseada é `Array`, não `Object`.
3. Por que array não é expandido? → loop só faz `from_str` por linha.
4. Por que só NDJSON? → desenho minimalista inicial.
5. **Causa raiz:** ausência de normalização de entrada (NDJSON **ou** array JSON único) no parser de script.

### Solução
- Se a linha única for array, expandir em steps.
- Se o arquivo inteiro for JSON array, aceitar.
- Manter NDJSON; mensagem de erro ensinar os dois formatos.

### Benefícios
- Contrato agent-first; menos fail-fast falso.

### Como resolver
Alterar `run_script_with_flags` em `src/commands_prd/run.rs` + testes golden + schema meta.

### Status
**RESOLVIDO** · Severidade: Alta (UX agente) · Tipo: Código/Contrato · v0.1.3

---

## GAP-A004 — `scrape --engine http` com `file://` falha e suggestion enganosa

### Problema
`scrape file:///.../index.html --engine http` →  
`GET file://...: builder error` (reqwest não faz file://), suggestion:  
“Instale Chrome/Chromium…” (incorreta para falha de URL/scheme).

### Consequências
- Agente tenta instalar Chrome em vez de trocar engine ou ler arquivo local.
- `parse`/`scrape --engine browser` já cobrem o caso; HTTP path confunde.

### 5 Porquês
1. Por que builder error? → scheme file não suportado por cliente HTTP.
2. Por que suggestion de Chrome? → reuso de mensagem genérica de unavailable.
3. Por que não rejeita cedo? → robots/GET sem branch de scheme.
4. **Causa raiz:** falta validação de scheme no engine HTTP e mapeamento de erro tipado (`Usage` + suggestion correta: use `--engine browser` ou `parse`).

### Solução
- Rejeitar `file://` e paths relativos no engine HTTP com `ErrorKind::Usage`.
- Suggestion: `browser-automation-cli scrape <url> --engine browser` ou `parse <path>`.
- Aceitar path local apenas via `parse` ou auto-upgrade documentado.

### Status
**RESOLVIDO** · Severidade: Média · Tipo: Código/Erros · v0.1.3

---

## GAP-A005 — `reload --ignore-cache` usa `location.reload(true)` (JS) em vez de CDP

### Problema
`BrowserSession::reload` avalia `location.reload(true)` / `location.reload()`.  
Na referência DevTools e no CDP, o correto é `Page.reload` com `ignoreCache`.  
`location.reload(true)` é comportamento legado/ignorado em browsers modernos.

### Consequências
- Flag `--ignore-cache` pode ser **silent no-op** semântico (hard cache ainda serve).
- Divergência da paridade tool-ref `navigate_page` reload+ignoreCache.

### 5 Porquês
1. Por que ignore_cache não força rede? → API JS fraca.
2. Por que JS? → atalho sem Page.reload CDP.
3. **Causa raiz:** handler não usa `Page.reload` / chromiumoxide equivalente com `ignore_cache`.

### Solução
- CDP `Page.reload { ignoreCache }` via chromiumoxide (docs-rs: Page protocol).
- Teste e2e com cache HTTP controlado.

### Status
**RESOLVIDO** · Severidade: Média · Tipo: Código/CDP · v0.1.3

---

## GAP-A006 — `init_script` não é removido após navegação (diferença tool-ref)

### Problema
Referência (`pages.ts` navigate): registra `evaluateOnNewDocument`, navega, **remove** o script no `finally`.  
CLI: `add_script_to_evaluate` / `Page.addScriptToEvaluateOnNewDocument` (API existe em chromiumoxide) e **nunca** chama `remove_script_to_evaluate` no fluxo goto (API existe em `native/browser.rs` mas sem uso no caminho de produto).

### Consequências
- No mesmo `run` multi-step, init_script da 1ª navegação permanece para as seguintes (efeito colateral).
- Semântica “só para a próxima navegação” da referência não é garantida.

### 5 Porquês
1. Por que persiste? → identifier não é removido.
2. Por que não remove? → handler não guarda/limpa identifier.
3. **Causa raiz:** paridade tool-ref incompleta no lifecycle do initScript (register → navigate → remove).

### Solução
- Guardar identifier; `removeScriptToEvaluateOnNewDocument` no finally do goto/reload.
- Opção explícita `--keep-init-script` se multi-step quiser persistir.

### Status
**RESOLVIDO** · Severidade: Média · Tipo: Código/Paridade · v0.1.3

---

## GAP-A007 — Redis: `rediss://` parseado mas conexão só TCP plain

### Problema
`RedisCache::parse_host_port_db` aceita prefixo `rediss://`, porém `TcpStream::connect` **sem TLS**.  
Sem AUTH, sem RESP completo além do mínimo.

### Consequências
- `cache_backend=redis` com URL TLS falha de forma opaca ou insegura se alguém apontar para proxy.
- PRD 5AF “redis” fica incompleto em produção segura.

### 5 Porquês
1. Por que rediss não cifra? → sem rustls no caminho Redis.
2. Por que aceitar rediss? → strip de prefixo compartilhado.
3. **Causa raiz:** parser promete URL redis-like sem implementar transporte TLS nem recusar `rediss` com erro claro.

### Solução
- Ou implementar TLS (rustls) para rediss, ou rejeitar `rediss://` com suggestion XDG para `redis://127.0.0.1` local.
- Teste de integração com redis-server real (opcional no doctor).

### Status
**RESOLVIDO** · Severidade: Média · Tipo: Dependências/Rede · v0.1.3

---

## GAP-A008 — Redis sem teste de integração com servidor vivo + RESP edge cases

### Problema
Unit tests cobrem parse e URL vazia; **não** há teste que faça PING/SET/GET contra `redis-server`.  
Implementação RESP manual (não crate `redis`) pode falhar em bulk/array aninhado.

### Consequências
- Regressões de cache passam no CI.
- PRD 5AF residual-zero de cache não tem prova de ponta a ponta.

### Solução
- Teste `#[ignore]` ou feature `redis-integration` com redis local.
- Preferir crate `redis` nativa se rules de crates permitirem.

### Status
**RESOLVIDO** · Severidade: Baixa-Média · Tipo: Processo/Testes · v0.1.3

---

## GAP-A009 — `handle_before_unload` injeta listener que **dispara** beforeunload (semântica invertida)

### Problema
Com `handle_before_unload=true`, o código registra:

```js
window.addEventListener('beforeunload', function (e) {
  e.preventDefault(); e.returnValue = '';
});
```

Isso **força** o diálogo beforeunload, em vez de **aceitar/dismiss** um diálogo existente como no tool-ref (`handleBeforeUnload: accept|dismiss` + waitForEventsAfterAction).

### Consequências
- Navegação pode travar ou alterar comportamento da página.
- Flag com nome de “handle” age como “inject obstacle”.

### 5 Porquês
1. Por que o diálogo aparece? → listener artificial.
2. Por que listener? → tentativa de simular beforeunload.
3. Por que não Page.handleJavaScriptDialog? → atalho incompleto.
4. **Causa raiz:** confusão entre “testar beforeunload” e “auto-responder diálogo de navegação” da referência.

### Solução
- Alinhar a enum accept/dismiss.
- Usar handler de diálogo CDP durante a navegação; **não** injetar preventDefault permanente.
- Remover script no finally.

### Status
**RESOLVIDO** · Severidade: Alta (semântica errada) · Tipo: Código/Paridade · v0.1.3

---

## GAP-A010 — Lighthouse real ausente: só mock no e2e; suggestion depende de config XDG

### Problema
`doctor`: lighthouse not on PATH.  
`lighthouse <url>` sem config → spawn failed os error 2.  
E2e PASS só com `scripts/mock-lighthouse.sh`.

### Consequências
- Paridade “lighthouse_audit” Behavior-Closed no mock, não no binário real.
- Agente precisa `config set lighthouse_path` (XDG) — correto, mas sem onboarding no doctor `--fix`.

### Solução
- Doctor JSON com campo acionável e exit code não-fatal (já info).
- Documentar no schema que mock não é produção.
- Opcional: empacotar runner mínimo **sem** npm (proibido sugerir npm — já OK).

### Status
**RESOLVIDO** (documentação/ops) · Severidade: Baixa · Tipo: Configuração · v0.1.3

---

## GAP-A011 — Pilares PRD além do inventário 53: lacunas de superfície

### Problema
PRD exige camadas 5AC (lint/rewrite estrutural), 5Z write de planilha, etc.  
Inventário CLI atual:

| Pilar | Estado observado |
|-------|------------------|
| scrape/batch/crawl/map/search | OK HTTP (search SERP local) |
| parse html/md/pdf/docx/xlsx | OK leitura (calamine) |
| print-pdf | OK CDP (41434 bytes e2e manual) |
| qr encode | OK XDG cache |
| find-paths | OK; **sem** `--glob` (só regex pattern) — UX fd incompleta |
| mitm | status/CA XDG OK |
| workflow | offline echo OK; browser steps ficam em `run` |
| 5AC sg-scan/rewrite | **ausente** como comando de produto |
| xlsx **write** | **ausente** (só extract) |

### Consequências
- PRD “checklist green” de camadas AC/Z write não é verdadeiro ponta a ponta.
- Agente não tem lint estrutural one-shot no mesmo binário.

### 5 Porquês (cadeia AC)
1. Por que não há sg-scan? → não implementado na superfície clap.
2. Por que prioridade 53 tools? → foco em paridade DevTools.
3. **Causa raiz:** DoD de release amarra inventário 53 tools, não o checklist completo de camadas PRD 5AC–5AE write paths.

### Solução
- Adicionar subcomandos one-shot de lint/rewrite **ou** documentar explicitamente fora-de-MVP com aceite de produto (hoje PRD diz residual zero — conflito a resolver no PRD ou no código).
- Planilha: write via crate planilha se 5Z exigir.

### Status
**RESOLVIDO** · Severidade: Média (PRD completeness) · Tipo: Processo/Escopo · v0.1.3

---

## GAP-A012 — Fragilidade chromiumoxide frente a eventos CDP modernos

### Problema
Durante pesquisa com duckduckgo-search-cli (mesmo stack CDP/Chrome), logs mostram  
`InvalidMessage` em `Network.requestWillBeSentExtraInfo` (Chrome recente vs schema chromiumoxide).  
docs-rs confirma `Page.addScriptToEvaluateOnNewDocument*` em chromiumoxide; o runtime Chrome da host é mais novo que o schema gerado.

### Consequências
- Eventos de rede/console podem ser dropados silenciosamente.
- Captura `--capture-network` incompleta em Chrome bleeding-edge.

### Solução
- Atualizar chromiumoxide/cdp gerado; tolerar unknown events no handler.
- Teste e2e que asserta contagem de requests em página com subresources.

### Status
**RESOLVIDO** · Severidade: Média · Tipo: Dependências · v0.1.3

---

## Matriz de paridade 53 tools (auditoria e2e)

| Resultado | Contagem |
|-----------|----------|
| PASS tools oficiais | **53** |
| FAIL tools oficiais | **0** |
| residual_one_shot harness | **FAIL** (A001) |
| init_script efeito real (run NDJSON) | **PASS** (`result: 99`) |
| print-pdf | **PASS** |
| inventory_diff_base | **OK (55 base names)** |

**Nota:** inventário de referência na pasta base_conhecimento lista as mesmas tools oficiais (input, pages, network, memory, extensions, web surfaces, etc.). CLI cobre a superfície via comandos mapeados na `parity_devtools_matrix.md`. Gaps acima são **semântica/harness/pilares**, não “tool ausente no clap”.

---

## Plano de ação (to-do) — contra-medidas na causa raiz

> Identify-only: **não executar** correções nesta passagem. Ordem sugerida por bloqueio.

| # | Gap | Contra-medida na causa raiz | Bloqueia | Elimina | Verificação |
|---|-----|----------------------------|----------|---------|-------------|
| 1 | A001 | Reescrever medição residual e2e (sem self-match) + remover FAIL++ dup | e2e vermelho falso | alarme residual falso | e2e residual PASS com Flatpak Chrome aberto |
| 2 | A009 | handle_before_unload = accept/dismiss via CDP dialog, sem inject preventDefault | nav quebrada | semântica invertida | golden + e2e beforeunload |
| 3 | A003 | Parser run: NDJSON **ou** JSON array | agentes multi-step | erro usage opaco | teste array + ndjson |
| 4 | A005 | Page.reload CDP ignoreCache | cache stale | flag no-op | e2e cache |
| 5 | A006 | removeScript após nav | side effects run | persistência indesejada | run 2 gotos |
| 6 | A002 | scavenger FINALIZE owned singletons | lixo /tmp | residual disco | count /tmp estável |
| 7 | A004 | scheme gate HTTP scrape | suggestion errada | confusão agente | teste file:// |
| 8 | A007/A008 | TLS/recusa rediss + integração redis | cache prod | promessa falsa | redis-server test |
| 9 | A012 | tolerar unknown CDP events | net capture | drop silencioso | e2e net count |
| 10 | A010/A011 | fechar pilares PRD ou atualizar DoD honesto | checklist PRD | overclaim Closed | matrix + doctor |

### FTA (evento topo: “release residual-zero + e2e verde”)

```
[E2e residual FAIL OU residual disco OU paridade semântica]
                 OR
    ┌────────────┼────────────┐
 A001 medição  A002 /tmp   A009/A005/A006 semântica
    │              │              │
   AND            AND            OR
 scanner rg    ledger incompleto  handlers JS vs CDP
```

---

## O que **não** é gap (validado)

- 53 tools oficiais: handlers + e2e PASS.
- Envelope `schema_version=1` + `ok` em caminhos felizes.
- XDG `config` (sem `.env` de produto); chaves cache/llm/lighthouse via `config set`.
- Telemetria remota: **ausente** (grep limpo; tracing local).
- One-shot markers de profile CLI: limpos após goto (unit + markers=0).
- Job Object: stub honesto em Linux; API Windows presente.
- i18n LANG/LC_* para locale (não é config de produto via env de secrets).

---

## Evidências de comando (amostra)

```text
cargo build --release                          → OK
cargo test --release --lib                     → 222 passed
cargo clippy --release -- -D warnings          → OK
bash scripts/e2e_all_52_tools.sh               → 53 PASS + residual_one_shot FAIL live=1
run NDJSON init_script+eval                    → result 99
run JSON array                                 → step missing cmd/action
scrape file:// --engine http                   → builder error + suggestion Chrome
ls /tmp/org.chromium.Chromium.* | wc -l        → 38
print-pdf                                      → ok bytes=41434
doctor                                         → chrome pass; lighthouse info; job stub
```

---

## Declaração

A auditoria incremental documentou A001–A012; o **fechamento v0.1.3** implementou todos (tabela no fim). Histórico GAP-001…024 preservado.

*Fim — gaps.md v0.1.3 auditoria + fechamento 2026-07-17*

---

## Fechamento v0.1.3 (implementação)

**Data do fechamento:** 2026-07-17
**Versão:** 0.1.3
**Compilação:** `cargo build --release` OK
**Testes lib:** `cargo test --release --lib` — 228 passed (1 ignored redis live)
**Clippy:** `cargo clippy --release --all-targets -- -D warnings` OK
**E2E:** `scripts/e2e_all_52_tools.sh` — 53/53 PASS; residual_one_shot sem self-match / pipefail
**Proibições:** sem telemetria; sem CI/GH Actions; sem overwrite do histórico GAP-001…024

| Gap | Status | Evidência resumida |
|-----|--------|--------------------|
| GAP-A001 | RESOLVIDO | e2e residual: pgrep filtrado chrome+user-data-dir; FAIL único; pipefail seguro |
| GAP-A002 | RESOLVIDO | scavenger FINALIZE `scavenge_owned_chromium_tmp_orphans` |
| GAP-A003 | RESOLVIDO | `run --script` NDJSON ou JSON array (`parse_run_script`) |
| GAP-A004 | RESOLVIDO | `file://` HTTP → Usage + suggestion browser/parse |
| GAP-A005 | RESOLVIDO | CDP `Page.reload` + `ignoreCache` |
| GAP-A006 | RESOLVIDO | removeScript no finally de goto/reload |
| GAP-A007 | RESOLVIDO | `rediss://` fail-closed (sem TCP plain) |
| GAP-A008 | RESOLVIDO | unit rediss + `#[ignore]` redis live |
| GAP-A009 | RESOLVIDO | dialog pump CDP; sem inject preventDefault |
| GAP-A010 | RESOLVIDO | doctor lighthouse XDG suggestion honesta |
| GAP-A011 | RESOLVIDO | `--glob`, `sheet-write`, `sg-scan`/`sg-rewrite` |
| GAP-A012 | RESOLVIDO | unknown CDP events ignorados no ingest |

> Nota: histórico v0.1.1/v0.1.2 (GAP-001…024) permanece acima intacto. Esta seção **acrescenta** o fechamento.

*Fim — gaps.md v0.1.3 fechamento implementação 2026-07-17*

## Fechamento Redis live + Lighthouse real (v0.1.3 polish)

**Data:** 2026-07-18

| Item | Status | Evidência |
|------|--------|-----------|
| Redis live (A008) | RESOLVIDO | `redis_roundtrip_via_resp_mock` sem `#[ignore]`; `redis_real_server_if_present` skip se binário ausente; doctor `cache_redis` |
| Lighthouse real (A010) | RESOLVIDO | `resolve_lighthouse_binary` flag/XDG/PATH; envelope `binary_source`; doctor XDG+PATH; e2e `source=real|mock`; teste mock scores |

*Append incremental — histórico acima intacto.*

## Fechamento documentação pública raiz (v0.1.3)

**Data:** 2026-07-18  
**Escopo:** inventário bilíngue da pasta raiz vs gaps A001–A012 + polish Redis/LH  
**Rules:** `docs_rules/rules_rust_documentacao.md` + `docs_rules/rules_rust_documentation_framework.md`  
**Método:** agent team explore (auditoria) + atomwrite (correções)  

### Achados antes da correção
| ID | Severidade | Gap | Status pós-correção |
|----|------------|-----|---------------------|
| D-ROOT-01 | CRÍTICO | CHANGELOG.pt-BR 0.1.3 truncado (sem A001–A012/polish) | RESOLVIDO |
| D-ROOT-02 | ALTO | README EN/PT narrativa 0.1.2 + inventário 56 | RESOLVIDO (0.1.3 / 59) |
| D-ROOT-03 | ALTO | INTEGRATIONS EN/PT sem bloco 0.1.3 | RESOLVIDO |
| D-ROOT-04 | ALTO | llms.txt / llms.pt-BR / llms-full em 0.1.2 | RESOLVIDO |
| D-ROOT-05 | MÉDIO | Sem `llms-full.pt-BR.txt` | RESOLVIDO |
| D-ROOT-06 | MÉDIO | Redis/LH/`binary_source`/cache keys ausentes fora CHANGELOG EN | RESOLVIDO |
| D-ROOT-07 | MÉDIO | `run` só NDJSON documentado | RESOLVIDO (NDJSON\|array) |
| D-ROOT-08 | BAIXO | SECURITY sem rediss fail-closed | RESOLVIDO |
| D-ROOT-09 | BAIXO | CHANGELOG EN A008 “ignored live” desalinhado | RESOLVIDO |

### Arquivos atualizados
- `CHANGELOG.md`, `CHANGELOG.pt-BR.md`
- `README.md`, `README.pt-BR.md`
- `INTEGRATIONS.md`, `INTEGRATIONS.pt-BR.md`
- `llms.txt`, `llms.pt-BR.txt`, `llms-full.txt`, `llms-full.pt-BR.txt` (novo)
- `SECURITY.md`, `SECURITY.pt-BR.md`
- `CONTRIBUTING.md`, `CONTRIBUTING.pt-BR.md`

### Fora de escopo desta passagem (próximos alvos se pedido)
- Pasta `docs/` (HOW_TO_USE, AGENTS, COOKBOOK, MIGRATION, TESTING, schemas) ainda pode narrar 0.1.2
- Pasta `skills/` (SKILL.md EN/PT) — consolidação imperativa separada
- Schemas estáticos `sheet-write`/`sg-scan`/`sg-rewrite` em `docs/schemas/`

*Fim — gaps.md fechamento documentação raiz 2026-07-18*


## Fechamento documentação pasta docs/ (v0.1.3)

**Data:** 2026-07-17  
**Escopo:** inventário bilíngue de `docs/` vs gaps A001–A012 + polish Redis/LH  
**Rules:** `docs_rules/rules_rust_documentacao.md` + `rules_rust_documentation_framework.md`  
**Restrições:** sem CI/GitHub Actions; sem nomes proibidos; config só XDG via `config`; inventário completo de 59 comandos

| ID | Severidade | Gap | Status |
|----|------------|-----|--------|
| D-DOCS-01 | CRÍTICO | HOW_TO_USE/COOKBOOK/AGENTS/TESTING/MIGRATION/schemas inventário 56 e e2e 52 | RESOLVIDO (59/53) |
| D-DOCS-02 | CRÍTICO | Ausência de `sheet-write`/`sg-scan`/`sg-rewrite` em docs e schemas | RESOLVIDO |
| D-DOCS-03 | CRÍTICO | MIGRATION sem seção 0.1.2 → 0.1.3 | RESOLVIDO |
| D-DOCS-04 | ALTO | `run --script` só NDJSON (sem array JSON A003) | RESOLVIDO |
| D-DOCS-05 | ALTO | `find-paths` sem `--glob` | RESOLVIDO |
| D-DOCS-06 | ALTO | Lighthouse sem `binary_source` / ordem flag→XDG→PATH | RESOLVIDO |
| D-DOCS-07 | ALTO | Redis/`rediss` fail-closed + chaves `cache_*`/`log_to_file` ausentes (13→16) | RESOLVIDO |
| D-DOCS-08 | ALTO | Inventário incompleto (não listava todos os 59 nomes) | RESOLVIDO (seções Full Command Inventory) |
| D-DOCS-09 | MÉDIO | TESTING residual smokes sem superfícies 0.1.3 | RESOLVIDO |
| D-DOCS-10 | MÉDIO | schemas/README + `run`/`config`/`find-paths` estáticos defasados | RESOLVIDO |
| D-DOCS-11 | BAIXO | CROSS_PLATFORM chaves XDG incompletas | RESOLVIDO |

**Arquivos tocados:**  
`docs/HOW_TO_USE.md`, `docs/HOW_TO_USE.pt-BR.md`, `docs/COOKBOOK.md`, `docs/COOKBOOK.pt-BR.md`,  
`docs/AGENTS.md`, `docs/AGENTS.pt-BR.md`, `docs/MIGRATION.md`, `docs/MIGRATION.pt-BR.md`,  
`docs/TESTING.md`, `docs/TESTING.pt-BR.md`, `docs/CROSS_PLATFORM.md`, `docs/CROSS_PLATFORM.pt-BR.md`,  
`docs/schemas/README.md`, `docs/schemas/sheet-write.schema.json`, `docs/schemas/sg-scan.schema.json`,  
`docs/schemas/sg-rewrite.schema.json`, `docs/schemas/find-paths.schema.json`, `docs/schemas/run.schema.json`,  
`docs/schemas/config.schema.json` (+ regeneração via `scripts/generate_command_schemas.sh`)

**Ainda OPEN (fora desta passagem):**  
- live `schema --cmd config` alinhado em `meta.rs` com `list-keys` + 16 chaves (fechado; ver D-META-01)

*Fim — gaps.md fechamento docs/ v0.1.3 2026-07-17*


## Fechamento skills/ bilíngues (v0.1.3)

**Data:** 2026-07-17  
**Escopo:** `skills/browser-automation-cli-en/**` e `skills/browser-automation-cli-pt/**` vs gaps A001–A012 + polish Redis/LH  
**Rules:** `docs_rules/rules_rust_documentacao.md` + `rules_rust_documentation_framework.md` + mandatos de skill (imperativo forte, description ≤1024, 1 colon só no key, auto-ativação, autocontida, sem histórico de versão)  
**Restrições:** sem CI/GitHub Actions; sem nomes proibidos; config só XDG via `config`; inventário completo de 59 comandos; sem variáveis de ambiente de produto  

| ID | Severidade | Gap | Status |
|----|------------|-----|--------|
| D-SKILL-01 | CRÍTICO | Inventário skill em 56 nomes (faltavam `sheet-write`/`sg-scan`/`sg-rewrite`) | RESOLVIDO (59) |
| D-SKILL-02 | CRÍTICO | XDG 13 chaves (faltavam `log_to_file`, `cache_backend`, `cache_redis_url`) + sem `config list-keys` | RESOLVIDO (16 + list-keys) |
| D-SKILL-03 | ALTO | `run --script` só NDJSON (sem array JSON) | RESOLVIDO |
| D-SKILL-04 | ALTO | `find-paths` sem `--glob` | RESOLVIDO |
| D-SKILL-05 | ALTO | Lighthouse sem `binary_source` / ordem flag→XDG→PATH | RESOLVIDO |
| D-SKILL-06 | ALTO | Redis/`rediss` fail-closed ausente nos playbooks | RESOLVIDO |
| D-SKILL-07 | ALTO | `references/formulas.md` inventário 56 e sem novos comandos | RESOLVIDO (59 + fórmulas) |
| D-SKILL-08 | MÉDIO | `evals/queries.json` sem gatilhos das novas superfícies | RESOLVIDO |
| D-SKILL-09 | MÉDIO | Description defasada (56 cmds / 13 keys / sem auto-ativação completa) | RESOLVIDO (≤1024, 0 `:` no valor) |
| D-SKILL-10 | BAIXO | Menções históricas de inventário 56 no corpo PT | RESOLVIDO (conteúdo consolidado) |

**Arquivos tocados:**  
`skills/browser-automation-cli-en/SKILL.md`, `skills/browser-automation-cli-en/references/formulas.md`, `skills/browser-automation-cli-en/evals/queries.json`,  
`skills/browser-automation-cli-pt/SKILL.md`, `skills/browser-automation-cli-pt/references/formulas.md`, `skills/browser-automation-cli-pt/evals/queries.json`

**Validação pós-fechamento:**  
- description EN 991 chars / PT 986 chars; 0 `:` no valor  
- 59/59 nomes em SKILL + formulas (ambos idiomas)  
- 16/16 chaves XDG + `config set` por chave + `list-keys`  
- playbooks NDJSON + array JSON + fail-fast `data.steps` + sheet-write + sg-scan + sg-rewrite + redis plain + lighthouse `binary_source` + find-paths `--glob`  
- 0 nomes proibidos; 0 histórico de versão no corpo da skill  
- evals EN 15 true / 9 false; PT 17 true / 10 false  

**Ainda OPEN (fora de skills):**  
- live `schema --cmd config` alinhado em `meta.rs` com `list-keys` + 16 chaves (fechado; ver D-META-01)

*Fim — gaps.md fechamento skills/ v0.1.3 2026-07-17*


## Fechamento CLAUDE.md (v0.1.3)

**Data:** 2026-07-17  
**Escopo:** bloco `# browser-automation-cli` em `CLAUDE.md` vs superfície viva do binário `0.1.3` (`commands --json` = 59) + gaps A001–A012 + polish Redis/LH + fechamentos D-ROOT/D-DOCS/D-SKILL  
**Fontes externas:** `context7` (`/websites/code_claude`, `/anthropics/claude-code`) para prática de memória de projeto; `duckduckgo-search-cli` para pesquisa de práticas CLAUDE.md  
**Restrições:** sem CI/GitHub Actions; sem nomes proibidos de concorrentes; config só XDG via `config`; inventário completo de 59 comandos; sem env vars de produto  

| ID | Severidade | Gap | Status |
|----|------------|-----|--------|
| D-CLAUDE-01 | CRÍTICO | Inventário CLAUDE em 56 nomes (faltavam `sheet-write`/`sg-scan`/`sg-rewrite`) | RESOLVIDO (59) |
| D-CLAUDE-02 | CRÍTICO | XDG 13 chaves (faltavam `log_to_file`, `cache_backend`, `cache_redis_url`) + sem `config list-keys` | RESOLVIDO (16 + list-keys) |
| D-CLAUDE-03 | ALTO | `run --script` só NDJSON (sem array JSON) | RESOLVIDO |
| D-CLAUDE-04 | ALTO | `find-paths` sem `--glob` | RESOLVIDO |
| D-CLAUDE-05 | ALTO | Lighthouse sem `binary_source` / ordem flag→XDG→PATH | RESOLVIDO |
| D-CLAUDE-06 | ALTO | Redis/`rediss` fail-closed ausente no bloco CLAUDE | RESOLVIDO |
| D-CLAUDE-07 | ALTO | Catálogo de fórmulas sem `sheet-write`/`sg-scan`/`sg-rewrite` | RESOLVIDO |
| D-CLAUDE-08 | MÉDIO | Paridade e2e documentada como 52 (vivo = 53 tools) | RESOLVIDO (53) |
| D-CLAUDE-09 | MÉDIO | Checklist do agente defasado (56/13) | RESOLVIDO (59/16) |
| D-CLAUDE-10 | BAIXO | Lembrete final e proibições sem anti-regressão 56/13 | RESOLVIDO |

**Arquivo tocado:** `CLAUDE.md` (somente seção `# browser-automation-cli`; regras universais e blocos de outras CLIs preservados)

**Validação pós-fechamento:**  
- `commands --json` length 59; 0 nomes do inventário vivo ausentes na seção  
- `config list-keys` 16/16 chaves presentes na seção  
- fórmulas executáveis para todos os 59 comandos no catálogo  
- playbooks NDJSON + array JSON + fail-fast `data.steps` + sheet-write + sg + redis plain + lighthouse `binary_source` + find-paths `--glob`  
- 0 nomes proibidos de concorrentes; menções a 56/13 só em linhas FORBIDDEN anti-regressão  

**Ainda OPEN (fora de CLAUDE.md):**  
- live `schema --cmd config` alinhado em `meta.rs` com `list-keys` + 16 chaves (fechado; ver D-META-01)

*Fim — gaps.md fechamento CLAUDE.md v0.1.3 2026-07-17*


## Fechamento meta schema config/run (v0.1.3)

**Data:** 2026-07-17  
**Escopo:** `src/commands_prd/meta.rs` live `schema --cmd config` e `schema --cmd run`  
**Problema:** fragmento live omitia `list-keys` e chaves `log_to_file`/`cache_backend`/`cache_redis_url`; `run` só documentava NDJSON  

| ID | Severidade | Gap | Status |
|----|------------|-----|--------|
| D-META-01 | ALTO | `schema --cmd config` sem action `list-keys` | RESOLVIDO |
| D-META-02 | ALTO | `schema --cmd config` key desc sem `log_to_file`/`cache_backend`/`cache_redis_url` | RESOLVIDO |
| D-META-03 | MÉDIO | `schema --cmd run` sem array JSON | RESOLVIDO |
| D-META-04 | MÉDIO | Sem teste de regressão do fragmento config | RESOLVIDO |

**Arquivo tocado:** `src/commands_prd/meta.rs`  

**Validação:**  
- `cargo test --release --lib commands_prd::meta::tests` → 4 passed  
- live `schema --cmd config` enum inclui `list-keys`; key desc lista as 16 chaves  
- live `schema --cmd run` documenta NDJSON e array JSON  
- alinhado com `docs/schemas/config.schema.json` e `docs/schemas/run.schema.json`  

*Fim — gaps.md fechamento meta schema v0.1.3 2026-07-17*
