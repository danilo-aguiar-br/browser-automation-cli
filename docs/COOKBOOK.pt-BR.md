[English](COOKBOOK.md) | [Português Brasileiro](COOKBOOK.pt-BR.md)

# Cookbook — browser-automation-cli

> Receitas práticas com comandos prontos para copiar em trabalho browser one-shot. Ciclo de vida: BORN EXECUTE FINALIZE DIE.


## Nota de Latência
- O launch do Chrome domina o cold start em comandos com engine browser
- Prefira um script `run` a muitos launches separados quando os passos compartilham estado
- Scrape HTTP, crawl, map, search, parse, qr, image, video, find-paths, sheet-write, sg-scan e sg-rewrite evitam Chrome quando só precisa de conteúdo ou IO local
- Cada processo é BORN, EXECUTE, FINALIZE, DIE sem browser compartilhado entre invocações


## Referência de Valores Default
- Timeout global default é `0` (sem wall budget de processo salvo flag ou config XDG)
- Step timeout default é `0` (herda o timeout global)
- Headless é default salvo `--headed`
- JSON fica off salvo `--json`
- Settings de produto vêm só de flags e `config` (CLI XDG)
- Logging: `--verbose` / `--debug` / `-q` ou XDG `log_level`
- Cor: `config set color`; path do Chrome: `config set chrome_path`
- Resolva paths com `config path --json`


## Como Inicializar Config XDG
```bash
browser-automation-cli --json config init
browser-automation-cli --json config path
browser-automation-cli --json config show
browser-automation-cli --json config set timeout 60
browser-automation-cli --json config set lang en
browser-automation-cli --json config set namespace demo
browser-automation-cli --json config set artifacts_dir /tmp/browser-automation-cli-artifacts
browser-automation-cli --json config set ignore_robots false
browser-automation-cli --json config set encryption_key "replace-me-with-a-secret"
browser-automation-cli --json config set color true
browser-automation-cli --json config set log_level info
browser-automation-cli --json config set chrome_path /usr/bin/chromium
browser-automation-cli --json config set lighthouse_path ./scripts/mock-lighthouse.sh
browser-automation-cli --json config set dialog_settle_ms 2000
browser-automation-cli --json config list-keys
browser-automation-cli --json config get timeout
browser-automation-cli --json config get encryption_key
browser-automation-cli --json config get color
browser-automation-cli --json config get dialog_settle_ms
```
- `config init` cria dirs XDG e o `config.toml` default
- Descubra chaves e defaults com `config list-keys --json` (não fixe contagem; inclui `dialog_settle_ms` e mais)
- Flags sempre sobrescrevem o arquivo de config naquela invocação
- Settings de produto usam só flags e `config path|init|show|set|get|unset|list-keys`


## Como Desfazer uma Chave de Config
```bash
browser-automation-cli --json config set stealth_seed fleet-01
browser-automation-cli --json config get stealth_seed
browser-automation-cli --json config unset stealth_seed
browser-automation-cli --json config get stealth_seed
```
- `config unset <CHAVE>` restaura uma chave ao default embutido e é o inverso real de `set`
- `config set <chave> ""` não é inverso: em chave string grava um valor vazio que o caminho normal nunca produz
- Em chave numérica esse mesmo valor vazio é erro de parse, não reset
- Desfazer chave já ausente tem sucesso, então um script nunca precisa saber o estado anterior


## Como Configurar Chaves LLM no XDG
```bash
browser-automation-cli --json config set openrouter_api_key YOUR_KEY
browser-automation-cli --json config set llm_base_url https://openrouter.ai/api/v1
browser-automation-cli --json config set llm_model openai/gpt-4o-mini
browser-automation-cli --json config get openrouter_api_key
```
- Chaves ficam só no `config.toml` XDG
- `extract --llm` falha fechado quando `openrouter_api_key` está ausente


## Como Diagnosticar Saúde da Instalação
```bash
browser-automation-cli doctor --offline --quick --json
```
- Modo offline quick checa descoberta local do Chrome sem sondas de rede
- Use doctor completo sem `--quick` quando precisar de checks mais profundos
- Doctor também reporta higiene residual de disco (check `residual_disk` + topo `residual`)


## Como Verificar Higiene Residual-Zero de Disco
```bash
# Relatório residual path-light (BORN pode já ter scavenged orphans Singleton stale)
# O binário reduz o payload; nenhum processador JSON no prompt.
browser-automation-cli --json --fields residual doctor --offline --quick

# Só o veredito residual, de um envelope de 26 KB para uma linha
browser-automation-cli --json --fields checks --filter-rows 'id=residual_disk' \
  doctor --offline --quick

# Trabalho browser one-shot não deve deixar markers chrome CLI
# Nota: --url about:blank é smoke residual intencional (url presente); não é PDF em branco sem url (GAP-013)
browser-automation-cli --json print-pdf --url about:blank --path /tmp/browser-automation-cli-residual-check.pdf

# Re-cheque campos residual após DIE
browser-automation-cli doctor --offline --quick --json | jaq '.residual'
```
- Campos de topo `residual`: `scanned_roots`, `cli_marker_dirs`, `chromium_tmp_singleton_orphans`, `scavenge_safe_candidates`, `live_cli_marker_processes` (legado), `sibling_live_processes`, `orphan_marker_dirs`, `foreign_root_orphans`, `ghost_marker_processes`, `process_table_unavailable`
- Check id `residual_disk`: `fail` em `orphan_marker_dirs` ou `ghost_marker_processes`; `warn` quando restam dirs marker ou orphans Singleton; senão `pass`. Uma invocação irmã viva é saudável e nunca reprova.
- Residual-zero significa zero processos marker CLI vivos, zero dirs `browser-automation-cli-chrome-*`, zero lixo Singleton-only de Chromium tmp owned após DIE
- Age floor do GC cross-run stale é 60s; temp de Chrome Flatpak do host nunca é apagado
- Mantenedores (gates locais opcionais, só scripts locais do mantenedor):
  - `bash scripts/residual-check.sh`
  - `bash scripts/residual-stress.sh`


## Como Encolher o Envelope com --fields
```bash
# Envelope completo do doctor mede 26277 bytes neste host
browser-automation-cli --json doctor --offline --quick

# Um único caminho pontuado leva a mesma resposta a 80 bytes
browser-automation-cli --json --fields residual.ghost_marker_processes doctor --offline --quick

# Caminhos são relativos a data, então metadata resolve e data.metadata não
browser-automation-cli --json --fields metadata scrape https://example.com --format metadata --engine http

# Caminho que não resolve é reportado, nunca descartado em silêncio
browser-automation-cli --json --fields residual.nao_existe doctor --offline --quick
```
- `--fields` recebe um CSV de caminhos pontuados e não é repetível
- Caminhos partem de `data`, então escreva `residual` e nunca `data.residual`
- A projeção reconstrói o aninhamento que cada caminho implica
- Caminho não resolvido cai em `agent_ops.unresolved_paths`, com `flag` e `path`
- A redução acontece dentro do binário, sem processador de JSON externo


## Como Contar Linhas com --count-only
```bash
# Substitui todo o payload de linhas por uma contagem única
browser-automation-cli --json --fields checks --count-only doctor --offline --quick

# Conta apenas as linhas que o filtro mantém
browser-automation-cli --json --fields checks --filter-rows status=info --count-only \
  doctor --offline --quick
```
- `--count-only` emite `{"count": N}` no lugar das linhas
- `agent_ops.total` e `agent_ops.matched` seguem reportando a aritmética do filtro
- Use para dimensionar o resultado antes de pagar pelas linhas


## Como Ordenar, Limitar e Deduplicar Linhas
```bash
# Ordem determinística mais teto rígido de linhas
browser-automation-cli --json --fields checks --sort-rows id --limit-rows 3 \
  --truncate-content 24 doctor --offline --quick

# Mantém a primeira linha de cada status distinto
browser-automation-cli --json --fields checks --dedupe-by status \
  --truncate-content 24 doctor --offline --quick

# Estreita para uma linha por id, depois limita
browser-automation-cli --json --fields checks --filter-rows id=residual_disk --limit-rows 1 \
  doctor --offline --quick
```
- `--sort-rows` recebe um caminho pontuado e compara números numericamente
- `--limit-rows` aplica depois de filtro, dedupe e ordenação
- `--dedupe-by` mantém a primeira linha de cada valor distinto
- `--filter-rows` aceita `key=value`, `key!=value` e `key~substring`
- Campo ausente nunca casa, então ausência não é diferença sob `!=`


## Como Limitar Payload com --truncate-content e --max-output-bytes
```bash
# Corta toda string do payload em 24 caracteres
browser-automation-cli --json --fields checks --filter-rows id=chrome --truncate-content 24 \
  doctor --offline --quick

# Teto rígido de bytes; linhas são descartadas a partir do fim
browser-automation-cli --json --fields checks --max-output-bytes 400 doctor --offline --quick
```
- `--truncate-content N` corta strings e marca `agent_ops.truncated` como true
- `--max-output-bytes` descarta linhas inteiras e reporta `agent_ops.omitted_rows`
- As duas flags são globais e valem em qualquer comando que emite JSON
- Combine com `--fields` quando uma projeção ainda ficar grande demais


