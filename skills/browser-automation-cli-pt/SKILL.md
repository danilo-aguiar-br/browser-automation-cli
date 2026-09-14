---
name: browser-automation-cli
description: Esta skill DEVE ser usada quando a tarefa exigir abrir, ler, operar, coletar ou diagnosticar páginas web, e DEVE ativar de forma proativa mesmo quando o usuário não nomear a browser-automation-cli. Ela cobre navegar, clicar, digitar e preencher formulários, snapshot de acessibilidade, screenshot e PDF, scrape, crawl, mapa de site, sitemap e feed, extração com LLM, parse de PDF, DOCX e planilha, console, rede, HAR e MITM, descoberta de APIs REST e GraphQL, emulação de dispositivo, Lighthouse, trace de performance, heap, extensões, gravação e replay de interações, QR, planilha XLSX, imagem, vídeo e áudio locais, janela headed invisível para Cloudflare Turnstile e auditoria de residual-zero. Ela recebe URLs, seletores, arquivos locais e roteiros multi-passo, e entrega envelope JSON com exit code ramificável, artefatos em disco, payload reduzido e prova do modo de janela. Ela SEMPRE configura por flag e config XDG e NUNCA por variável de ambiente.
---
# browser-automation-cli


## Missão e Ativação
### OBRIGATÓRIO
- DEVE ativar esta skill em toda tarefa que abre, lê, opera, coleta ou mede uma página web, mesmo sem o usuário nomear a CLI
- DEVE ativar esta skill também para mídia local, parse de documento, QR e planilha, que rodam sem Chrome
- DEVE invocar SEMPRE o binário `browser-automation-cli` pelo nome completo
- DEVE passar `--json` em TODA invocação programática e parsear SOMENTE stdout com `jaq`
- DEVE ler o exit code ANTES do stdout e exigir `ok` true ANTES de ler `data`
- DEVE passar `--timeout` explícito em toda chamada que abre navegador
### PROIBIDO
- NUNCA use apelido nem nome abreviado do binário
- NUNCA parseie stderr como JSON
- NUNCA mascare exit code com `|| true`
- NUNCA invente flag, valor, chave ou subcomando ausente do binário
- NUNCA use esta skill para documentação oficial de biblioteca, crate Rust, SSH ou banco de dados


## Descoberta da Superfície Viva
### OBRIGATÓRIO
- DEVE descobrir a superfície pelo binário e NUNCA pela memória
- DEVE rodar `commands` para o inventário e `schema <cmd>` para o contrato de um comando
- DEVE rodar `<cmd> --help` para ver as flags locais e globais daquele comando
- DEVE rodar `doctor --offline --quick` quando o host parecer errado, porque ele NÃO lança o Chrome
- DEVE confirmar toda chave de passo contra `schema <cmd>` ANTES de serializar um passo novo
### Fórmulas Prontas
- EXECUTE `browser-automation-cli --json commands --detail`
- EXECUTE `browser-automation-cli --json schema --cmd scrape`
- EXECUTE `browser-automation-cli scrape --help`
- EXECUTE `browser-automation-cli --json version` e `browser-automation-cli --json locale`
- EXECUTE `browser-automation-cli --json doctor --offline --quick --fix`
- EXECUTE `browser-automation-cli completions bash` e `browser-automation-cli man --out /tmp/b.1`


## Ciclo de Vida One-Shot
### OBRIGATÓRIO
- DEVE tratar cada processo como BORN, EXECUTE, FINALIZE e DIE, com o Chrome nascendo e morrendo dentro dele
- DEVE saber que NÃO existe daemon, sessão persistente nem estado entre processos
- DEVE tratar a ref `@eN` como válida SOMENTE dentro do processo que a gerou
- DEVE colocar todo trabalho multi-passo num único `run --script`
- DEVE tratar `exec` como passo único inline, com a mesma superfície dos passos de `run`
- DEVE saber que o Chrome lançado pela CLI NÃO abre porta TCP de DevTools e fala CDP por pipe através de uma ponte loopback de cliente único
- DEVE carregar estado autenticado entre processos SOMENTE com `storage export` e `storage import`
- DEVE carregar captura de tráfego entre processos SOMENTE com `mitm capture-url` e `--capture-path`
- DEVE usar o Chrome do sistema ou apontar o binário com `config set chrome_path`
### PROIBIDO
- NUNCA reutilize `@eN` num segundo processo
- NUNCA conecte ferramenta externa, depurador ou segundo cliente CDP nesse Chrome
- NUNCA espere que captura de console ou de rede sobreviva ao DIE


