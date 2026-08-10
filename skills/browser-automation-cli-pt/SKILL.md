---
name: browser-automation-cli
description: Esta skill DEVE ser usada quando a tarefa exigir operar a CLI browser-automation-cli para automação Chrome via CDP, scraping local, mídia local e diagnóstico de páginas. DEVE ativar em navegar, clicar, digitar, submit, fill-form, storage export e import, snapshot de acessibilidade com refs @eN, screenshot, PDF, extract com LLM, scrape multi-formato com rawHtml, batch-scrape, crawl, map, search, parse de PDF DOCX XLSX ODS, monitor, QR, sheet-write, sg-scan, sg-rewrite, find-paths, console, rede, MITM em loopback, captura de tráfego com HAR, descoberta de endpoints REST e GraphQL, emulate, perf, lighthouse, screencast, heap, extension, webmcp, workflow, run multi-passo, record de interações replayáveis, image info convert resize exif download, video info convert trim thumbnail manifest, audio info convert trim download. Entrega fórmulas de argv, oito flags de redução de payload, envelope JSON, exit codes, 204 chaves XDG sem variáveis de ambiente, robots e residual-zero em disco.
---

# browser-automation-cli

## Regra Zero
### REQUIRED
- DEVE usar SEMPRE o binário `browser-automation-cli` por extenso; NUNCA alias `bac`
- DEVE passar `--json` em TODA invocação programática; parsear SOMENTE stdout; silenciar stderr com `-q` ou `--quiet`
- DEVE checar exit code ANTES de confiar no stdout; validar `.ok == true` antes de `.data`; parsear com `jaq` e NUNCA `jq`
- DEVE consultar `references/formulas.md` para superfície argv exaustiva
### FORBIDDEN
- NUNCA invente variável de ambiente de produto; NUNCA use `.env` como runtime; NUNCA mascare exit com `|| true`; NUNCA parseie stderr como JSON

## Descoberta Obrigatória
### REQUIRED
- DEVE resolver superfície viva por descoberta — `browser-automation-cli --json commands`, `schema <cmd>`, `schema --cmd <cmd>`, `config list-keys`, `config path`
- DEVE rodar `<cmd> --help` quando schema não bastar; `doctor --offline --quick` quando host parecer errado
### FORBIDDEN
- NUNCA invente flag ausente do schema/help nem flags wishlist do PRD; NUNCA recuse chave de config por memória

## Identidade e Ciclo de Vida
### REQUIRED
- DEVE tratar cada processo como BORN → EXECUTE → FINALIZE → DIE; Chrome nasce e morre no mesmo processo
- DEVE manter multi-passo com refs `@eN` em UM `run --script`; `@eN` morre com o processo
- DEVE usar Chrome de sistema ou `config set chrome_path`
- DEVE mapear DevTools→produto — click→`press`, fill→`write`, take_screenshot→`grab`, take_snapshot→`view`, type_text→`type`, press_key→`keys`, navigate_page→`goto|back|forward|reload`, evaluate_script→`eval`, list_network_requests→`net list`, list_console_messages→`console list`
- DEVE tratar `exec` como passo único; multi-passo DEVE usar `run --script`
### FORBIDDEN
- NUNCA reutilize `@eN` entre processos; NUNCA assuma daemon/sessão sticky/remota/telemetria; NUNCA chame nomes DevTools como subcomando

