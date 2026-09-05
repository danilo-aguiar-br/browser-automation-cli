---
name: browser-automation-cli
description: Esta skill DEVE ser usada quando a tarefa exigir operar a CLI browser-automation-cli para automação Chrome via CDP, scraping local, mídia local e diagnóstico de páginas. DEVE ativar em navegar, clicar, digitar, submit, fill-form, storage export e import, snapshot de acessibilidade com refs @eN, screenshot, PDF, extract com LLM, scrape multi-formato com rawHtml, batch-scrape, crawl, map, search, parse de PDF DOCX XLSX ODS, monitor, QR, sheet-write, sg-scan, sg-rewrite, find-paths, console, rede, MITM em loopback, captura de tráfego com HAR, descoberta de endpoints REST e GraphQL, emulate, perf, lighthouse, screencast, heap, extension, webmcp, workflow, run multi-passo, record de interações replayáveis, image info convert resize exif download, video info convert trim thumbnail manifest, audio info convert trim download. Entrega fórmulas de argv, oito flags de redução de payload, envelope JSON, exit codes, 217 chaves XDG sem variáveis de ambiente, robots e residual-zero em disco.
---

# browser-automation-cli

## Regra Zero
### OBRIGATÓRIO
- DEVE invocar SEMPRE o binário `browser-automation-cli` por extenso
- DEVE passar `--json` em TODA invocação programática
- DEVE parsear SOMENTE stdout, passar `-q` ou `--quiet` para silenciar stderr, e checar o exit code ANTES de confiar no stdout
- DEVE exigir `.ok == true` antes de `.data`; parsear com `jaq`, NUNCA com `jq`
### PROIBIDO
- NUNCA invente o alias `bac`, nome abreviado do binário, variável de ambiente de produto nem `.env` como configuração de runtime
- NUNCA mascare exit code com `|| true`; NUNCA parseie stderr como JSON

## Descoberta Obrigatória
### OBRIGATÓRIO
- DEVE resolver a superfície viva por descoberta, NUNCA por contagem memorizada
- DEVE rodar `--json commands`, `--json schema <cmd>` ou `schema --cmd <cmd>`, `--json config list-keys`, `--json config path`
- DEVE rodar `<cmd> --help` quando schema não bastar, `doctor --offline --quick` quando host parecer errado, e consultar `references/formulas.md` para a superfície argv exaustiva
### PROIBIDO
- NUNCA invente flag ausente do schema/help nem flags wishlist do PRD; NUNCA recuse chave de config por memória

## Identidade e Ciclo de Vida
### OBRIGATÓRIO
- DEVE tratar cada processo como BORN → EXECUTE → FINALIZE → DIE; Chrome nasce e morre no mesmo processo
- DEVE manter multi-passo com refs `@eN` em UM `run --script`; `@eN` morre com o processo
- DEVE usar Chrome de sistema ou apontá-lo com `config set chrome_path`
- DEVE mapear DevTools→produto — click→`press`, fill→`write`, take_screenshot→`grab`, take_snapshot→`view`, type_text→`type`, press_key→`keys`, navigate_page→`goto|back|forward|reload`, evaluate_script→`eval`, list_network_requests→`net list`, list_console_messages→`console list`
- DEVE tratar `exec` como passo único; multi-passo DEVE usar `run --script`
### PROIBIDO
- NUNCA reutilize `@eN` entre processos; NUNCA assuma daemon/sessão sticky/remota/telemetria; NUNCA chame nomes DevTools como subcomando

## Redução de Payload (todos os 71 comandos)
### OBRIGATÓRIO
- DEVE reduzir com as flags do próprio binário, NUNCA canalizando stdout por `jaq`
- DEVE usar `--fields PATHS` para projetar caminhos pontilhados (CSV)
- DEVE usar `--filter-rows EXPR` com `key=value`, `key!=value` ou `key~substring` (repetível, com AND)
- DEVE usar `--limit-rows N`, `--sort-rows PATH`, `--dedupe-by PATH`, `--count-only` em payload de lista
- DEVE usar `--truncate-content CHARS` e `--max-output-bytes BYTES` para limitar tamanho, e ler `agent_ops.truncated`, o único sinal que separa payload curto de payload cortado
- DEVE tratar filtro sem casamento como lista vazia com `ok: true` e nunca como erro, sabendo que campo ausente nunca casa, nem sob `!=`
- Medido: `doctor --offline --quick` tem 26.277 bytes e `--fields residual.ghost_marker_processes` tem 80
- DEVE saber que as oito globais de redução são `--fields`, `--filter-rows`, `--limit-rows`, `--sort-rows`, `--dedupe-by`, `--count-only`, `--truncate-content`, `--max-output-bytes`
- DEVE tratar `--select`, `--filter`, `--limit` e `--sort` como flags LOCAIS de comandos específicos, que convivem com as globais no mesmo help de `image info` e de `scrape`
- DEVE passar UM CSV único em `--fields`, porque repetir a flag devolve `ok:false`, `error.kind` usage e exit 2
- DEVE escrever caminho relativo a `data`, como `residual`, e NUNCA `data.residual`, cuja forma prefixada devolve `data` vazio com exit 0 num erro SILENCIOSO
- DEVE ler `agent_ops.unresolved_paths` para detectar caminho que não resolveu
- DEVE estreitar com `--fields <lista>` antes de `--count-only`, porque `--count-only commands` sozinho sai exit 2 por `data` ter mais de uma lista
- DEVE tratar a flag como NECESSÁRIA e NÃO suficiente para `agent_ops`, que só emerge com `total`, `matched` e `truncated` quando há algo a reportar
### PROIBIDO
- NUNCA canalize por `jaq`/`jq` para encolher payload — esse trabalho é do binário
- NUNCA confunda a família global de oito flags com as flags locais sem sufixo
- NUNCA presuma que `agent_ops` existe só porque você passou uma flag de redução
### Padrão Correto
- DEVE executar `browser-automation-cli --json --fields checks --filter-rows 'id=residual_disk' doctor --offline --quick`, trocando o filtro por `--count-only` quando só a contagem importar