## Modo de Janela e Display Privado
### OBRIGATÓRIO
- DEVE saber que `browser_mode` vale `auto`, `headed` ou `headless`
- DEVE saber que `auto` resolve para headed dentro de um Xvfb privado SOMENTE no Linux com `Xvfb` no PATH e sem `--no-xvfb`
- DEVE saber que `auto` resolve para headless em todo o resto, e que `--no-xvfb` com `auto` mantém headless
- DEVE tratar `--headed` e `--headless` como atalhos de `--browser-mode`, e a flag SEMPRE vence `config set browser_mode`
- DEVE saber que `--headed` no Linux com Xvfb desenha a janela dentro do Xvfb privado, fora da tela do operador, inclusive em desktop Wayland
- DEVE saber que esse isolamento vem do pino `--ozone-platform=x11`, aplicado SOMENTE quando o Xvfb privado subiu
- DEVE passar `--headed --no-xvfb` como a ÚNICA rota para pôr a janela de propósito no display atual
- DEVE usar headed OBRIGATORIAMENTE contra desafio Cloudflare Turnstile, porque headless NÃO emite o token
- DEVE saber que runs headed concorrentes recebem displays privados distintos, então paralelismo headed é seguro
- DEVE saber que o Xvfb privado exige cookie de autenticação e é desmontado no DIE sem intervenção sua
- DEVE ler `display_backend`, que vale `headless`, `xvfb` ou `host` e diz o display REALMENTE usado após o lançamento
- DEVE saber que `display_backend` reporta só a intenção antes de qualquer lançamento, e que SOMENTE `host` pinta na tela do operador
- DEVE provar o modo com `browser_mode_requested`, `browser_mode_effective` e `browser_mode_source`, que vale `default`, `xdg` ou `flag`
- DEVE ler essa testemunha de `run` UMA vez no topo do envelope
- DEVE ler `browser_mode_auto_resolves` no check `virtual_display` do `doctor` para saber o que `auto` fará neste host
- DEVE ler `launch_args` de `doctor --fingerprint` para ver o argv real entregue ao Chrome, que vale `null` antes de qualquer lançamento
### PROIBIDO
- NUNCA conclua o modo de janela pela flag que você passou
- NUNCA trate `browser_mode_source` igual a `default` como exigência provada
- NUNCA parseie o texto `message` do check `virtual_display`
- NUNCA rode `config set chrome_legacy_oxide_launch true`, porque esse caminho reabre porta DevTools sem autenticação, nunca sobe o Xvfb e desenha janela headed no display do operador
### Fórmulas Prontas
- EXECUTE `browser-automation-cli --json --fields checks --filter-rows 'id=virtual_display' doctor --offline --quick`
- EXECUTE `browser-automation-cli --timeout 90 --json --headed scrape https://example.com --engine browser --format markdown` para Turnstile sem janela no desktop
- EXECUTE `browser-automation-cli --timeout 120 --json --headed --no-xvfb goto https://example.com` SOMENTE quando o operador exigir ver a janela
- EXECUTE `browser-automation-cli --json config set browser_mode headless` para gravar o padrão persistente do host
- EXECUTE `browser-automation-cli --timeout 60 --json doctor --fingerprint` e leia `launch_args`


## Envelope JSON e Exit Codes
### OBRIGATÓRIO
- DEVE esperar sucesso com `schema_version`, `ok` true e `data`
- DEVE esperar falha com `ok` false e `error` contendo `kind`, `message` e `exit_code`
- DEVE ler `data.steps` parciais quando um `run` falha
- DEVE ler `runtime_enable_used`, booleano que diz se o domínio Runtime do CDP foi ligado nesta execução
- DEVE ler `serp_endpoint` no envelope de `search`, e tratar `unknown` como endpoint que NÃO garante limite, região nem janela de tempo
- DEVE tratar `search` sem resultado orgânico como falha com `error.kind` igual a `data`, lendo `serp_endpoint` e `search_base_url` dentro de `data`
- DEVE ler `data.binary_source` de `lighthouse` e NUNCA tratar `mock` como auditoria real
- DEVE ler `data.dialog_settled` após `dialog accept` ou `dialog dismiss`, e NUNCA inserir espera artificial quando vier true
- DEVE retentar SOMENTE falha transitória de lançamento ou de rede
### Exit Codes
- DEVE tratar `0` como sucesso e `2` como usage, corrigindo o argv ANTES de repetir
- DEVE tratar `6` como blocked e `64` como capability-disabled, inclusive caminho fora das raízes permitidas
- DEVE tratar `65` como data, inclusive `--expect` falho sob `--expect-exit-code`
- DEVE tratar `66` como no-input, `69` como unavailable e `70` como software, browser ou protocol
- DEVE tratar `74` como io, `75` como precondition e `78` como config
- DEVE tratar `124` como timeout, inclusive quando um lançamento do Chrome que falhou é desmontado sob `--timeout`
- DEVE tratar `130` como cancelado, inclusive por sinal de término, e `141` como broken-pipe


