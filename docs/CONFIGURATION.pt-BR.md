[English](CONFIGURATION.md)


# Referência de Configuração


- Referência canônica XDG de toda chave durável de configuração do `browser-automation-cli`


## Como a Configuração é Resolvida
- O produto NÃO lê variáveis de ambiente de produto em nenhuma circunstância
- Toda configuração durável vive no arquivo XDG `config.toml`
- A precedência é flag de CLI primeiro, chave XDG depois e padrão embutido por último
- Uma flag de CLI sobrescreve a chave XDG só na invocação que a carrega
- Uma chave XDG sobrescreve o padrão embutido em toda invocação naquele host
- Uma chave ausente no `config.toml` cai no padrão embutido documentado abaixo
- Uma chave sem padrão embutido permanece desativada até você defini-la ou passar a flag correspondente
- Segredos como `openrouter_api_key` e `encryption_key` são gravados com permissão `0600`
- Segredos nunca aparecem em log, em envelope JSON nem no stderr humano
- O bypass de robots continua exigindo `--ignore-robots` e `--i-accept-robots-risk` juntos na linha de comando
- Descubra o arquivo vencedor com `config path`


## Comandos de Configuração
- `config init` cria o arquivo de configuração XDG quando ele não existe
- `browser-automation-cli --json config init`
- `config path` imprime os caminhos resolvidos de configuração e de estado
- `browser-automation-cli --json config path`
- `config show` imprime os valores guardados no `config.toml`
- `browser-automation-cli --json config show`
- `config get <key>` lê o valor guardado de uma única chave
- `browser-automation-cli --json config get timeout`
- `config set <key> <value>` grava um valor durável
- `browser-automation-cli --json config set dialog_settle_ms 3000`
- `config unset <key>` restaura uma chave ao padrão embutido
- `browser-automation-cli --json config unset dialog_settle_ms`
- `config unset` numa chave já ausente continua tendo sucesso
- `config set <key> ""` não desfaz nada, porque grava string vazia ou falha no parse de número
- `config list-keys` lista toda chave aceita pelo binário em execução
- `browser-automation-cli --json config list-keys`

### Gravar e desfazer uma chave
- Grave o valor com `config set dialog_settle_ms 3000`
- Confirme o valor guardado com `config get dialog_settle_ms`
- Desfaça a gravação com `config unset dialog_settle_ms`
- O envelope do `unset` informa `was_set: true` quando a chave estava no arquivo
- Leia o padrão embutido em `config list-keys`, e nunca em `config get`
- Medido na 0.2.0 com um arquivo XDG novo: `config get http_timeout_secs` devolveu `0` enquanto `config list-keys` informa o padrão `30`

```bash
browser-automation-cli --json config set dialog_settle_ms 3000
browser-automation-cli --json config get dialog_settle_ms
browser-automation-cli --json config unset dialog_settle_ms
browser-automation-cli --json config list-keys
```


## Núcleo e Idioma
- `lang` — sobrescreve o idioma das mensagens humanas, aceitando `en` ou `pt-BR`, com `pt` puro rejeitado, padrão nenhum
- O `lang` seleciona o idioma apenas do campo `suggestion`
- A `message` é o diagnóstico técnico e permanece em inglês em qualquer locale, a mesma divisão que `rustc` e `git` entregam — veja `docs/AGENTS.pt-BR.md`
- `timeout` — timeout global de execução em segundos, padrão `0`
- `artifacts_dir` — diretório de saída dos artefatos gerados, padrão nenhum
- `namespace` — namespace isolado de estado do produto, padrão nenhum
- `encryption_key` — material de chave para cifrar o estado de sessão, padrão nenhum
- `color` — habilita cores ANSI na saída humana enviada ao stderr, padrão nenhum


## Registro de Log
- `log_level` — filtro de tracing aplicado quando as flags de argv estão silenciosas, sem qualquer leitura de `RUST_LOG`, padrão `error`
- `log_to_file` — grava logs JSON locais rotacionados sob o diretório de estado XDG, nunca remotos, padrão `false`
- `mitm/redact_policy.json` — default de redação de segredos persistido sob o diretório de estado XDG, gravado por `mitm redact --secrets true|false`
- Lido apenas quando nem `--mitm-redact-secrets` nem `--mitm-no-redact-secrets` estão no argv
- O argv sempre vence e o padrão embutido é redigir
- `max_log_files` — número de arquivos de log rotacionados retidos, na faixa de 1 até 90, padrão `14`
- `log_rotation` — política de rotação, aceitando `daily`, `hourly` ou `never`, padrão `daily`


