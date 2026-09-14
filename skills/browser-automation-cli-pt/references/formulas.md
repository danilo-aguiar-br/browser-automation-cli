# Fórmulas Prontas de browser-automation-cli


## Contrato das Fórmulas
- DEVE copiar cada fórmula literalmente e trocar SOMENTE o valor de exemplo
- DEVE invocar SEMPRE o binário `browser-automation-cli` por extenso com `--json`
- DEVE checar o exit code, exigir `ok` igual a true e só então ler `data`
- DEVE conferir cada flag em `browser-automation-cli <cmd> --help` e cada chave de passo em `schema <cmd>` antes de adaptar
- DEVE passar `--timeout` explícito em toda fórmula que abre navegador
- DEVE rodar `pick` e `select-option` SOMENTE por `exec` ou por passo de `run`, porque como subcomando de topo saem com exit 2
- DEVE rodar `console list`, `console get`, `net list` e `net get` SOMENTE como passo de `run`, porque no topo saem com exit 2
- DEVE saber que chave desconhecida num passo de `run` ou de `exec` é RECUSADA com exit 2 e `error.kind` igual a `usage` antes de lançar o navegador
- DEVE manter todo passo com ref `@eN` dentro de UM único `run --script`, porque a ref morre com o processo
- DEVE contornar robots SOMENTE com AMBAS `--ignore-robots` e `--i-accept-robots-risk`
- NUNCA rode `config set chrome_legacy_oxide_launch true`, porque esse caminho reabre porta DevTools sem autenticação e desenha a janela no display do operador
- NUNCA embuta `mitm`, `storage`, `config`, `workflow` nem `extension install` e `extension uninstall` dentro de `run`


## Meta e Descoberta
- EXECUTE `browser-automation-cli --json doctor --offline --quick` e LEIA `data.residual` e o check `residual_disk`
- EXECUTE `browser-automation-cli --json --fields checks --filter-rows 'id=virtual_display' doctor --offline --quick` e LEIA `browser_mode_auto_resolves`, NUNCA a mensagem do check
- EXECUTE `browser-automation-cli --json --timeout 60 doctor --fingerprint` e LEIA `launch_args`, que traz o argv real entregue ao Chrome e vale `null` antes de qualquer lançamento
- EXECUTE `browser-automation-cli --json --timeout 60 doctor --fix` e APLIQUE a sugestão de reparo anexada a cada check que falhou
- EXECUTE `browser-automation-cli --json commands --detail` e LEIA a descrição, a categoria e as superfícies de cada comando
- EXECUTE `browser-automation-cli --json schema goto` e LEIA as chaves aceitas pelo comando
- EXECUTE `browser-automation-cli --json schema --cmd run` como a forma equivalente por flag
- EXECUTE `browser-automation-cli --json version` e LEIA a versão do binário
- EXECUTE `browser-automation-cli --json locale` e LEIA o idioma resolvido
- EXECUTE `browser-automation-cli --json completions bash` e TROQUE `bash` por `zsh`, `fish`, `elvish` ou `powershell` conforme o shell
- EXECUTE `browser-automation-cli --json man --out /tmp/browser-automation-cli.1` e LEIA a página roff gravada


## Configuração XDG
- EXECUTE `browser-automation-cli --json config path` e LEIA os caminhos XDG resolvidos, NUNCA invente um caminho
- EXECUTE `browser-automation-cli --json config init` para criar o layout XDG e o `config.toml` padrão
- EXECUTE `browser-automation-cli --json config show` e LEIA os valores efetivos
- EXECUTE `browser-automation-cli --json config list-keys` e LEIA toda chave aceita com o padrão antes de qualquer `config set`
- EXECUTE `browser-automation-cli --json config get timeout` e LEIA o valor de UMA chave
- EXECUTE `browser-automation-cli --json config get` e LEIA o conjunto inteiro quando nenhuma chave for passada
- EXECUTE `browser-automation-cli --json config set dialog_settle_ms 2000` e CONFIRA a chave em `config list-keys` antes de gravar
- EXECUTE `browser-automation-cli --json config set proxy_username operador` e GUARDE credencial de proxy SOMENTE por `proxy_username` e `proxy_password`, NUNCA em argv
- EXECUTE `browser-automation-cli --json config set browser_mode headless` para gravar o padrão persistente que `--browser-mode`, `--headless` e `--headed` vencem numa execução
- EXECUTE `browser-automation-cli --json config set user_data_dir /tmp/perfil-persistente` SOMENTE como decisão explícita de abrir mão do residual-zero
- EXECUTE `browser-automation-cli --json config unset user_data_dir` para restaurar o padrão ausente que preserva o residual-zero
- EXECUTE `browser-automation-cli --json config unset dialog_settle_ms` para devolver a chave ao padrão embutido, NUNCA `config set` com string vazia
- EXECUTE `browser-automation-cli --json --fields keys --filter-rows 'key=chrome_legacy_oxide_launch' config list-keys` e EXIJA `default` igual a `false`, sem NUNCA gravar `true` nessa chave