## Redução de Payload (todos os 69 comandos)
### REQUIRED
- DEVE reduzir com as flags do próprio binário, NUNCA canalizando stdout por `jaq`
- DEVE usar `--fields PATHS` para projetar caminhos pontilhados (CSV)
- DEVE usar `--filter-rows EXPR` com `key=value`, `key!=value` ou `key~substring` (repetível, com AND)
- DEVE usar `--limit-rows N`, `--sort-rows PATH`, `--dedupe-by PATH`, `--count-only` em payload de lista
- DEVE usar `--truncate-content CHARS` e `--max-output-bytes BYTES` para limitar tamanho
- DEVE ler `agent_ops.truncated` — é o único sinal que separa payload curto de payload cortado
- DEVE tratar filtro sem casamento como lista vazia com `ok: true`, nunca como erro
- DEVE estreitar com `--fields <key>` primeiro quando o erro disser que data tem mais de uma lista
- DEVE saber que campo ausente nunca casa, nem sob `!=`
- Medido: `doctor --offline --quick` tem 26.277 bytes; `--fields residual.ghost_marker_processes` tem 80
- DEVE saber que as oito flags globais de redução são `--fields`, `--filter-rows`, `--limit-rows`, `--sort-rows`, `--dedupe-by`, `--count-only`, `--truncate-content`, `--max-output-bytes`
- DEVE tratar `--select`, `--filter`, `--limit` e `--sort` como flags LOCAIS de comandos específicos
- Medido: `image info --help` expõe `--select` local E `--fields` global lado a lado
- Medido: `scrape --help` expõe `--select` local mais `--filter-rows` e `--limit-rows` globais
- DEVE passar UM CSV único em `--fields`; a flag NÃO é repetível
- Medido: `--fields residual --fields checks` devolve `ok:false`, `error.kind` usage, exit 2
- DEVE escrever caminho relativo a `data`, como `residual`, NUNCA `data.residual`
- Medido: `--fields data.residual` devolve `data` vazio com exit 0 — erro SILENCIOSO
- DEVE ler `agent_ops.unresolved_paths` para detectar caminho que não resolveu
- DEVE estreitar com `--fields <lista>` antes de usar `--count-only` em payload multi-lista
- Medido: `--count-only commands` sozinho sai exit 2 por `data` ter mais de uma lista
- DEVE tratar rodar a flag como condição NECESSÁRIA e NÃO suficiente para `agent_ops`
- DEVE saber que `agent_ops` é omitido quando não há nada a reportar
- Medido: `--fields commands commands` devolve só `data`, `ok`, `schema_version`, sem `agent_ops`
- Medido: `--fields commands --limit-rows 3 commands` devolve `agent_ops` com `total`, `matched`, `truncated`
### FORBIDDEN
- NUNCA canalize por `jaq`/`jq` para encolher payload — esse trabalho é do binário
- NUNCA confunda a família global de oito flags com as flags locais sem sufixo
- NUNCA presuma que `agent_ops` existe só porque você passou uma flag de redução
### Padrão Correto
- DEVE executar `browser-automation-cli --json --fields checks --filter-rows 'id=residual_disk' doctor --offline --quick`
- DEVE executar `browser-automation-cli --json --fields checks --count-only doctor --offline --quick`