## Como Fixar Uma Identidade Stealth Entre Processos
```bash
# Sem semente, cada um destes é uma máquina diferente para a outra ponta
browser-automation-cli --timeout 60 --json goto https://example.com

# Com semente, a frota inteira de processos one-shot parece um browser só
browser-automation-cli --timeout 60 --json --stealth-seed fleet-01 goto https://example.com
browser-automation-cli --timeout 60 --json --stealth-seed fleet-01 scrape https://example.com --format text

# Torne durável em vez de repetir a flag
browser-automation-cli --json config set stealth_seed fleet-01
browser-automation-cli --json config set stealth_profile chrome-linux
```
- Stealth é LIGADO por padrão e mascara os marcadores de automação que um Chrome real nunca expõe
- `--stealth-profile` aceita `auto`, `chrome-linux`, `chrome-win`, `chrome-mac`, e `auto` segue o host
- Liste os tokens pelo binário: `browser-automation-cli --json --stealth-profile list version` ou `commands --json`
- Sem `--stealth-seed` cada execução sorteia identidade nova, então um crawl de 50 URLs se apresenta como 50 máquinas
- `--stealth-seed` fixa `hardwareConcurrency`, `deviceMemory`, vendor/renderer da GPU, `history.length` e o build do Chrome. Não varia User-Agent, `navigator.platform`, idiomas, fuso, tela nem `plugins.length`
- O launch aplica métricas 1920×1080 para `screen` não ficar no padrão headless 800×600. `resize` / `emulate --viewport` também definem `screen`; passe `--screen 1920x1080`, um passo de `run` `"screen":"2560x1440"` ou `config set screen 2560x1440`
- O envelope responde `screen_source` ao lado de `screen`, e os tokens são `argv`, `step`, `xdg`, `derived` e `floor`
- `floor` significa que existia um override explícito e o piso do viewport era maior, então o número devolvido é o piso e não o que você pediu
- Leia `screen_source` antes de confiar em `screen`: quem lê só o número não distingue um pedido que sobreviveu de um que o piso sobrepôs
- `doctor --fingerprint` (sem `--quick`) pontua a página ao vivo e falha se ela contradisser o plano. `--quick` pontua só a identidade planejada
- As chaves XDG são `stealth` (`true`), `stealth_profile` (`auto`), `stealth_seed` (sem padrão)
- `browser_mode` (`auto`) é `auto|headed|headless`; `auto` resolve para headless e o `doctor` reporta o modo efetivo
- Desligue os patches nesta execução com `--no-stealth` quando estiver testando seu próprio front end

## Como Escrever um Arquivo `run --script` que o Agente Consiga Parsear

Cada passo é um objeto JSON completo em uma única linha física. Um `printf` com aspas simples esmaga as aspas do JavaScript dentro de `eval` e a página reporta `SyntaxError: Invalid regular expression flags`. Quebrar a expressão em várias linhas é NDJSON inválido (`EOF while parsing a string`).

```bash
SCRIPT="$(mktemp)"
cat > "$SCRIPT" <<'EOF'
{"cmd":"goto","url":"about:blank"}
{"cmd":"eval","expression":"(/hello/i).test(\"hello\")"}
EOF
browser-automation-cli --json -q run --script "$SCRIPT"
```


## Como Sair por um Proxy de Saída
```bash
browser-automation-cli --json --proxy socks5://127.0.0.1:1080 \
  scrape https://example.com --format text --engine http

browser-automation-cli --timeout 60 --json --proxy http://127.0.0.1:8888 \
  --proxy-bypass '127.0.0.1,localhost' goto https://example.com

# Credenciais pertencem ao XDG, nunca ao argv
browser-automation-cli --json config set proxy_url http://127.0.0.1:8888
browser-automation-cli --json config set proxy_username agent
browser-automation-cli --json config set proxy_password secret
```
- `--proxy` aceita `http`, `https` e `socks5`, e vale igualmente para o Chrome e para o motor HTTP
- `--proxy-bypass` usa a sintaxe de bypass-list do Chrome
- As chaves XDG são `proxy_url`, `proxy_bypass`, `proxy_username`, `proxy_password`
- Guarde as credenciais somente no XDG, porque argv aparece na tabela de processos
- `cdp_proxy_bypass_loopback` (`true`) sempre ignora loopback para o canal de controle CDP sobreviver ao proxy
- `robots_user_agent` define o token de user-agent contra o qual as regras do robots.txt são casadas


## Como Modelar o Input Como Humano
```bash
# Cinemática humana reprodutível
browser-automation-cli --timeout 60 --json --input-profile human --input-seed 42 \
  goto https://example.com

# Um evento por ação, exatamente determinístico
browser-automation-cli --timeout 60 --json --input-profile direct goto https://example.com

browser-automation-cli --json config set input_profile human
```
- `human` é o padrão e interpola trajetórias do ponteiro, aplica dwell entre press e release e ritma a digitação
- `--input-seed` semeia o jitter para que uma execução `human` reproduza exatamente
- Medido em 2026-09-04 nesta árvore: o custo do ritmo `human` cresce de forma superlinear com o tamanho digitado, em 2281 ms para 1 caractere, 14236 ms para 2 e 95781 ms para 4
- Um `type` longo sob `human` pode então esgotar o `--timeout` e devolver exit 124, e é por isso que a receita `direct` acima não trata apenas de determinismo
- Este é um defeito ABERTO registrado no `gaps.md`, então recorra a `--input-profile direct` em campos longos em vez de aumentar o `--timeout` até caber
- Chaves de cinemática: `input_move_steps` (`24`), `input_move_gap_ms` (`12`), `input_click_dwell_ms` (`65`)
- Chaves de cinemática: `input_key_dwell_ms` (`45`), `input_type_delay_ms` (`95`), `input_target_jitter_px` (`3`)
- Chaves de scroll: `input_scroll_tick_px` (`100`), `input_scroll_max_ticks` (`40`), `input_scroll_settle_rounds` (`3`)


## Como Aquecer a Sessão Antes da URL Alvo
```bash
# Cai primeiro na raiz da origem, depois na URL profunda
browser-automation-cli --timeout 60 --json --warmup goto https://example.com/deep/page

# Aqueça o ponto de entrada real quando a borda entrega a sessão em outro lugar
browser-automation-cli --timeout 60 --json --warmup-url https://example.com/login \
  goto https://example.com/app

# Headed no Linux sem o display virtual privado
browser-automation-cli --timeout 60 --json --headed --no-xvfb goto https://example.com
```
- `--warmup` dá à sessão cookies e cadeia de referrer antes da requisição alvo
- `--warmup-url` implica `--warmup`, então passá-la sozinha basta
- `--no-xvfb` só faz sentido em modo headed no Linux


## Como Manter o Fingerprint HTTP/2 Constante
```bash
browser-automation-cli --json config set http2_enabled true
browser-automation-cli --json config set http2_initial_stream_window_size 6291456
browser-automation-cli --json config set http2_initial_connection_window_size 15663105
browser-automation-cli --json config set http2_max_header_list_size 262144
browser-automation-cli --json config set http2_max_frame_size 16384
browser-automation-cli --json config set http2_adaptive_window false
```
- `http2_enabled` (`true`) negocia HTTP/2 no cliente HTTP compartilhado
- As quatro chaves de janela e tamanho carregam os defaults mostrados acima
- `http2_adaptive_window` (`false`) fica desligado para manter o fingerprint constante


## Como Afirmar Sobre o Payload Emitido
```bash
# Só reporta: o exit continua 0 e agent_ops.expectation_unmet lista as falhas
browser-automation-cli --json --expect 'ok=true' doctor --offline --quick

# Opte por reprovar a execução
browser-automation-cli --json --expect 'ok=true' --expect-exit-code doctor --offline --quick
```
- `--expect` aceita `key=value`, `key!=value` e `key~substring`, é repetível e conjuga tudo por AND
- `--expect-exit-code` sai com `65` quando alguma expectativa falha
- Ela fica desligada por padrão porque mudar exit code por conteúdo de dado quebraria em silêncio os chamadores que já ramificam nele


## Como Abrir uma Página e Fazer Snapshot
```bash
browser-automation-cli --timeout 60 --json goto https://example.com

cat > /tmp/goto-view.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"view"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/goto-view.browser-automation.jsonl
```
- `goto` standalone navega e encerra o processo
- Use `run` para o `view` ver a mesma página em um lifecycle
- Snapshot de acessibilidade emite refs `@eN` para passos posteriores de press e write


## Como Clicar e Preencher em Um Processo
```bash
cat > /tmp/form.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"eval","expression":"document.body.insertAdjacentHTML('beforeend','<form><input name=q><button type=button>Go</button></form>');'ok'"}
{"cmd":"view"}
{"cmd":"write","target":"input","value":"hello"}
{"cmd":"press","target":"button"}
JSONL
browser-automation-cli --timeout 90 --json run --script /tmp/form.browser-automation.jsonl
```
- O formulário é injetado para a receita ser autossuficiente numa página que não tem nenhum
- Numa página real esse passo `eval` não existe
- Mantenha click e fill no mesmo processo para seletores e refs `@eN` permanecerem válidos
- Launches separados não compartilham refs de acessibilidade


## Como Scrollar e Assertar em um Script Run
```bash
cat > /tmp/scroll-assert.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"scroll","dy":1500}
{"cmd":"assert","url_contains":"example.com"}
{"cmd":"assert","text_contains":"Example Domain"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/scroll-assert.browser-automation.jsonl
```
- `dy` / `dx` são aliases de `delta_y` / `delta_x`
- `url_contains` / `text_contains` são aliases de assert
- `kind` é a segunda grafia de `action`, e `{"cmd":"assert","kind":"step",...}` afirma sobre o envelope de um step ANTERIOR por caminho JSON
- Essa forma lê `path` mais um entre `equals`, `contains` ou `exists`: `{"cmd":"assert","kind":"step","path":"result","equals":"OK"}` e `{"cmd":"assert","kind":"step","path":"console_count","exists":true}`
- Em fail-fast, o envelope de erro pode incluir `data.steps` parcial