## Flags Globais
- EXECUTE `browser-automation-cli --json --json-steps --timeout 90 --step-timeout 20 run --script /tmp/passos.jsonl` e LEIA um objeto NDJSON por passo no stdout
- EXECUTE `browser-automation-cli --json --quiet --plain --lang pt-BR --correlation-id pedido-42 version` e LEIA `correlation_id` ecoado no envelope
- EXECUTE `browser-automation-cli --json -q --lang en version` e SAIBA que `pt` puro é recusado e que o JSON de máquina fica em inglês
- EXECUTE `browser-automation-cli --json --verbose doctor --offline --quick` para stderr em nível info
- EXECUTE `browser-automation-cli --json --debug doctor --offline --quick` para o detalhe máximo de tracing no stderr
- EXECUTE `browser-automation-cli --json --timeout 120 --max-concurrency 4 batch-scrape --urls-file /tmp/urls.txt --format text` para limitar o paralelismo de I/O
- EXECUTE `browser-automation-cli --json --timeout 60 --browser-mode auto goto https://example.com` e LEIA `browser_mode_requested`, `browser_mode_effective`, `browser_mode_source` e `display_backend`
- EXECUTE `browser-automation-cli --json --timeout 60 --headless goto https://example.com` para EXIGIR headless nesta execução acima de qualquer `browser_mode` gravado
- EXECUTE `browser-automation-cli --json --timeout 60 --headed goto https://example.com` e SAIBA que no Linux com Xvfb a janela é desenhada no Xvfb privado, fora da tela do operador, e `display_backend` vale `xvfb`
- EXECUTE `browser-automation-cli --json --timeout 60 --headed --no-xvfb goto https://example.com` como a ÚNICA rota para ver a janela no display atual e LEIA `display_backend` igual a `host`
- EXECUTE `browser-automation-cli --json --timeout 60 --no-stealth goto https://example.com` para desligar os patches anti-detecção nesta execução
- EXECUTE `browser-automation-cli --json --timeout 60 --stealth-profile auto --stealth-seed frota-42 goto https://example.com` para fixar a mesma identidade entre processos
- EXECUTE `browser-automation-cli --json --stealth-profile list version` para listar os perfis `auto`, `chrome-linux`, `chrome-win` e `chrome-mac`
- EXECUTE `browser-automation-cli --json --timeout 60 --warmup goto https://example.com/pagina/profunda` para visitar a raiz da origem antes do alvo
- EXECUTE `browser-automation-cli --json --timeout 60 --warmup-url https://example.com/login goto https://example.com/pagina/profunda` para aquecer outra URL no lugar da raiz
- EXECUTE `browser-automation-cli --json --timeout 60 --input-profile human --input-seed 7 run --script /tmp/passos.jsonl` para reproduzir o mesmo jitter e TROQUE por `--input-profile direct` para um evento por ação
- EXECUTE `browser-automation-cli --json --timeout 60 --artifacts-dir /tmp/artefatos --dump-on-failure --capture-console --capture-network run --script /tmp/passos.jsonl` para gravar evidência de console e rede quando falhar
- EXECUTE `browser-automation-cli --json --timeout 60 --min-delay-ms 1500 scrape https://example.com --format text` e SAIBA que a espera efetiva é o MÁXIMO entre a flag, `scrape_min_delay_ms` e o `Crawl-delay`
- EXECUTE `browser-automation-cli --json --timeout 60 --proxy socks5://127.0.0.1:1080 --proxy-bypass 'localhost,127.0.0.1' scrape https://example.com --format text` e NUNCA ponha usuário ou senha na URL do proxy
- EXECUTE `browser-automation-cli --json --allow-outside-roots parse /var/tmp/relatorio.pdf` SOMENTE como aceitação explícita de risco para ler fora das raízes permitidas
- EXECUTE `browser-automation-cli --json --timeout 60 --ignore-robots --i-accept-robots-risk scrape https://example.com --format text` com as DUAS flags, porque uma sozinha NÃO contorna robots
- EXECUTE `browser-automation-cli --json --timeout 120 --category-memory heap take --path /tmp/s.heapsnapshot --url https://example.com` para liberar a família `heap`
- EXECUTE `browser-automation-cli --json --timeout 60 --category-extensions extension list` para liberar a família `extension`
- EXECUTE `browser-automation-cli --json --timeout 60 --category-third-party devtools3p list --url https://example.com` para liberar a família `devtools3p`
- EXECUTE `browser-automation-cli --json --timeout 60 --category-webmcp webmcp list --url https://example.com` para liberar a família `webmcp`
- EXECUTE `browser-automation-cli --json --timeout 60 --experimental-vision click-at --x 10 --y 20` para liberar `click-at`
- EXECUTE `browser-automation-cli --json --timeout 60 --experimental-screencast screencast start --path /tmp/cast` para liberar `screencast`
- EXECUTE `browser-automation-cli --json --timeout 60 --mitm --mitm-har /tmp/c.har --mitm-hosts example.com --mitm-ca-dir /tmp/ca goto https://example.com` para gravar HAR, estreitar a decriptação e apontar a CA
- EXECUTE `browser-automation-cli --json --timeout 60 --mitm --mitm-max-body-bytes 65536 --mitm-no-media-bodies --mitm-ws --mitm-redact-secrets goto https://example.com` e SAIBA que `--mitm-ws` e `--mitm-redact-secrets` só reafirmam o padrão
- EXECUTE `browser-automation-cli --json --timeout 60 --mitm --mitm-no-redact-secrets goto https://example.com` SOMENTE quando o próprio segredo for o objeto da depuração
- EXECUTE `browser-automation-cli --json --fields checks --filter-rows 'status!=pass' --sort-rows id --dedupe-by id --limit-rows 5 doctor --offline --quick` e LEIA `agent_ops.truncated`
- EXECUTE `browser-automation-cli --json --fields checks --count-only doctor --offline --quick` e LEIA a contagem em vez das linhas
- EXECUTE `browser-automation-cli --json --timeout 60 --truncate-content 200 --max-output-bytes 4096 scrape https://example.com --format markdown` e LEIA `agent_ops.truncated` e `agent_ops.unresolved_paths`
- EXECUTE `browser-automation-cli --json --fields checks --filter-rows 'id=residual_disk' --expect 'status=pass' --expect-exit-code doctor --offline --quick` para sair com 65 quando a expectativa falhar