## Flags Globais
### OBRIGATÓRIO
- DEVE aceitar flags globais antes ou depois do subcomando, e descobri-las em `<cmd> --help`
- DEVE passar `--json`; `--json-steps` em `run`; `--timeout`; `--step-timeout` em `run`; `--max-concurrency`; `--artifacts-dir`; `--correlation-id`; `--plain`
- DEVE passar `--capture-console` no MESMO processo de `console`/assert console; `--capture-network` no MESMO processo de `net`
- DEVE passar `--headed` somente para debug interativo; `--lang en` ou `--lang pt-BR`
- DEVE elevar tracing com `--verbose` ou `--debug` ou `config set log_level` — NUNCA env
- DEVE passar gates só quando a família exigir — `--category-memory` (`heap`), `--category-extensions` (`extension`), `--category-third-party` (`devtools3p`), `--category-webmcp` (`webmcp`), `--experimental-vision` (`click-at`), `--experimental-screencast` (`screencast`)
- DEVE passar `--mitm` e combinar com `--mitm-har|--mitm-hosts|--mitm-ca-dir|--mitm-ws|--mitm-max-body-bytes|--mitm-no-media-bodies|--mitm-redact-secrets|--mitm-no-redact-secrets` somente quando a intercepção exigir
- DEVE saber que a redação de segredos é LIGADA por padrão porque a captura vai a disco, e que `--mitm-redact-secrets` só reafirma esse padrão sem mudar nada
- DEVE desligar o mascaramento por UMA de DUAS rotas, a flag `--mitm-no-redact-secrets` neste processo ou a política persistente `mitm redact --secrets false`
- DEVE saber que pedir mascarar e desmascarar na mesma execução resolve MASCARANDO, que é a leitura segura de uma contradição sobre segredos
- DEVE passar `--dump-on-failure` com `--artifacts-dir` e com `--capture-console` ou `--capture-network`, sempre no MESMO processo, porque a captura morre com o DIE
- DEVE passar `--allow-outside-roots` como aceitação explícita de risco para ler e gravar FORA das raízes permitidas, e preferir a chave XDG `allowed_roots` a ela
- DEVE saber que o stealth é LIGADO por padrão e mascara os marcadores de automação que um Chrome real nunca expõe, e passar `--no-stealth` para desligar os patches nesta execução
- DEVE passar `--stealth-profile auto|chrome-linux|chrome-win|chrome-mac`, e preferir `auto`, que segue o host e quase sempre está certo
- DEVE passar `--stealth-seed <SEED>` para fixar uma identidade entre processos (`hardwareConcurrency`, `deviceMemory`, GPU, `history.length`, build do Chrome — não UA/platform/screen), porque sem semente um crawl de 50 URLs se apresenta como 50 máquinas distintas
- DEVE executar `browser-automation-cli --json doctor --fingerprint` para auditar coerência de identidade, e listar perfis com `--stealth-profile list` ou `commands --json`
- DEVE passar `--min-delay-ms <MS>` para elevar o piso de cortesia por origem nesta invocação, sabendo que a espera efetiva é o MÁXIMO entre a flag, o XDG `scrape_min_delay_ms` e o `Crawl-delay`
- DEVE passar `--proxy <URL>` (`http`, `https`, `socks5`) como proxy de saída para o Chrome E para o motor HTTP, com `--proxy-bypass <HOSTS>` na sintaxe de bypass-list do Chrome
- DEVE guardar credenciais de proxy com `config set proxy_username` e `config set proxy_password` no XDG, NUNCA em argv, porque a tabela de processos expõe argv
- DEVE ajustar o fingerprint HTTP/2 do motor `--engine http`, que desalinhado identifica o cliente como automatizado mesmo com cabeçalhos reais, SOMENTE por XDG, com `config set http2_enabled`, `http2_adaptive_window`, `http2_max_frame_size`, `http2_initial_stream_window_size`, `http2_initial_connection_window_size` e `http2_max_header_list_size`
- DEVE saber que NENHUMA dessas seis chaves tem flag equivalente na linha de comando
- DEVE executar `config set stealth false`, `config set stealth_profile` e `config set stealth_seed` como equivalentes persistentes das flags por processo
- DEVE passar `--input-profile human|direct`, sendo `human` o padrão que interpola trajetórias do ponteiro, aplica dwell entre press e release e ritma a digitação
- DEVE passar `--input-seed <SEED>` para semear o jitter e reproduzir exatamente uma execução `human`, porque sem semente o jitter vem do sistema e duas execuções diferem
- DEVE passar `--warmup` para visitar a raiz da origem antes da URL alvo e carregar cookies e cadeia de referrer, ou `--warmup-url <URL>` para aquecer outra URL no lugar dessa raiz
- DEVE passar `--browser-mode auto|headless|headed` como modo de janela canônico, com `--headless` e `--headed` como atalhos de dois valores dele
- DEVE entender a relação das duas rotas: a FLAG governa esta execução e VENCE, enquanto `config set browser_mode <valor>` governa o padrão persistente do host
- MEDIDO: o `--help` de RAIZ não imprime flag global alguma, então NUNCA conclua que uma global não existe sem conferir em `<cmd> --help`
- DEVE passar `--no-xvfb` somente em modo headed no Linux, para pular o display virtual privado e usar o display atual
- DEVE passar `--expect <EXPR>` com `key=value`, `key!=value` ou `key~substring` para afirmar o payload emitido (repetível, com AND)
- DEVE passar `--expect-exit-code` para sair com 65 quando algum `--expect` falhar, em vez de apenas reportar
- DEVE saber que `--expect-exit-code` é desligado por padrão porque mudar exit code por conteúdo de dado quebraria chamadores em silêncio
### PROIBIDO
- NUNCA espere captura sobreviver ao DIE; NUNCA ligue gate por padrão; NUNCA omita `--json` em pipeline de agente
- NUNCA passe credenciais de proxy em argv; NUNCA declare plataforma estrangeira em `--stealth-profile` quando o host disser outra coisa