## Redução de Payload
### OBRIGATÓRIO
- DEVE encolher o payload com as flags do binário, e NUNCA canalizando stdout por `jaq`
- DEVE passar `--fields` com UM CSV único de caminhos relativos a `data`, como `residual` e NUNCA `data.residual`
- DEVE passar `--filter-rows` com `key=value`, `key!=value` ou `key~substring`, repetível e combinado por AND
- DEVE passar `--limit-rows`, `--sort-rows` e `--dedupe-by` em payload de lista
- DEVE passar `--count-only` para receber só `count`, estreitando antes com `--fields` quando `data` tiver mais de uma lista
- DEVE passar `--truncate-content` para cortar cada string e `--max-output-bytes` como teto rígido de bytes
- DEVE ler `agent_ops.truncated` como o ÚNICO sinal de corte e `agent_ops.unresolved_paths` como caminho que não resolveu
- DEVE tratar filtro sem casamento como lista vazia com `ok` true, sabendo que campo ausente NUNCA casa
- DEVE tratar `--select`, `--filter`, `--limit` e `--sort` como flags LOCAIS escritas DEPOIS do subcomando
### PROIBIDO
- NUNCA repita `--fields`, porque a repetição sai com exit 2
- NUNCA presuma que `agent_ops` existe só porque você passou uma flag de redução
### Fórmulas Prontas
- EXECUTE `browser-automation-cli --json --fields checks --count-only doctor --offline --quick`
- EXECUTE `browser-automation-cli --json --limit-rows 5 --fields commands commands`
- EXECUTE `browser-automation-cli --json --fields keys --filter-rows 'key~proxy' config list-keys`
- EXECUTE `browser-automation-cli --json --truncate-content 2000 --max-output-bytes 60000 scrape https://example.com --format markdown`


## Flags Globais
### Saída e Tempo
- DEVE passar `--json` para o envelope e `--json-steps` para um objeto NDJSON por passo de `run`
- DEVE passar `-q` ou `--quiet` para calar logs humanos no stderr
- DEVE passar `-v` ou `--verbose` para tracing info e `--debug` para detalhe máximo
- DEVE passar `--plain` para stderr sem cores ANSI
- DEVE passar `--lang en` ou `--lang pt-BR` para forçar o idioma das mensagens
- DEVE passar `--correlation-id <ID>` para ecoar uma chave de junção em envelopes e passos
- DEVE passar `--timeout <SECS>` como teto do processo inteiro e `--step-timeout <SECS>` como teto de cada passo de `run`
- DEVE passar `--max-concurrency <N>` para limitar o fan-out de batch, crawl e CDP
- DEVE passar `--artifacts-dir <DIR>` para screenshots, PDFs e evidências
- DEVE passar `--allow-outside-roots` SOMENTE como aceitação explícita de risco, e SEMPRE preferir `config set allowed_roots`
- DEVE usar `--fields`, `--filter-rows`, `--limit-rows`, `--sort-rows`, `--dedupe-by`, `--count-only`, `--truncate-content` e `--max-output-bytes` conforme a seção `Redução de Payload`
### Navegador e Identidade
- DEVE passar `--browser-mode`, `--headed`, `--headless` e `--no-xvfb` conforme a seção `Modo de Janela e Display Privado`
- DEVE passar `--capture-console` no MESMO processo de `console` e `--capture-network` no MESMO processo de `net`
- DEVE passar `--dump-on-failure` com `--artifacts-dir` e uma flag de captura para gravar evidência na falha
- DEVE manter o stealth LIGADO e passar `--no-stealth` SOMENTE para desligar os patches nesta execução
- DEVE passar `--stealth-profile auto`, que segue o host, e usar `chrome-linux`, `chrome-win`, `chrome-mac` ou `list` SOMENTE quando bater com o host ou para listar
- DEVE passar `--stealth-seed <SEED>` para fixar a mesma identidade entre os processos de um crawl
- DEVE saber que a semente também guarda o major do Chrome lançado em `state_dir/stealth/host-major-<hash>.txt`, enquanto sem semente ou sob `--no-stealth` nada dele toca o disco
- DEVE ler `user_agent_major_source` em todo envelope de `scrape`, onde `projected` indica User-Agent sobrescrito, `host_binary` major lido do Chrome deste host e `host_unprobed` a tabela da crate sem sonda
- DEVE tratar `user_agent_major_source` igual a `null` como stealth desligado, ou no `--engine browser` como nenhum lançamento ainda
- DEVE saber que o patch NUNCA emula `navigator.userAgentData`, que só existe em contexto seguro, e que headed com o profile do host mantém o objeto e o User-Agent nativos do Chrome
- DEVE saber que headless ou profile estrangeiro envia no override um `userAgentMetadata` completo, então o JavaScript, o `getHighEntropyValues` e todo header `sec-ch-ua-*` contam a mesma versão
- DEVE ler `planned_version_source` de `doctor --fingerprint` como `null` com override, `chrome_binary` sem override e com binário sondado, e `crate_table` quando a sonda falhou
- DEVE ler o mismatch `ua_data_brands_vs_user_agent` como divergência de major entre `ua_data_brands` e o User-Agent
- DEVE passar `--input-profile human` para ritmo humano ou `direct` para um evento por ação
- DEVE passar `--input-seed <SEED>` para reproduzir exatamente uma execução `human`
- DEVE passar `--warmup` para visitar a raiz da origem antes do alvo, ou `--warmup-url <URL>` para aquecer outra URL
### Rede Proxy e Robots
- DEVE passar `--proxy <URL>` com `http`, `https` ou `socks5` como proxy de saída do Chrome e do motor HTTP
- DEVE passar `--proxy-bypass <HOSTS>` na sintaxe de bypass-list do Chrome
- DEVE gravar credencial de proxy SOMENTE com `config set proxy_username` e `config set proxy_password`, e NUNCA em argv
- DEVE passar `--min-delay-ms <MS>` para elevar o piso de cortesia, sabendo que vale o MÁXIMO entre a flag, `scrape_min_delay_ms` e `Crawl-delay`
- DEVE passar `--ignore-robots` e `--i-accept-robots-risk` JUNTAS, conforme a seção `Residual-Zero e Robots`
### Portões de Categoria
- DEVE passar `--category-memory` para `heap` e `--category-extensions` para `extension`
- DEVE passar `--category-third-party` para `devtools3p` e `--category-webmcp` para `webmcp`
- DEVE passar `--experimental-vision` para `click-at` e `--experimental-screencast` para `screencast`
- NUNCA ligue portão sem a família que o exige
### MITM
- DEVE passar `--mitm` para rotear o Chrome deste processo por um proxy MITM local
- DEVE passar `--mitm-har <FILE>` para gravar HAR no FINALIZE e `--mitm-hosts <HOSTS>` para decifrar só esses hosts
- DEVE passar `--mitm-ca-dir <DIR>` SOMENTE para trocar o diretório da CA
- DEVE passar `--mitm-max-body-bytes <BYTES>` para limitar corpo retido e `--mitm-no-media-bodies` para descartar mídia
- DEVE saber que `--mitm-ws` e `--mitm-redact-secrets` só reafirmam o padrão e NÃO mudam nada
- DEVE passar `--mitm-no-redact-secrets` para manter `Authorization` e `Cookie` legíveis, sabendo que pedir os dois resolve MASCARANDO
### Asserção
- DEVE passar `--expect <EXPR>` com `key=value`, `key!=value` ou `key~substring`, repetível e com AND
- DEVE ler `agent_ops.expectation_unmet` para ver cada expectativa falha
- DEVE passar `--expect-exit-code` para sair com 65, porque sem ela o exit continua 0
- EXECUTE `browser-automation-cli --json --fields offline --expect 'offline=true' --expect-exit-code doctor --offline --quick`


