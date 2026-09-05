# browser-automation-cli — Fórmulas de Argv

## Uso
- DEVE copiar fórmulas literalmente; trocar só placeholders; binário `browser-automation-cli` por extenso; `--json` SEMPRE
- DEVE parsear só stdout; checar exit antes de confiar; validar `.ok` com `jaq` antes de ler `.data`
- DEVE descobrir a superfície com `commands --json`, `schema <cmd> --json`, `config list-keys --json`
- NUNCA invente alias, env de produto ou flag ausente

## Contrato
- Após `dialog accept|dismiss` real DEVE ler `.data.dialog_settled`; quando true NÃO faça wait artificial antes da próxima observação da página
- Dialog multi-aba é indexado por `session_id`; troca de aba com dialog aberto habilita o domínio em regime best-effort
- DEVE definir o settle com `config set dialog_settle_ms`; run wait usa `wait_timeout_ms`; run scrape usa `format|formats` (text NÃO despeja html monstro)
- `select-option` e `pick` nativos disparam input e depois change, e reportam `via: native_select`
- `submit` submete o form ou o dono do campo e espera nav/request; storage exige `--path` (`--url` quando a origem precisa carregar no mesmo processo); export em 0600; FORA de run
- grab só png|jpeg|webp — NUNCA avif; lighthouse `binary_source` real|mock — NUNCA trate mock como validação de LHR
- mitm/storage/extension install|uninstall FORA de run; `exec` é passo único e multi-passo é `run --script`