## Binários Externos
- `chrome_path` — caminho absoluto do binário Chrome ou Chromium, padrão nenhum
- `lighthouse_path` — caminho absoluto da CLI `lighthouse`, padrão nenhum
- `ffmpeg_path` — caminho absoluto do `ffmpeg`, opcional para codificar screencast e converter vídeo ou extrair MP3, padrão nenhum
- `lighthouse_timeout_secs` — teto de tempo real da CLI `lighthouse` em segundos, na faixa de 1 até 3600, padrão `300`
- `ffmpeg_timeout_secs` — teto de tempo real da codificação `ffmpeg` em segundos, na faixa de 1 até 3600, padrão `120`


## LLM e Webhooks
- `openrouter_api_key` — chave de API do provedor de LLM, armazenada com permissão `0600`, padrão nenhum
- `llm_base_url` — URL base compatível com a API OpenAI, padrão nenhum
- `llm_model` — identificador do modelo de LLM usado por padrão, padrão nenhum
- `llm_http_timeout_secs` — timeout HTTP bloqueante para chamadas de LLM e webhook em segundos, padrão `60`
- `webhook_post_timeout_secs` — timeout do POST de webhook operacional em segundos, padrão `15`
- `webhook_retry_base_delay_ms` — atraso base de retentativa de webhook em milissegundos, dobrando a cada tentativa, padrão `50`
- `webhook_max_attempts` — número máximo de tentativas de webhook, incluindo a primeira, padrão `3`


## Cache e Redis
- `cache_backend` — backend de cache, aceitando `sqlite`, `memory` ou `redis`, padrão `sqlite`
- `cache_redis_url` — URL do Redis quando o backend é `redis`, padrão nenhum
- `redis_allow_remote` — permite hosts Redis fora do loopback, padrão `false`
- `redis_connect_timeout_secs` — timeout de conexão TCP com o Redis em segundos, padrão `2`
- `redis_io_timeout_secs` — timeout de entrada e saída do stream RESP do Redis em segundos, padrão `3`
- `cache_max_resp_bulk_bytes` — teto de tamanho da bulk string RESP do Redis em bytes, padrão `16777216`
- `cache_max_resp_line_bytes` — teto de tamanho da linha RESP do Redis em bytes, padrão `16777216`
- `scrape_http_cache_ttl_secs` — período de validade do cache L2 de respostas HTTP de scrape em segundos, padrão `3600`
- `file_parse_cache_ttl_secs` — período de validade do cache L2 de parse de arquivo local em segundos, padrão `86400`


## HTTP e Segurança de Rede
- `http_ssrf_mode` — política HTTP contra SSRF, aceitando `strict`, `allow_loopback` ou `off`, padrão `strict`
- `http_timeout_secs` — timeout total do cliente HTTP compartilhado em segundos, padrão `30`
- `http_connect_timeout_secs` — timeout da fase de conexão HTTP em segundos, padrão `10`
- `http_redirect_max` — número máximo de redirecionamentos HTTP seguidos pelos clientes do produto, padrão `10`
- `http_pool_max_idle_per_host` — número máximo de conexões ociosas do pool `reqwest` por host, padrão `4`
- `scrape_max_body_bytes` — número máximo de bytes do corpo em scrape HTTP, padrão `5000000`
- `browser_scrape_max_body_bytes` — número máximo de bytes do corpo nos auxiliares de scrape do motor de navegador, padrão `2000000`
- `search_base_url` — URL base do endpoint HTML de busca, ao qual `?q=` é anexado, padrão `https://html.duckduckgo.com/html/`
- `user_data_dir` — diretório de perfil persistente do Chrome, opt-in, padrão nenhum
- Ausente por padrão, e a ausência é o que preserva o residual-zero: o launch recebe um perfil descartável e a execução não deixa nada em disco
- Defina apenas quando um detector atestar sessão entre invocações, porque perfil persistente é diretório que esta CLI nunca apaga por você
- Criado com modo 0700 em Unix
- `--profile` no argv vence esta chave


## Robots e Polidez
- `ignore_robots` — ignora `robots.txt` por padrão, sendo que as flags de risco continuam obrigatórias, padrão `false`
- `robots_loopback_exempt` — hosts de loopback pulam o `robots.txt`, e `false` passa a aplicá-lo contra `localhost`, padrão `true`
- `robots_probe_timeout_secs` — timeout da requisição de `robots.txt` em segundos, padrão `5`
- `robots_max_body_bytes` — limite de bytes do corpo do `robots.txt`, como proteção contra estouro de memória, padrão `524288`
- `scrape_min_delay_ms` — atraso mínimo entre requisições GET de mesma origem em milissegundos, padrão `0`
- `scrape_honor_meta_robots` — respeita as diretivas `meta robots` e `X-Robots-Tag` do tipo `noindex`, padrão `true`
- `scrape_honor_nofollow` — pula links com `rel=nofollow` durante a descoberta do crawl, padrão `true`
- `scrape_delay_jitter_ratio` — razão de variação aleatória do atraso de polidez na faixa `0.0..=1.0`, com `0` desligando, padrão `0.2`