## Config XDG
### OBRIGATÓRIO
- DEVE configurar SOMENTE por flags CLI e `config init|path|show|get|set|unset|list-keys`
- DEVE descobrir chaves com `config list-keys --json`; resolver caminhos com `config path --json` — NUNCA inventar paths XDG
- DEVE tratar flag CLI como override do valor gravado
- DEVE setar segredos com `config set encryption_key` e `config set openrouter_api_key`
- DEVE saber que `config unset <CHAVE>` é o inverso de `set` e restaura o default embutido, enquanto `config set <chave> "" ` NÃO é, salvo em `user_data_dir`, onde a string vazia limpa o opt-in

- DEVE setar binários com `chrome_path`, `lighthouse_path` e `ffmpeg_path`, cache com `cache_backend sqlite|memory|redis` mais Redis plain em `cache_redis_url`, e comportamento com `dialog_settle_ms` e `log_level`
- DEVE consultar `references/xdg-keys.md` para o inventário completo das chaves XDG com padrão e descrição
### PROIBIDO
- NUNCA invente env de produto; NUNCA logue segredos/cookies; NUNCA use `rediss://`; NUNCA configure redis sem `cache_redis_url`

## Contrato Argv e Superfície
### OBRIGATÓRIO
- DEVE passar `grab --path` (nunca posicional); `grab --format png|jpeg|webp`; `--quality`/`--element` somente quando necessário
- DEVE passar `print-pdf --path` SEMPRE; `print-pdf --url` em one-shot (recusa blank)
- DEVE passar `view --detailed` para a árvore de acessibilidade completa (o argv é `--detailed`, NUNCA `--verbose`), e `view --allow-empty` só quando o snapshot em branco for intencional
- DEVE passar `type <TEXTO>` com `--target` OU `--focus-only`
- DEVE passar `fill-form --fields-json '[{"target":"@eN","value":"x"}]'` e `cookie set --cookies-json '[...]'` (NUNCA payload por `--json`)
- DEVE passar `submit <ALVO>` (form ou campo dono); `submit --timeout-ms` e `--include-snapshot` somente quando necessário
- DEVE passar `storage export|import --path` obrigatório; `--url` no mesmo processo quando origem autenticada for necessária
- DEVE passar `mitm block --host`; `mitm allow --host` (host obrigatório); `mitm ws list|get`
- DEVE passar `reload --ignore-cache` (NUNCA em `goto`); `goto --handle-before-unload accept|dismiss`
- DEVE passar `sheet-write <in> -o <out.xlsx>`; `emulate` por flags (NUNCA `--device`); `assert url <v> --contains`
- DEVE passar `workflow run --manifest`; `--journal` somente quando necessário; `eval --file-path`; `--service-worker-id` para SW
- DEVE descobrir `pick`/`select-option` via schema e invocar na superfície reportada; usar `locale`/`man` sem Chrome
- DEVE passar `record --url` e `record --path` obrigatórios; `--seconds` e `--max-events` opcionais
- DEVE ler a chave `html` após `scrape --format html` e `rawHtml` após `scrape --format rawHtml`
- Medido em `--engine http`: `html` e `rawHtml` são chaves DISTINTAS com payloads DISTINTOS
### PROIBIDO
- NUNCA trate `rawHtml` como alias de `html`; quem assume alias lê chave errada e recebe vazio
- NUNCA use caminho posicional nu em grab/print-pdf; NUNCA rode print-pdf one-shot sem `--url`; NUNCA use avif
- NUNCA embuta `mitm`, `storage` ou `extension install|uninstall` dentro de `run`
- NUNCA use `view --verbose` (o correto é `--detailed`); NUNCA use `fill-form --json` nem `cookie set --json` como payload