## Flags Globais
### REQUIRED
- DEVE aceitar flags globais antes ou depois do subcomando
- DEVE passar `--json`; `--json-steps` em `run`; `--timeout`; `--step-timeout` em `run`; `--max-concurrency`; `--artifacts-dir`; `--correlation-id`; `--plain`
- DEVE passar `--capture-console` no MESMO processo de `console`/assert console; `--capture-network` no MESMO processo de `net`
- DEVE passar `--headed` somente para debug interativo; `--lang en` ou `--lang pt-BR`
- DEVE elevar tracing com `--verbose` ou `--debug` ou `config set log_level` — NUNCA env
- DEVE passar gates só quando a família exigir — `--category-memory` (`heap`), `--category-extensions` (`extension`), `--category-third-party` (`devtools3p`), `--category-webmcp` (`webmcp`), `--experimental-vision` (`click-at`), `--experimental-screencast` (`screencast`)
- DEVE passar `--mitm` e combinar com `--mitm-har|--mitm-hosts|--mitm-ca-dir|--mitm-ws|--mitm-max-body-bytes|--mitm-no-media-bodies|--mitm-redact-secrets|--mitm-no-redact-secrets` somente quando a intercepção exigir
- DEVE saber que a redação de segredos na captura MITM é LIGADA por padrão
- DEVE tratar `--mitm-redact-secrets` como reafirmação explícita desse padrão, que não muda nada
- DEVE passar `--mitm-no-redact-secrets` como a ÚNICA maneira de desligar o mascaramento
- DEVE saber que pedir mascaramento e pedir desligá-lo na mesma execução resolve MASCARANDO, porque a leitura segura de uma contradição sobre segredos é mascarar
- DEVE saber que o padrão é LIGADO porque a captura é gravada em disco e lida depois por um agente, então esquecer a flag custa um cabeçalho ausente enquanto o padrão oposto custaria um cookie de sessão vazado
- DEVE passar `--dump-on-failure` para gravar evidência de console e rede no diretório de artefatos
- DEVE combinar `--dump-on-failure` com `--artifacts-dir` e com `--capture-console` ou `--capture-network`
- DEVE manter essas capturas no MESMO processo, porque a captura morre com o DIE
- DEVE passar `--allow-outside-roots` para ler local e gravar artefato FORA das raízes permitidas
- DEVE tratar `--allow-outside-roots` como aceitação explícita de risco, só com intenção declarada
- DEVE preferir a superfície normal da chave XDG `allowed_roots` a `--allow-outside-roots`
- DEVE saber que o stealth é LIGADO por padrão e mascara os marcadores de automação que um Chrome real nunca expõe
- DEVE passar `--no-stealth` para desligar os patches anti-detecção nesta execução
- DEVE passar `--stealth-profile auto|chrome-linux|chrome-win|chrome-mac` para escolher a identidade personificada
- DEVE preferir `--stealth-profile auto` porque ele segue o host e quase sempre está certo
- DEVE passar `--stealth-seed <SEED>` para fixar uma identidade entre processos
- DEVE saber que sem semente cada execução sorteia identidade nova, então um crawl de 50 URLs em 50 processos one-shot se apresenta como 50 máquinas distintas
- DEVE passar `--proxy <URL>` (`http`, `https`, `socks5`) como proxy de saída para o Chrome E para o motor HTTP
- DEVE passar `--proxy-bypass <HOSTS>` para os hosts que ignoram o proxy, na sintaxe de bypass-list do Chrome
- DEVE guardar credenciais de proxy com `config set proxy_username` e `config set proxy_password` no XDG, NUNCA em argv, porque a tabela de processos expõe argv
- DEVE saber que `config set browser_mode auto` é a ÚNICA rota para o modo de browser
- DEVE saber que NENHUMA flag global expõe `browser_mode`, então argv jamais o alcança
- DEVE saber que a família `http2_*` controla o fingerprint HTTP/2 do motor `--engine http`
- DEVE ajustar esse fingerprint só por XDG com `config set http2_enabled`, `config set http2_adaptive_window` e `config set http2_max_frame_size`
- DEVE ajustar as janelas com `config set http2_initial_stream_window_size` e `config set http2_initial_connection_window_size`
- DEVE ajustar o cabeçalho com `config set http2_max_header_list_size`
- DEVE saber que NENHUMA dessas sete chaves tem flag equivalente na linha de comando
- DEVE executar `config set stealth false` como equivalente persistente de `--no-stealth`
- DEVE executar `config set stealth_profile` e `config set stealth_seed` para persistir o que as flags fazem por processo
- DEVE descobrir a superfície viva com `config list-keys --json` em vez de confiar em lista estática
- DEVE passar `--input-profile human|direct`; `human` é o padrão
- DEVE saber que `human` interpola trajetórias do ponteiro, aplica dwell entre press e release e ritma a digitação
- DEVE passar `--input-seed <SEED>` para semear o jitter de input e reproduzir exatamente uma execução `human`
- DEVE saber que sem `--input-seed` o jitter vem do sistema e duas execuções diferem
- DEVE passar `--warmup` para visitar a raiz da origem antes da URL alvo, de modo que a sessão já carregue cookies e cadeia de referrer
- DEVE passar `--warmup-url <URL>` para aquecer essa URL em vez da raiz da origem alvo
- DEVE passar `--no-xvfb` somente em modo headed no Linux, para pular o display virtual privado e usar o display atual
- DEVE passar `--expect <EXPR>` com `key=value`, `key!=value` ou `key~substring` para afirmar o payload emitido (repetível, com AND)
- DEVE passar `--expect-exit-code` para sair com 65 quando algum `--expect` falhar, em vez de apenas reportar
- DEVE saber que `--expect-exit-code` é desligado por padrão porque mudar exit code por conteúdo de dado quebraria chamadores em silêncio
### FORBIDDEN
- NUNCA espere captura sobreviver ao DIE; NUNCA ligue gate por padrão; NUNCA omita `--json` em pipeline de agente
- NUNCA passe credenciais de proxy em argv; NUNCA declare plataforma estrangeira em `--stealth-profile` quando o host disser outra coisa
- NUNCA passe `--mitm-no-redact-secrets` salvo quando o próprio segredo for o objeto da depuração