## Scrape e Crawl
- `scrape_default_engine` — motor de scrape usado quando a CLI omite `--engine`, aceitando `http` ou `browser`, padrão `http`
- `scrape_use_sitemap` — prefere o `sitemap.xml` ao mapear um site, padrão `true`
- `scrape_max_text_chars` — número máximo de caracteres de texto ou markdown nos envelopes de scrape, com `0` removendo o teto, padrão `32768`
- `scrape_summary_chars` — número máximo de caracteres do formato `summary` de scrape, padrão `400`
- `scrape_feed_max_entries` — número máximo de entradas mantidas pelo formato `feed` de scrape, cobrindo RSS, Atom e JSON Feed, padrão `50`
- `scrape_follow_rel_next` — segue links de paginação com `rel=next` durante o crawl, padrão `false`
- `scrape_dedup_similar` — colapsa páginas quase duplicadas por similaridade de conteúdo em `crawl` e `batch-scrape`, padrão `false`
- `scrape_no_cache` — ignora o cache de resposta na LEITURA e sempre busca na origem, padrão `false`
- A resposta nova continua sendo gravada, então uma chamada que faz bypass atualiza a entrada para quem vier depois em vez de deixar uma entrada velha
- `--no-cache` no `scrape` sobrescreve por invocação
- Não há como dizer o mesmo com `scrape_http_cache_ttl_secs`: o TTL `0` já significa "nunca expira", que é o oposto, e a chave o rejeita
- O `monitor check` faz bypass incondicional e ignora esta chave, porque um corpo vindo do cache o fazia comparar uma página armazenada consigo mesma e reportar `changed: false`
- `scrape_dedup_similar_distance` — distância de Hamming do SimHash, de 0 até 64, abaixo da qual as páginas são quase duplicadas, padrão `3`
- `scrape_sitemap_max_bytes` — número máximo de bytes do corpo do sitemap, padrão `2000000`
- `scrape_charset_peek_bytes` — janela de inspeção usada para detectar o charset, em bytes, padrão `4096`
- `scrape_crawl_limit_max` — orçamento máximo de páginas do crawl, atuando como teto contra abuso para `--limit`, padrão `500`
- `scrape_crawl_max_depth` — profundidade máxima da busca em largura (BFS) para `crawl` e `map`, padrão `10`
- `scrape_search_limit_max` — orçamento máximo de resultados de busca, atuando como teto contra abuso, padrão `50`
- `scrape_max_parse_bytes` — tamanho máximo de arquivo local aceito para parse antes da rejeição, em bytes, padrão `50000000`
- `max_urls_file_bytes` — número máximo de bytes da lista informada em `batch-scrape --urls-file`, padrão `8388608`


## Imagem
- `image_max_input_bytes` — número máximo de bytes de entrada para decodificar, converter ou redimensionar imagem local, padrão `32000000`
- `image_max_pixels` — produto máximo de largura por altura na decodificação de imagem, como proteção contra bomba de descompressão, padrão `64000000`
- `image_default_format` — formato padrão da conversão de imagem, aceitando `png`, `jpeg`, `webp` ou `gif`, padrão `png`
- `image_default_quality` — qualidade padrão com perda, de 1 até 100, para conversão e redimensionamento de imagem, padrão `85`
- `image_download_max_bytes` — número máximo de bytes do corpo HTTP no download de imagem, padrão `32000000`
- `image_avif_speed` — velocidade do codificador AVIF, de 1 até 10, sendo 1 a mais lenta e de melhor resultado, exigindo a feature `image-avif`, padrão `6`
- `default_jpeg_quality` — qualidade JPEG de 1 até 100 quando `grab` omite `--quality`, padrão `80`


## Vídeo e Áudio
- `video_max_input_bytes` — número máximo de bytes na materialização de vídeo vindo do stdin ou na verificação prévia do caminho, padrão `512000000`
- `video_download_max_bytes` — número máximo de bytes do corpo HTTP no download de vídeo, padrão `512000000`
- `video_default_container` — contêiner padrão da conversão de vídeo, aceitando `mp4`, `webm`, `mkv`, `mov`, `avi` ou `m4v`, padrão `mp4`
- `video_default_crf` — valor CRF padrão, de 1 até 51, para recodificação de vídeo com perda, padrão `23`
- `video_default_audio_bitrate` — taxa de bits padrão na conversão de vídeo para MP3, por exemplo `192k`, padrão `192k`
- `audio_max_input_bytes` — número máximo de bytes na materialização de áudio vindo do stdin ou na verificação prévia do caminho, padrão `256000000`
- `audio_download_max_bytes` — número máximo de bytes do corpo HTTP no download de áudio, padrão `256000000`
- `audio_default_format` — formato padrão da conversão de áudio, aceitando `mp3`, `m4a`, `ogg`, `opus`, `flac`, `wav` ou `aac`, padrão `mp3`
- `audio_default_bitrate` — taxa de bits padrão na codificação de áudio com perda, por exemplo `192k`, padrão `192k`