## Navegação
- EXECUTE `browser-automation-cli --json --timeout 60 goto https://example.com --init-script 'window.__pronto=1' --handle-before-unload accept --navigation-timeout-ms 15000` e LEIA `browser_mode_effective`
- EXECUTE `browser-automation-cli --json --timeout 60 back` para voltar no histórico do processo
- EXECUTE `browser-automation-cli --json --timeout 60 forward` para avançar no histórico do processo
- EXECUTE `browser-automation-cli --json --timeout 60 reload --ignore-cache --init-script 'window.__pronto=1' --handle-before-unload dismiss` e NUNCA passe `--ignore-cache` a `goto`
- EXECUTE `browser-automation-cli --json --timeout 60 wait --ms 500 --text Example --selector 'h1, main' --min-count 1 --state load --wait-timeout-ms 10000 --include-snapshot` e LEIA `matched_selector`
- EXECUTE `browser-automation-cli --json --timeout 60 wait --network-idle 500 --dom-stable 300 --wait-timeout-ms 15000` para esperar rede ociosa e DOM estável


## Interação
- EXECUTE `browser-automation-cli --json --timeout 60 press @e1 --dblclick --include-snapshot` e LEIA o snapshot anexado
- EXECUTE `browser-automation-cli --json --timeout 60 --experimental-vision click-at --x 120 --y 340 --dblclick --include-snapshot` para clicar por coordenada CSS
- EXECUTE `browser-automation-cli --json --timeout 60 write @e2 'olá' --include-snapshot` para preencher input, select, checkbox ou radio
- EXECUTE `browser-automation-cli --json keys Enter --timeout 60 --include-snapshot` para pressionar uma tecla
- EXECUTE `browser-automation-cli --json --timeout 60 type 'olá mundo' --target @e2 --clear --submit Enter --include-snapshot` para digitar num alvo
- EXECUTE `browser-automation-cli --json --timeout 60 type 'olá mundo' --focus-only` para digitar no elemento JÁ focado sem resolver alvo
- EXECUTE `browser-automation-cli --json --timeout 60 hover @e1 --include-snapshot` para passar o ponteiro sobre o alvo
- EXECUTE `browser-automation-cli --json --timeout 60 drag --from @e1 --to @e2 --anchor before --synthetic-payload '{"items":[{"mimeType":"text/plain","data":"x"}],"dragOperationsMask":1}' --include-snapshot` e SAIBA que o payload sintético ignora o `dragstart` da página
- EXECUTE `browser-automation-cli --json --timeout 60 drag --from @e1 --to-x 400 --to-y 220` para soltar numa coordenada absoluta
- EXECUTE `browser-automation-cli --json --timeout 60 submit '#login' --timeout-ms 8000 --include-snapshot` para submeter o form e esperar navegação ou requisição
- EXECUTE `browser-automation-cli --json --timeout 60 fill-form --fields-json '[{"target":"@e3","value":"x"}]' --include-snapshot` e NUNCA passe o payload por `--json`
- EXECUTE `browser-automation-cli --json --timeout 60 upload @e4 /tmp/arquivo.txt --include-snapshot` para anexar arquivo a um input de arquivo
- EXECUTE `browser-automation-cli --json --timeout 60 scroll --target @e5 --delta-x 100 --delta-y 400 --include-snapshot` para rolar por delta
- EXECUTE `browser-automation-cli --json --timeout 60 scroll --to-x 0 --to-y 2000` para rolar a janela até um deslocamento absoluto
- EXECUTE `browser-automation-cli --json --timeout 60 exec pick --target @e1 --option Anomalia` e LEIA `via`
- EXECUTE `browser-automation-cli --json --timeout 60 exec select-option --target @e2 --option Alta` e LEIA `via` igual a `native_select` em select nativo
- EXECUTE `browser-automation-cli --json --timeout 90 run --script /tmp/escolha.jsonl` com as linhas `{"cmd":"goto","url":"https://example.com"}` e `{"cmd":"view"}` e `{"cmd":"pick","target":"@e1","option":"Anomalia"}` e `{"cmd":"select-option","target":"@e2","option":"Alta"}`


## Leitura e Artefatos
- EXECUTE `browser-automation-cli --json --timeout 60 view --detailed --path /tmp/arvore.txt` e NUNCA use `view --verbose`
- EXECUTE `browser-automation-cli --json --timeout 60 view --allow-empty` SOMENTE quando o snapshot em branco for intencional
- EXECUTE `browser-automation-cli --json --timeout 60 text @e1` e LEIA o texto visível do alvo
- EXECUTE `browser-automation-cli --json --timeout 60 attr @e1 href` e LEIA o valor do atributo
- EXECUTE `browser-automation-cli --json --timeout 60 extract @e1 --attr href` para ler atributo em vez de texto
- EXECUTE `browser-automation-cli --json --timeout 120 extract --url https://example.com --llm --question 'Qual é o título?' --schema-json /tmp/esquema.json` com `openrouter_api_key` gravada no XDG
- EXECUTE `browser-automation-cli --json --timeout 60 eval '(el)=>el.textContent' --args '["@e1"]' --dialog-action accept --file-path /tmp/eval.json --typed` e LEIA `data.value` e `data.value_type`
- EXECUTE `browser-automation-cli --json --timeout 60 --category-extensions eval 'chrome.runtime.id' --service-worker-id sw-1` para avaliar dentro do service worker da extensão
- EXECUTE `browser-automation-cli --json --timeout 60 grab --path /tmp/pagina.png --format png --full-page` e NUNCA use caminho posicional
- EXECUTE `browser-automation-cli --json --timeout 60 grab --path /tmp/elemento.webp --format webp --quality 80 --element @e1 --include-base64` SOMENTE quando o base64 no envelope for necessário ao agente
- EXECUTE `browser-automation-cli --json --timeout 60 print-pdf --path /tmp/pagina.pdf --url https://example.com` com `--url` SEMPRE em one-shot
- EXECUTE `browser-automation-cli --json --timeout 60 assert url example.com --contains` para afirmar substring da URL
- EXECUTE `browser-automation-cli --json --timeout 60 assert text Example --target h1` para afirmar texto dentro do alvo