## Configuração XDG
### OBRIGATÓRIO
- DEVE configurar SOMENTE por flag e por `config init`, `config path`, `config show`, `config get`, `config set`, `config unset` e `config list-keys`
- DEVE saber que a flag SEMPRE vence a chave XDG gravada
- DEVE usar `config unset <chave>` para restaurar o padrão embutido
- DEVE gravar segredos como `openrouter_api_key` e `encryption_key` SOMENTE por `config set`
- DEVE gravar binários com `chrome_path`, `lighthouse_path` e `ffmpeg_path`
- DEVE usar `cache_backend` igual a `redis` SOMENTE com `cache_redis_url` em `redis://`
- DEVE mover o teto dos buffers de `net` e `console` SOMENTE com `config set event_tracker_max_entries`
- DEVE ajustar o fingerprint HTTP/2 do motor http SOMENTE pelas chaves `http2_*`, que NÃO têm flag
- DEVE consultar `references/xdg-keys.md` para padrão e descrição de toda chave
### PROIBIDO
- PROIBIDO usar variável de ambiente, `.env` ou `export` como configuração do produto
- NUNCA registre segredo, cookie ou token em log
### Fórmulas Prontas
- EXECUTE `browser-automation-cli --json config init` e `browser-automation-cli --json config path`
- EXECUTE `browser-automation-cli --json config show` e `browser-automation-cli --json config get timeout`
- EXECUTE `browser-automation-cli --json config set dialog_settle_ms 2000`
- EXECUTE `browser-automation-cli --json config unset dialog_settle_ms`


