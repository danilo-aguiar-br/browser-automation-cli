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