## Config XDG
### REQUIRED
- DEVE configurar SOMENTE por flags CLI e `config init|path|show|get|set|unset|list-keys`
- DEVE descobrir chaves com `config list-keys --json`; resolver caminhos com `config path --json` — NUNCA inventar paths XDG
- DEVE tratar flag CLI como override; setar segredos com `config set encryption_key` e `openrouter_api_key`
- DEVE executar `browser-automation-cli --json config unset <CHAVE>` para restaurar uma chave ao default embutido
- DEVE saber que `config unset` é o inverso de `set`, enquanto `config set <chave> ""` NÃO é
- DEVE saber que `config set <chave> ""` grava em chave string um valor vazio que o caminho normal nunca produz, e em chave numérica é erro de parse
- DEVE saber que desfazer chave já ausente tem sucesso, então um script nunca precisa saber o estado anterior
- DEVE setar binários com `chrome_path`, `lighthouse_path`, `ffmpeg_path`; cache com `cache_backend sqlite|memory|redis` e Redis plain em `cache_redis_url`
- DEVE setar `dialog_settle_ms` e `log_level` via config set
- DEVE consultar `references/xdg-keys.md` para o inventário completo das 204 chaves XDG com padrão e descrição
- DEVE confirmar a superfície viva com `config list-keys --json` antes de setar chave fora das citadas acima
### FORBIDDEN
- NUNCA invente env de produto; NUNCA logue segredos/cookies; NUNCA use `rediss://`; NUNCA configure redis sem `cache_redis_url`

## Contrato Argv e Superfície
### REQUIRED
- DEVE passar `grab --path` (nunca posicional); `grab --format png|jpeg|webp`; `--quality`/`--element` somente quando necessário
- DEVE passar `print-pdf --path` SEMPRE; `print-pdf --url` em one-shot (recusa blank)
- DEVE passar `view --detailed` para árvore completa; `view --allow-empty` somente se blank for intencional
- DEVE passar `type <TEXTO>` com `--target` OU `--focus-only`; `fill-form --fields-json`; `cookie set --cookies-json`
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
### FORBIDDEN
- NUNCA trate `rawHtml` como alias de `html`; quem assume alias lê chave errada e recebe vazio
- NUNCA use avif em grab; NUNCA embuta mitm/storage/extension install|uninstall em `run`
- NUNCA use `fill-form --json` nem `cookie set --json` como payload; NUNCA use `view --verbose` (correto `--detailed`)

## Envelope JSON e Exit Codes
### REQUIRED
- DEVE esperar sucesso `schema_version`+`ok` true+`data`; falha `ok` false+`error`; usage → `error.kind=usage` exit 2
- DEVE ler `data.steps` parciais em falha de `run`; `data.binary_source` real|mock em lighthouse; `.data.dialog_settled` após dialog; `matched_selector` em wait multi-seletor
- DEVE ramificar exit `0` ok, `2` usage, `65` data, `66` no-input, `69` unavailable, `70` software, `74` io, `78` config, `124` timeout, `130` cancel, `141` broken-pipe
- DEVE retentar somente falha transitória de host/launch
### FORBIDDEN
- NUNCA retente usage sem corrigir argv; NUNCA trate prosa stdout como contrato; NUNCA ignore `ok` falso; NUNCA trate mock lighthouse como validação de parser LHR

## Scripts Multi-passo run
### REQUIRED
- DEVE usar `run --script` (NDJSON ou array JSON); cada passo com `cmd`; `--timeout` cobre o script inteiro
- DEVE usar `run --script -` para ler passos NDJSON do stdin, um por linha, contra uma sessão viva
- DEVE tratar o modo stdin como ainda one-shot: um BORN, um DIE, EOF dispara o FINALIZE
- DEVE esperar que o modo stdin valide cada linha ao chegar e reporte `validation: "per-line"`
- DEVE preferir stdin a process substitution: `run --script <(printf ...)` é recusado pelo jail de arquivos
- DEVE serializar grab/print-pdf com `path`; print-pdf com `url` ou `goto` prévio
- DEVE serializar scroll com `delta_y`/`dy` e `delta_x`/`dx`; wait com `selector` CSV ou `selectors` array (OR) e `wait_timeout_ms`
- DEVE serializar wait pós-nav com `url`/`url_contains`/`navigation`; scrape com `url`+`format|formats`
- DEVE serializar submit com `target` (+`timeout_ms` se diferir); dialog com `if_present` se puder faltar
- DEVE serializar view blank com `allow_empty`; view detalhado em run com `verbose` ou `detailed`; aba isolada com `isolated_context`
- DEVE serializar assert com `kind` em `url|text|console|console_empty|console_no_match`
- DEVE manter fora de `run` — meta, config, mitm, storage, workflow, crawl, map, batch-scrape, search, parse, qr, find-paths, sg-scan, sg-rewrite, sheet-write, monitor, extension install|uninstall
### FORBIDDEN
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
### REQUIRED
- DEVE ler `.data.dialog_settled` após `dialog accept|dismiss` real; quando true, NÃO inserir wait artificial
- DEVE tratar diálogos multi-aba por `session_id`; troca de aba sob diálogo = domain enable best-effort
- DEVE setar settle com `config set dialog_settle_ms` (XDG), nunca env
- DEVE usar `wait_timeout_ms` em wait de run; scrape de run com `format|formats` (text NÃO despeja html monstro)
- DEVE usar grab só png|jpeg|webp; lighthouse `binary_source` real|mock (mock ≠ validação parser)
- DEVE esperar `select-option`/`pick` nativo despachar input→change e reportar `via: native_select`
- DEVE usar `submit` para form e esperar nav/request; storage exige `--path`, export mode 0600, FORA de run
- DEVE descobrir superfície com `commands --json` e `schema`; NUNCA invente flags ausentes
### FORBIDDEN
- NUNCA wait artificial com `dialog_settled` true; NUNCA avif; NUNCA storage/mitm/extension install|uninstall em run