## Globais
- DEVE executar `browser-automation-cli --json --json-steps --timeout 90 --step-timeout 20 --capture-console --capture-network run --script /tmp/steps.jsonl`
- DEVE executar `browser-automation-cli --json -q --plain --max-concurrency 4 --artifacts-dir /tmp/arts --correlation-id req-42 goto https://example.com`
- DEVE executar `browser-automation-cli --json --quiet goto https://example.com` como a forma longa de `-q`, que suprime o log humano de stderr sem tocar no stdout
- DEVE executar `browser-automation-cli --json --verbose doctor --offline --quick` para elevar stderr a info e `browser-automation-cli --json --debug doctor --offline --quick` para o detalhe máximo de tracing, ou persistir com `config set log_level`
- DEVE executar `browser-automation-cli --json --lang pt-BR version` e `browser-automation-cli --json --lang en version` para trocar o idioma da saída humana, sabendo que o JSON de máquina permanece em inglês e que `pt` puro é RECUSADO
- DEVE executar `browser-automation-cli --json --allow-outside-roots parse /caminho/fora/das/raizes/doc.pdf` como aceitação explícita de risco para ler local e gravar artefato fora das raízes permitidas
- DEVE executar `browser-automation-cli --json --dump-on-failure --artifacts-dir /tmp/arts --capture-console --capture-network goto https://example.com` para que uma falha grave a evidência de console e rede no diretório de artefatos
- DEVE executar `browser-automation-cli --json --min-delay-ms 1500 scrape https://example.com --format text --engine http`, sabendo que a espera efetiva é o MÁXIMO entre a flag, a chave XDG `scrape_min_delay_ms` e o `Crawl-delay` do site
- DEVE executar `browser-automation-cli --json --browser-mode headed goto https://example.com` e `browser-automation-cli --json --browser-mode headless goto https://example.com`, com `--headless` e `--headed` como atalhos de dois dos três valores
- DEVE executar `browser-automation-cli --json --headless goto about:blank` para EXIGIR headless nesta execução, o que vence uma chave XDG `browser_mode headed` gravada por outra tarefa
- MEDIDO: `browser-automation-cli --json --browser-mode headless goto about:blank` devolve `browser_mode_source` igual a `flag`, e sem a flag devolve `default`
- MEDIDO: `browser-automation-cli --help` de RAIZ tem quatorze linhas e NÃO imprime nenhuma flag global; a superfície global aparece em `<cmd> --help`, por exemplo `browser-automation-cli goto --help`
- NUNCA conclua que uma flag global não existe por não achá-la no `--help` de raiz; CONFIRME sempre no help de um subcomando
- DEVE passar `--category-memory` (heap), `--category-extensions` (extension), `--category-third-party` (devtools3p), `--category-webmcp` (webmcp)
- DEVE passar `--experimental-vision` (click-at) e `--experimental-screencast` (screencast)
- DEVE passar `--mitm` + `--mitm-har|--mitm-hosts|--mitm-ca-dir|--mitm-ws|--mitm-max-body-bytes|--mitm-no-media-bodies|--mitm-redact-secrets|--mitm-no-redact-secrets` só quando intercepção exigir
- DEVE executar `browser-automation-cli --timeout 60 --json --mitm --mitm-har /tmp/c.har --mitm-hosts example.com,api.example.com --mitm-ca-dir /tmp/ca goto https://example.com` para gravar HAR, estreitar a decriptação por host e apontar o diretório da CA
- DEVE executar `browser-automation-cli --timeout 60 --json --mitm --mitm-max-body-bytes 65536 --mitm-no-media-bodies --mitm-ws goto https://example.com` para limitar o corpo retido por troca, descartar corpos de imagem, vídeo e áudio, e reafirmar a captura de frames WebSocket
- DEVE executar `browser-automation-cli --timeout 60 --json --mitm --mitm-redact-secrets goto https://example.com` como reafirmação do padrão e `--mitm-no-redact-secrets` como a única forma de desligá-lo
- `--mitm-ws` reafirma o default: frames WebSocket são sempre capturados sob `--mitm`, então passar a flag não muda nada
- DEVE saber que a redação de segredos na captura MITM é LIGADA por padrão, então `--mitm-redact-secrets` apenas a reafirma e não muda nada
- DEVE executar `browser-automation-cli --json --mitm --mitm-no-redact-secrets mitm capture-url https://example.com` para manter legíveis os valores de Authorization e Cookie nesta execução
- DEVE saber que existem DUAS rotas para desligar o mascaramento, a flag por processo acima e a política persistente `mitm redact --secrets false`, então NENHUMA delas é rota única
- DEVE saber que passar `--mitm-redact-secrets` e `--mitm-no-redact-secrets` juntas resolve MASCARANDO, porque a leitura segura de uma contradição sobre segredos é mascarar
- DEVE saber que o padrão é LIGADO porque a captura vai para disco e é lida depois por um agente, então esquecer a flag custa um cabeçalho ausente enquanto o padrão oposto custaria um cookie de sessão vazado
- NUNCA passe `--mitm-no-redact-secrets` salvo quando o próprio segredo for o objeto da depuração
- DEVE contornar robots só com ambas `--ignore-robots --i-accept-robots-risk`
- DEVE executar `browser-automation-cli --json --no-stealth goto https://example.com` só quando os patches anti-detecção precisarem ficar desligados; stealth é LIGADO por padrão
- DEVE executar `browser-automation-cli --json --stealth-profile auto goto https://example.com` (`chrome-linux|chrome-win|chrome-mac` só com plataforma estrangeira intencional)
- DEVE executar `browser-automation-cli --json --stealth-seed my-fleet-42 goto https://example.com` para fixar uma identidade entre processos one-shot
- DEVE executar `browser-automation-cli --json --stealth-profile list version` para listar os perfis válidos direto do binário
- DEVE executar `browser-automation-cli --json doctor --fingerprint --quick` para auditar a coerência de webdriver, plataforma e tela
- DEVE escrever `run --script` como heredoc entre aspas com um objeto JSON por linha física (nunca `printf` com aspas simples ao redor do JS)
- DEVE executar `browser-automation-cli --json --proxy socks5://127.0.0.1:1080 scrape https://example.com --format text --engine http`
- DEVE executar `browser-automation-cli --json --proxy http://127.0.0.1:8080 --proxy-bypass 'localhost,127.0.0.1,*.internal' goto https://example.com`
- DEVE guardar credenciais de proxy com `config set proxy_username` e `config set proxy_password`; NUNCA em argv, porque a tabela de processos expõe argv
- DEVE executar `browser-automation-cli --json --input-profile human --input-seed 7 press @e1` (`--input-profile direct` para um evento por ação)
- DEVE executar `browser-automation-cli --json --warmup goto https://example.com/deep/page`
- DEVE executar `browser-automation-cli --json --warmup-url https://example.com/login goto https://example.com/deep/page`
- DEVE executar `browser-automation-cli --json --headed --no-xvfb goto https://example.com` só no Linux com modo headed
- DEVE executar `browser-automation-cli --json --expect 'ok=true' --expect 'data.title~Example' scrape https://example.com --format metadata --engine http`
- DEVE executar `browser-automation-cli --json --expect 'ok=true' --expect-exit-code doctor --offline --quick` para transformar expectativa não atendida em exit 65

## Meta
- DEVE executar `browser-automation-cli --json doctor --offline --quick` e `doctor --fix` só se reparo for necessário
- DEVE executar `browser-automation-cli --json commands`; `schema goto`; `schema --cmd wait`; `version`; `locale`
- DEVE executar `browser-automation-cli completions bash` (zsh|fish|elvish|powershell); `man --out /tmp/browser-automation-cli.1`