## Inventário de Comandos
### OBRIGATÓRIO
- DEVE conhecer estes 71 — doctor, commands, schema, version, locale, goto, view, press, click-at, write, keys, type, wait, hover, drag, submit, fill-form, select-option, pick, upload, back, forward, reload, eval, grab, print-pdf, monitor, run, exec, record, extract, text, scroll, cookie, storage, attr, assert, console, net, page, dialog, scrape, batch-scrape, crawl, map, sitemap, feed, search, parse, qr, image, video, audio, find-paths, sg-scan, sg-rewrite, sheet-write, mitm, workflow, config, emulate, resize, perf, lighthouse, screencast, heap, extension, devtools3p, webmcp, completions, man
- DEVE saber que `pick` e `select-option` saem com exit 2 como subcomando de topo e DEVEM ir por `exec` ou `run`
- DEVE saber que `console list`, `console get`, `net list` e `net get` saem com exit 2 no topo e existem SOMENTE como passo de `run`
- DEVE saber que `console clear` e `console dump` funcionam no topo
### Famílias Locais Sem Chrome
- DEVE usar `image`, `video` e `audio` para mídia local
- DEVE usar `parse` para extrair texto de HTML, Markdown, TXT, PDF, DOCX, XLSX e ODS
- DEVE usar `qr`, `sheet-write`, `find-paths`, `sg-scan` e `sg-rewrite` como ferramentas locais
- DEVE usar `workflow run`, `workflow resume` e `workflow status` para DAG com journal
- DEVE usar `sitemap`, `feed`, `map` e `search` pelo motor HTTP
- DEVE saber que `sg-rewrite` é dry-run por padrão e só grava com `--apply`
- EXECUTE `browser-automation-cli --json find-paths --glob '**/*.rs' --limit 200 .`
- EXECUTE `browser-automation-cli --json sg-scan . --limit 100` e `browser-automation-cli --json sg-rewrite . --apply` só após revisar o dry-run
- EXECUTE `browser-automation-cli --json sheet-write /tmp/rows.csv -o /tmp/out.xlsx --sheet Data --force`
- EXECUTE `browser-automation-cli --json workflow resume --manifest /tmp/wf.json --journal /tmp/wf.journal`


## Navegação e Interação
### OBRIGATÓRIO
- DEVE passar `goto <URL>` com `--navigation-timeout-ms` e `--handle-before-unload accept` ou `dismiss`
- DEVE passar `reload --ignore-cache` para recarga forçada, e NUNCA essa flag em `goto`
- DEVE passar `view --detailed` para a árvore de acessibilidade completa, e NUNCA `--verbose`
- DEVE passar `view --allow-empty` SOMENTE quando o snapshot em branco for intencional
- DEVE passar `type <TEXTO>` com `--target` ou com `--focus-only`
- DEVE passar `fill-form --fields-json` e `cookie set --cookies-json`, e NUNCA payload por `--json`
- DEVE passar `submit <ALVO>` para enviar o form e esperar navegação ou requisição
- DEVE passar `grab --path` e `print-pdf --path`, e NUNCA caminho posicional
- DEVE passar `print-pdf --url` em one-shot, porque página em branco é recusada
- DEVE passar `--include-snapshot` na ação para receber refs novas no mesmo processo
- DEVE ler `matched_selector` após `wait` com vários seletores
- DEVE saber que `select-option` e `pick` nativos reportam `via` igual a `native_select`
- DEVE saber que `storage export` grava em modo 0600
- DEVE passar `emulate` por flags e NUNCA inventar `--device`
### Fórmulas Prontas
- EXECUTE `browser-automation-cli --timeout 60 --json goto https://example.com --init-script 'window.__ready=1' --handle-before-unload accept --navigation-timeout-ms 15000`
- EXECUTE `browser-automation-cli --timeout 60 --json exec select-option --target '#prioridade' --option Alta`
- EXECUTE `browser-automation-cli --timeout 60 --json grab --path /tmp/p.webp --format webp --quality 80 --full-page`
- EXECUTE `browser-automation-cli --timeout 60 --json print-pdf --path /tmp/p.pdf --url https://example.com`
- EXECUTE `browser-automation-cli --timeout 60 --json --experimental-vision click-at --x 10 --y 20 --include-snapshot`
- EXECUTE `browser-automation-cli --timeout 60 --json storage export --path /tmp/auth.json --url https://example.com`
- EXECUTE `browser-automation-cli --timeout 60 --json storage import --path /tmp/auth.json --url https://example.com`
- EXECUTE `browser-automation-cli --timeout 60 --json cookie set --cookies-json '[{"name":"a","value":"b","url":"https://example.com"}]'`
- EXECUTE `browser-automation-cli --timeout 60 --json emulate --viewport '390x844x3,mobile,touch' --network-conditions 'Slow 3G'`