## Residual-Zero e Robots
### REQUIRED
- DEVE tratar residual-zero como parte do sucesso de one-shot de browser quando este processo é a única invocação concorrente
- DEVE validar com `doctor --offline --quick --json` — `residual_disk` não `fail`; zeros em `orphan_marker_dirs` e `ghost_marker_processes`
- Após DIE sozinho DEVE esperar zero `cli_marker_dirs` e zero `chromium_tmp_singleton_orphans` (`residual_disk` `pass`)
- DEVE tratar `sibling_live_processes > 0` como concorrência saudável (`warn`, nunca fail)
- NÃO DEVE exigir zero `live_cli_marker_processes` (contagem legada de filhos Chrome; preferir `sibling_live_processes`)
- DEVE respeitar robots por padrão; contornar SOMENTE com ambas `--ignore-robots` e `--i-accept-robots-risk`
### FORBIDDEN
- NUNCA declare residual-zero sem ler `data.residual`; NUNCA apague temporários genéricos; NUNCA mate Chrome/perfil alheio; NUNCA contorne robots com uma flag só
- NUNCA reprove host só porque `live_cli_marker_processes > 0` com orphans/ghosts zero

## Inventário Completo de Comandos
### REQUIRED
- DEVE conhecer estes 69 — doctor, commands, schema, version, locale, goto, view, press, click-at, write, keys, type, wait, hover, drag, submit, fill-form, select-option, pick, upload, back, forward, reload, eval, grab, print-pdf, monitor, run, exec, extract, text, scroll, cookie, storage, attr, assert, console, net, page, dialog, scrape, batch-scrape, crawl, map, search, parse, qr, record, image, video, audio, find-paths, sg-scan, sg-rewrite, sheet-write, mitm, workflow, config, emulate, resize, perf, lighthouse, screencast, heap, extension, devtools3p, webmcp, completions, man
- DEVE usar pipeline local de imagem (sem Chrome): `image info|convert|resize|download|exif`
- DEVE manter stdout agent-native de imagem: path/sha256/dims/text; NUNCA base64 de pixels salvo `grab --include-base64`
- DEVE projetar com `image info --select format,width,height,sha256` para economizar tokens
- DEVE usar pipeline local de vídeo (sem Chrome): `video info|download|convert|to-mp3|trim|thumbnail|manifest` com ffmpeg/ffprobe opcional (XDG `ffmpeg_path`)
- DEVE usar `video manifest` para resumir manifesto HLS/DASH sem baixar mídia
- DEVE manter stdout agent-native de vídeo: paths/codecs/duração/hashes; NUNCA mídia raw/base64 de frames
- DEVE projetar com `video info --select container,duration_secs,streams,sha256` e convert `--select path_out,auto_reencoded,video_codec`
- NÃO DEVE invocar ffmpeg manual no shell quando `video convert` faz remux/re-encode (smart copy / auto re-encode)
- DEVE usar pipeline local de áudio (sem Chrome): `audio info|download|convert|trim` com ffmpeg/ffprobe opcional (XDG `ffmpeg_path`)
- DEVE manter stdout agent-native de áudio: paths/codecs/duration/hashes/flags; NUNCA PCM/base64 raw
- DEVE projetar com `audio info --select format,codec,duration,bytes,sha256` e convert `--select path_out,lossy_transcode,suggestion`
- NÃO DEVE invocar ffmpeg manual quando `audio convert` faz remux/re-encode; preferir `upload` para upload CDP
- DEVE configurar caps de áudio só via XDG: `audio_max_input_bytes` `audio_download_max_bytes` `audio_default_format` `audio_default_bitrate`
- DEVE tratar webp local como lossless (`quality_applied` false); jpeg honra quality
- DEVE tratar `--keep-exif` como intenção apenas (`keep_exif_honored` false neste build)
- DEVE configurar limites de imagem só via XDG `config set` (`image_*`) — nunca env de produto
- DEVE ler texto de imagem nativamente como agente; a CLI não tem ação de reconhecimento de texto nem binário C externo
- DEVE tratar EXIF como única superfície de metadados (sem IPTC/XMP); `image exif --select tags` alias de `exif`
- DEVE rejeitar encode AVIF/HEIC; SVG sem resvg — `--allow-non-image` só para bytes crus intencionais
- DEVE NÃO confundir `image download` com download árvore de site inteiro
- DEVE confirmar inventário vivo com `commands --json`