## Abas Cookies Storage e Diálogos
- EXECUTE `browser-automation-cli --json --timeout 60 page` e LEIA a URL e o título da página atual
- EXECUTE `browser-automation-cli --json --timeout 60 page info` como a forma explícita do mesmo relatório
- EXECUTE `browser-automation-cli --json --timeout 60 page list` para listar as abas deste processo
- EXECUTE `browser-automation-cli --json --timeout 60 page new --url https://example.com --background --isolated-context sessao-a` para abrir aba em contexto isolado sem foco
- EXECUTE `browser-automation-cli --json --timeout 60 page select 0 --bring-to-front` para selecionar a aba e trazer a janela à frente
- EXECUTE `browser-automation-cli --json --timeout 60 page select --page-id 1 --no-bring-to-front` para selecionar a aba sem erguer a janela
- EXECUTE `browser-automation-cli --json --timeout 60 page close --index 1` para fechar a aba por índice
- EXECUTE `browser-automation-cli --json --timeout 60 page close --page-id 1` como a forma por `pageId`
- EXECUTE `browser-automation-cli --json --timeout 60 page tab-id` e LEIA o id estável da aba ativa
- EXECUTE `browser-automation-cli --json --timeout 60 cookie list --url https://example.com` para listar cookies do escopo da URL
- EXECUTE `browser-automation-cli --json --timeout 60 cookie set --cookies-json '[{"name":"a","value":"b","url":"https://example.com"}]'` e NUNCA passe o payload por `--json`
- EXECUTE `browser-automation-cli --json --timeout 60 cookie clear --all` para limpar o pote inteiro deste processo
- EXECUTE `browser-automation-cli --json --timeout 60 storage export --path /tmp/auth.json --url https://example.com` e SAIBA que o arquivo nasce com modo 0600
- EXECUTE `browser-automation-cli --json --timeout 60 storage import --path /tmp/auth.json --url https://example.com` para restaurar cookies e storage da origem
- EXECUTE `browser-automation-cli --json --timeout 60 dialog accept --text Ana --if-present` e LEIA `data.dialog_settled`, sem wait artificial quando vier true
- EXECUTE `browser-automation-cli --json --timeout 60 dialog dismiss --if-present` para dispensar o diálogo sem falhar quando ele não existir


## Console e Rede
- EXECUTE `browser-automation-cli --json --timeout 90 --capture-console run --script /tmp/console.jsonl` com as linhas `{"cmd":"goto","url":"https://example.com"}` e `{"cmd":"console","action":"list","page_idx":0,"page_size":50,"types":"log,warning,error","include_preserved":true,"service_worker_id":"sw-1"}` e LEIA `dropped_oldest`
- EXECUTE `browser-automation-cli --json --timeout 90 --capture-console run --script /tmp/console.jsonl` com as linhas `{"cmd":"goto","url":"https://example.com"}` e `{"cmd":"console","action":"get","id":0,"include_preserved":true}` para ler UMA mensagem pelo mesmo índice da lista
- EXECUTE `browser-automation-cli --json --timeout 60 --capture-console console clear` para descartar as mensagens capturadas neste processo
- EXECUTE `browser-automation-cli --json --timeout 60 --capture-console console dump --path /tmp/console.json` para gravar as mensagens capturadas em arquivo
- EXECUTE `browser-automation-cli --json --timeout 90 --capture-network run --script /tmp/rede.jsonl` com as linhas `{"cmd":"goto","url":"https://example.com"}` e `{"cmd":"net","action":"list","page_idx":0,"page_size":50,"resource_types":"Document,XHR,Fetch","include_preserved":true}` e LEIA `resourceType` em cada registro
- EXECUTE `browser-automation-cli --json --timeout 90 --capture-network run --script /tmp/rede.jsonl` com as linhas `{"cmd":"goto","url":"https://example.com"}` e `{"cmd":"net","action":"get","id":"0","request_path":"/tmp/req.bin","response_path":"/tmp/res.bin","include_preserved":true}` para gravar os corpos da requisição
- EXECUTE `browser-automation-cli --json --timeout 60 --capture-console assert console --level error --max 0` no MESMO processo da captura
- EXECUTE `browser-automation-cli --json --timeout 60 --capture-console assert console-empty` para exigir zero mensagens de qualquer nível
- EXECUTE `browser-automation-cli --json --timeout 60 --capture-console assert console-no-match --pattern 'TypeError|ReferenceError'` para exigir que nenhuma mensagem case a regex