## Como Capturar Screenshot Full-page
```bash
cat > /tmp/grab.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"grab","path":"/tmp/page.png","full_page":true}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/grab.browser-automation.jsonl

# As mesmas flags no subcomando grab após um passo anterior no mesmo processo:
# browser-automation-cli --timeout 60 --json grab --path /tmp/page.png --full-page
```
- Path é a flag `--path`, não argumento posicional
- `full_page` no NDJSON mapeia para `--full-page` na CLI


## Como Imprimir uma Página em PDF
```bash
browser-automation-cli --json print-pdf --url https://example.com --path /tmp/page.pdf

# Dentro de multi-passo run (GAP-001 / GAP-017)
cat > /tmp/pdf.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"print-pdf","path":"/tmp/page-from-run.pdf"}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/pdf.run.json
```
- Usa CDP `Page.printToPDF` em processo one-shot
- Passe `--url` para navegar antes do print, ou imprima a página atual dentro de um script `run` após `goto`
- PDF em about:blank vazio é recusado sem conteúdo navegado ou `url` no step/CLI (GAP-013); navegue com `goto` antes (não use `allow_empty` de `view` aqui)
- `landscape` é chave EXCLUSIVA de step: `{"cmd":"print-pdf","landscape":true}` chega ao CDP `Page.printToPDF` e gira a página, e o padrão é retrato
- O SUBCOMANDO `print-pdf` não tem flag `--landscape`, então a impressão girada só é alcançável de dentro de um script `run`
- O step aceita também `init_script` e `navigation_timeout_ms`, que valem para a navegação que ele faz quando há `url`, e são ignorados quando ele imprime a página já aberta


## Como Monitorar Mudança de Página Contra Baseline
```bash
browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/mon.base --write-baseline
browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/mon.base
```
- Primeira chamada com `--write-baseline` grava o hash/texto baseline
- Chamadas posteriores comparam com o arquivo baseline sem gravar salvo nova solicitação


## Como Esperar Multi-texto (OR)
```bash
cat > /tmp/wait-or.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","text":["Example Domain","Example"],"ms":5000}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/wait-or.browser-automation.jsonl

# Forma CLI com --text repetível (semântica OR):
# browser-automation-cli --timeout 60 --json wait --text "Example Domain" --text "Example" --ms 5000
```
- `--text` repetível resolve quando qualquer valor listado aparece
- Combine com `ms`, `selector`/`selectors`, `url`/`url_contains`/`navigation` ou `state` conforme necessário


## Como Esperar Multi-seletor ou URL (v0.1.4)
```bash
cat > /tmp/wait-multi.browser-automation.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"wait","selector":"h1, body","ms":5000},
  {"cmd":"wait","url_contains":"example.com","ms":5000},
  {"cmd":"wait","url":"https://example.com/","ms":5000},
  {"cmd":"wait","navigation":true,"ms":5000}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/wait-multi.browser-automation.json

# Multi-seletor CSS OR na forma CLI:
browser-automation-cli --timeout 60 --json wait --selector 'h1, body' --ms 5000
```
- Multi-seletor CSS OR: `#a, #b` ou arrays `selectors` no run
- Campos run: `url` (exato), `url_contains`, `navigation: true` (ciclo de load booleano — não string `"load"`)
- Espera multi-seletor bem-sucedida pode incluir `matched_selector` nos dados de resultado
- Ainda combina com multi-texto OR e `ms`


## Como Streamar Passos com --json-steps
```bash
cat > /tmp/steps.array.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"wait","ms":200},
  {"cmd":"view"}
]
JSON
browser-automation-cli --timeout 60 --json --json-steps run --script /tmp/steps.array.json
```
- Global `--json-steps` streama uma linha NDJSON por passo: `step`, `cmd`, `ok`, `result`
- Envelope final `--json` ainda inclui `ok` e `steps[].data` completo
- Útil para feedback progressivo de agente sem re-spawnar Chrome


## Como Fazer Pick e Select-option no Run
```bash
cat > /tmp/pick.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"pick","target":"[role=combobox]","option":"Option label"},
  {"cmd":"select-option","target":"select#country","option":"BR"}
]
JSON
# browser-automation-cli --timeout 90 --json run --script /tmp/pick.run.json
browser-automation-cli --json schema select-option
browser-automation-cli --json schema pick
```
- `pick` / `select-option` são inventário de agente + run/exec/schema (não subcomandos clap standalone)
- Exigem `target` (trigger) e `option` (texto, seletor ou label de role)
- Em `<select>` nativo, a CLI despacha `input` e depois `change` e reporta `via: native_select` (GAP-055)
- Descubra argv com `schema pick` ou `schema select-option`


## Como Aceitar Diálogo e Continuar (dialog_settled)
```bash
cat > /tmp/dialog-settled.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"dialog","action":"accept"},
  {"cmd":"view"}
]
JSON
# browser-automation-cli --timeout 60 --json run --script /tmp/dialog-settled.run.json \
#   | jaq '.data.steps[] | select(.cmd=="dialog") | .data.dialog_settled'

browser-automation-cli --json config set dialog_settle_ms 2000
```
- Após accept/dismiss real, o envelope de dados inclui o booleano `dialog_settled` (GAP-054)
- Happy path é `true` quando `Page.javascriptDialogClosed` foi observado — **não** invente wait antes do próximo passo de página
- Caminho soft: `dialog accept --if-present` quando o diálogo pode estar ausente


## Como Isolar Diálogos Entre Abas (multi-aba)
```bash
cat > /tmp/dialog-multitab.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"page","action":"new","url":"https://example.org"},
  {"cmd":"page","action":"select","index":0},
  {"cmd":"dialog","action":"accept","if_present":true},
  {"cmd":"page","action":"select","index":1},
  {"cmd":"view"}
]
JSON
# browser-automation-cli --timeout 90 --json run --script /tmp/dialog-multitab.run.json
```
- Diálogos são chaveados por `session_id` CDP (forwarders de página carimbam `Page::session_id`)
- Responder um diálogo em uma aba não rouba a entrada do mapa de outra aba
- Enable de domínio em `tab_switch` é best-effort sob orçamento de diálogo modal
- `page select` aceita `index`, que é 0-based e tem `page_id` e `pageId` como aliases, ou `tab_id`, que é 1-BASED porque é o número que o `page list` imprime
- `index` vence quando os dois vêm juntos, e `tab_id: 0` é recusado em vez de lido em silêncio como a primeira aba


## Como Esperar com wait_timeout_ms em Run
```bash
cat > /tmp/wait-timeout.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"wait","selector":"h1","wait_timeout_ms":2000},
  {"cmd":"wait","text":["Example Domain"],"wait_timeout_ms":5000}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/wait-timeout.run.json
```
- A chave pública de prazo é `wait_timeout_ms` (GAP-053); o parser de run a honra (não é descarte silencioso)
- Também válido na CLI: `wait --selector h1 --wait-timeout-ms 2000`


## Como Fazer Scrape com format text Dentro de Run
```bash
cat > /tmp/scrape-text.run.json <<'JSON'
[
  {"cmd":"scrape","url":"https://example.com","format":"text"}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/scrape-text.run.json
```
- Passos de run aceitam `format` / `formats` (GAP-057) com a mesma forma do `scrape` de topo
- Pedir só `text` não deve despejar um campo `html` grande no resultado do passo
- Um passo de run RECUSA `engine` com `kind: usage`: a sessão já está viva, então o engine foi fixado no lançamento e nenhum passo o move
- Para um fetch HTTP one-shot sem sessão, use o `scrape <url> --engine http` de topo


## Como Capturar grab em webp (não avif)
```bash
cat > /tmp/grab-webp.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"grab","path":"/tmp/page.webp","format":"webp"}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/grab-webp.run.json

# CLI: grab --path /tmp/page.webp --format webp
# Formatos de encode: png | jpeg | webp apenas. AVIF foi removido na v0.1.6.
```


## Como Enviar um Formulário (submit)
```bash
# Mire o <form> em si ou qualquer campo dentro dele
# browser-automation-cli --timeout 60 --json submit "form#login" --timeout-ms 10000

cat > /tmp/submit.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"write","target":"input[name=q]","value":"hello"},
  {"cmd":"submit","target":"form","timeout_ms":10000}
]
JSON
# browser-automation-cli --timeout 90 --json run --script /tmp/submit.run.json
```
- `submit` espera navegação ou requisição concluída após o envio do formulário
- Descubra argv com `schema submit --json`


## Como Exportar e Importar Storage
```bash
# Exporta cookies + localStorage + sessionStorage para path explícito (mode 0600)
# browser-automation-cli --timeout 60 --json storage export --path /tmp/auth-state.json --url https://example.com

# Importa estado de auth portátil e navega para aplicar o estado restaurado
# browser-automation-cli --timeout 60 --json storage import --path /tmp/auth-state.json --url https://example.com
```
- Path é sempre explícito (`--path`); nunca um default XDG implícito
- `--url` opcional navega antes (export) ou depois da restauração (import)
- Descubra argv com `schema storage --json`


## Como Assertar Console Vazio ou Sem Match
```bash
cat > /tmp/assert-console.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"assert","kind":"console_empty"},
  {"cmd":"assert","kind":"console_no_match","pattern":"TypeError"}
]
JSON
browser-automation-cli --capture-console --timeout 60 --json run --script /tmp/assert-console.run.json

# Formas CLI (GAP-025):
# browser-automation-cli --capture-console --json assert console-empty
# browser-automation-cli --capture-console --json assert console-no-match --pattern TypeError
```
- Exige `--capture-console` no mesmo processo
- Kinds no run: `console_empty` / `console_no_match`; CLI: `console-empty` / `console-no-match`