## SVG, GIF e Manifestos
- `svg_max_bytes` — número máximo de bytes do código-fonte SVG aceito antes da rasterização, padrão `4000000`
- `svg_max_depth` — profundidade máxima de aninhamento XML aceita em um código-fonte SVG, padrão `128`
- `svg_max_entities` — número máximo de declarações `<!ENTITY>` toleradas na DTD do SVG, com `0` rejeitando qualquer uma, padrão `0`
- `gif_max_frames` — número máximo de quadros de animação decodificados de um GIF, padrão `2000`
- `manifest_max_bytes` — número máximo de bytes aceitos no corpo de um manifesto HLS ou DASH, padrão `8000000`
- `manifest_max_variants` — número máximo de entradas de variante ou representação emitidas por envelope de manifesto, padrão `500`


## Motor Chrome e Ciclo de Vida
- `chrome_search_paths` — caminhos ordenados de descoberta do Chrome ou Chromium, separados pelo separador da plataforma, com o valor vazio usando o layout embutido de cada sistema, padrão nenhum
- `chrome_legacy_oxide_launch` — inicia o Chrome via `chromiumoxide` em vez do caminho de auto-spawn, servindo como recuo de estabilização e perdendo o alvo de encerramento residual, padrão `false`
- Gravar `chrome_legacy_oxide_launch` como `true` reabre uma porta DevTools em loopback sem autenticação e nunca sobe o Xvfb privado, o que desfaz as duas correções de headed e de segurança da `0.2.0`
- `chrome_startup_timeout_secs` — espera pela prontidão do CDP no auto-spawn do Chrome em segundos, medida na ponte do pipe de DevTools, padrão `20`
- A ponte dá ao cliente CDP o dobro desse orçamento para conectar
- `chrome_default_timeout_ms` — timeout padrão por operação do motor Chrome em milissegundos, padrão `25000`
- `browser_close_wait_secs` — orçamento de espera por `Browser.close` e pelo término do processo durante a fase FINALIZE, em segundos, padrão `5`
- `residual_orphan_min_age_secs` — idade mínima antes que um perfil marcador de dono morto se torne coletável, em segundos, padrão `60`
- `platform_child_wait_secs` — prazo de espera pelo processo filho da plataforma em segundos, padrão `5`
- `platform_child_poll_ms` — intervalo de sondagem de saída do processo filho durante o FINALIZE em milissegundos, padrão `50`
- `shutdown_deadline_secs` — prazo rígido de desligamento aguardando a saída do navegador, em segundos, padrão `30`


## CDP e Eventos
- `cdp_connection_probe_timeout_secs` — timeout da sondagem de vitalidade `Browser.getVersion` do CDP em segundos, padrão `3`
- `cdp_discovery_timeout_secs` — timeout da descoberta HTTP do CDP nas sondagens de `/json/version`, em segundos, padrão `2`
- Nenhum caminho de lançamento lê essa chave: o Chrome lançado pelo próprio produto usa o pipe de DevTools e não expõe `/json/version`, e a prontidão do Lightpanda usa `lightpanda_discovery_timeout_ms`
- `cdp_discovery_max_body_bytes` — número máximo de bytes do corpo HTTP de descoberta CDP em `/json/version` e `/json/list`, padrão `1048576`
- Só a sondagem de prontidão do Lightpanda lê essa chave, porque o Chrome lançado pelo próprio produto usa o pipe de DevTools
- `cdp_event_broadcast_capacity` — capacidade do canal local de difusão de eventos CDP dentro do processo, padrão `4096`
- `cdp_event_drain_poll_ms` — fatia de sondagem no esvaziamento de eventos CDP durante a espera por navegação, em milissegundos, padrão `100`
- `cdp_network_idle_settle_ms` — janela de estabilização de rede ociosa do CDP em milissegundos, padrão `500`
- `cdp_target_event_wait_ms` — espera curta por evento de target do CDP em milissegundos, padrão `600`
- `event_tracker_max_entries` — tamanho do anel em memória do rastreador de console e rede por sessão de página, padrão `1000`
- `capture_preserved_rings` — fronteiras de navegação mantidas para `--include-preserved` de console e rede, padrão `3`
- `event_pump_slice_ms` — fatia da bomba de eventos usada em `wait` e `eval`, em milissegundos, padrão `50`
- `eval_drain_slice_ms` — fatia de esvaziamento durante a espera pelos resultados de `Runtime.evaluate`, em milissegundos, padrão `40`
- `extension_attach_poll_ms` — fatia de sondagem ao anexar uma extensão, em milissegundos, padrão `150`
- `extension_attach_poll_iters` — iterações de sondagem ao anexar extensão, padrão `20`
- Fatia vezes iterações dá a espera total