## Envelope JSON e Exit Codes
### OBRIGATÓRIO
- DEVE esperar sucesso com `schema_version` mais `ok` true mais `data`, falha com `ok` false mais `error`, e argv inválido como `error.kind` igual a `usage` com exit 2
- DEVE ler `data.steps` parciais em falha de `run`; `matched_selector` em wait multi-seletor
- DEVE ler `data.binary_source` real|mock em lighthouse; NUNCA trate mock como validação do parser LHR
- DEVE ler `.data.dialog_settled` após `dialog accept|dismiss` real; quando true NÃO insira wait artificial
- DEVE ler `browser_mode_requested` para saber o modo de janela que o argv ou a chave XDG PEDIU
- DEVE ler `browser_mode_effective`, que vale `headless` ou `headed` e diz o modo que o processo REALMENTE usou
- DEVE ler `browser_mode_source`, que vale `default`, `xdg` ou `flag` e nomeia a camada de precedência que decidiu o modo
- DEVE tratar `browser_mode_source` igual a `default` como headless por sorte do padrão, e NUNCA como requisito provado; só `flag` e `xdg` sustentam uma exigência
- DEVE ler a testemunha de `run` UMA vez no TOPO do envelope, porque `run` REMOVE as cinco chaves de cada passo e publica uma cópia única no topo
- DEVE ler `display_backend`, que vale `headless`, `xvfb` ou `host` e nomeia o display que sustentou a janela
- DEVE ler `runtime_enable_used`, booleano que diz se o domínio Runtime do CDP foi ligado nesta execução
- DEVE ler `serp_endpoint` no envelope de `search`, que vale `known` quando o endpoint entende os parâmetros de dimensão e `unknown` quando não entende, caso em que limite e paginação podem ser ignorados pelo buscador
- NUNCA conclua o modo de janela pela flag que você passou; o par requested mais effective é a ÚNICA prova
- DEVE tratar `search` sem resultado orgânico como FALHA DECLARADA, com `ok` falso e `error.kind` igual a `data`, e NUNCA como sucesso com lista vazia
- DEVE ler `serp_endpoint` TAMBÉM em `data` no envelope de FALHA, ao lado de `search_base_url`, porque é esse par que separa endpoint desconhecido de web realmente vazia
- DEVE ramificar exit `0` ok, `2` usage, `65` data, `66` no-input, `69` unavailable, `70` software, `74` io, `78` config, `124` timeout, `130` cancel, `141` broken-pipe
- DEVE retentar somente falha transitória de host/launch
### PROIBIDO
- NUNCA retente usage sem corrigir argv; NUNCA trate prosa stdout como contrato; NUNCA ignore `ok` falso; NUNCA trate mock lighthouse como validação de parser LHR