## Scripts Multi-passo
### OBRIGATÓRIO
- DEVE usar `run --script <arquivo>` com NDJSON de um passo por linha ou array JSON, e toda linha com a chave `cmd`
- DEVE saber que `--timeout` cobre o script inteiro e `--step-timeout` cobre cada passo
- DEVE usar `run --script -` para ler NDJSON do stdin numa sessão viva, com um BORN, um DIE e validação por linha
- DEVE saber que chave desconhecida num passo de `run` é RECUSADA com exit 2 ANTES de lançar o navegador
- DEVE serializar `console` e `net` como passos com `action` igual a `list` ou `get`
- DEVE refazer `view` após todo passo `eval`, porque ele emite `refs_invalidated` true e mata as refs
- DEVE reproduzir a gravação de `record` com `run --script` sobre o arquivo gravado
### PROIBIDO
- NUNCA use `run --script <(...)`, porque o jail de arquivos recusa process substitution
- NUNCA divida passos com `@eN` entre processos
- NUNCA ponha `mitm`, `storage`, `config`, `workflow`, `crawl`, `map`, `batch-scrape`, `search`, `parse`, `qr`, `find-paths`, `sg-scan`, `sg-rewrite`, `sheet-write`, `monitor`, `extension install` ou `extension uninstall` dentro de `run`
### Passos Prontos
- EXECUTE `browser-automation-cli --timeout 90 --json --json-steps --capture-network --capture-console run --script /tmp/steps.jsonl`
- EXECUTE `printf '%s\n' '{"cmd":"goto","url":"https://example.com"}' '{"cmd":"view"}' | browser-automation-cli --timeout 60 --json run --script -`
- EXECUTE `browser-automation-cli --timeout 60 --json record --url https://example.com --path /tmp/rec.jsonl --seconds 30 --max-events 200`
- EXECUTE o passo `{"cmd":"wait","selectors":["h1","main"],"wait_timeout_ms":10000}`
- EXECUTE o passo `{"cmd":"view","verbose":true}`
- EXECUTE o passo `{"cmd":"write","target":"@e1","value":"olá"}`
- EXECUTE o passo `{"cmd":"submit","target":"#user","timeout_ms":8000}`
- EXECUTE o passo `{"cmd":"pick","target":"@e2","option":"Alta"}`
- EXECUTE o passo `{"cmd":"dialog","action":"accept","if_present":true}`
- EXECUTE o passo `{"cmd":"net","action":"list","resource_types":"Document,XHR,Fetch","page_size":50}`
- EXECUTE o passo `{"cmd":"console","action":"list","types":"error,warning","include_preserved":true}`


## Scraping e Coleta
### OBRIGATÓRIO
- DEVE começar com `--engine http`, que NÃO sobe navegador
- DEVE trocar para `--engine browser` SOMENTE quando a página depender de JavaScript ou de Turnstile
- DEVE pedir formatos com `--format` em CSV ou repetindo a flag, conferindo os 15 valores em `scrape --help`
- DEVE saber que `attributes`, `html` e `rawHtml` são chaves distintas no `data`
- DEVE ler `unsupported_format` e `content_kind` ANTES da chave do formato pedido, e ler o corpo em `text` quando `unsupported_format` vier preenchido
- DEVE passar `--only-main-content` para recortar a página antes do parse
- DEVE usar `batch-scrape` para lista fechada, `crawl` para seguir links e `map` para só enumerar URLs
- DEVE usar `monitor check` com baseline para detectar mudança de página
- DEVE usar `extract --llm` ou `scrape --format json` SOMENTE com `openrouter_api_key` gravada
### PROIBIDO
- NUNCA trate `rawHtml` como alias de `html`
- NUNCA conclua corpo vazio a partir de chave ausente
- NUNCA use `--engine browser` por hábito, porque ele custa um Chrome inteiro
- NUNCA chame `crawl` quando `map` já responde
### Fórmulas Prontas
- EXECUTE `browser-automation-cli --json scrape https://example.com --format markdown,links,metadata --engine http --only-main-content`
- EXECUTE `browser-automation-cli --json scrape https://example.com --format attributes --attribute-selector a --attribute-name href`
- EXECUTE `browser-automation-cli --timeout 120 --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 4 --output-mode ndjson`
- EXECUTE `browser-automation-cli --timeout 300 --json crawl https://example.com --limit 20 --max-depth 2 --exclude-path /login`
- EXECUTE `browser-automation-cli --timeout 60 --json map https://example.com --limit 200 --use-sitemap true`
- EXECUTE `browser-automation-cli --timeout 60 --json sitemap https://example.com --limit 100`
- EXECUTE `browser-automation-cli --timeout 60 --json feed https://example.com/feed.xml`
- EXECUTE `browser-automation-cli --timeout 60 --json search 'example domain' --limit 10 --country br`
- EXECUTE `browser-automation-cli --json parse /tmp/doc.pdf --redact-pii`
- EXECUTE `browser-automation-cli --timeout 120 --json extract https://example.com --llm --question 'Qual é o título' --schema-json /tmp/s.json`
- EXECUTE `browser-automation-cli --timeout 60 --json monitor check --url https://example.com --baseline /tmp/b.txt --write-baseline --diff-mode json`
- EXECUTE `browser-automation-cli --json qr encode --text https://example.com --format png --path /tmp/qr.png` e `browser-automation-cli --json qr decode --path /tmp/qr.png`