## Motor Lightpanda
- `lightpanda_startup_timeout_secs` — espera pela inicialização do processo Lightpanda em segundos, padrão `10`
- `lightpanda_session_timeout_secs` — duração máxima da sessão informada em `--timeout` do Lightpanda, em segundos, na faixa de 1 até 604800, padrão `604800`
- `lightpanda_poll_interval_ms` — intervalo de sondagem da prontidão do CDP do Lightpanda em milissegundos, padrão `100`
- `lightpanda_discovery_timeout_ms` — timeout por sondagem de descoberta CDP durante a espera pelo Lightpanda, em milissegundos, padrão `500`
- `lightpanda_max_log_lines` — anel limitado de log da inicialização do Lightpanda, contado em linhas por fluxo, padrão `40`
- `lightpanda_ready_slice_ms` — fatia de esvaziamento após a saída do processo filho Lightpanda, antes de capturar os logs, em milissegundos, padrão `25`
- `lightpanda_cdp_connect_timeout_secs` — timeout da tentativa de conexão CDP com o Lightpanda em segundos, padrão `5`
- `lightpanda_target_init_timeout_secs` — espera pela inicialização do target do Lightpanda após a conexão, em segundos, padrão `10`


## Interação e Esperas
- `interact_settle_ms` — atraso de estabilização da interface após clique, digitação ou ação de extensão, em milissegundos, padrão `200`
- `dialog_settle_ms` — espera máxima após responder um diálogo JavaScript até o evento `javascriptDialogClosed`, em milissegundos, padrão `2000`
- `network_idle_window_ms` — janela de silêncio usada por `wait --network-idle` em milissegundos, padrão `500`
- `dom_stable_window_ms` — janela de silêncio usada por `wait --dom-stable-ms` em milissegundos, padrão `500`
- `drag_move_steps` — número de posições intermediárias do mouse sintetizadas em um arrasto HTML5, padrão `6`
- `drag_move_gap_ms` — atraso entre as posições sintetizadas de arrasto em milissegundos, padrão `16`
- `input_profile` — modelagem de input padrão quando `--input-profile` está ausente: `human` sintetiza trajetória, tiques de roda e eventos de tecla, padrão `human`
- `direct` mantém o despacho anterior a 0.1.8
- A flag continua vencendo
- `browser_mode` — modo de janela: `auto` resolve para `headed` dentro de um display virtual privado no Linux com Xvfb no PATH e sem `--no-xvfb`, e para `headless` em qualquer outro caso, padrão `auto`
- `headed` coloca uma janela real no seu display, e no Linux essa janela é renderizada em um display virtual privado quando há Xvfb
- `headless` é o mais barato e o mais detectável
- `--headed` continua vencendo
- Inverter o padrão de `auto` tem custo de latência e é decisão separada
- O `doctor` reporta para que `auto` resolve neste host no check `virtual_display`, então a resposta nunca diverge do binário
- `stealth` — patches de anti-detecção aplicados antes da primeira navegação, padrão `true`
- `--no-stealth` desliga por uma execução
- `stealth_profile` — identidade personificada: `auto`, `chrome-linux`, `chrome-win`, `chrome-mac`, padrão `auto`
- `auto` segue o host, e é o único valor que não contradiz os hashes de Canvas e WebGL que a GPU real produz
- `doctor --fingerprint` (não é chave XDG) — campos do envelope `measurement_scope` (`linux-headless-xvfb`), `unmeasured_os` (`macos`, `windows`), `measurement_note`
- Canvas/WebGL/áudio ao vivo só foram pontuados em Linux headless + Xvfb
- Os mesmos tipos compilam em macOS e Windows
- `proxy_url` — proxy de saída para o Chrome e para o motor HTTP (`http`, `https`, `socks5`, `socks5h`), padrão nenhum
- Guarde credenciais aqui em vez de `--proxy`, onde a tabela de processos as expõe
- `proxy_bypass` — hosts que ignoram o proxy, na sintaxe de bypass-list do Chrome, padrão nenhum
- `proxy_username` — nome de conta do proxy, enviado como basic auth, padrão nenhum
- Fica aqui e não no argv, onde a tabela de processos o exporia
- `proxy_password` — senha do proxy, enviada como basic auth, padrão nenhum
- Nunca ecoada por `config get` nem `config show`
- `cdp_proxy_bypass_loopback` — sempre ignorar o loopback quando o Chrome roda sob `--proxy`, padrão `true`
- O canal de controle CDP é loopback, então um proxy que o captura produz um browser que nunca responde — reportado como timeout de inicialização do Chrome, o que culpa o componente errado
- `stealth_seed` — fixa a identidade de stealth para reproduzir o mesmo fingerprint entre processos, padrão nenhum
- A ausência significa identidade nova a cada execução, que é o padrão justamente porque cachear identidade a grava em disco
- `http2_enabled` — oferecer `h2` no ALPN, padrão `true`
- O ALPN é visível em claro durante o handshake TLS e o Chrome sempre lista `h2`, então um cliente que só oferece `http/1.1` já respondeu "não sou browser" antes de enviar um byte
- `http2_initial_stream_window_size` — `SETTINGS_INITIAL_WINDOW_SIZE` anunciado ao par, padrão `6291456`
- Os defaults de biblioteca ficam três ordens de magnitude longe do Chrome
- `http2_initial_connection_window_size` — janela de controle de fluxo no nível da conexão anunciada ao par, padrão `15663105`
- `http2_max_header_list_size` — `SETTINGS_MAX_HEADER_LIST_SIZE` anunciado ao par, padrão `262144`
- `http2_max_frame_size` — `SETTINGS_MAX_FRAME_SIZE` anunciado ao par, na faixa `16384..=16777215`, padrão `16384`
- `http2_adaptive_window` — deixar a pilha HTTP/2 redimensionar janelas dinamicamente, padrão `false`
- Desligado mantém os valores anunciados fixos, que é o que torna o fingerprint reproduzível
- `robots_user_agent` — token de user-agent contra o qual as regras do `robots.txt` são avaliadas, padrão nenhum
- Defina quando o stealth enviar um User-Agent de navegador, para que as regras avaliadas sejam as que valem para a requisição realmente enviada
- `input_move_steps` — posições intermediárias do ponteiro sintetizadas em um movimento (perfil `human`), padrão `24`
- `input_move_gap_ms` — atraso entre as posições sintetizadas do ponteiro, em milissegundos, padrão `12`
- `input_click_dwell_ms` — tempo de retenção entre `mousePressed` e `mouseReleased`, em milissegundos, padrão `65`
- `input_key_dwell_ms` — tempo de retenção entre `keyDown` e `keyUp`, em milissegundos, padrão `45`
- `input_type_delay_ms` — atraso entre caracteres durante a digitação, em milissegundos, padrão `95`
- `input_scroll_tick_px` — distância de rolagem carregada por um tique de roda sintetizado, em pixels CSS, padrão `100`
- `input_scroll_max_ticks` — teto do número de tiques de roda que um gesto de rolagem sintetiza, padrão `40`
- Cada tique é um round-trip CDP, então sem teto o custo da rolagem cresce linearmente com a distância pedida e um `--delta-y` grande esgota o timeout do comando
- Acima do teto cada tique carrega mais pixels
- O percurso total não muda e só a granularidade degrada
- `input_target_jitter_px` — raio do deslocamento aleatório aplicado ao alvo do clique, em pixels CSS, padrão `3`
- `input_scroll_settle_rounds` — rodadas extras permitidas para entregar um delta de roda que o renderizador descartou, padrão `3`
- `input_timing_distribution` — forma da dispersão sorteada em torno de cada atraso de input: `lognormal`, `normal` ou `uniform`, padrão `lognormal`
- `lognormal` é o padrão porque o intervalo humano entre teclas é assimétrico à direita, e um sorteio simétrico reproduz a largura da distribuição humana sem a assimetria dela
- Ela governa apenas o ritmo rápido
- A cauda de pausas longas é `input_word_pause_permille`
- Toda média recebe um piso de 5% de dispersão e é truncada entre um quarto e quatro vezes ela mesma, então zerar um desvio padrão não compra a variância zero que um detector lê como máquina
- `input_move_steps_stddev` — desvio padrão do orçamento de amostras de ponteiro por gesto, para que dois movimentos sobre a mesma distância não carreguem o mesmo número de posições intermediárias, padrão `6`
- Um drag reescala esse valor para o próprio orçamento menor em vez de herdar o número absoluto
- `input_move_gap_stddev_ms` — desvio padrão do atraso entre as posições sintetizadas do ponteiro, em milissegundos, padrão `5`
- `input_click_dwell_stddev_ms` — desvio padrão da retenção entre pressionar e soltar o botão, em milissegundos, padrão `26`
- `input_key_dwell_stddev_ms` — desvio padrão da retenção entre `keyDown` e `keyUp`, em milissegundos, padrão `18`
- `input_type_delay_stddev_ms` — desvio padrão do atraso entre caracteres, em milissegundos, padrão `40`
- Um chamador que pede o próprio ritmo de digitação recebe essa dispersão reescalada na mesma proporção, então metade da média vira metade da largura, em vez de um valor absoluto que já não cabe
- `input_scroll_tick_stddev_px` — desvio padrão da distância que um tique de roda sintetizado carrega, em pixels CSS, padrão `25`
- `input_word_pause_ms` — média da pausa extra tomada em um limite de palavra ou de frase, em milissegundos, ela mesma dispersa por metade do próprio valor, padrão `320`
- Essa pausa é o que produz a cauda longa à direita de um traço de digitação, que nenhum jitter em torno da média por caractere cria
- `input_word_pause_permille` — chance em mil de um limite de palavra ou de frase ganhar essa pausa longa, padrão `120`
- `0` remove a cauda e deixa só o ritmo rápido
- `input_typo_permille` — chance em mil de um caractere ser digitado errado, apagado com `Backspace` e redigitado, padrão `0`
- O campo termina sempre com exatamente o texto pedido
- `0` por padrão, e é a única chave de humanização que é: todas as outras dispersam TEMPO, que a página não lê como valor diferente, enquanto esta muda o FLUXO DE CARACTERES, então um ouvinte de `input` vê o prefixo errado e pode autocompletar ou navegar com ele
- A tecla errada é sempre uma vizinha física na fileira QWERTY
- `support_settle_ms` — estabilização da thread de suporte para os auxiliares síncronos, em milissegundos, padrão `80`
- `nav_micro_settle_ms` — microestabilização de navegação após transições de página, em milissegundos, padrão `100`