## Scrape e Coleta
- EXECUTE `browser-automation-cli --json --timeout 60 scrape https://example.com --format markdown,links,metadata,images,jsonld --engine http --only-main-content --select source_url,title,markdown --max-text-chars 800` e LEIA `unsupported_format` antes da chave pedida
- EXECUTE `browser-automation-cli --json --timeout 60 scrape https://example.com --format html,rawHtml,text --include-selector main --exclude-selector nav --redact-pii --with-content-hash --header 'Accept-Language: pt-BR' --no-cache true` e LEIA `html` e `rawHtml` como chaves DISTINTAS
- EXECUTE `browser-automation-cli --json --timeout 120 scrape https://example.com --format screenshot,summary,product,branding --engine browser --wait-ms 500 --action '{"cmd":"press","target":"#carregar-mais"}'` para agir na página antes de coletar
- EXECUTE `browser-automation-cli --json --timeout 120 scrape https://example.com --format json --schema-json /tmp/esquema.json --question 'Qual é o preço?'` com `openrouter_api_key` gravada no XDG
- EXECUTE `browser-automation-cli --json --timeout 60 scrape https://example.com --format attributes --attribute-selector a --attribute-name href --webhook-url http://127.0.0.1:8787/gancho` e PAREIE cada seletor com um nome na mesma ordem
- EXECUTE `browser-automation-cli --json --timeout 60 scrape https://example.com/feed.xml --format feed` e SAIBA que `--format` aceita os 15 valores text, markdown, html, rawHtml, links, metadata, screenshot, summary, product, branding, images, jsonld, json, feed e attributes
- EXECUTE `browser-automation-cli --json --timeout 120 batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2 --engine http --only-main-content --select source_url,text --max-text-chars 500 --filter http_error=false --output-mode csv --sort source_url --dedup-key source_url` para uma lista fechada de URLs
- EXECUTE `browser-automation-cli --json --timeout 180 batch-scrape --urls-file /tmp/urls.txt --format markdown --engine browser --output-mode ndjson --dedup-similar true --include-selector article --exclude-selector footer --redact-pii --with-content-hash --webhook-url http://127.0.0.1:8787/gancho` para páginas que dependem de JavaScript
- EXECUTE `browser-automation-cli --json --timeout 180 crawl https://example.com --limit 20 --max-depth 2 --format text --same-host --engine http --select source_url,text --max-text-chars 500 --filter http_error=false --output-mode ndjson --sort source_url --dedup-key source_url --only-main-content --include-selector main --exclude-selector nav --redact-pii --with-content-hash` para descobrir páginas a partir da semente
- EXECUTE `browser-automation-cli --json --timeout 180 crawl https://example.com --no-same-host --include-path /docs --exclude-path /tag --include-regex 'guia' --exclude-regex 'rascunho' --use-sitemap true --ignore-query-params --follow-rel-next true --dedup-similar true --output-mode llms-txt --webhook-url http://127.0.0.1:8787/gancho` para gerar resumo de site
- EXECUTE `browser-automation-cli --json --timeout 60 crawl https://example.com --sitemap-only --dry-run` e LEIA o plano efetivo sem buscar nada
- EXECUTE `browser-automation-cli --json --timeout 120 map https://example.com --limit 50 --max-depth 2 --select urls,count --include-path /docs --exclude-path /tag --use-sitemap true --search guia --sort url --dedup-key url --include-subdomains --ignore-query-params` e LEIA `urls` e `count`
- EXECUTE `browser-automation-cli --json --timeout 60 map https://example.com --sitemap-only` para listar SOMENTE URLs do sitemap
- EXECUTE `browser-automation-cli --json --timeout 60 sitemap https://www.rust-lang.org --limit 50 --select urls,count --include-path /learn --exclude-path /tag --search guia --sort url --dedup-key url --include-subdomains --ignore-query-params` e LEIA `urls` como lista de strings
- EXECUTE `browser-automation-cli --json --timeout 60 feed https://blog.rust-lang.org/feed.xml --select title,source_url,feed --header 'Accept-Language: en' --no-cache true` para ler RSS, Atom ou JSON Feed sem navegador
- EXECUTE `browser-automation-cli --json --timeout 60 search 'example domain' --limit 10 --select results --sort url --dedup-key url --include-domains example.com --country br --search-lang pt --time-filter w` e LEIA `serp_endpoint`
- EXECUTE `browser-automation-cli --json --timeout 60 search 'example domain' --exclude-domains pinterest.com` e TRATE `ok` false com `error.kind` igual a `data` como busca sem resultado orgânico, lendo `data.serp_endpoint` e `data.search_base_url`
- EXECUTE `browser-automation-cli --json parse /tmp/documento.pdf --redact-pii --format text,markdown,summary` para extrair texto de pdf, docx, xlsx ou ods sem navegador
- EXECUTE `browser-automation-cli --json parse /tmp/pagina.html --format markdown,links,metadata` e SAIBA que HTML aceita todo formato de scrape


## Ferramentas Locais
- EXECUTE `browser-automation-cli --json find-paths '\.rs$' /tmp/projeto --extension rs --hidden --no-ignore --max-depth 4 --type f --limit 200` para enumerar caminhos locais sem navegador
- EXECUTE `browser-automation-cli --json find-paths --glob '**/*.toml' /tmp/projeto --type f` para filtrar por glob
- EXECUTE `browser-automation-cli --json sg-scan /tmp/projeto --limit 100` e LEIA os achados estruturais
- EXECUTE `browser-automation-cli --json sg-rewrite /tmp/projeto` para o relatório dry-run antes de gravar
- EXECUTE `browser-automation-cli --json sg-rewrite /tmp/projeto --apply` SOMENTE depois de revisar o dry-run


## Imagem
- EXECUTE `browser-automation-cli --json image info --path /tmp/a.jpg --include-gps --select format,width,height,sha256,exif` e LEIA as dimensões e o sha256
- EXECUTE `browser-automation-cli --json image info --paths-file /tmp/imagens.txt --select path,format,width,height` para inspecionar um lote
- EXECUTE `browser-automation-cli --json image info --stdin --select format,bytes` com os bytes da imagem no stdin
- EXECUTE `browser-automation-cli --json image convert --path /tmp/a.png --format jpeg --quality 85 -o /tmp/a.jpg --strip-exif` e NUNCA peça avif nem heic
- EXECUTE `browser-automation-cli --json image convert --stdin --format webp --out /tmp/a.webp --keep-exif` e LEIA `keep_exif_honored` e `quality_applied`
- EXECUTE `browser-automation-cli --json image convert --paths-file /tmp/imagens.txt --format png` para converter um lote
- EXECUTE `browser-automation-cli --json image resize --path /tmp/a.png --width 640 --keep-aspect -o /tmp/a-640.jpg --format jpeg --quality 80` para redimensionar pixels
- EXECUTE `browser-automation-cli --json image resize --stdin --width 320 --height 240 --out /tmp/b.png` com os bytes no stdin
- EXECUTE `browser-automation-cli --json image resize --paths-file /tmp/imagens.txt --width 1024 --keep-aspect` para redimensionar um lote
- EXECUTE `browser-automation-cli --json --timeout 60 image download https://example.com/a.png -o /tmp/a.png --max-bytes 10485760 --require-image` para baixar com verificação de magic
- EXECUTE `browser-automation-cli --json --timeout 60 image download https://example.com/dado.bin --out /tmp/dado.bin --allow-non-image` SOMENTE para bytes crus intencionais
- EXECUTE `browser-automation-cli --json image exif --path /tmp/a.jpg --include-gps --select path,count,exif` e LEIA as tags EXIF
- EXECUTE `browser-automation-cli --json image exif --stdin --select tags` com os bytes no stdin
- EXECUTE `browser-automation-cli --json image exif --paths-file /tmp/imagens.txt --select path,tag_count` para um lote