## Como Usar Schema Posicional
```bash
browser-automation-cli --json schema run
browser-automation-cli --json schema wait
browser-automation-cli --json schema --cmd assert
```
- `schema <cmd>` posicional e `schema --cmd <cmd>` são ambos válidos (GAP-022)
- Prefira posicional para UX de agente


## Como View com --allow-empty
```bash
browser-automation-cli --json view --allow-empty

cat > /tmp/view-empty.run.json <<'JSON'
[
  {"cmd":"view","allow_empty":true}
]
JSON
browser-automation-cli --timeout 30 --json run --script /tmp/view-empty.run.json
```
- about:blank vazio recusa sucesso silencioso salvo `--allow-empty` / `allow_empty:true` (GAP-012)
- Prefira navegar com `goto` antes de `view` em fluxos normais


## Como Handle Beforeunload (GAP-003)
```bash
# Aceite ou descarte beforeunload durante a navegação
browser-automation-cli --timeout 60 --json goto https://example.com --handle-before-unload accept
browser-automation-cli --timeout 60 --json goto https://example.com --handle-before-unload dismiss
browser-automation-cli --timeout 60 --json reload --handle-before-unload accept
browser-automation-cli --timeout 60 --json reload --ignore-cache --handle-before-unload dismiss

# Campo handle_before_unload no passo de run
cat > /tmp/beforeunload.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com","handle_before_unload":"accept"},
  {"cmd":"reload","ignore_cache":true,"handle_before_unload":"dismiss"}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/beforeunload.run.json
```
- Valores: `accept` ou `dismiss` (CLI `--handle-before-unload`; campo de run `handle_before_unload`)
- Arma o auto-accept ou auto-dismiss de diálogo do CDP só naquela navegação
- As opções de goto também incluem `--init-script` e `--navigation-timeout-ms`


## Como Abrir Contexto Isolado (GAP-004)
```bash
# Flag sozinha vira default-isolated; nome opcional depois da flag
browser-automation-cli --timeout 60 --json page new --isolated-context
browser-automation-cli --timeout 60 --json page new --isolated-context my-ctx --url https://example.com

# Run: isolated_context como string ou true
cat > /tmp/page-iso.run.json <<'JSON'
[
  {"cmd":"page","action":"new","isolated_context":true},
  {"cmd":"page","action":"new","isolated_context":"agent-ctx","url":"https://example.com"}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/page-iso.run.json
```
- `page new --isolated-context` sem valor usa `default-isolated`
- O run aceita `isolated_context: true` (vira `default-isolated`) ou uma string nomeada
- Contexto compartilhado quando o campo ou a flag é omitido


## Como Usar fill-form no Run
```bash
cat > /tmp/fill-form.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"fill-form","fields":[{"target":"input","value":"hello"},{"target":"textarea","value":"world"}]}
]
JSON
# browser-automation-cli --timeout 90 --json run --script /tmp/fill-form.run.json

# Forma CLI (fields JSON via fill-form --fields-json; o --json global é só envelope):
# browser-automation-cli --json fill-form --fields-json '[{"target":"input","value":"hello"}]'
```
- O run aceita array `fields` (ou string/array `json`) de `{target|uid|selector|ref, value|text}`
- Prefira um processo só com `goto` para os seletores seguirem válidos


## Como Fazer console dump com Array Vazio (GAP-021)
```bash
browser-automation-cli --capture-console --json console dump --path /tmp/console.json
# Sempre um array JSON válido — [] quando vazio
jaq -e 'type == "array"' /tmp/console.json
```
- `console dump` sempre grava um array JSON válido (`[]` quando vazio)
- Habilite `--capture-console` no mesmo processo que produz mensagens para obter dumps não vazios


## Como Listar Requests de Network
```bash
cat > /tmp/nav.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":400}
{"cmd":"net","action":"list"}
JSONL
browser-automation-cli --capture-network --timeout 60 --json run --script /tmp/nav.jsonl
```
- O buffer de captura registra `resourceType` ao lado de `requestId`, `method` e `url`, então `resource_types` seleciona de verdade
- Requisição que o protocolo envia sem tipo é gravada como `Other`, valor real do CDP, então ela continua selecionável em vez de sumir
- Tipo fora do vocabulário do CDP é recusado com exit 2 antes de qualquer browser subir, então lista vazia significa que a página não tinha aquele recurso
- `dropped_oldest` conta as entradas que o teto do anel descartou, então resposta truncada diz isso em vez de passar um subconjunto por conjunto inteiro; o teto é a chave XDG `event_tracker_max_entries`
- Crie o arquivo de script na receita antes do `run`
- Capture deve estar habilitado no mesmo processo que navega
- `net list` após processo separado não vê captura anterior


## Como Avaliar JavaScript
```bash
cat > /tmp/eval.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"eval","expression":"document.title"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/eval.browser-automation.jsonl

# eval isolado roda contra about:blank salvo se você já navegou no mesmo processo
# browser-automation-cli --json eval 'document.title'
```
- Prefira `run` quando a expressão depende do conteúdo da página
- A expressão pode ser valor simples ou declaração de função `() => ...`


## Como Emular Viewport Mobile e Rede
```bash
cat > /tmp/emulate.browser-automation.jsonl <<'JSONL'
{"cmd":"emulate","user_agent":"Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X)","viewport":"390x844x3,mobile,touch","network_conditions":"Slow 3G"}
{"cmd":"goto","url":"https://example.com"}
{"cmd":"resize","width":390,"height":844}
{"cmd":"view"}
JSONL
browser-automation-cli --stealth-profile chrome-mac --timeout 90 --json run --script /tmp/emulate.browser-automation.jsonl

# Composição isolada (não existe flag de preset --device):
# browser-automation-cli --json emulate \
#   --user-agent "Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X)" \
#   --viewport "390x844x3,mobile,touch" \
#   --network-conditions "Slow 3G"
```
- O stealth é LIGADO por padrão e recusa um `--user-agent` cuja família de plataforma contradiga o perfil resolvido
- `--no-stealth` NÃO contorna essa recusa, porque ele resolve o perfil do host em vez de desligar a checagem
- Um user agent de iPhone é da família Apple, então só `--stealth-profile chrome-mac` é coerente com ele
- Não existe flag de preset `--device`
- Compose user agent, viewport e condições de rede você mesmo
- Presets de rede incluem Offline, No throttling, Slow 3G, Fast 3G, Slow 4G, Fast 4G


## Como Fazer Scrape com Markdown via HTTP
```bash
browser-automation-cli --json scrape https://example.com --format markdown --engine http
# CLEAN STDOUT agent-native (sem jq):
browser-automation-cli --json scrape https://example.com --engine http \
  --format markdown --only-main-content \
  --select source_url,title,markdown,status_code --max-text-chars 8000
```
- Formatos: `text`, `markdown`, `html`, `links`, `metadata`, `summary`, `product`, `branding`, `raw-html`, `screenshot`, `images`
- Engine `http` usa reqwest e pula o Chrome (prefira `http` quando HTML estático bastar)
- `--select` projeta campos no binário; `--max-text-chars` limita text/markdown/html (XDG `scrape_max_text_chars` como padrão)
- Superfície local one-shot scraping-oriented — **não** é a hosted scraping SaaS (sem SaaS de CAPTCHA nem de proxy)

## Como Mapear com Sitemap e Filtros de Path
```bash
browser-automation-cli --json map https://example.com --limit 20 --use-sitemap \
  --include-path /docs --exclude-path /admin --select urls,count
browser-automation-cli --json crawl https://example.com --limit 10 --format markdown \
  --filter http_error=false --select source_url,title,markdown --output-mode json
```
- `--use-sitemap` default segue XDG `scrape_use_sitemap` (true)
- `--filter` é AND `key=value` / `key!=value` em pages
- `--output-mode ndjson` emite um objeto por linha



## Como Ler Um Sitemap Declarado Ou Um Feed
```bash
browser-automation-cli --json sitemap https://example.com --limit 200 \
  --include-path /docs --exclude-path /admin --select urls,count
browser-automation-cli --json feed https://example.com/feed.xml --select title,entries
```
- O `sitemap` é o `map --sitemap-only` com a flag fixa: ele lê o documento DECLARADO, as dicas `Sitemap:` do `robots.txt` e o `sitemapindex` aninhado, e nunca percorre o grafo de links
- Não existe `--depth` no `sitemap`, porque lista declarada não é fronteira e não há grafo de links a limitar
- O `feed` parseia RSS, Atom e JSON Feed a partir do corpo BRUTO pelo motor HTTP; aceita `--select`, `--header` e `--no-cache`
- As flags que moldam HTML estão ausentes do `feed` em vez de ignoradas: um seletor destruiria um documento XML ou JSON
- O Chrome não é oferecido para o `feed`, porque renderizar um feed produz o visualizador XML do navegador em vez do feed

## Como Scrape Multi-formato (GAP-009)
```bash
browser-automation-cli --json scrape https://example.com --format markdown,html,links --engine http
browser-automation-cli --timeout 60 --json scrape https://example.com --format markdown --format links --engine browser
browser-automation-cli --json scrape https://example.com --formats markdown,links --engine http
```
- CSV ou `--format` repetível devolve vários campos de format em uma invocação (GAP-009)
- O alias `--formats` é aceito onde há suporte (GAP-018)
- Envelope inclui saída por format quando mais de um format é pedido