## Screencast e Perf
- `screencast_jpeg_quality` — qualidade JPEG do screencast via CDP, de 1 até 100, padrão `60`
- `screencast_ffmpeg_framerate` — taxa de quadros de entrada do `ffmpeg` no screencast, em quadros por segundo, padrão `10`
- `screencast_start_pump_iters` — iterações imediatas de bombeamento logo após `Page.startScreencast`, padrão `15`
- `screencast_stop_pump_iters` — iterações de esvaziamento antes de `Page.stopScreencast`, padrão `40`
- `perf_autostop_settle_ms` — estabilização da parada automática de perf após carga ou recarga, em milissegundos, padrão `500`
- `perf_trace_inner_slice_ms` — fatia interna de sondagem do trace de perf em milissegundos, padrão `20`
- `perf_trace_outer_slice_ms` — intervalo externo de sondagem do trace de perf em milissegundos, padrão `50`
- `perf_trace_outer_iters` — número máximo de iterações da sondagem externa do trace de perf, padrão `100`
- `perf_trace_inner_iters` — iterações internas de esvaziamento do trace de perf após a conclusão, padrão `5`


## Heap
- `heap_snapshot_max_bytes` — teto de tamanho do arquivo de snapshot de heap analisado offline, em bytes, padrão `536870912`
- `heap_max_retainers` — número máximo de retentores devolvidos por operação de nó do heap, padrão `200`
- `heap_max_edges` — número máximo de arestas devolvidas por operação de nó do heap, padrão `200`
- `heap_max_paths` — número máximo de caminhos na enumeração de caminhos do heap, padrão `32`
- `heap_max_path_depth` — profundidade máxima dos caminhos do heap, padrão `8`
- `heap_max_class_nodes` — teto da lista `class_nodes` do heap, padrão `500`
- `heap_dominator_max_states` — teto de estados visitados no cálculo de dominadores, como proteção contra grafos patológicos, padrão `50000`
- `heap_outer_iters` — número máximo de iterações da sondagem externa do snapshot de heap, padrão `200`
- `heap_inner_iters` — iterações internas de esvaziamento do snapshot de heap após a conclusão, padrão `10`
- `heap_final_iters` — iterações finais de esvaziamento do snapshot de heap, padrão `20`