## Vídeo
- EXECUTE `browser-automation-cli --json video info --path /tmp/in.mp4 --select container,duration_secs,streams,sha256` e LEIA os streams sem despejo de mídia
- EXECUTE `browser-automation-cli --json video info --stdin --select container,duration` com o vídeo no stdin
- EXECUTE `browser-automation-cli --json video info --paths-file /tmp/videos.txt --select path,container` para um lote
- EXECUTE `browser-automation-cli --json --timeout 120 video download https://example.com/v.mp4 -o /tmp/v.mp4 --max-bytes 52428800 --require-video --select path,bytes` para baixar mídia direta
- EXECUTE `browser-automation-cli --json --timeout 120 video download https://example.com/v.bin --out /tmp/v.bin --allow-non-video` SOMENTE para corpo não vídeo intencional
- EXECUTE `browser-automation-cli --json --timeout 300 video convert --path /tmp/in.mov --format mp4 -o /tmp/out.mp4 --video-codec h264 --audio-codec aac --crf 23 --no-faststart --strip-metadata --select path_out,auto_reencoded,video_codec` e LEIA `auto_reencoded`
- EXECUTE `browser-automation-cli --json --timeout 300 video convert --stdin --format webm --out /tmp/out.webm --drop-audio` com o vídeo no stdin
- EXECUTE `browser-automation-cli --json --timeout 600 video convert --paths-file /tmp/videos.txt --format mkv` para converter um lote
- EXECUTE `browser-automation-cli --json --timeout 300 video to-mp3 --path /tmp/in.mp4 -o /tmp/a.mp3 --bitrate 192k --audio-stream 0 --select path_out` para extrair o áudio
- EXECUTE `browser-automation-cli --json --timeout 300 video to-mp3 --stdin --out /tmp/b.mp3` com o vídeo no stdin
- EXECUTE `browser-automation-cli --json --timeout 600 video to-mp3 --paths-file /tmp/videos.txt` para um lote
- EXECUTE `browser-automation-cli --json --timeout 300 video trim --path /tmp/in.mp4 --start 5 --duration 10 -o /tmp/corte.mp4 --format mp4 --video-codec copy --audio-codec copy --select path_out,duration` para cortar por duração
- EXECUTE `browser-automation-cli --json --timeout 300 video trim --stdin --start 5 --to 15 --out /tmp/corte.webm` para cortar até um instante
- EXECUTE `browser-automation-cli --json --timeout 600 video trim --paths-file /tmp/videos.txt --start 0 --duration 30` para um lote
- EXECUTE `browser-automation-cli --json --timeout 120 video thumbnail --path /tmp/in.mp4 --at 3 -o /tmp/quadro.png --select path_out` para extrair um quadro
- EXECUTE `browser-automation-cli --json --timeout 120 video thumbnail --stdin --out /tmp/quadro.jpg` com o vídeo no stdin
- EXECUTE `browser-automation-cli --json --timeout 300 video thumbnail --paths-file /tmp/videos.txt --at 1` para um lote
- EXECUTE `browser-automation-cli --json video manifest --path /tmp/lista.m3u8 --base-url https://example.com/hls/lista.m3u8 --select kind,variant_count,representations` para resumir HLS ou DASH sem baixar mídia
- EXECUTE `browser-automation-cli --json video manifest --stdin --select kind` com o manifesto no stdin
- EXECUTE `browser-automation-cli --json video manifest --paths-file /tmp/manifestos.txt` para um lote


## Áudio
- EXECUTE `browser-automation-cli --json audio info --path /tmp/in.wav --select format,codec,duration,bytes,sha256` e LEIA o codec e a duração
- EXECUTE `browser-automation-cli --json audio info --stdin --select codec` com o áudio no stdin
- EXECUTE `browser-automation-cli --json audio info --paths-file /tmp/audios.txt --select path,codec` para um lote
- EXECUTE `browser-automation-cli --json --timeout 120 audio download https://example.com/a.mp3 -o /tmp/a.mp3 --max-bytes 20971520 --require-audio --select path,bytes` para baixar mídia direta
- EXECUTE `browser-automation-cli --json --timeout 120 audio download https://example.com/a.bin --out /tmp/a.bin --allow-non-audio` SOMENTE para corpo não áudio intencional
- EXECUTE `browser-automation-cli --json --timeout 300 audio convert --path /tmp/in.wav --format mp3 -o /tmp/a.mp3 --codec mp3 --bitrate 192k --sample-rate 44100 --channels 2 --audio-stream 0 --strip-metadata --select path_out,lossy_transcode,suggestion` e LEIA `lossy_transcode`
- EXECUTE `browser-automation-cli --json --timeout 300 audio convert --stdin --format flac --out /tmp/a.flac` com o áudio no stdin
- EXECUTE `browser-automation-cli --json --timeout 600 audio convert --paths-file /tmp/audios.txt --format opus` para um lote
- EXECUTE `browser-automation-cli --json --timeout 300 audio trim --path /tmp/a.mp3 --start 1 --duration 5 -o /tmp/corte.mp3 --format mp3 --codec mp3 --bitrate 128k --select path_out,duration` para cortar por duração
- EXECUTE `browser-automation-cli --json --timeout 300 audio trim --stdin --start 1 --to 6 --out /tmp/corte.ogg` para cortar até um instante
- EXECUTE `browser-automation-cli --json --timeout 600 audio trim --paths-file /tmp/audios.txt --start 0 --duration 30` para um lote