## Como Ler metadata Expandido
```bash
# Página que declara tags Open Graph e Twitter card
browser-automation-cli --json --fields metadata \
  scrape https://blog.rust-lang.org/2024/02/08/Rust-1.76.0/ --format metadata --engine http

# Página enxuta: chaves não declaradas ficam ausentes, não null
browser-automation-cli --json --fields metadata scrape https://example.com \
  --format metadata --engine http
```
- `metadata` colhe Open Graph, Dublin Core, `article:`, Twitter card, canonical, favicon, charset e `html_lang`
- As chaves saem achatadas como `og_title`, `dc_creator`, `article_published_time`, `twitter_card`
- Campo que a página não declara é omitido, nunca emitido como null
- Teste presença de chave, jamais comparação com null


## Como Escolher Entre rawHtml e html
```bash
# rawHtml devolve o documento exatamente como veio
browser-automation-cli --json --fields rawHtml --truncate-content 90 \
  scrape https://docs.rs/serde/latest/serde/ --format rawHtml --engine http

# html devolve o corpo após extração de conteúdo principal e filtros de seletor
browser-automation-cli --json --fields html --truncate-content 90 \
  scrape https://docs.rs/serde/latest/serde/ --format html --engine http --only-main-content
```
- `rawHtml` cai sob a chave `rawHtml` e começa no doctype
- `html` cai sob a chave `html` e começa na raiz extraída
- Nessa página de docs o documento cru tem 25628 chars contra 8185 extraídos
- Eles não são mais alias um do outro, então escolha o que você quer
- Use `rawHtml` para fidelidade e `html` para conteúdo que você vai reprocessar


## Como Fazer Scrape com Engine Browser e Formatos
```bash
browser-automation-cli --timeout 60 --json scrape https://example.com --format markdown --engine browser
browser-automation-cli --timeout 60 --json scrape https://example.com --format links --engine browser
```
- Engine `browser` usa CDP via Chrome
- A engine browser captura `outerHTML` e aplica `--format` (markdown/html/links/metadata/…)
- Use browser quando o conteúdo precisa de renderização JS


## Como Enviar Resultado de Scrape a um Webhook do Operador
```bash
browser-automation-cli --json scrape https://example.com --format markdown --engine http \
  --webhook-url https://127.0.0.1:9000/hook
```
- `--webhook-url` é um POST one-shot do operador com os dados do resultado do scrape
- `crawl` e `batch-scrape` aceitam a mesma flag para o envelope da coleção
- `crawl --include-regex` / `--exclude-regex` filtram path ou URL (regex inválida é usage, exit 2)
- Não é telemetria de produto; o destino fica sob controle do operador


## Como Fazer Batch-scrape a Partir de Arquivo de URLs
```bash
cat > /tmp/urls.txt <<'URLS'
# uma URL por linha
https://example.com
https://example.org
URLS
browser-automation-cli --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2
browser-automation-cli --timeout 120 --json batch-scrape --urls-file /tmp/urls.txt --format markdown --engine browser --concurrency 1
```
- Default de `batch-scrape` é `--engine http`; em 0.1.4 aceita `--engine browser` (GAP-010)
- Crie o arquivo de URLs antes de invocar o comando


## Como Fazer Crawl com Same-host
```bash
browser-automation-cli --json crawl https://example.com --limit 20 --max-depth 2 --format text --same-host
browser-automation-cli --timeout 120 --json crawl https://example.com --limit 5 --max-depth 1 --engine browser --same-host
```
- `--same-host` é flag booleana sem valor
- Não escreva `--same-host true`
- Default de crawl é HTTP BFS; `--engine browser` usa CDP por página (GAP-010)
- Com `--same-host` permanece no host da semente


## Como Mapear um Site
```bash
browser-automation-cli --json map https://example.com --limit 50 --max-depth 2
```
- Map descobre URLs a partir de uma semente sem extração completa de página
- Caminho HTTP; sem launch de Chrome


## Como Fazer Search
```bash
browser-automation-cli --json search "example domain" --limit 10
```
- Search local retorna links estilo SERP HTTP ou resultados de mapa de URLs
- Limit limita a contagem de resultados


## Como Parsear Arquivos Locais (HTML, PDF, DOCX, XLSX, ODS)
```bash
cat > /tmp/page.html <<'HTML'
<!doctype html>
<html><head><title>Demo</title></head>
<body><h1>Hello parse</h1><p>Local file text.</p></body></html>
HTML
browser-automation-cli --json parse /tmp/page.html
browser-automation-cli --json parse tests/fixtures/hello.pdf
browser-automation-cli --json parse tests/fixtures/hello.docx --redact-pii
# browser-automation-cli --json parse /tmp/sheet.xlsx
# browser-automation-cli --json parse /tmp/sheet.ods --redact-pii
```
- Parse extrai texto de html, md, txt, pdf, docx, xlsx ou ods local
- `--redact-pii` redige padrões comuns de PII no texto extraído
- Crie o HTML de exemplo antes do primeiro comando; use fixtures do repo para PDF/DOCX


## Como Extrair com LLM
```bash
browser-automation-cli --json config set openrouter_api_key YOUR_KEY
browser-automation-cli --json config set llm_base_url https://openrouter.ai/api/v1
browser-automation-cli --json config set llm_model openai/gpt-4o-mini
browser-automation-cli --json extract https://example.com --llm --question 'What is the title?'
```
- Sem a chave XDG, o comando falha fechado com envelope de usage
- `--schema-json` opcional para extração estruturada com schema local


## Como Codificar e Decodificar QR Codes
```bash
browser-automation-cli --json qr encode --text 'hello' --format png --path /tmp/qr.png
browser-automation-cli --json qr decode --path /tmp/qr.png
```
- Não exige Chrome
- Formatos de encode incluem `png`, `svg` e `terminal`


## Como Processar Imagens Localmente (agent-native)
```bash
# Download com SSRF + teto de corpo + magic (sem Chrome)
# browser-automation-cli --json image download 'https://example.com/a.png' -o /tmp/a.png
# Cria o PNG localmente para a receita ser autossuficiente (grab não tem --url e precisa de página viva)
printf '%s\n' '{"cmd":"goto","url":"https://example.com"}' '{"cmd":"grab","format":"png","path":"/tmp/a.png"}' | browser-automation-cli --json run --script -
# Projeção compacta do envelope (anti-token)
browser-automation-cli --json image info --path /tmp/a.png --select format,width,height,sha256
# Convert (o re-encode remove EXIF; webp local é lossless — quality vale para jpeg)
browser-automation-cli --json image convert --path /tmp/a.png --format webp -o /tmp/a.webp
# Screenshot sem base64 de pixels; opt-in: grab --include-base64
browser-automation-cli --json grab --format webp --path /tmp/g.webp
# Envie o arquivo convertido para um file input (Chrome one-shot / run)
# --script recebe um caminho de arquivo ou `-` para NDJSON via stdin; JSON inline não é uma forma
printf '%s\n' '{"cmd":"goto","url":"https://example.com"}' '{"cmd":"eval","expression":"document.body.insertAdjacentHTML('"'"'beforeend'"'"','"'"'<input type=file>'"'"');'"'"'ok'"'"'"}' '{"cmd":"upload","target":"input[type=file]","path":"/tmp/a.webp"}' | browser-automation-cli --json run --script -
```
- O campo de arquivo é injetado para a receita ser autossuficiente numa página que não tem nenhum
- Numa página real de upload esse campo já existe e o passo `eval` não
- A linha comentada de `image download` preserva o ensino de SSRF e teto de corpo sem depender de uma URL que precise existir
- `grab` não tem `--url`, então ele roda dentro de `run` depois de um `goto` e fotografa a página viva
- Nunca despeja base64 de pixels por padrão (stdout agent-native)
- Limites via XDG: `image_max_input_bytes`, `image_max_pixels`, `image_download_max_bytes`
- Magic bytes definem o formato (extensão não é confiável); AVIF/HEIC rejeitados; GIF `frame_count` = 1 (sem reassemble multi-frame)
- `image download` = URL de imagem única (SSRF + body cap) — não é download árvore de site inteiro
- Só EXIF (`kamadak-exif`); sem IPTC/XMP; `--select tags` alias de `exif`
- SVG: sem resvg; use `--allow-non-image` só para bytes crus intencionais
- Sem ação de OCR: o agente lê imagens nativamente, então OCR embutido era middleware redundante