## Scripts Multi-passo run
### OBRIGATÓRIO
- DEVE usar `run --script` (NDJSON ou array JSON); cada passo com `cmd`; `--timeout` cobre o script inteiro
- DEVE usar `run --script -` para ler passos NDJSON do stdin contra uma sessão viva, ainda one-shot, com um BORN, um DIE, EOF disparando o FINALIZE, e cada linha validada ao chegar sob `validation: "per-line"`
- DEVE preferir stdin a process substitution: `run --script <(printf ...)` é recusado pelo jail de arquivos
- DEVE serializar grab/print-pdf com `path`; print-pdf com `url` ou `goto` prévio
- DEVE serializar scroll com `delta_y`/`dy` e `delta_x`/`dx`; wait com `selector` CSV ou `selectors` array (OR) e `wait_timeout_ms`
- DEVE serializar wait pós-nav com `url`/`url_contains`/`navigation`, scrape com `url` mais `format|formats`, submit com `target` e `timeout_ms` se diferir, e dialog com `if_present` se puder faltar
- DEVE serializar view blank com `allow_empty`, view detalhado com `verbose` ou `detailed`, aba isolada com `isolated_context`, e assert com `kind` em `url|text|console|console_empty|console_no_match`
- DEVE manter fora de `run` — meta, config, mitm, storage, workflow, crawl, map, batch-scrape, search, parse, qr, find-paths, sg-scan, sg-rewrite, sheet-write, monitor, extension install|uninstall
### PROIBIDO
- NUNCA divida passos `@eN` entre processos; NUNCA ignore `data.steps` parciais; NUNCA use `exec` como multi-passo
### Passos Críticos em Uma Linha
- DEVE serializar `{"cmd":"goto","url":"https://example.com","handle_before_unload":"accept","navigation_timeout_ms":15000}`
- DEVE serializar `{"cmd":"wait","selector":"h1, main, #content","wait_timeout_ms":10000}`
- DEVE serializar `{"cmd":"view","verbose":true}` e `{"cmd":"write","target":"@e1","value":"olá"}`
- DEVE serializar `{"cmd":"submit","target":"#user","timeout_ms":8000}` e `{"cmd":"scrape","url":"https://example.com","format":"text"}`
- DEVE serializar `{"cmd":"pick","target":"@e1","option":"Anomalia"}` e `{"cmd":"select-option","target":"@e2","option":"Alta"}`
- DEVE serializar `{"cmd":"dialog","action":"accept","if_present":true}`
- DEVE serializar `{"cmd":"grab","path":"/tmp/p.png","format":"png"}` e `{"cmd":"print-pdf","path":"/tmp/p.pdf","url":"https://example.com"}`
- DEVE conferir cada chave contra `schema <cmd> --json` antes de adaptar qualquer passo

## Leis Agent-First
### OBRIGATÓRIO
- DEVE tratar diálogos multi-aba por `session_id`, com troca de aba sob diálogo em domain enable best-effort, e settle por `config set dialog_settle_ms` no XDG e nunca por env
- DEVE esperar `select-option`/`pick` nativo despachar input→change e reportar `via: native_select`
- DEVE usar `submit` para form e esperar nav/request, sabendo que storage exige `--path`, grava export em modo 0600 e roda FORA de run
### PROIBIDO
- NUNCA wait artificial com `dialog_settled` true; NUNCA avif; NUNCA storage/mitm/extension install|uninstall em run

## Residual-Zero e Robots
### OBRIGATÓRIO
- DEVE tratar residual-zero como parte do sucesso de one-shot quando este processo é a única invocação concorrente, e validar os campos residuais e o check `residual_disk` com `doctor --offline --quick --json`
- DEVE exigir `residual_disk` não `fail`, com zero em `orphan_marker_dirs` e em `ghost_marker_processes`, e após DIE sozinho também zero em `cli_marker_dirs` e em `chromium_tmp_singleton_orphans` (`residual_disk` `pass`)
- DEVE tratar `sibling_live_processes > 0` como concorrência saudável (`warn`, nunca fail)
- NÃO DEVE exigir zero `live_cli_marker_processes` (contagem legada de filhos Chrome; preferir `sibling_live_processes`)
- DEVE tratar definir a chave XDG `user_data_dir` como a decisão EXPLÍCITA de abrir mão do residual-zero, porque o perfil persiste ao DIE e o varredor só julga diretórios com o prefixo de marcador do produto
- DEVE executar `config set user_data_dir <caminho>` para essa rota e `config unset user_data_dir` para voltar ao padrão AUSENTE, que preserva o residual-zero
- DEVE saber que em Unix esse diretório nasce com permissão 0700, porque perfil persistente guarda cookie e token
- DEVE respeitar robots por padrão; contornar SOMENTE com ambas `--ignore-robots` e `--i-accept-robots-risk`
### PROIBIDO
- NUNCA declare residual-zero sem ler `data.residual`; NUNCA apague temporários genéricos do host; NUNCA mate Chrome do usuário nem Flatpak alheio
- NUNCA contorne robots com uma flag só; NUNCA invente env de bypass de robots
- NUNCA reprove host só porque `live_cli_marker_processes > 0` com orphans/ghosts zero