## Emulação e Perf
- EXECUTE `browser-automation-cli --json --timeout 60 emulate --user-agent 'Mozilla/5.0' --locale pt-BR --timezone America/Sao_Paulo --latitude=-23.55 --longitude=-46.63 --media print --network-conditions 'Slow 3G' --cpu-throttling-rate 4 --color-scheme dark --extra-headers '{"X-Teste":"1"}' --viewport '390x844x3,mobile,touch' --screen 390x844` e NUNCA use `--device`
- EXECUTE `browser-automation-cli --json --timeout 60 emulate --offline` para forçar a página offline
- EXECUTE `browser-automation-cli --json --timeout 60 resize --width 1280 --height 720 --scale 2 --mobile --screen 1280x720` para redimensionar o viewport
- EXECUTE `browser-automation-cli --json --timeout 90 perf start --path /tmp/trace.json --reload --auto-stop` para gravar o trace do carregamento
- EXECUTE `browser-automation-cli --json --timeout 90 perf stop --path /tmp/trace.json` e LEIA os conjuntos de insight disponíveis
- EXECUTE `browser-automation-cli --json perf insight --path /tmp/trace.json --name LCPBreakdown` para analisar um trace salvo offline e NUNCA combine `--path` com `--insight-set-id`
- EXECUTE `browser-automation-cli --json --timeout 90 perf insight --insight-set-id set-1 --insight-name DocumentLatency` para analisar o conjunto da sessão viva
- EXECUTE `browser-automation-cli --json --timeout 180 lighthouse https://example.com --out-dir /tmp/lh --device mobile --mode navigation --lighthouse-path /usr/local/bin/lighthouse` e LEIA `data.binary_source`, NUNCA tratando `mock` como validação
- EXECUTE `browser-automation-cli --json --timeout 60 --experimental-screencast screencast start --path /tmp/cast` com `--path` como DIRETÓRIO dos quadros
- EXECUTE `browser-automation-cli --json --timeout 60 --experimental-screencast screencast stop --path /tmp/cast.webm` com `--path` como ARQUIVO de vídeo


## Heap
- EXECUTE `browser-automation-cli --json --timeout 120 --category-memory heap take --path /tmp/s.heapsnapshot --url https://example.com` com `--url` SEMPRE, porque sem ele o one-shot fotografa `about:blank`
- EXECUTE `browser-automation-cli --json --category-memory heap close --path /tmp/s.heapsnapshot` para liberar o handle aberto
- EXECUTE `browser-automation-cli --json --category-memory heap compare --base /tmp/a.heapsnapshot --current /tmp/b.heapsnapshot --class-index 3` e LEIA o crescimento do lado `--current`
- EXECUTE `browser-automation-cli --json --category-memory heap summary --path /tmp/s.heapsnapshot` e LEIA os totais por classe
- EXECUTE `browser-automation-cli --json --category-memory heap details --path /tmp/s.heapsnapshot --filter-name Array --page-idx 0 --page-size 50` para paginar os detalhes de classe
- EXECUTE `browser-automation-cli --json --category-memory heap class-nodes --path /tmp/s.heapsnapshot --id 7 --filter-name Array --page-idx 0 --page-size 50` para listar nós de uma classe
- EXECUTE `browser-automation-cli --json --category-memory heap dominators --path /tmp/s.heapsnapshot --node 42` e NUNCA use `--node-id`
- EXECUTE `browser-automation-cli --json --category-memory heap dup-strings --path /tmp/s.heapsnapshot --page-idx 0 --page-size 50` para listar strings duplicadas
- EXECUTE `browser-automation-cli --json --category-memory heap edges --path /tmp/s.heapsnapshot --node 42 --page-idx 0 --page-size 50` para listar arestas de saída
- EXECUTE `browser-automation-cli --json --category-memory heap retainers --path /tmp/s.heapsnapshot --node 42 --page-idx 0 --page-size 50` para listar quem retém o nó
- EXECUTE `browser-automation-cli --json --category-memory heap paths --path /tmp/s.heapsnapshot --node 42 --max-depth 8 --max-nodes 10000 --max-siblings 20` para enumerar caminhos até a raiz do GC
- EXECUTE `browser-automation-cli --json --category-memory heap object-details --path /tmp/s.heapsnapshot --node 42` e LEIA tamanho, distância e tamanho retido


## Extensões e Terceiros
- EXECUTE `browser-automation-cli --json --timeout 60 --category-extensions extension list` para listar as extensões carregadas NESTE processo
- EXECUTE `browser-automation-cli --json --timeout 60 --category-extensions extension install /tmp/extensao` para lançar o Chrome com a extensão desempacotada
- EXECUTE `browser-automation-cli --json --timeout 60 --category-extensions extension reload abcdefghijklmnop --path /tmp/extensao` para recarregar a extensão pelo id
- EXECUTE `browser-automation-cli --json --timeout 60 --category-extensions extension trigger abcdefghijklmnop --path /tmp/extensao` para disparar a ação pelo service worker
- EXECUTE `browser-automation-cli --json --timeout 60 --category-extensions extension uninstall abcdefghijklmnop` para desinstalar a extensão pelo id
- EXECUTE `browser-automation-cli --json --timeout 60 --category-third-party devtools3p list --url https://example.com` com `--url`, porque sem ele a descoberta roda em página em branco
- EXECUTE `browser-automation-cli --json --timeout 60 --category-third-party devtools3p exec NomeDaFerramenta --params '{}' --url https://example.com` para executar uma ferramenta pelo nome
- EXECUTE `browser-automation-cli --json --timeout 60 --category-webmcp webmcp list --url https://example.com` para listar as ferramentas declaradas pela página
- EXECUTE `browser-automation-cli --json --timeout 60 --category-webmcp webmcp exec NomeDaFerramenta --input '{}' --url https://example.com` para executar uma ferramenta pelo nome