## Config XDG
- DEVE executar `browser-automation-cli --json config init|path|show|list-keys`; `config get`; `config get timeout`; `config set <k> <v>`
- DEVE executar `browser-automation-cli --json config unset <CHAVE>` para restaurar uma chave ao default embutido
- DEVE saber que `config unset` é o inverso de `set`, enquanto `config set <chave> ""` NÃO é inverso na regra geral
- DEVE tratar `user_data_dir` como EXCEÇÃO medida a essa regra, porque `config set user_data_dir ""` limpa o opt-in em vez de gravar um nome de diretório vazio
- DEVE saber que o varredor de residual julga SOMENTE diretórios cujo nome começa com o prefixo de marcador do produto, e apenas sob as raízes que ele escaneia, então o perfil persistente do operador NUNCA é coletado
- DEVE saber que `config set <chave> ""` grava em chave string um valor vazio que o caminho normal nunca produz, e em chave numérica é erro de parse
- DEVE saber que desfazer chave já ausente tem sucesso, então um script nunca precisa saber o estado anterior
- DEVE descobrir a superfície viva de chaves com `config list-keys --json` antes de qualquer `config set`
- DEVE consultar `references/xdg-keys.md` para o inventário completo das chaves com padrão e descrição
- DEVE setar binários com `chrome_path`, `lighthouse_path`, `ffmpeg_path`
- DEVE setar segredos com `encryption_key` e `openrouter_api_key`
- DEVE setar cache com `cache_backend sqlite|memory|redis` e Redis plain em `cache_redis_url`
- DEVE setar comportamento com `dialog_settle_ms`, `log_level`, `lang en|pt-BR`, `timeout`, `artifacts_dir`, `http_ssrf_mode strict|allow_loopback|off`, `log_rotation daily|hourly|never`
- NUNCA reintroduza lista inline de chaves nesta skill; a lista inline envelhece e trunca a superfície viva
- NUNCA `rediss://`; NUNCA logue segredos; NUNCA redis sem `cache_redis_url`

## Navegação / espera / snapshot / interação
- DEVE executar `browser-automation-cli --timeout 60 --json goto https://example.com --init-script 'window.__x=1' --handle-before-unload accept --navigation-timeout-ms 15000`
- DEVE executar `browser-automation-cli --json back`; `forward`; `reload --ignore-cache`
- DEVE executar `browser-automation-cli --json wait --ms 500`; `wait --selector "h1, main, #content" --wait-timeout-ms 10000 --include-snapshot`; `wait --text Example --wait-timeout-ms 5000`; `wait --state networkidle --wait-timeout-ms 15000`
- DEVE executar `browser-automation-cli --json view --detailed`; `view --path /tmp/view.txt --allow-empty` só se blank intencional
- DEVE executar `browser-automation-cli --json press @e1 --dblclick --include-snapshot`; `--experimental-vision click-at --x 10 --y 20`
- DEVE executar `browser-automation-cli --json write @e2 "olá"`; `keys Enter`; `type "olá" --target @e2 --clear --submit Enter`; `hover @e1`; `drag --from @e1 --to @e2`
- DEVE executar `browser-automation-cli --json type "olá" --focus-only --clear --submit Enter` para digitar no elemento JÁ focado sem resolver alvo, que é a alternativa exclusiva a `--target`
- DEVE executar `browser-automation-cli --json fill-form --fields-json '[{"target":"@e3","value":"x"}]'`; `upload @e4 /tmp/a.txt`; `submit "#user" --timeout-ms 8000 --include-snapshot`
- DEVE executar `browser-automation-cli --json exec pick --target @e1 --option Anomalia`; `exec select-option --target @e2 --option Alta`; `scroll --delta-y 400 --delta-x 100`
- Passos — `{"cmd":"pick","target":"@e1","option":"Anomalia"}` · `{"cmd":"select-option","target":"@e2","option":"Alta"}` · `{"cmd":"submit","target":"#user","timeout_ms":8000}` · `{"cmd":"wait","selector":"h1","wait_timeout_ms":10000}`
- NUNCA `--ignore-cache` em goto; NUNCA `view --verbose`; NUNCA `fill-form --json` payload

## Leitura / artefatos
- DEVE executar `browser-automation-cli --json extract @e1 --attr href`; `--timeout 120 extract --llm --question "título?" --schema-json /tmp/s.json https://example.com`
- DEVE executar `browser-automation-cli --json text @e1`; `attr @e1 href`; `eval 'document.title' --file-path /tmp/eval.json`; `eval '(el)=>el.textContent' --args '["@e1"]' --dialog-action accept`
- DEVE executar `browser-automation-cli --category-extensions --json eval 'chrome.runtime.id' --service-worker-id <sw-id>`
- DEVE executar `browser-automation-cli --json grab --path /tmp/p.png --format png --full-page`; `grab --path /tmp/p.webp --format webp --quality 80 --element @e1`
- DEVE executar `browser-automation-cli --timeout 60 --json print-pdf --path /tmp/p.pdf --url https://example.com`
- Passos — `{"cmd":"grab","path":"/tmp/p.png","format":"png"}` · `{"cmd":"print-pdf","path":"/tmp/p.pdf","url":"https://example.com"}`
- NUNCA grab avif; NUNCA omita `--path` em grab/print-pdf; NUNCA omita `--url` em print-pdf one-shot