## Como Processar Vídeos Localmente (agent-native)
```bash
# Fabrica um insumo determinístico para a receita ser autossuficiente
ffmpeg -nostdin -loglevel error -f lavfi -i testsrc=size=320x240:rate=10:duration=3 -f lavfi -i sine=frequency=440:duration=3 -c:v libx264 -c:a aac -shortest -y /tmp/in.mp4
ffmpeg -nostdin -loglevel error -i /tmp/in.mp4 -c copy -f hls -hls_time 1 -hls_playlist_type vod -master_pl_name master.m3u8 -y /tmp/stream.m3u8
# Sonda magic + streams (ffprobe opcional; só JSON de path e meta — nunca mídia crua no stdout)
browser-automation-cli --json video info --path /tmp/in.mp4 --select container,duration_secs,streams,sha256
# aliases de agente também: --select format,bytes,path → container,size_bytes,path
# Convert e remux: smart copy quando muxável; auto re-encode para WebM a partir de H.264 (sem ffmpeg manual)
browser-automation-cli --json video convert --path /tmp/in.mp4 --format webm -o /tmp/out.webm --select path_out,auto_reencoded,video_codec,bytes_out
# Extrai o áudio
browser-automation-cli --json video to-mp3 --path /tmp/in.mp4 -o /tmp/a.mp3
# Trim e frame de thumbnail (path→path)
browser-automation-cli --json video trim --path /tmp/in.mp4 --start 0 --duration 2 -o /tmp/clip.mp4
browser-automation-cli --json video thumbnail --path /tmp/in.mp4 --at 1 -o /tmp/thumb.png
# Resume manifesto HLS .m3u8 ou DASH .mpd sem baixar nenhuma mídia
browser-automation-cli --json video manifest --path /tmp/master.m3u8
# Download direto de URL de mídia (SSRF + teto de corpo + magic) — não é player de site nem yt-dlp
# browser-automation-cli --json video download 'https://example.com/clip.mp4' -o /tmp/in.bin
# Envie para um formulário (reusa o upload CDP existente)
# --script recebe um caminho de arquivo ou `-` para NDJSON via stdin; JSON inline não é uma forma
printf '%s\n' '{"cmd":"goto","url":"https://example.com"}' '{"cmd":"eval","expression":"document.body.insertAdjacentHTML('"'"'beforeend'"'"','"'"'<input type=file>'"'"');'"'"'ok'"'"'"}' '{"cmd":"upload","target":"input[type=file]","path":"/tmp/out.webm"}' | browser-automation-cli --json run --script -
```
- O campo de arquivo é injetado para a receita ser autossuficiente numa página que não tem nenhum
- Numa página real de upload esse campo já existe e o passo `eval` não
- As duas linhas de ffmpeg fabricam um insumo determinístico para a receita ser autossuficiente
- Numa sessão real o vídeo vem do usuário e essas duas linhas não existem
- Requer `ffmpeg`/`ffprobe` opcional do SO (XDG `ffmpeg_path` / PATH); nunca linka libav no crate de produto
- Limites via XDG: `video_max_input_bytes`, `video_download_max_bytes`, `video_default_container`, `video_default_crf`, `video_default_audio_bitrate`, `ffmpeg_timeout_secs`
- Magic bytes definem o container; extensão não é confiável; só path→path (sem carregar o arquivo inteiro no processo da CLI)
- Campos de honestidade do agente: `stream_copy`, `auto_reencoded`, `reencode_reason`, `faststart_applied`
- `video manifest` lê só a estrutura de HLS `.m3u8` e DASH `.mpd`; nunca busca segmentos
- Fora do core: playback adaptativo HLS/DASH, yt-dlp, encode H.264 pure-Rust

## Como Processar Áudio Local (path→path)
```bash
# Sonda (magic + ffprobe opcional) — sem dump de mídia
browser-automation-cli --json audio info --path /tmp/in.wav --select format,codec,duration,bytes,sha256
# Converte para MP3 (ffmpeg opcional; smart copy quando muxável)
browser-automation-cli --json audio convert --path /tmp/in.wav --format mp3 -o /tmp/a.mp3
# Extrai o áudio de um container de vídeo (-vn)
browser-automation-cli --json audio convert --path /tmp/clip.mp4 --format m4a -o /tmp/a.m4a
# Trim
browser-automation-cli --json audio trim --path /tmp/a.mp3 --start 1 --duration 5 -o /tmp/cut.mp3
# Download direto de URL de mídia (SSRF + teto de corpo + magic)
# browser-automation-cli --json audio download 'https://example.com/a.mp3' -o /tmp/a.mp3
# Envie para um formulário (upload CDP existente)
browser-automation-cli --json upload @e1 /tmp/a.mp3
```
- Requer `ffmpeg`/`ffprobe` opcional do SO (XDG `ffmpeg_path` / PATH); nunca linka libav
- Limites via XDG: `audio_max_input_bytes`, `audio_download_max_bytes`, `audio_default_format`, `audio_default_bitrate`, `ffmpeg_timeout_secs`
- Magic bytes definem o container; extensão não é confiável; envelope pode marcar `lossy_transcode` em recompressão lossy→lossy
- Fora do core: I/O de dispositivo cpal, BPM e fingerprint, stack de encode pure-Rust, yt-dlp/HLS


## Como Encontrar Paths no Disco
```bash
browser-automation-cli --json find-paths 'Cargo.*' .
browser-automation-cli --json find-paths --glob '**/*.rs' .
```
- Descoberta de paths estilo fd sob o nome do binário `browser-automation-cli`
- Use `--glob` para filtros estilo shell (GAP-A011)
- Sem launch de Chrome


## Como Localizar Sugestões (pt-BR)
```bash
browser-automation-cli --lang pt-BR --json click-at --x 1 --y 1
browser-automation-cli --json config set lang pt-BR
```
- Sugestões humanas localizam para `pt-BR` via `--lang` ou XDG `lang`
- Cliques por coordenada com sucesso ainda exigem `--experimental-vision`


## Como Capturar com MITM
```bash
browser-automation-cli --json mitm init-ca
browser-automation-cli --json mitm start --seconds 30
browser-automation-cli --json mitm status
browser-automation-cli --json mitm list --limit 100
browser-automation-cli --json mitm har --out /tmp/capture.har
browser-automation-cli --json mitm redact
browser-automation-cli --json mitm domains
browser-automation-cli --json mitm apis
browser-automation-cli --json mitm graphql
browser-automation-cli --json mitm ws
```
- Bind apenas em 127.0.0.1 com porta efêmera
- Material de CA fica sob XDG data (`mitm/ca`)
- `start` mantém o proxy one-shot vivo por `--seconds` e então sai
- Exporte HAR com `--out` **obrigatório**


## Como MITM capture-url One-shot
```bash
browser-automation-cli --json mitm init-ca
browser-automation-cli --json mitm capture-url https://example.com --seconds 30 --har /tmp/cap.har
browser-automation-cli --json mitm list
browser-automation-cli --json mitm har --out /tmp/capture.har
```
- Compose one-shot: proxy local + Chrome + navega URL + captura (GAP-011)
- Allowlist opcional `--hosts` para intercept TLS
- Flags globais de rota-via-MITM também existem: `--mitm`, `--mitm-har`, `--mitm-redact-secrets`, …


## Como Rodar, Resumir e Ver Status de Workflow
```bash
cat > /tmp/wf.json <<'JSON'
{
  "name": "demo",
  "steps": [
    {"id": "ping", "cmd": "echo", "args": {"message": "start"}},
    {
      "id": "fetch",
      "cmd": "scrape",
      "args": {"url": "https://example.com", "engine": "http", "format": "text"},
      "depends_on": ["ping"]
    }
  ]
}
JSON
browser-automation-cli --json workflow run --manifest /tmp/wf.json
browser-automation-cli --json workflow resume --manifest /tmp/wf.json
browser-automation-cli --json workflow status --name demo
```
- Resume pula passos já `ok` no journal SQLite
- Só passos offline; multi-passo browser com `@eN` permanece em `run --script`
- Comandos offline suportados incluem noop, echo, parse, scrape (http), batch-scrape


## Como Rodar Auditoria Lighthouse
```bash
# Exige um binário lighthouse real no PATH
browser-automation-cli --timeout 180 --json lighthouse https://example.com

# Binário mock para smoke local sem instalar o lighthouse real
browser-automation-cli --timeout 60 --json lighthouse https://example.com \
  --lighthouse-path ./scripts/mock-lighthouse.sh
```
- Ordem de resolve: flag `--lighthouse-path` → XDG `lighthouse_path` → PATH
- Envelope reporta `binary_source` como `real` ou `mock`
- Passe `--lighthouse-path` ou XDG `lighthouse_path` para binário externo ou script mock
- Lighthouse em si não está embutido na CLI


## Como Inspecionar Heap Snapshots
```bash
cat > /tmp/heap.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"heap","action":"take","path":"/tmp/snap.heapsnapshot"}
JSONL
browser-automation-cli --category-memory --timeout 120 --json run --script /tmp/heap.browser-automation.jsonl
browser-automation-cli --category-memory --json heap summary --path /tmp/snap.heapsnapshot
```
- Análise profunda de heap exige `--category-memory`
- Summary lê path de snapshot existente via `--path`


## Como Gerar Completions de Shell
```bash
browser-automation-cli completions bash
browser-automation-cli completions zsh
browser-automation-cli completions fish
```
- Caminho de completions é leve e não lança Chrome
- Redirecione stdout para o diretório de completions do shell conforme necessário



## Como Escrever Planilhas (sheet-write)
```bash
printf 'name,score\nalice,10\nbob,9\n' > /tmp/rows.csv
browser-automation-cli --json sheet-write /tmp/rows.csv -o /tmp/out.xlsx --sheet Data
```
- Escreve um XLSX simples a partir de CSV ou JSON array-of-objects
- Sem Chrome
- Use `--sheet` para nomear a planilha (padrão `Sheet1`)


## Como Fazer Lint Estrutural Com sg-scan
```bash
browser-automation-cli --json sg-scan . --limit 100
```
- Lint estrutural one-shot para padrões proibidos de produto
- Sem Chrome
- `--limit 0` significa findings ilimitados


## Como Dry-run e Aplicar sg-rewrite
```bash
browser-automation-cli --json sg-rewrite .
browser-automation-cli --json sg-rewrite . --apply
```
- Padrão é relatório dry-run
- Passe `--apply` para gravar correções known-safe
- Sem Chrome