## Inventário Completo de Comandos
### OBRIGATÓRIO
- DEVE conhecer estes 71 — doctor, commands, schema, version, locale, goto, view, press, click-at, write, keys, type, wait, hover, drag, submit, fill-form, select-option, pick, upload, back, forward, reload, eval, grab, print-pdf, monitor, run, exec, extract, text, scroll, cookie, storage, attr, assert, console, net, page, dialog, scrape, batch-scrape, crawl, map, search, parse, qr, record, image, video, audio, find-paths, sg-scan, sg-rewrite, sheet-write, sitemap, feed, mitm, workflow, config, emulate, resize, perf, lighthouse, screencast, heap, extension, devtools3p, webmcp, completions, man
- DEVE usar pipeline local de imagem (sem Chrome): `image info|convert|resize|download|exif`
- DEVE usar pipeline local de vídeo (sem Chrome): `video info|download|convert|to-mp3|trim|thumbnail|manifest` com ffmpeg/ffprobe opcional (XDG `ffmpeg_path`)
- DEVE usar pipeline local de áudio (sem Chrome): `audio info|download|convert|trim` com ffmpeg/ffprobe opcional (XDG `ffmpeg_path`)
- DEVE manter stdout agent-native de mídia — path, sha256, dimensões, codecs, duração e texto; NUNCA base64 de pixels salvo `grab --include-base64`, NUNCA frames raw, NUNCA PCM
- DEVE projetar com `image info --select format,width,height,sha256` para economizar tokens
- DEVE projetar com `video info --select container,duration_secs,streams,sha256` e convert `--select path_out,auto_reencoded,video_codec`
- DEVE projetar com `audio info --select format,codec,duration,bytes,sha256` e convert `--select path_out,lossy_transcode,suggestion`
- DEVE usar `video manifest` para resumir manifesto HLS/DASH sem baixar mídia
- NÃO DEVE invocar ffmpeg manual no shell quando `video convert` ou `audio convert` faz remux/re-encode; preferir `upload` para upload CDP
- DEVE configurar caps de áudio só via XDG: `audio_max_input_bytes` `audio_download_max_bytes` `audio_default_format` `audio_default_bitrate`
- DEVE configurar limites de imagem só via XDG `config set` (`image_*`) — nunca env de produto
- DEVE tratar webp local como lossless (`quality_applied` false), enquanto jpeg honra quality, e `--keep-exif` é intenção apenas (`keep_exif_honored` false)
- DEVE ler texto de imagem nativamente como agente, porque a CLI não tem ação de reconhecimento de texto nem binário C externo
- DEVE tratar EXIF como única superfície de metadados (sem IPTC/XMP), com `image exif --select tags` como alias de `exif`
- DEVE rejeitar encode AVIF/HEIC e saber que SVG não tem resvg, com `--allow-non-image` só para bytes crus intencionais
- DEVE NÃO confundir `image download` com download de árvore de site inteiro, e confirmar o inventário vivo com `commands --json`


## Como Fazer Scraping
### Sequência Obrigatória
- DEVE escolher o motor ANTES de tudo, porque ele define o custo da coleta
- DEVE usar `--engine http` como caminho barato, que não sobe navegador, e trocar para `--engine browser` só quando a página depender de JavaScript
- DEVE pedir formatos com `--format` em CSV ou repetindo a flag
- DEVE pedir `rawHtml` para corpo bruto e `html` para corpo processado, cada um entregue sob a chave de mesmo nome
- DEVE aplicar `--only-main-content` para recortar a página antes do parse, e encolher o envelope com as oito flags globais de redução
- DEVE executar `browser-automation-cli --json scrape https://example.com --format markdown,links,metadata --engine http --only-main-content`
### Escala
- DEVE usar `batch-scrape --urls-file <FILE> --concurrency <N>` para lista fechada, `crawl <URL> --limit <N> --max-depth <N>` para descoberta a partir de semente e `map <URL> --limit <N>` quando só precisar enumerar URLs
### Armadilhas Medidas
- DEVE assumir que robots é respeitado por padrão em toda coleta, e contornar SOMENTE com AMBAS `--ignore-robots` e `--i-accept-robots-risk`
- NUNCA passe apenas uma das duas flags, porque uma sozinha NÃO contorna
- NUNCA use `--engine browser` por hábito, porque ele custa um Chrome inteiro
- NUNCA peça `rawHtml` quando `markdown` responde, porque o payload explode