## Abas / cookies / storage / dialog / assert
- DEVE executar `browser-automation-cli --json page list`; `page info`; `page new --isolated-context s-a --url https://example.com`; `page select 0 --bring-to-front`; `page close --index 1`; `page tab-id`
- DEVE executar `browser-automation-cli --json cookie list --url https://example.com`; `cookie set --cookies-json '[{"name":"a","value":"b","url":"https://example.com"}]'`; `cookie clear --all`
- DEVE executar `browser-automation-cli --json storage export --path /tmp/auth.json --url https://example.com`; `storage import --path /tmp/auth.json --url https://example.com`
- DEVE executar `browser-automation-cli --json dialog accept --text Ana --if-present`; `dialog dismiss --if-present`
- DEVE executar `browser-automation-cli --json assert url example.com --contains`; `assert text "Example" --target h1`; `--capture-console assert console-empty`; `--capture-console assert console --level error --max 0`
- DEVE executar `browser-automation-cli --capture-console --json assert console-no-match --pattern 'TypeError|ReferenceError'`, em que `--pattern` é OBRIGATÓRIO e é uma expressão regular que NENHUMA mensagem de console pode casar
- DEVE passar `--capture-console` no MESMO processo de `assert console-no-match`, porque sem captura o buffer está vazio e a asserção passa por ausência de dado e não por ausência de erro
- Passo — `{"cmd":"dialog","action":"accept","if_present":true}`

## Console / rede
- DEVE executar `browser-automation-cli --capture-console --json console clear`; `console dump --path /tmp/console.json`; `console list`, `console get`, `net list` e `net get` são passos de `run --script` e recusam no topo com exit 2
- DEVE serializar `{"cmd":"console","action":"list","types":"log,warning,error"}` e `{"cmd":"console","action":"get","id":0}` em `run --script` com `--capture-console`
- DEVE serializar `{"cmd":"net","action":"list","resource_types":"Document,XHR,Fetch"}` e `{"cmd":"net","action":"get","id":0,"request_path":"/tmp/req.json","response_path":"/tmp/res.json"}` em `run --script` com `--capture-network`
- DEVE tirar todo token de `resource_types` do vocabulário CDP, sob pena de recusa com exit 2 — Document, Stylesheet, Image, Media, Font, Script, TextTrack, XHR, Fetch, Prefetch, EventSource, WebSocket, Manifest, SignedExchange, Ping, CSPViolationReport, Preflight, FedCM, Other
- DEVE ler `resourceType` em cada registro e `dropped_oldest` no envelope, e mover o teto só com `config set event_tracker_max_entries <N>`
- DEVE capturar console/rede no MESMO processo dos comandos

## Scrape / coleta / locais
- DEVE executar `browser-automation-cli -q --json scrape https://example.com --format markdown --select source_url,title,markdown --max-text-chars 800 --only-main-content`
- DEVE executar multi-format `scrape … --format markdown,jsonld --select source_url,title,markdown` · `--redact-pii --with-content-hash` · browser `--engine browser --wait-ms 500`
- DEVE executar `browser-automation-cli -q --json scrape https://example.com --format rawHtml --engine http`
- DEVE executar `browser-automation-cli -q --ignore-robots --i-accept-robots-risk --json search "<Q>" --limit 10` e tratar busca sem resultado orgânico como FALHA DECLARADA, porque o comando devolve `ok` falso com `error.kind` igual a `data` e exit 65, e NUNCA `ok` verdadeiro com `count` zero
- DEVE saber que `serp_endpoint` prova o descasamento de endpoint nos DOIS envelopes, porque a classificação é feita no ponto em que o endpoint é decidido
- DEVE ler `data.serp_endpoint` e `data.search_base_url` no envelope de falha, que carregam a mesma informação da mensagem em forma legível por máquina
- NUNCA conclua que a web não tem resposta sem antes ler `data.serp_endpoint`, porque `unknown` acusa configuração errada e não ausência de resultado
- DEVE tratar `error.kind` igual a `data` em `search` como aviso de que o endpoint configurado devolveu apenas os próprios links de navegação, e conferir `search_base_url` antes de concluir que a web não tem resposta
- Formatos — os 14 valores de `--format` são text, markdown, html, rawHtml, links, metadata, screenshot, summary, product, branding, images, jsonld, json, feed (aliases md meta body shot); aceita CSV ou repetição da flag
- Engines — `--engine` aceita SOMENTE `http` (reqwest mais scraper) e `browser` (CDP); o default vem da chave XDG `scrape_default_engine`, hoje `http`
- DEVE executar `browser-automation-cli -q --json sitemap https://www.rust-lang.org --limit 50 --select urls,count`; estreitar com `--search docs --include-path /blog --exclude-path /tag --include-subdomains --ignore-query-params --sort <CAMPO> --dedup-key <CAMPO>`
- DEVE ler `urls` como array de URLs em STRING e `count` como o tamanho dele; não existe objeto por URL, então NUNCA projete `loc` nem `lastmod`
- DEVE executar `browser-automation-cli -q --json feed https://blog.rust-lang.org/feed.xml --select title,source_url,feed`; acrescentar `--header "Accept-Language: en"` e `--no-cache` só quando necessário
- DEVE saber que `sitemap` lê o sitemap declarado e `feed` lê RSS ou Atom, então NENHUM dos dois sobe navegador
- NUNCA confunda o COMANDO `feed` com o valor `--format feed` do `scrape`; são superfícies diferentes