## Como Fazer Scraping
### Sequência Obrigatória
- DEVE escolher o motor ANTES de tudo, porque ele define o custo da coleta
- DEVE usar `--engine http` como caminho barato, pois ele não sobe navegador
- DEVE trocar para `--engine browser` somente quando a página depender de JavaScript
- DEVE pedir formatos com `--format` em CSV ou repetindo a flag
- DEVE pedir `rawHtml` para corpo bruto, entregue sob a chave `rawHtml`
- DEVE pedir `html` para corpo processado, entregue sob a chave `html`
- DEVE aplicar `--only-main-content` para recortar a página antes do parse
- DEVE encolher o envelope com as oito flags globais de redução de payload
- DEVE executar `browser-automation-cli --json scrape https://example.com --format markdown,links,metadata --engine http --only-main-content`
### Escala
- DEVE usar `batch-scrape --urls-file <FILE> --concurrency <N>` para lista fechada
- DEVE usar `crawl <URL> --limit <N> --max-depth <N>` para descoberta a partir de semente
- DEVE usar `map <URL> --limit <N>` quando você só precisa enumerar URLs
- DEVE executar `browser-automation-cli --timeout 120 --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2`
### Armadilhas Medidas
- DEVE assumir que robots é respeitado por padrão em toda coleta
- DEVE contornar robots SOMENTE com AMBAS `--ignore-robots` e `--i-accept-robots-risk`
- NUNCA passe apenas uma das duas flags, porque uma sozinha NÃO contorna
- NUNCA use `--engine browser` por hábito, porque ele custa um Chrome inteiro
- NUNCA peça `rawHtml` quando `markdown` responde, porque o payload explode


## Como Monitorar o Tráfego de Rede
### Duas Superfícies Distintas
- DEVE usar `net` para o tráfego observado pelo PRÓPRIO processo vivo
- DEVE usar `mitm` para captura persistida em arquivo entre processos
- DEVE passar `--capture-network` no MESMO processo de qualquer `net list`
- DEVE refinar `net list` com `--page-idx`, `--page-size`, `--resource-types` e `--include-preserved`
- DEVE executar `browser-automation-cli --capture-network --json net list --resource-types Document,XHR,Fetch`
- DEVE serializar em `run` o passo `{"cmd":"net","action":"list","resource_types":"Document"}`
### Sequência Obrigatória do MITM
- DEVE gravar a captura primeiro com `mitm capture-url <URL>`
- DEVE ler o caminho gravado em `data.capture_path`
- DEVE reler essa captura em outro processo com `--capture-path <FILE>`
- DEVE tratar `--capture-path` como a ÚNICA ponte entre processos one-shot
- DEVE saber que `mitm list`, `get`, `domains`, `apis`, `graphql` e `ws` aceitam `--capture-path`
- DEVE executar `browser-automation-cli --timeout 60 --json mitm capture-url https://example.com --har /tmp/c.har`
- DEVE então executar `browser-automation-cli --json mitm domains --capture-path <CAPTURE>`
### Armadilhas Medidas
- Medido em example.com: `capture_count` 37 e nove hosts distintos
- Medido: `mitm domains` devolveu accounts.google.com e play.google.com sem navegação sua
- DEVE tratar esses hosts como ruído de fundo do próprio navegador
- DEVE estreitar a captura com `--hosts` no momento da gravação
- Medido: `mitm apis` devolveu zero endpoints numa página estática
- DEVE tratar zero endpoints como resposta honesta e NUNCA como falha