## Como Monitorar o Tráfego de Rede
### Duas Superfícies Distintas
- DEVE usar `net` para o tráfego observado pelo PRÓPRIO processo vivo
- DEVE usar `mitm` para captura persistida em arquivo entre processos
- DEVE passar `--capture-network` no MESMO processo de qualquer `net list`
- DEVE refinar `net list` com `--page-idx`, `--page-size`, `--resource-types` e `--include-preserved`
- NUNCA chame `net list` como subcomando de topo: ele recusa com exit 2, porque o buffer de captura morre com o processo que o encheu
### O Filtro de Tipo de Recurso
- DEVE passar `--resource-types` como UMA lista separada por vírgula, casada de forma EXATA e sem diferenciar maiúsculas
- DEVE tirar todo token do vocabulário CDP — Document, Stylesheet, Image, Media, Font, Script, TextTrack, XHR, Fetch, Prefetch, EventSource, WebSocket, Manifest, SignedExchange, Ping, CSPViolationReport, Preflight, FedCM, Other
- DEVE esperar que um token desconhecido seja RECUSADO com exit 2 e `error.kind` usage, nomeando o ofensor ANTES de qualquer lançamento de Chrome, então um erro de digitação custa um parse e nunca um navegador
- DEVE ler `resourceType` em todo registro capturado; requisição cujo tipo o CDP omitiu é gravada como `Other` e NUNCA sem a chave
- DEVE tratar resultado vazio como prova de que a página não tinha aquele recurso, porque um erro de digitação não alcança mais esse ramo
### Teto de Buffer e Truncagem Declarada
- DEVE ler `dropped_oldest` nos envelopes de `net` e `console`, que conta os registros descartados para manter o buffer sob o teto, e reconstruir o que a página produziu como `total` mais `dropped_oldest`
- DEVE mover esse teto SOMENTE com `config set event_tracker_max_entries <N>`; nenhuma flag o expõe
- DEVE passar `--include-preserved` também em `net get` e `console get`, e não só nas formas `list`, para que um índice enderece o MESMO registro nos dois
### Sequência Obrigatória do MITM
- DEVE gravar a captura primeiro com `mitm capture-url <URL>`
- DEVE ler o caminho gravado em `data.capture_path`
- DEVE reler essa captura em outro processo com `--capture-path <FILE>`
- DEVE tratar `--capture-path` como a ÚNICA ponte entre processos one-shot, aceita por `mitm list`, `get`, `domains`, `apis`, `graphql` e `ws`
- DEVE executar `browser-automation-cli --timeout 60 --json mitm capture-url https://example.com --har /tmp/c.har` e então `mitm domains --capture-path <CAPTURE>`
### Armadilhas Medidas
- Medido em example.com: `capture_count` 37 e nove hosts distintos, entre eles accounts.google.com e play.google.com, que são ruído de fundo do navegador e não navegação sua
- DEVE estreitar a captura com `--hosts` na gravação, e tratar os zero endpoints que `mitm apis` devolve numa página estática como resposta honesta e NUNCA falha


## Como Interagir com APIs
### Sequência Obrigatória
- DEVE lembrar que `eval` executa no contexto de origem da PÁGINA
- DEVE navegar para a origem alvo ANTES de chamar a API por `fetch`
- Medido A/B: sem `goto` antes o `fetch` devolve a string `Failed to fetch`, e com `goto` para a mesma origem devolve `ok:200`
- DEVE passar `--typed` para receber `data.value` e `data.value_type`
- DEVE encadear `goto` e `eval` num único `run --script` para um só processo
- DEVE executar `browser-automation-cli --json --json-steps --timeout 90 run --script /tmp/api.jsonl`
### Erros e Refs
- DEVE envolver toda chamada em try/catch e retornar a mensagem de erro
- Medido: promise rejeitada sem try/catch devolve valor nulo com exit 0, que é falha SILENCIOSA e NUNCA resposta vazia
- DEVE saber que promise é resolvida automaticamente, sem qualquer chave de await
- DEVE ler `refs_invalidated` true em todo passo `eval` e refazer `view` para obter refs novas
- NUNCA reutilize `@eN` capturado antes de um `eval`
### Armadilhas Medidas
- Medido: chave desconhecida num passo de `run` é aceita em SILÊNCIO com ok true, então DEVE conferir cada chave contra `schema <cmd> --json`, e DEVE carregar estado autenticado com `storage export` mais `storage import` FORA de `run`


## Playbooks de Execução
### OBRIGATÓRIO
- DEVE executar fórmulas literalmente; validar envelope após cada invocação; consultar `references/formulas.md`
### PROIBIDO
- NUNCA adapte fórmula sem `schema <cmd> --json`

### A. Diagnóstico
- DEVE executar `browser-automation-cli --json doctor --offline --quick`; `commands`; `schema <cmd>`; `version`; `locale`; `completions bash`; `man --out /tmp/b.1`
- DEVE executar `config init|path|show|list-keys`; `config get timeout`; `config set dialog_settle_ms 2000`; `config unset dialog_settle_ms`

### B. Navegação e inspeção
- DEVE executar `browser-automation-cli --timeout 60 --json goto https://example.com --init-script 'window.__ready=1' --handle-before-unload accept --navigation-timeout-ms 15000`
- DEVE executar `browser-automation-cli --json view --detailed`; `back`; `forward`; `reload --ignore-cache`; `text @e1`; `attr @e1 href`; `eval 'document.title' --file-path /tmp/eval.json`

### C. Interação
- DEVE executar `press @e1 --include-snapshot`; `--experimental-vision click-at --x 10 --y 20`; `write @e2 "texto"`; `type "olá" --target @e2 --clear --submit Enter`; `keys Enter`; `hover @e1`; `drag --from @e1 --to @e2`; `upload @e4 /tmp/a.txt`
- DEVE executar `submit "#user" --timeout-ms 8000`; `fill-form --fields-json '[{"target":"@e3","value":"x"}]'`
- DEVE executar `exec pick --target @e1 --option Anomalia`; `exec select-option --target @e2 --option Alta`; `wait --selector "h1, main, #content" --wait-timeout-ms 10000`; `scroll --delta-y 400`