## Redução de Payload Vale Para Toda Fórmula Acima
- DEVE encolher qualquer envelope com as oito flags GLOBAIS `--fields`, `--filter-rows`, `--limit-rows`, `--sort-rows`, `--dedupe-by`, `--count-only`, `--truncate-content`, `--max-output-bytes`
- DEVE executar `browser-automation-cli --json --fields checks --sort-rows id --dedupe-by id doctor --offline --quick` para ordenar por caminho pontilhado, com números comparando numericamente, e descartar linha cujo valor repete, guardando a primeira
- DEVE executar `browser-automation-cli --json --fields checks --truncate-content 120 --max-output-bytes 4096 doctor --offline --quick` para cortar toda string a N caracteres e impor um teto duro de bytes emitidos, que solta linhas do fim e marca `truncated`
- DEVE executar `browser-automation-cli --json --fields commands --max-items 3 commands`, porque `--max-items` é alias ACEITO de `--limit-rows` e carrega a grafia do contrato agent-native usado por outras CLIs deste ecossistema
- MEDIDO: essa invocação devolve três itens com `agent_ops.total` 71, `agent_ops.matched` 71 e `agent_ops.truncated` verdadeiro
- DEVE distinguir `--limit-rows` e seu alias `--max-items`, que limitam o que é EMITIDO, do `--limit` local de um comando, que limita o que é BUSCADO, porque são dois números genuinamente diferentes
- DEVE executar `browser-automation-cli --json --fields checks --filter-rows 'id=residual_disk' --limit-rows 5 doctor --offline --quick`
- DEVE ancorar todo caminho de `--fields` em `data` e NUNCA escrever o prefixo `data.`, porque a forma prefixada devolve payload vazio com exit 0
- DEVE ler `agent_ops.truncated` e `agent_ops.unresolved_paths` antes de confiar em payload reduzido
- NUNCA confunda essas oito com as flags LOCAIS `--select`, `--filter`, `--sort` e `--limit` que comandos individuais expõem
- NUNCA trate `rawHtml` como alias de `html`; `--format html` devolve a chave `html` e `--format rawHtml` devolve a chave `rawHtml`, com payloads distintos
- DEVE executar `batch-scrape --urls-file /tmp/u.txt --filter http_error=false --output-mode csv --select source_url,text` · `batch-scrape --urls-file /tmp/u.txt --format text --concurrency 2 --engine browser` · `crawl … --dedup-key source_url --output-mode ndjson` · `map … --search docs --sitemap-only`
- DEVE executar `browser-automation-cli --json crawl https://example.com --limit 20 --max-depth 2 --format text --same-host`; `map https://example.com --limit 50`; `search "example domain" --limit 10`
- DEVE executar `browser-automation-cli --json parse /tmp/doc.pdf`; `parse /tmp/planilha.ods --redact-pii`; `parse /tmp/page.html --format markdown,links,metadata` (HTML aceita todo formato de scrape; pdf/docx/sheet/csv/txt aceitam text, markdown e summary, e recusam pelo nome os formatos só de DOM com exit 2)
- DEVE executar `browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/b.baseline --write-baseline --engine http`
- DEVE executar `browser-automation-cli --json qr encode --text https://example.com --format png --path /tmp/qr.png`; `qr decode --path /tmp/qr.png`
- DEVE executar `browser-automation-cli --json image info --path /tmp/a.png --select format,width,height,sha256`; `image convert --path /tmp/a.png --format webp -o /tmp/a.webp`; `image download https://example.com/a.png -o /tmp/a.png`; `image resize --path /tmp/a.png --width 640 --keep-aspect -o /tmp/a-640.webp --format webp --quality 80`
- DEVE executar `browser-automation-cli --json video info --path /tmp/in.mp4 --select format,bytes,path` (aliases → container/size_bytes); `video convert --path /tmp/in.mp4 --format webm -o /tmp/out.webm --select path_out,auto_reencoded,bytes_out`; `video to-mp3 --path /tmp/in.mp4 -o /tmp/a.mp3`; `video trim --path /tmp/in.mp4 --start 0 --duration 0.5 -o /tmp/clip.mp4`; `video thumbnail --path /tmp/in.mp4 --at 0 -o /tmp/thumb.png`; `--timeout 120 video download https://example.com/v.mp4 -o /tmp/v.mp4 --max-bytes 52428800 --require-video`
- DEVE setar caps video via XDG após list-keys: `video_max_input_bytes` `video_download_max_bytes` `video_default_container` `video_default_crf` `video_default_audio_bitrate` `ffmpeg_path` `ffmpeg_timeout_secs`
- DEVE executar `browser-automation-cli --json audio info --path /tmp/in.wav --select format,codec,duration,bytes,sha256` · `audio convert --path /tmp/in.wav --format mp3 -o /tmp/a.mp3` · `audio convert --path /tmp/clip.mp4 --format m4a -o /tmp/a.m4a` · `audio trim --path /tmp/a.mp3 --start 1 --duration 5 -o /tmp/cut.mp3` · `audio download https://example.com/a.mp3 -o /tmp/a.mp3` · depois `upload @e1 /tmp/a.mp3`
- DEVE setar caps audio via XDG após list-keys: `audio_max_input_bytes` `audio_download_max_bytes` `audio_default_format` `audio_default_bitrate` `ffmpeg_path` `ffmpeg_timeout_secs`
- NUNCA despeje bytes/base64 de mídia no stdout; só path→path; NUNCA alegue HLS/yt-dlp/encode pure-Rust como produto
- DEVE executar `browser-automation-cli --json find-paths --glob '**/*.rs' . --type f --limit 200`; `sg-scan . --limit 100`; `sg-rewrite .`; `sg-rewrite . --apply`; `sheet-write /tmp/rows.csv -o /tmp/out.xlsx --sheet Data`
- Passos — `{"cmd":"scrape","url":"https://example.com","format":"text"}` · `{"cmd":"scrape","url":"https://example.com","formats":"markdown,links"}`
- DEVE contornar robots só com `browser-automation-cli --ignore-robots --i-accept-robots-risk --json scrape https://example.com --format text --engine http`