## Como Interagir com APIs
### Sequência Obrigatória
- DEVE lembrar que `eval` executa no contexto de origem da PÁGINA
- DEVE navegar para a origem alvo ANTES de chamar a API por `fetch`
- Medido A/B: sem `goto` antes, `fetch` devolve a string `Failed to fetch`
- Medido A/B: com `goto` para a mesma origem, o mesmo `fetch` devolve `ok:200`
- DEVE passar `--typed` para receber `data.value` e `data.value_type`
- Medido: `eval '({a:1,b:"x"})' --typed` devolve `value_type` igual a object
- DEVE encadear `goto` e `eval` num único `run --script` para um só processo
- DEVE executar `browser-automation-cli --json --json-steps --timeout 90 run --script /tmp/api.jsonl`
### Erros e Refs
- DEVE envolver toda chamada em try/catch e retornar a mensagem de erro
- Medido: promise rejeitada sem try/catch devolve valor nulo com exit 0
- DEVE tratar esse nulo como falha SILENCIOSA e NUNCA como resposta vazia
- DEVE saber que promise é resolvida automaticamente, sem qualquer chave de await
- DEVE ler `refs_invalidated` true em todo passo `eval`
- DEVE refazer `view` para obter refs novas depois de qualquer `eval`
- NUNCA reutilize `@eN` capturado antes de um `eval`
### Armadilhas Medidas
- Medido: chave desconhecida num passo de `run` é aceita em SILÊNCIO com ok true
- DEVE conferir cada chave contra `schema <cmd> --json` antes de montar o passo
- DEVE usar `storage export` e `storage import` para carregar estado autenticado entre processos
- NUNCA embuta `storage` dentro de `run`