## Rede Console e MITM
### OBRIGATÓRIO
- DEVE usar `net` para o tráfego do PRÓPRIO processo vivo e `mitm` para captura em arquivo relida por outro processo
- DEVE passar `resource_types` como UMA lista CSV casada de forma exata contra o vocabulário CDP, como `Document`, `XHR`, `Fetch`, `WebSocket` e `Other`
- DEVE esperar exit 2 para tipo desconhecido, ANTES de qualquer lançamento
- DEVE ler `dropped_oldest` em `net` e `console` e reconstruir o total como `total` mais `dropped_oldest`
- DEVE passar `include_preserved` em `list` e em `get` para que o índice aponte o MESMO registro
- DEVE ler `data.capture_path` após `mitm capture-url` e repassar esse arquivo com `--capture-path`
- DEVE usar `--capture-path` em `mitm status`, `mitm list`, `mitm get`, `mitm har`, `mitm export`, `mitm domains`, `mitm apis`, `mitm graphql`, `mitm ws list` e `mitm ws get`
- DEVE estreitar a decifragem com `--hosts` e o registro com `--capture-hosts`, porque o Chrome gera tráfego de fundo
- DEVE tratar zero endpoints em `mitm apis` numa página estática como resposta honesta
- DEVE usar `mitm redact --secrets false` SOMENTE para gravar a política persistente sem máscara
### Fórmulas Prontas
- EXECUTE `browser-automation-cli --json mitm init-ca`
- EXECUTE `browser-automation-cli --timeout 60 --json mitm capture-url https://example.com --seconds 30 --har /tmp/c.har --hosts example.com --capture-hosts example.com`
- EXECUTE `browser-automation-cli --json mitm domains --capture-path /tmp/capture.json`
- EXECUTE `browser-automation-cli --json mitm export --format ndjson --out /tmp/c.ndjson --capture-path /tmp/capture.json`
- EXECUTE `browser-automation-cli --json mitm block --host example.com --path /ads`
- EXECUTE `browser-automation-cli --timeout 60 --json --mitm --mitm-har /tmp/run.har --mitm-no-media-bodies goto https://example.com`
- EXECUTE `browser-automation-cli --timeout 60 --json --capture-console assert console-no-match --pattern TypeError`


## APIs pela Página
### OBRIGATÓRIO
- DEVE lembrar que `eval` executa no contexto de origem da PÁGINA
- DEVE navegar para a origem alvo ANTES do `fetch`, senão ele devolve `Failed to fetch`
- DEVE encadear `goto` e `eval` num único `run --script` com `typed` true para ler `data.value` e `data.value_type`
- DEVE envolver todo `fetch` em try/catch e devolver a mensagem de erro
- DEVE tratar valor nulo com exit 0 de promise rejeitada como falha silenciosa
- DEVE saber que promise é resolvida automaticamente, sem chave de await
- DEVE usar `mitm apis` e `mitm graphql` para descobrir endpoints REST e GraphQL ANTES de chamá-los
- DEVE passar `eval --file-path` para resultado grande e `--service-worker-id` para executar num service worker
### Fórmulas Prontas
- EXECUTE `printf '%s\n' '{"cmd":"goto","url":"https://example.com"}' '{"cmd":"eval","expression":"(async()=>{try{const r=await fetch(\"/api\");return r.status}catch(e){return String(e)}})()","typed":true}' | browser-automation-cli --timeout 90 --json run --script -`
- EXECUTE `browser-automation-cli --json mitm apis --kind rest --capture-path /tmp/capture.json`
- EXECUTE `browser-automation-cli --json mitm graphql --limit 20 --capture-path /tmp/capture.json`