## Emulação / perf / lighthouse / screencast / heap
- DEVE executar `browser-automation-cli --json emulate --user-agent "Mozilla/5.0" --viewport "390x844x3,mobile,touch" --network-conditions "Slow 3G" --color-scheme dark`
- DEVE executar `browser-automation-cli --json resize --width 1280 --height 720`; `perf start --path /tmp/trace.json --reload --auto-stop`; `perf stop --path /tmp/trace.json`; `perf insight --name DocumentLatency`
- DEVE executar `browser-automation-cli --json perf insight --path /tmp/trace.json` para analisar um trace salvo OFFLINE, sem subir navegador; o caminho é limitado pelas raízes permitidas, então um trace fora delas é recusado com `read path outside allowed roots`
- NUNCA combine `--path` com `--insight-set-id`: trace offline não tem conjunto de insight, e o par é RECUSADO com erro de uso em vez de analisar o arquivo inteiro em silêncio
- DEVE executar `browser-automation-cli --json lighthouse https://example.com --out-dir /tmp/lh` como a forma mínima do comando, com `<URL>` posicional e absoluta
- DEVE executar `browser-automation-cli --timeout 180 --json lighthouse https://example.com --out-dir /tmp/lh --device desktop --mode navigation` e ler `data.binary_source`
- DEVE passar `--device desktop|mobile`, sendo `desktop` o padrão, e `--mode navigation|snapshot`, sendo `navigation` o padrão e `snapshot` mapeado para navigation nesta CLI one-shot
- DEVE executar `browser-automation-cli --json lighthouse https://example.com --lighthouse-path /usr/local/bin/lighthouse` para apontar o binário externo, o que vence PATH e a chave XDG `lighthouse_path`
- DEVE executar `browser-automation-cli --experimental-screencast --json screencast start --path /tmp/cast`; `screencast stop --path /tmp/cast.webm`
- DEVE tratar `--path` de `screencast start` como o DIRETÓRIO em que os frames são bufferizados, e o `--path` de `screencast stop` como o ARQUIVO de vídeo escrito
- DEVE passar `--experimental-screencast` em AMBAS as invocações, porque o gate é global e não sobrevive ao DIE
- DEVE executar `browser-automation-cli --category-memory --json heap take --path /tmp/s.heapsnapshot --url https://example.com`; `heap close --path /tmp/s.heapsnapshot`; `heap summary --path /tmp/s.heapsnapshot`
- DEVE passar `heap take --url <URL>` SEMPRE, porque sem ele a sessão one-shot fotografa `about:blank`, que é uma medição correta de nada, e a política de robots vale nessa navegação como vale em `goto`
- DEVE tratar `--path` de `heap take` como obrigatório e apontar para arquivo `.heapsnapshot`
- DEVE executar `browser-automation-cli --category-memory --json heap compare --base /tmp/a.heapsnapshot --current /tmp/b.heapsnapshot --class-index 3`, com `--base` e `--current` OBRIGATÓRIOS e `--class-index` como filtro opcional de classe
- DEVE ler o crescimento sempre do lado `--current`, porque `--base` é a linha de base contra a qual o delta é reportado
- DEVE executar `browser-automation-cli --category-memory --json heap compare --base /tmp/a.heapsnapshot --current /tmp/b.heapsnapshot`; `heap details --path /tmp/s.heapsnapshot --filter-name Array`; `heap class-nodes --path /tmp/s.heapsnapshot --id 7`
- DEVE executar `browser-automation-cli --category-memory --json heap dominators --path /tmp/s.heapsnapshot --node 42`; `heap dup-strings --path /tmp/s.heapsnapshot`; `heap edges --path /tmp/s.heapsnapshot --node 42`; `heap retainers --path /tmp/s.heapsnapshot --node 42`; `heap paths --path /tmp/s.heapsnapshot --node 42`; `heap object-details --path /tmp/s.heapsnapshot --node 42`
- NUNCA `emulate --device`; NUNCA `--node-id` (use `--node`)