### D. Artefatos
- DEVE executar `grab --path /tmp/p.png --format webp --quality 80 --full-page`; `--timeout 60 print-pdf --path /tmp/p.pdf --url https://example.com`
- DEVE executar `monitor check --url https://example.com --baseline /tmp/b.baseline --write-baseline --engine http`; `qr encode --text <URL> --format png --path /tmp/qr.png`; `qr decode --path /tmp/qr.png`

### E. Scrape e extração
- DEVE executar `scrape https://example.com --format markdown,links,metadata --engine http --only-main-content`
- DEVE executar `batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2`; `crawl https://example.com --limit 20 --max-depth 2 --format text`; `map https://example.com --limit 50`; `search "example domain" --limit 10`
- DEVE executar `parse /tmp/doc.pdf`; `parse /tmp/planilha.ods --redact-pii`; `--timeout 120 extract --llm --question "título?" --schema-json /tmp/s.json https://example.com`

### F. Console e rede
- DEVE executar `--capture-console console dump --path /tmp/console.json`; `assert console-empty`; `assert console-no-match --pattern TypeError`, sempre com `--capture-console` no mesmo processo
- DEVE saber que `console list`, `console get`, `net list` e `net get` recusam no topo com exit 2 e existem SOMENTE como passo de `run --script`, enquanto `console clear` e `console dump` de topo CONTINUAM funcionando

### G. Abas, cookies, storage, diálogos
- DEVE executar `page new --isolated-context s-a --url https://example.com`; `page list`; `page select 0 --bring-to-front`; `cookie set --cookies-json '[{"name":"a","value":"b","url":"https://example.com"}]'`; `cookie list`
- DEVE executar `storage export --path /tmp/auth.json --url https://example.com`; `storage import --path /tmp/auth.json --url https://example.com`
- DEVE executar `dialog accept --if-present` e ler `.data.dialog_settled`; `assert url example.com --contains`

### H. MITM
- DEVE executar `mitm init-ca`; `mitm capture-url https://example.com --har /tmp/c.har`; `mitm block --host example.com --path /ads`; `mitm allow --host example.com`; `mitm ws list --limit 50`; `mitm redact --secrets false`

### I. Perf e memória
- DEVE executar `emulate --user-agent "Mozilla/5.0" --viewport "390x844x3,mobile,touch" --network-conditions "Slow 3G"`; `resize --width 1280 --height 720`; `perf start`; `perf stop --path /tmp/trace.json`
- DEVE executar `--timeout 180 lighthouse https://example.com --out-dir /tmp/lh --device desktop` e ler `data.binary_source`; `--experimental-screencast screencast start --path /tmp/cast`; `screencast stop --path /tmp/cast.webm`
- DEVE executar `--category-memory heap take --path /tmp/s.heapsnapshot --url https://example.com`, e consultar `references/formulas.md` para `heap summary`, `compare`, `retainers` e os demais verbos de heap

### J. Ferramentas locais
- DEVE executar `find-paths --glob '**/*.rs' .`; `sg-scan . --limit 100`; `sg-rewrite .`; `sg-rewrite . --apply`; `sheet-write /tmp/rows.csv -o /tmp/out.xlsx --sheet Data`

### K. Extensões e terceiros
- DEVE executar `--category-extensions extension list`; `extension install /tmp/ext`; `extension reload <id>`; `extension uninstall <id>`; `--category-third-party devtools3p list --url https://example.com`; `--category-webmcp webmcp list --url https://example.com`

### L. Workflow e multi-passo
- DEVE executar `workflow run --manifest /tmp/wf.json --journal /tmp/wf.journal`; `workflow resume --manifest /tmp/wf.json`; `workflow status --name demo`; `--timeout 90 --json --json-steps run --script /tmp/steps.jsonl`; `exec goto https://example.com`

### M. Gravação e replay
- DEVE executar `record --url https://example.com --path /tmp/steps.jsonl --seconds 30 --max-events 200`, que grava interações da página como NDJSON reproduzível
- DEVE fechar o ciclo com `--json --json-steps run --script /tmp/steps.jsonl` sobre a gravação
- DEVE saber que `--seconds` default é 30, `--max-events` default é 200, e o primeiro teto atingido vence

## Proibições Absolutas
### PROIBIDO
- NUNCA invente alias `bac` nem env de produto nem `export` de chaves de produto
- NUNCA use `jq` no lugar de `jaq`; NUNCA confie em stdout sem exit+`.ok`
- NUNCA reutilize `@eN` entre processos; NUNCA embuta mitm/storage/extension install|uninstall em `run`; NUNCA declare residual-zero sem doctor; NUNCA invente flags ausentes do schema vivo
- NUNCA use avif; NUNCA trate lighthouse mock como parser válido; NUNCA contorne robots com uma flag; NUNCA trate `exec` como multi-passo; NUNCA use `view --verbose` nem `fill-form --json`/`cookie set --json` como payload