## Como Encontrar Paths Com --glob
```bash
browser-automation-cli --json find-paths --glob '**/*.rs' .
browser-automation-cli --json find-paths 'Cargo.*' . --extension rs
```
- `--glob` é filtro glob estilo shell (GAP-A011)
- Pattern regex e `--glob` combinam com outros filtros
- Sem Chrome


## Como Rodar Script em Array JSON
```bash
cat > /tmp/demo.array.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"view"}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/demo.array.json
```
- `run --script` aceita NDJSON **ou** um array JSON de objetos de passo
- Mesmo ciclo de vida: BORN EXECUTE FINALIZE DIE
- Erros fail-fast ainda podem incluir `data.steps` parcial
- Envelope final inclui `steps[].data` completo quando `--json` está set
- `{"cmd":"include","path":"outro.ndjson"}` costura outro script no momento da carga; ele é expandido pelo preflight e nunca despachado, então um erro de digitação nele reprova antes de o navegador subir
- `include` lê `path` primeiro, depois `script`, depois `file` — três grafias de um mesmo alvo, e vence a primeira presente


## Como Ler binary_source do Lighthouse
```bash
browser-automation-cli --timeout 60 --json lighthouse https://example.com \
  --lighthouse-path ./scripts/mock-lighthouse.sh \
  | jaq '.data.binary_source // .binary_source // .'
```
- Ordem de resolve: flag `--lighthouse-path` → XDG `lighthouse_path` → PATH
- Envelope reporta `binary_source` como `real` ou `mock`
- Mock é honesty para e2e/smoke, não auditoria de produção


## Como Configurar Cache Redis Com Honesty
```bash
browser-automation-cli --json config set cache_backend redis
browser-automation-cli --json config set cache_redis_url redis://127.0.0.1:6379
browser-automation-cli doctor --offline --quick --json
```
- Cache só via XDG com `config set` / `config get` / `config list-keys`
- Use apenas `redis://`; `rediss://` é fail-closed (cliente TCP plain)
- Doctor reporta `cache_redis` quando cache Redis está configurado


## Como Cobrir Demais Comandos de Interação e Página
```bash
# keys / type / hover / drag / upload (mesmo processo da navegação)
cat > /tmp/interact.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"keys","key":"Tab"}
{"cmd":"type","text":"hello","focus_only":true}
{"cmd":"hover","target":"a"}
{"cmd":"text","target":"h1"}
{"cmd":"attr","selector":"a","name":"href"}
{"cmd":"page","action":"list"}
JSONL
browser-automation-cli --timeout 90 --json run --script /tmp/interact.browser-automation.jsonl

# subcomandos dialog accept/dismiss (não --action); caminho suave quando opcional
browser-automation-cli --timeout 60 --json reload --ignore-cache
browser-automation-cli --json dialog accept --if-present
browser-automation-cli --json dialog dismiss --if-present
browser-automation-cli --json exec --help >/dev/null

# dialog dentro do run (a forma NDJSON usa action + if_present opcional)
cat > /tmp/dialog.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"dialog","action":"accept","if_present":true}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/dialog.run.json

# superfícies com gate de categoria (flags explícitas)
browser-automation-cli --category-extensions --json extension list
browser-automation-cli --category-third-party --json devtools3p list
browser-automation-cli --category-webmcp --json webmcp list
browser-automation-cli --experimental-screencast --json screencast --help >/dev/null
browser-automation-cli --category-memory --json heap --help >/dev/null
browser-automation-cli --json perf --help >/dev/null
browser-automation-cli --json resize --help >/dev/null
browser-automation-cli completions bash >/dev/null
```
- Cada nome de agente aparece em `commands --json` (**71**)
- `select-option` / `pick` aparecem no inventário e só em run/schema
- Prefira `schema <name>` antes de inventar argv em superfícies com gate


## Como Descobrir Schemas de Comando
```bash
browser-automation-cli commands --json
browser-automation-cli schema goto --json
browser-automation-cli schema --cmd scrape --json
browser-automation-cli schema print-pdf --json
browser-automation-cli schema monitor --json
browser-automation-cli schema qr --json
browser-automation-cli schema find-paths --json
browser-automation-cli schema sheet-write --json
browser-automation-cli schema sg-scan --json
browser-automation-cli schema sg-rewrite --json
browser-automation-cli schema run --json
browser-automation-cli schema pick --json
browser-automation-cli schema select-option --json
browser-automation-cli schema submit --json
browser-automation-cli schema storage --json
browser-automation-cli schema batch-scrape --json
browser-automation-cli schema config --json
browser-automation-cli schema mitm --json
browser-automation-cli schema workflow --json
browser-automation-cli schema locale --json
browser-automation-cli schema man --json
```
- `commands` lista a superfície voltada a agentes (**71** nomes)
- `schema <cmd>` ou `schema --cmd` imprime um fragmento JSON Schema de um comando
- Útil para registro de tools em frameworks de agentes


## Como Pipear JSON com jaq
```bash
browser-automation-cli doctor --offline --quick --json | jaq -e '.ok == true'
browser-automation-cli --json scrape https://example.com --format metadata --engine http \
  | jaq '.data // .'
browser-automation-cli commands --json | jaq '.data.commands // .commands // .'
```
- Prefira `--json` para stdout legível por máquina
- Filtros `jaq` mantêm a cola de agentes pequena e determinística


## Como Contornar robots.txt com Dual Flags
```bash
# Honra robots por padrão (sem flags de contorno)
browser-automation-cli --json scrape https://example.com --format text --engine http

# Contorne apenas quando as duas flags estiverem juntas
browser-automation-cli --ignore-robots --i-accept-robots-risk --json \
  scrape https://example.com --format text --engine http
```
- Política default honra robots.txt
- `--ignore-robots` sozinho falha; `--i-accept-robots-risk` sozinho falha
- Ambas as flags são exigidas quando você aceita o risco de bypass


## Como Listar Cookies
```bash
cat > /tmp/cookie.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"cookie","action":"list"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/cookie.browser-automation.jsonl
```
- Helpers de cookie operam na página ativa no mesmo processo
- Filtro opcional de URL existe em `cookie list --url`
- `{"cmd":"cookie","action":"set","cookies":[...]}` recebe objetos cujos campos aceitos são os do struct com que o produto serializa cookies, então o que o `cookie list` emite é exatamente o que o `cookie set` aceita de volta
- Esses campos são `name`, `value`, `url`, `domain`, `path`, `expires`, `size`, `httpOnly`, `secure`, `session` e `sameSite`; os campos `priority`, `partitionKey`, `sourceScheme`, `sourcePort` e `sameParty` do CDP são recusados de propósito, porque nada mais na CLI os modela


## Como Listar Mensagens de Console
```bash
cat > /tmp/console.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"eval","expression":"console.log('hello-cookbook')"}
{"cmd":"console","action":"list"}
JSONL
browser-automation-cli --capture-console --timeout 60 --json run --script /tmp/console.browser-automation.jsonl
```
- Habilite `--capture-console` no mesmo processo que produz as mensagens
- Filtre tipos com `--types log,warning,error,info,debug` na forma CLI
- `console dump` sempre grava um array JSON válido (`[]` quando vazio)


## Como Fazer Assert de URL ou Texto
```bash
cat > /tmp/assert.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"assert","kind":"url","value":"example.com","contains":true}
{"cmd":"assert","kind":"text","value":"Example Domain"}
{"cmd":"assert","url_contains":"example.com"}
{"cmd":"assert","text_contains":"Example Domain"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/assert.browser-automation.jsonl
```
- Assert falha o processo quando a condição não é atendida
- Assert de URL suporta match exato ou semântica contains (`contains` ou `url_contains`)
- Assert de texto pode mirar seletor via `target` ou usar `text_contains`


## Como Ler Um Atributo Com attr
```bash
browser-automation-cli --json schema attr
cat > /tmp/attr.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"attr","target":"a","name":"href"}
JSONL
browser-automation-cli --timeout 90 --json run --script /tmp/attr.browser-automation.jsonl
```
- `attr` recebe dois argumentos posicionais, `<TARGET>` e `<NAME>`
- O passo de `run` grafa os mesmos dois argumentos como `target` e `name`
- A leitura cai para a propriedade do DOM quando o atributo HTML está ausente
- `attr` exige página viva, então um agente one-shot chega nele por `run --script`


## Como Clicar Num Elemento Com press
```bash
browser-automation-cli --json schema press
cat > /tmp/press.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"press","target":"a","include_snapshot":true}
JSONL
browser-automation-cli --timeout 120 --json run --script /tmp/press.browser-automation.jsonl
```
- `press` é o nome de produto para a ação `click` do DevTools
- Passe `--dblclick` na forma CLI ou `"dblclick": true` no passo de `run` para clique duplo
- `include_snapshot` anexa um snapshot de acessibilidade enxuto logo após o clique
- As refs `@eN` desse snapshot vivem SOMENTE dentro deste único processo


## Como Preencher Um Campo Com write
```bash
browser-automation-cli --json schema write
cat > /tmp/write.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"eval","expression":"document.body.insertAdjacentHTML('beforeend','<input id=q>');'ok'"}
{"cmd":"write","target":"#q","value":"example"}
JSONL
browser-automation-cli --timeout 120 --json run --script /tmp/write.browser-automation.jsonl
```
- `write` é um preenchimento inteligente que cobre input de texto, select, checkbox e radio
- A forma CLI é `write <TARGET> <VALUE>` com os dois argumentos posicionais
- Prefira `write` a `type` quando você quer o valor final gravado de uma vez
- Todo passo `eval` invalida as refs `@eN`, então refaça `view` depois dele