## Extensões / terceiros / MITM / workflow
- DEVE executar `browser-automation-cli --category-extensions --json extension list`; `extension install /tmp/ext`; `extension reload <id>`; `extension trigger <id>`; `extension uninstall <id>`
- MEDIDO: sem o gate, `browser-automation-cli --json extension list` devolve `ok` falso com `error.kind` igual a `capability-disabled` e a mensagem `extension requires --category-extensions`
- DEVE saber que `extension list` NÃO tem flag própria além das globais, e que ele lista somente as extensões carregadas NESTE processo one-shot
- DEVE encadear `extension install /tmp/ext` e `extension list` no MESMO processo, porque a extensão morre com o DIE
- DEVE executar `browser-automation-cli --category-third-party --json devtools3p list --url https://example.com`; `devtools3p exec Tool --params '{}'`
- DEVE saber que `--url` em `devtools3p list` é OPCIONAL e abre a página antes da descoberta; sem ele a descoberta roda contra a página em branco e lista nada
- DEVE executar `browser-automation-cli --category-webmcp --json webmcp list --url https://example.com`; `webmcp exec Tool --input '{}'`
- DEVE passar `--url` em `webmcp list` para abrir a página antes da descoberta, porque a lista de ferramentas vive no documento e não no binário
- DEVE executar `browser-automation-cli --json mitm init-ca`; `mitm start --seconds 30`; `mitm status`; `mitm list --limit 50`; `mitm get 0`; `mitm har --out /tmp/c.har`; `mitm export --format ndjson --out /tmp/c.ndjson`
- DEVE executar `browser-automation-cli --json mitm domains`; `mitm apis`; `mitm graphql --limit 100`; `mitm ws list --limit 50`; `mitm ws get 0`
- DEVE executar `browser-automation-cli --json mitm block --host example.com --path /ads`; `mitm allow --host example.com`
- DEVE executar `browser-automation-cli --json mitm redact` SEM argumento para MOSTRAR a política efetiva de mascaramento sem gravar nada
- DEVE executar `browser-automation-cli --json mitm redact --secrets false` para parar de mascarar e `--secrets true` para restaurar, porque `--secrets` exige valor explícito
- DEVE executar `browser-automation-cli --json mitm status` para ler caminhos da CA, contagem da captura e política de bind
- DEVE saber que NÃO existe o subcomando `mitm config`, e que `browser-automation-cli --json mitm config` devolve `ok` falso com `error.kind` igual a `usage`, a mensagem `error: unrecognized subcommand 'config'` e `exit_code` 2
- DEVE fazer a configuração do MITM por `mitm redact`, `mitm status`, `mitm init-ca` e pelas chaves XDG, e NUNCA por um `mitm config` inexistente
- DEVE saber que os subcomandos reais de `mitm` são `status`, `list`, `get`, `har`, `export`, `domains`, `apis`, `init-ca`, `start`, `capture-url`, `graphql`, `ws`, `block`, `allow` e `redact`
- DEVE executar `browser-automation-cli --timeout 60 --json mitm capture-url https://example.com --har /tmp/c.har`
- DEVE executar `browser-automation-cli --json workflow run --manifest /tmp/wf.json --journal /tmp/wf.journal`; `workflow resume --manifest /tmp/wf.json`; `workflow status --name demo`
- NUNCA mitm/extension install|uninstall em run