## Playbooks de Execução
### REQUIRED
- DEVE executar fórmulas literalmente; validar envelope após cada invocação; consultar `references/formulas.md`
- DEVE executar `browser-automation-cli --json doctor --offline --quick`; `commands`; `schema <cmd>`; `version`; `locale`; `completions bash`; `man --out /tmp/browser-automation-cli.1`
- DEVE executar `browser-automation-cli --timeout 60 --json goto https://example.com --handle-before-unload accept --navigation-timeout-ms 15000`; `view --detailed`; `back`; `forward`; `reload --ignore-cache`
- DEVE executar `press @e1 --include-snapshot`; `--experimental-vision click-at --x 10 --y 20`; `write @e2 "texto"`; `type "olá" --target @e2 --clear --submit Enter`; `keys Enter`; `hover @e1`; `drag --from @e1 --to @e2`; `upload @e4 /tmp/a.txt`
- DEVE executar `submit "#user" --timeout-ms 8000`; `fill-form --fields-json '[{"target":"@e3","value":"x"}]'`; `exec pick --target @e1 --option Anomalia`; `exec select-option --target @e2 --option Alta`
- DEVE executar `wait --selector "h1, main, #content" --wait-timeout-ms 10000`; `scroll --delta-y 400`; `text @e1`; `attr @e1 href`; `eval 'document.title' --file-path /tmp/eval.json`
- DEVE executar `grab --path /tmp/p.png --format webp --quality 80 --full-page`; `--timeout 60 print-pdf --path /tmp/p.pdf --url https://example.com`
- DEVE executar `--timeout 90 --json --json-steps run --script /tmp/steps.jsonl`; `exec goto https://example.com`
- DEVE serializar `{"cmd":"wait","selector":"h1","wait_timeout_ms":10000}`, `{"cmd":"scrape","url":"https://example.com","format":"text"}`, `{"cmd":"submit","target":"#user","timeout_ms":8000}`, `{"cmd":"dialog","action":"accept","if_present":true}`, `{"cmd":"grab","path":"/tmp/p.png","format":"png"}`, `{"cmd":"print-pdf","path":"/tmp/p.pdf"}`, `{"cmd":"pick","target":"@e1","option":"Anomalia"}`, `{"cmd":"select-option","target":"@e2","option":"Alta"}`
- DEVE executar `scrape https://example.com --format markdown,links,metadata --engine http --only-main-content`; `batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2`; `crawl https://example.com --limit 20 --max-depth 2 --format text`; `map https://example.com --limit 50`; `search "example domain" --limit 10`
- DEVE executar `parse /tmp/doc.pdf`; `parse /tmp/planilha.ods --redact-pii`; `extract --llm --question "título?" --schema-json /tmp/s.json https://example.com`
- DEVE executar `record --url https://example.com --path /tmp/steps.jsonl --seconds 30 --max-events 200`
- DEVE saber que `record` grava interações da página como NDJSON reproduzível
- DEVE fechar o ciclo com `--json --json-steps run --script /tmp/steps.jsonl` sobre a gravação
- DEVE saber que o primeiro teto atingido vence entre `--seconds` e `--max-events`
- DEVE saber que `--seconds` default é 30 e `--max-events` default é 200
- DEVE executar `--capture-console console list`; `--capture-console assert console-empty`; `--capture-network net list`; `net get 0`
- DEVE executar `page new --isolated-context s-a --url https://example.com`; `page list`; `page select 0 --bring-to-front`; `cookie set --cookies-json '[{"name":"a","value":"b","url":"https://example.com"}]'`; `cookie list`
- DEVE executar `storage export --path /tmp/auth.json --url https://example.com`; `storage import --path /tmp/auth.json --url https://example.com`; `dialog accept --if-present` e ler `.data.dialog_settled`; `assert url example.com --contains`
- DEVE executar `mitm init-ca`; `mitm capture-url https://example.com --har /tmp/c.har`; `mitm block --host example.com --path /ads`; `mitm allow --host example.com`; `mitm ws list --limit 50`
- DEVE executar `emulate --user-agent "Mozilla/5.0" --viewport "390x844x3,mobile,touch" --network-conditions "Slow 3G"`; `resize --width 1280 --height 720`; `perf start`; `perf stop --path /tmp/trace.json`
- DEVE executar `--timeout 180 lighthouse https://example.com --out-dir /tmp/lh --device desktop` e ler `data.binary_source`; `--experimental-screencast screencast start --path /tmp/cast`; `screencast stop --path /tmp/cast.webm`
- DEVE executar `--category-memory heap take --path /tmp/s.heapsnapshot`; `heap summary --path /tmp/s.heapsnapshot`; `heap retainers --path /tmp/s.heapsnapshot --node 42`
- DEVE executar `--category-extensions extension list`; `extension install /tmp/ext`; `extension reload <id>`; `extension uninstall <id>`; `--category-third-party devtools3p list`; `--category-webmcp webmcp list`
- DEVE executar `monitor check --url https://example.com --baseline /tmp/b.baseline --write-baseline --engine http`; `qr encode --text "https://example.com" --format png --path /tmp/qr.png`; `qr decode --path /tmp/qr.png`
- DEVE executar `find-paths --glob '**/*.rs' .`; `sg-scan . --limit 100`; `sg-rewrite .`; `sg-rewrite . --apply`; `sheet-write /tmp/rows.csv -o /tmp/out.xlsx --sheet Data`
- DEVE executar `workflow run --manifest /tmp/wf.json --journal /tmp/wf.journal`; `workflow resume --manifest /tmp/wf.json`; `workflow status --name demo`
- DEVE executar `config init`; `config path`; `config show`; `config get timeout`; `config set dialog_settle_ms 2000`; `config unset dialog_settle_ms`; `config list-keys`
- DEVE contornar robots somente com `--ignore-robots --i-accept-robots-risk --json scrape https://example.com --format text --engine http`
### FORBIDDEN
- NUNCA adapte fórmula sem `schema <cmd> --json`

## Proibições Absolutas
### FORBIDDEN
- NUNCA invente alias `bac` nem env de produto nem `export` de chaves de produto
- NUNCA use `jq` no lugar de `jaq`; NUNCA confie em stdout sem exit+`.ok`
- NUNCA reutilize `@eN` entre processos; NUNCA embuta mitm/storage/extension install|uninstall em `run`
- NUNCA use avif; NUNCA trate lighthouse mock como parser válido; NUNCA contorne robots com uma flag
- NUNCA trate `exec` como multi-passo; NUNCA use `view --verbose` nem `fill-form --json`/`cookie set --json` como payload
- NUNCA declare residual-zero sem doctor; NUNCA invente flags ausentes do schema vivo