## MITM
- `monitor_diff_max_bytes` — teto em bytes do payload de `monitor check --diff-mode`, padrão `65536`
- Uma página reescrita por inteiro produz um diff com a página duas vezes, e quem chamou perguntou o que mudou, não pediu tudo
- `diff_truncated` avisa quando o teto valeu, e `added_count` / `removed_count` seguem reportando o tamanho real
- `mitm_list_limit_max` — teto de itens nas operações de listagem e consulta do MITM, padrão `10000`
- `mitm_proxy_seconds_max` — janela máxima do proxy MITM em execução única, em segundos, padrão `600`
- `mitm_chrome_settle_ms` — estabilização após a inicialização do Chrome no MITM, antes de navegar, em milissegundos, padrão `150`
- `mitm_capture_wait_min_ms` — piso da espera de captura MITM após navegar, em milissegundos, padrão `800`
- `mitm_capture_wait_max_ms` — teto da espera de captura MITM após navegar, em milissegundos, padrão `8000`
- `mitm_ws_frames_cap` — teto de quadros WebSocket mantidos em memória por processo de captura, padrão `500`
- `mitm_ws_preview_chars` — truncamento da prévia de texto WebSocket, contado em caracteres Unicode, padrão `256`
- `mitm_ca_cache_size` — tamanho do cache de certificados dinâmicos do MITM, contado em hosts, padrão `1000`
- `mitm_rebind_attempts` — número de novas tentativas de bind do proxy MITM quando a porta está temporariamente em uso, padrão `3`