## Fórmulas de Rede e API
- DEVE gravar captura com `browser-automation-cli --timeout 60 --json mitm capture-url https://example.com --har /tmp/c.har --seconds 10 --hosts example.com`
- DEVE ler o caminho gravado em `data.capture_path` no envelope de `mitm capture-url`
- DEVE usar `--hosts` como allowlist que estreita a intercepção TLS
- DEVE reler a captura com `browser-automation-cli --json mitm domains --capture-path /tmp/capture.json`
- DEVE reler com `mitm apis --capture-path /tmp/capture.json`; `mitm graphql --capture-path /tmp/capture.json --limit 100`
- DEVE reler com `mitm list --capture-path /tmp/capture.json --limit 50`; `mitm get 0 --capture-path /tmp/capture.json`; `mitm ws list --capture-path /tmp/capture.json`
- `--capture-path` lê captura gravada por OUTRA invocação e é a ÚNICA ponte entre processos
- Medido em example.com: `capture_count` 37 e 9 hosts distintos
- Medido: a captura inclui ruído de fundo do Chrome como accounts.google.com e play.google.com
- DEVE filtrar esse ruído por host antes de concluir qualquer análise
- Medido: `mitm apis --capture-path` devolveu zero endpoints numa página estática
- Zero endpoints é resposta honesta e NUNCA é falha da captura
- DEVE serializar `{"cmd":"net","action":"list","resource_types":"Document,XHR,Fetch","page_size":50}` em `run --script`
- `net list` só enxerga tráfego com `--capture-network` no MESMO processo
- DEVE paginar com as chaves de PASSO `page_idx` e `page_size`, e incluir preservados com `include_preserved`; as flags de argv homônimas existem mas são inalcançáveis, porque a forma de topo recusa antes
- DEVE serializar `{"cmd":"net","action":"get","id":0,"request_path":"/tmp/req.json","response_path":"/tmp/res.json"}` em `run --script`
- DEVE serializar `{"cmd":"net","action":"list","resource_types":"Document"}` em `run` com `--capture-network`
- DEVE navegar ANTES de chamar API por `eval` porque `eval` roda no contexto de origem da PÁGINA
- Medido A/B: sem `goto` antes, `fetch` devolve a string `Failed to fetch`
- Medido A/B: com `goto` antes para a mesma origem, devolve `ok:200`
- DEVE executar `browser-automation-cli --json eval 'fetch("/api").then(async r=>({ok:r.status})).catch(e=>({err:String(e)}))' --typed`
- `--typed` devolve `data.value` e `data.value_type` em vez do legado `data.result`
- Medido: `eval '({a:1,b:"x"})' --typed` devolve `value_type` igual a object
- Promise é resolvida automaticamente e NUNCA existe chave de await
- Medido: promise rejeitada sem try/catch devolve valor nulo com exit 0
- Essa falha é SILENCIOSA, portanto DEVE envolver toda chamada de API em try/catch ou `.catch`
- DEVE usar a chave `typed` no passo `eval` dentro de `run`
- Passo `eval` emite `refs_invalidated` true, portanto refs `@eN` morrem depois do eval
- DEVE recapturar refs com `view` depois de qualquer `eval` dentro de `run`
- Armadilha medida: chave desconhecida num passo de `run` é aceita em silêncio com ok true e exit 0
- DEVE conferir cada nome de chave com `schema <cmd> --json` antes de serializar o passo


## exec / run / residual
- DEVE executar `browser-automation-cli --json exec goto https://example.com`; `exec wait --selector h1 --wait-timeout-ms 2000`; `exec submit --target "#user" --timeout-ms 8000`; `exec pick --target @e1 --option Anomalia`; `exec select-option --target @e2 --option Alta`; `exec scrape --url https://example.com --format text`
- DEVE executar `browser-automation-cli --timeout 90 --json --json-steps run --script /tmp/steps.jsonl`
- DEVE executar `browser-automation-cli --json record --url https://example.com --path /tmp/rec.ndjson --seconds 30 --max-events 200`
- DEVE tratar `record` como gravador de interações da página que emite NDJSON reproduzível; `--url` e `--path` são OBRIGATÓRIOS; `--seconds` default 30 e `--max-events` default 200; o primeiro teto atingido vence
- DEVE fechar o ciclo record → replay com `browser-automation-cli --timeout 90 --json --json-steps run --script /tmp/rec.ndjson`
- DEVE serializar `{"cmd":"goto","url":"https://example.com","handle_before_unload":"accept","navigation_timeout_ms":15000}`, `{"cmd":"wait","selector":"h1, main, #content","wait_timeout_ms":10000}`, `{"cmd":"view","verbose":true}`, `{"cmd":"write","target":"@e1","value":"olá"}`, `{"cmd":"submit","target":"#user","timeout_ms":8000}`, `{"cmd":"scrape","url":"https://example.com","format":"text"}`, `{"cmd":"pick","target":"@e1","option":"Anomalia"}`, `{"cmd":"select-option","target":"@e2","option":"Alta"}`, `{"cmd":"dialog","action":"accept","if_present":true}`, `{"cmd":"grab","path":"/tmp/p.png","format":"png","full_page":true}`, `{"cmd":"print-pdf","path":"/tmp/p.pdf"}`, `{"cmd":"assert","kind":"url","url_contains":"example.com"}`, `{"cmd":"scroll","dy":400}`, `{"cmd":"page","action":"new","isolated_context":true}`
- DEVE validar residual com `browser-automation-cli -q --json doctor --offline --quick` — `residual_disk` não fail; zeros em `orphan_marker_dirs` e `ghost_marker_processes`; após DIE sozinho também zeros em `cli_marker_dirs` e `chromium_tmp_singleton_orphans` (`residual_disk` pass). NUNCA exigir zero `live_cli_marker_processes`; `sibling_live_processes>0` é concorrência saudável
- FORA de run: meta config mitm storage workflow crawl map batch-scrape search parse qr image video find-paths sg-scan sg-rewrite sheet-write monitor extension install/uninstall nested run/exec (mídia path-light é top-level)
- NUNCA trate exec como multi-passo; NUNCA divida `@eN` entre processos; NUNCA mate Chrome do usuário
- NUNCA apague em massa temporários genéricos do host; NUNCA mate Chrome Flatpak alheio