## MITM
- EXECUTE `browser-automation-cli --json mitm init-ca` UMA vez para criar a CA local no XDG
- EXECUTE `browser-automation-cli --json --timeout 60 mitm capture-url https://example.com --seconds 20 --har /tmp/c.har --hosts example.com --capture-hosts example.com` e LEIA `data.capture_path`
- EXECUTE `browser-automation-cli --json --timeout 45 mitm start --seconds 30` para subir o proxy em 127.0.0.1 com porta efêmera
- EXECUTE `browser-automation-cli --json mitm status --capture-path /tmp/captura.json` e LEIA os caminhos da CA e a política de bind
- EXECUTE `browser-automation-cli --json mitm list --host example.com --limit 50 --capture-path /tmp/captura.json` para listar as trocas capturadas
- EXECUTE `browser-automation-cli --json mitm get 0 --capture-path /tmp/captura.json` para ler UMA troca pelo id
- EXECUTE `browser-automation-cli --json mitm har --out /tmp/c.har --capture-path /tmp/captura.json` para exportar HAR 1.2
- EXECUTE `browser-automation-cli --json mitm export --format ndjson --out /tmp/c.ndjson --capture-path /tmp/captura.json` para exportar a captura em `json` ou `ndjson`
- EXECUTE `browser-automation-cli --json mitm domains --capture-path /tmp/captura.json` e FILTRE hosts de fundo do navegador antes de concluir
- EXECUTE `browser-automation-cli --json mitm apis --kind rest --capture-path /tmp/captura.json` e TRATE zero endpoints em página estática como resposta honesta
- EXECUTE `browser-automation-cli --json mitm graphql --limit 100 --capture-path /tmp/captura.json` para listar operações GraphQL
- EXECUTE `browser-automation-cli --json mitm ws list --limit 100 --capture-path /tmp/captura.json` para listar frames WebSocket
- EXECUTE `browser-automation-cli --json mitm ws get 0 --capture-path /tmp/captura.json` para ler UM frame pelo id
- EXECUTE `browser-automation-cli --json mitm block --host example.com --path /anuncios` para bloquear host e prefixo de caminho
- EXECUTE `browser-automation-cli --json mitm allow --host example.com` para incluir o host na allowlist de intercepção TLS
- EXECUTE `browser-automation-cli --json mitm redact` SEM `--secrets` para MOSTRAR a política efetiva sem gravar nada
- EXECUTE `browser-automation-cli --json mitm redact --secrets false` para parar de mascarar de forma persistente e TROQUE por `--secrets true` para restaurar


## Workflow Run Exec e Record
- EXECUTE `browser-automation-cli --json --timeout 300 workflow run --manifest /tmp/wf.json --journal /tmp/wf.journal` para validar o DAG e executar os passos
- EXECUTE `browser-automation-cli --json --timeout 300 workflow resume --manifest /tmp/wf.json --journal /tmp/wf.journal` para retomar a partir do journal
- EXECUTE `browser-automation-cli --json workflow status --journal /tmp/wf.journal --name demo` e LEIA o estado de cada passo
- EXECUTE `browser-automation-cli --json --json-steps --timeout 90 run --script /tmp/passos.jsonl` com as linhas `{"cmd":"goto","url":"https://example.com","handle_before_unload":"accept","navigation_timeout_ms":15000}` e `{"cmd":"wait","selector":"h1, main","wait_timeout_ms":10000}` e `{"cmd":"view","verbose":true}` e LEIA `data.steps`
- EXECUTE `browser-automation-cli --json --timeout 90 run --script -` e ENVIE uma linha NDJSON por passo no stdin, NUNCA `<(...)`, que o jail de arquivos recusa
- EXECUTE `browser-automation-cli --json --timeout 60 exec goto https://example.com` como passo ÚNICO, NUNCA como multi-passo
- EXECUTE `browser-automation-cli --json --timeout 90 record --url https://example.com --path /tmp/gravacao.ndjson --seconds 30 --max-events 200` e SAIBA que o primeiro teto atingido vence
- EXECUTE `browser-automation-cli --json --json-steps --timeout 90 run --script /tmp/gravacao.ndjson` para reproduzir a gravação


## Monitor QR e Planilha
- EXECUTE `browser-automation-cli --json --timeout 60 monitor check --url https://example.com --baseline /tmp/linha-base.txt --write-baseline --engine http --diff-mode json` para comparar com a linha de base e dizer O QUE mudou
- EXECUTE `browser-automation-cli --json --timeout 120 monitor check --url https://example.com --baseline /tmp/linha-base.txt --engine browser --diff-mode git` para páginas que dependem de JavaScript
- EXECUTE `browser-automation-cli --json qr encode --text https://example.com --format png --path /tmp/qr.png` e TROQUE por `svg` ou `terminal`, omitindo `--path` para a matriz no stdout
- EXECUTE `browser-automation-cli --json qr decode --path /tmp/qr.png` e LEIA o conteúdo decodificado
- EXECUTE `browser-automation-cli --json sheet-write /tmp/linhas.csv -o /tmp/saida.xlsx --sheet Dados --force` para gravar XLSX a partir de CSV ou JSON