## Arquivos Locais e Raízes
- `allowed_roots` — raízes adicionais permitidas para leituras locais e escrita de artefatos, separadas pelo separador da plataforma, sendo que os padrões já cobrem o diretório atual, os diretórios XDG e o temporário, padrão nenhum
- `max_json_file_bytes` — número máximo de bytes de arquivos JSON ou NDJSON usados como script ou manifesto, padrão `33554432`
- `max_ndjson_line_bytes` — número máximo de bytes de uma única linha NDJSON em scripts de `run` e em traces, padrão `1048576`
- `max_cli_json_payload_bytes` — número máximo de bytes das cargas JSON passadas por flag da CLI, padrão `4194304`
- `max_sg_file_bytes` — número máximo de bytes de um arquivo-fonte lido por `sg-scan` e `sg-rewrite`, padrão `16777216`
- `run_max_include_depth` — profundidade máxima de aninhamento nas cadeias de inclusão de `run --script`, padrão `16`


## Orçamentos de Retentativa
- `retry_default_max_attempts` — número máximo padrão de tentativas, incluindo a primeira, padrão `3`
- `retry_base_delay_ms` — atraso base padrão de retentativa em milissegundos, padrão `50`
- `retry_max_delay_secs` — atraso máximo padrão de retentativa em segundos, padrão `2`
- `retry_budget_secs` — orçamento padrão de tempo real das retentativas em segundos, padrão `10`
- `retry_cdp_max_attempts` — número máximo de tentativas nas retentativas do CDP, padrão `4`
- `retry_cdp_base_delay_ms` — atraso base das retentativas do CDP em milissegundos, padrão `100`
- `retry_cdp_max_delay_secs` — atraso máximo das retentativas do CDP em segundos, padrão `3`
- `retry_cdp_budget_secs` — orçamento de tempo real das retentativas do CDP em segundos, padrão `15`
- `retry_http_max_attempts` — número máximo de tentativas nas retentativas de scrape HTTP, padrão `3`
- `retry_http_base_delay_ms` — atraso base das retentativas de scrape HTTP em milissegundos, padrão `75`
- `retry_http_max_delay_secs` — atraso máximo das retentativas de scrape HTTP em segundos, padrão `2`
- `retry_http_budget_secs` — orçamento de tempo real das retentativas de scrape HTTP em segundos, padrão `12`
- `retry_llm_max_attempts` — número máximo de tentativas nas retentativas HTTP de LLM, padrão `2`
- `retry_llm_base_delay_ms` — atraso base das retentativas HTTP de LLM em milissegundos, padrão `200`
- `retry_llm_max_delay_secs` — atraso máximo das retentativas HTTP de LLM em segundos, padrão `4`
- `retry_llm_budget_secs` — orçamento de tempo real das retentativas HTTP de LLM em segundos, padrão `20`


## Viewport e Estado
- `default_viewport_width` — largura padrão da janela do Chrome headless (`--window-size`) quando as opções de inicialização omitem o viewport, padrão `1920`
- Distinto de `screen`: é a janela do processo e o fallback de screenshot/screencast, não `screen.width`
- `default_viewport_height` — altura padrão da janela do Chrome headless (`--window-size`) quando as opções de inicialização omitem o viewport, padrão `1080`
- `screen` — tela da página `WxH` para `Emulation.setDeviceMetricsOverride` (`screen.width`/`screen.height`), padrão nenhum
- Ausente = espelha o viewport
- Argv `--screen` e o campo `screen` de emulate/resize no `run` ainda vencem
- Nunca menor que o viewport
- `state_collect_deadline_secs` — prazo externo da coleta de storage via CDP em segundos, padrão `5`
- `state_event_recv_secs` — fatia de recebimento de eventos de storage do CDP em segundos, padrão `2`
- `state_load_settle_ms` — atraso de estabilização após a navegação de `load_state`, em milissegundos, padrão `500`


## Descobrindo Chaves em Tempo de Execução
- Liste toda chave aceita pelo binário em execução com o envelope JSON
- `browser-automation-cli --json config list-keys`
- Cada entrada devolve o nome da chave, o padrão embutido e a descrição
- Leia o valor guardado de uma chave com `browser-automation-cli --json config get <key>`
- Inspecione todos os valores guardados com `browser-automation-cli --json config show`
- Confirme o arquivo vencedor com `browser-automation-cli --json config path`
- Trate a saída viva de `config list-keys` como fonte de verdade quando este documento e o binário divergirem
- Prefira a descoberta em tempo de execução a qualquer lista memorizada


## Veja Também
- [README](../README.pt-BR.md) para a visão geral do produto
- [Como Usar](HOW_TO_USE.pt-BR.md) para o guia prático de uso
- [Agentes](AGENTS.pt-BR.md) para o contrato de integração com agentes