## Mídia Local
### OBRIGATÓRIO
- DEVE usar `image info`, `image convert`, `image resize`, `image download` e `image exif`
- DEVE usar `video info`, `video download`, `video convert`, `video to-mp3`, `video trim`, `video thumbnail` e `video manifest`
- DEVE usar `audio info`, `audio download`, `audio convert` e `audio trim`
- DEVE apontar ffmpeg com `config set ffmpeg_path` quando ele não estiver no PATH
- DEVE projetar mídia com `--select` e processar lote com `--paths-file`
- DEVE tratar webp local como lossless, com `quality_applied` false
- DEVE tratar `--keep-exif` como intenção, com `keep_exif_honored` false
- DEVE ler texto de imagem como agente, porque a CLI NÃO tem OCR
- NUNCA peça base64 de pixels, frames crus ou PCM no stdout, salvo `grab --include-base64` intencional
- NUNCA rode ffmpeg manual quando `video convert` ou `audio convert` resolve
- NUNCA use AVIF nem HEIC como formato de saída
### Fórmulas Prontas
- EXECUTE `browser-automation-cli --json image convert --path /tmp/a.png --format jpeg --quality 85 -o /tmp/a.jpg`
- EXECUTE `browser-automation-cli --json image resize --path /tmp/a.png --width 800 --keep-aspect -o /tmp/b.png`
- EXECUTE `browser-automation-cli --json image info --path /tmp/a.png --select format,width,height,sha256`
- EXECUTE `browser-automation-cli --json image exif --path /tmp/a.jpg --select tags`
- EXECUTE `browser-automation-cli --json video info --path /tmp/v.mp4 --select container,duration_secs,streams`
- EXECUTE `browser-automation-cli --timeout 300 --json video convert --path /tmp/v.mov --format mp4 --video-codec h264 -o /tmp/v.mp4`
- EXECUTE `browser-automation-cli --timeout 120 --json video trim --path /tmp/v.mp4 --start 10 --duration 5 -o /tmp/c.mp4`
- EXECUTE `browser-automation-cli --json video manifest --path /tmp/m.m3u8 --base-url https://example.com/m.m3u8`
- EXECUTE `browser-automation-cli --timeout 120 --json audio convert --path /tmp/a.wav --format opus --bitrate 96k -o /tmp/a.opus`


## Diagnóstico Perf e Extensões
### OBRIGATÓRIO
- DEVE apontar o Lighthouse com `--lighthouse-path` ou `config set lighthouse_path`
- DEVE usar `perf start --reload --auto-stop --path` para trace de carga num único processo, e `perf insight --path` para analisar offline
- DEVE passar `heap take --url`, porque sem URL o alvo é `about:blank`
- DEVE analisar o snapshot com `heap summary`, `heap details`, `heap class-nodes`, `heap compare`, `heap dominators`, `heap dup-strings`, `heap edges`, `heap retainers`, `heap paths`, `heap object-details` e `heap close`
- DEVE usar `screencast start` com diretório e `screencast stop` com arquivo `.webm` ou `.mp4`
- DEVE usar `extension install`, `extension list`, `extension reload`, `extension trigger` e `extension uninstall`
- DEVE passar `--url` em `devtools3p list`, `devtools3p exec`, `webmcp list` e `webmcp exec`, porque one-shot NÃO tem página aberta
- DEVE usar `page info`, `page list`, `page new`, `page select`, `page close` e `page tab-id` para abas do próprio processo
### Fórmulas Prontas
- EXECUTE `browser-automation-cli --timeout 180 --json lighthouse https://example.com --out-dir /tmp/lh --device mobile`
- EXECUTE `browser-automation-cli --timeout 90 --json perf start --reload --auto-stop --path /tmp/trace.json`
- EXECUTE `browser-automation-cli --json perf insight --path /tmp/trace.json --name LCPBreakdown`
- EXECUTE `browser-automation-cli --timeout 90 --json --category-memory heap take --path /tmp/s.heapsnapshot --url https://example.com`
- EXECUTE `browser-automation-cli --json --category-memory heap retainers --path /tmp/s.heapsnapshot --node 42 --page-size 20`
- EXECUTE `browser-automation-cli --timeout 60 --json --experimental-screencast screencast stop --path /tmp/cast.webm`
- EXECUTE `browser-automation-cli --timeout 60 --json --category-extensions extension install /tmp/ext`
- EXECUTE `browser-automation-cli --timeout 60 --json --category-webmcp webmcp list --url https://example.com`


## Residual-Zero e Robots
### OBRIGATÓRIO
- DEVE tratar residual-zero como parte do sucesso de todo one-shot com navegador
- DEVE validar com `doctor --offline --quick`, lendo `data.residual` e o check `residual_disk`
- DEVE exigir `residual_disk` diferente de `fail`, com zero em `orphan_marker_dirs` e em `ghost_marker_processes`
- DEVE tratar `sibling_live_processes` maior que zero como concorrência saudável com `warn`
- DEVE saber que `config set user_data_dir` abre mão do residual-zero, e que `config unset user_data_dir` restaura
- DEVE respeitar robots por padrão e contornar SOMENTE com AMBAS `--ignore-robots` e `--i-accept-robots-risk`
- EXECUTE `browser-automation-cli --json --fields residual,checks --filter-rows 'id=residual_disk' doctor --offline --quick`
### PROIBIDO
- NUNCA declare residual-zero sem ler `data.residual`
- NUNCA apague temporários genéricos do host nem mate Chrome do usuário
- NUNCA contorne robots com uma flag só