## Como Enviar Uma Tecla Com keys
```bash
browser-automation-cli --json schema keys
cat > /tmp/keys.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"keys","key":"Tab"}
{"cmd":"keys","key":"Control+a"}
JSONL
browser-automation-cli --timeout 90 --json run --script /tmp/keys.browser-automation.jsonl
```
- A chave do passo de `run` é `key` no singular, e NUNCA `keys`
- Chave desconhecida no passo é aceita em silêncio com `ok` true, então confira contra `schema keys`
- Combinação usa a forma `Control+a` com sinal de mais
- `keys` envia a tecla para o que estiver com foco na página naquele momento


## Como Passar o Ponteiro Com hover
```bash
browser-automation-cli --json schema hover
cat > /tmp/hover.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"hover","target":"a"}
{"cmd":"view","detailed":true}
JSONL
browser-automation-cli --timeout 90 --json run --script /tmp/hover.browser-automation.jsonl
```
- `hover` move o ponteiro sobre o alvo sem clicar nele
- Use para abrir um menu que só é renderizado sob o ponteiro
- Encadeie `view` no mesmo processo para ler o que o hover revelou
- Alvo ausente falha com `error.kind` browser e exit code 70


## Como Arrastar Entre Dois Alvos Com drag
```bash
browser-automation-cli --json schema drag
cat > /tmp/drag.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"eval","expression":"document.body.insertAdjacentHTML('beforeend','<div id=src draggable=true>s</div><div id=dst>d</div>');'ok'"}
{"cmd":"drag","from":"#src","to":"#dst","synthetic_payload":{"items":[{"mimeType":"text/plain","data":"row-1"}],"dragOperationsMask":1}}
JSONL
browser-automation-cli --timeout 120 --json run --script /tmp/drag.browser-automation.jsonl
```
- `--from` é obrigatório e `--to` só é opcional quando você informa `--to-x` e `--to-y`
- `--anchor center|before|after` escolhe onde dentro do retângulo de destino a solta acontece
- Página cujo handler `dragstart` não preenche item de `DataTransfer` falha com `payload has no items array`
- `synthetic_payload` contorna esse handler e DEVE ser objeto JSON no passo de `run`, nunca string
- O payload aceita `items`, `dragOperationsMask`, `files` e `data`, onde `data` é a grafia de embrulho para páginas que emitem os campos de DragData um nível abaixo
- Cada objeto de `items` é vocabulário do `DragDataItem` do CDP: `mimeType` e `data` são obrigatórios, `title` e `baseURL` são opcionais e aceitos para que um payload que o Chrome honra nunca seja recusado aqui


## Como Andar no Histórico Com back e forward
```bash
browser-automation-cli --json schema back
browser-automation-cli --json schema forward
cat > /tmp/history.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"press","target":"a"}
{"cmd":"back"}
{"cmd":"forward"}
JSONL
browser-automation-cli --timeout 120 --json run --script /tmp/history.browser-automation.jsonl
browser-automation-cli --json back
browser-automation-cli --json forward
```
- `back` e `forward` não recebem argumento nenhum, na forma CLI e no passo de `run`
- Ambos respondem com `data.navigation`, `data.title` e `data.url`
- Um one-shot isolado tem sucesso com exit code 0 e reporta `about:blank`, porque processo novo não tem histórico
- Histórico real exige duas navegações dentro do MESMO processo, então use `run --script`


## Como Paginar Requisições Capturadas Com net
```bash
browser-automation-cli --json schema net
cat > /tmp/net.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"net","action":"list"}
{"cmd":"net","action":"list","page_idx":0,"page_size":20}
{"cmd":"net","action":"get","id":"0"}
JSONL
browser-automation-cli --capture-network --timeout 120 --json run --script /tmp/net.browser-automation.jsonl
```
- O buffer de captura registra `resourceType` ao lado de `requestId`, `method` e `url`, então `resource_types` seleciona de verdade
- Requisição que o protocolo envia sem tipo é gravada como `Other`, valor real do CDP, então ela continua selecionável em vez de sumir
- Tipo fora do vocabulário do CDP é recusado com exit 2 antes de qualquer browser subir, então lista vazia significa que a página não tinha aquele recurso
- `dropped_oldest` conta as entradas que o teto do anel descartou, então resposta truncada diz isso em vez de passar um subconjunto por conjunto inteiro; o teto é a chave XDG `event_tracker_max_entries`
- `net` só enxerga tráfego quando `--capture-network` viaja no MESMO processo
- `net list` estreita com `--resource-types`, `--page-idx`, `--page-size` e `--include-preserved`
- `net get <ID>` recebe o índice 0-based de `net list` ou o request id do CDP
- `net get` grava corpos em disco com `--request-path` e `--response-path`
- Processo que nunca navegou responde `count` 0 com `ok` true, o que é honesto e não é falha


## Como Gravar Passos Replayáveis Com record
```bash
browser-automation-cli --json schema record
browser-automation-cli --timeout 120 --json record --url https://example.com --path /tmp/rec.browser-automation.jsonl --seconds 3 --max-events 20
browser-automation-cli --timeout 120 --json --json-steps run --script /tmp/rec.browser-automation.jsonl
```
- `record` é autossuficiente, porque `--url` navega por você
- `--url` e `--path` são ambos obrigatórios
- `--seconds` tem padrão 30 e `--max-events` tem padrão 200
- O primeiro teto atingido vence, e `data.truncated` diz qual deles venceu
- O arquivo gravado é NDJSON de `run --script`, então a gravação é reproduzida sem edição


## Inventário Completo de Comandos (71)
- Fonte viva: `browser-automation-cli commands --json` (**71** nomes voltados a agentes)
- Superfície clap de produto é **69** nomes (exclui `select-option` / `pick` de inventário de agente)
- O e2e DevTools tool-ref cobre **53** tools (`scripts/e2e_all_52_tools.sh` é nome legado; a suite executa 53; lighthouse mock SKIP)
- Lista completa de comandos de agente (todos os **71**):
  - Meta / descoberta: `doctor`, `commands`, `schema`, `version`, `locale`, `completions`, `man`
  - Navegação: `goto`, `back`, `forward`, `reload`, `page`, `wait`, `dialog`
  - Interação: `press`, `click-at`, `write`, `keys`, `type`, `hover`, `drag`, `submit`, `fill-form`, `upload`, `scroll`
  - Agent inventory + run/exec/schema (not clap standalone): `select-option`, `pick`
  - Observação: `view`, `eval`, `text`, `attr`, `assert`, `cookie`, `storage`, `console`, `net`
  - Captura: `grab`, `print-pdf`, `monitor`, `screencast`, `lighthouse`
  - Multi-passo: `run`, `exec`, `record`
  - Extract/scrape: `extract`, `scrape`, `batch-scrape`, `crawl`, `map`, `sitemap`, `feed`, `search`, `parse`
  - IO local (sem Chrome): `qr`, `image`, `video`, `audio`, `find-paths`, `sheet-write`, `sg-scan`, `sg-rewrite`
  - Infra: `config`, `mitm`, `workflow`
  - Emulação/perf: `emulate`, `resize`, `perf`, `heap`
  - Portões de categoria: `extension`, `devtools3p`, `webmcp`
- Lista plana completa: `doctor`, `commands`, `schema`, `version`, `locale`, `goto`, `view`, `press`, `click-at`, `write`, `keys`, `type`, `wait`, `hover`, `drag`, `submit`, `fill-form`, `select-option`, `pick`, `upload`, `back`, `forward`, `reload`, `eval`, `grab`, `print-pdf`, `monitor`, `run`, `exec`, `record`, `extract`, `text`, `scroll`, `cookie`, `storage`, `attr`, `assert`, `console`, `net`, `page`, `dialog`, `scrape`, `batch-scrape`, `crawl`, `map`, `sitemap`, `feed`, `search`, `parse`, `qr`, `image`, `video`, `audio`, `find-paths`, `sg-scan`, `sg-rewrite`, `sheet-write`, `mitm`, `workflow`, `config`, `emulate`, `resize`, `perf`, `lighthouse`, `screencast`, `heap`, `extension`, `devtools3p`, `webmcp`, `completions`, `man`
- Vinte dos 71 comandos raiz possuem subcomandos, e os outros 51 recebem suas flags direto, sem subcomando
- Invoque um subcomando como família e depois verbo, como `page close` ou `heap take`
- `assert url|text|console|console-empty|console-no-match`
- `audio info|download|convert|trim`
- `console list|get|clear|dump`
- `cookie list|set|clear`
- `devtools3p list|exec`
- `dialog accept|dismiss`
- `extension list|install|reload|trigger|uninstall`
- `heap take|close|compare|summary|details|class-nodes|dominators|dup-strings|edges|retainers|paths|object-details`
- `image info|convert|resize|download|exif`
- `mitm status|list|get|har|export|domains|apis|init-ca|start|capture-url|graphql|ws|block|config|allow|redact`
- `monitor check`
- `net list|get`
- `page info|list|new|select|close|tab-id`
- `perf start|stop|insight`
- `qr encode|decode`
- `screencast start|stop`
- `storage export|import`
- `video info|download|convert|to-mp3|trim|thumbnail|manifest`
- `webmcp list|exec`
- `workflow run|resume|status`
- Quatro famílias exigem uma flag de categoria: `heap` exige `--category-memory`, `extension` exige `--category-extensions`, `devtools3p` exige `--category-third-party` e `webmcp` exige `--category-webmcp`
- `screencast` exige `--experimental-screencast`
- `cookie clear` exige `--all`, e omitir essa flag devolve um erro de uso
- Descubra argv com `schema <name> --json` para qualquer nome acima
