[English](CONFIGURATION.md)


# Referência de Configuração
> Referência canônica das 176 chaves XDG do `browser-automation-cli`.


## Como a Configuração é Resolvida
- O produto NÃO lê variáveis de ambiente de produto em nenhuma circunstância
- Toda configuração durável vive no arquivo XDG `config.toml`
- A precedência é: flag de CLI vence chave XDG, que vence o padrão embutido
- Uma chave ausente no `config.toml` cai no padrão embutido documentado abaixo
- Uma chave sem padrão embutido permanece desativada até você defini-la
- Segredos como `openrouter_api_key` e `encryption_key` são gravados com permissão `0600`
- Segredos nunca aparecem em log, em stdout ou em envelope JSON
- Descubra o caminho vencedor do arquivo com `config path`


## Comandos de Configuração
- `config init` cria o `config.toml` no diretório XDG do produto
- Execute `browser-automation-cli --json config init`
- `config path` imprime o caminho resolvido do arquivo de configuração
- Execute `browser-automation-cli --json config path`
- `config show` imprime a configuração efetiva já resolvida
- Execute `browser-automation-cli --json config show`
- `config get <key>` lê o valor efetivo de uma única chave
- Execute `browser-automation-cli --json config get timeout`
- `config set <key> <value>` grava um valor durável no `config.toml`
- Execute `browser-automation-cli --json config set dialog_settle_ms 2000`
- `config list-keys` lista todas as chaves aceitas com padrão e descrição
- Execute `browser-automation-cli --json config list-keys`


## Núcleo e Idioma
- `lang` — sobrescreve o idioma das mensagens humanas, aceitando `en` ou `pt-BR`, com `pt` puro rejeitado. Padrão: nenhum.
- `timeout` — timeout global de execução em segundos. Padrão: `0`.
- `artifacts_dir` — diretório de saída dos artefatos gerados. Padrão: nenhum.
- `ignore_robots` — ignora `robots.txt` por padrão, sendo que as flags de risco continuam obrigatórias. Padrão: nenhum.
- `namespace` — namespace isolado de estado do produto. Padrão: nenhum.
- `encryption_key` — material de chave para cifrar o estado de sessão. Padrão: nenhum.
- `color` — habilita cores ANSI na saída humana enviada ao stderr. Padrão: nenhum.


## Registro de Log
- `log_level` — filtro de tracing aplicado quando as flags de argv estão silenciosas, sem qualquer leitura de `RUST_LOG`. Padrão: `error`.
- `log_to_file` — grava logs JSON locais rotacionados sob o diretório de estado XDG, nunca remotos. Padrão: nenhum.
- `max_log_files` — número de arquivos de log rotacionados retidos, na faixa de 1 até 90. Padrão: `14`.
- `log_rotation` — política de rotação, aceitando `daily`, `hourly` ou `never`. Padrão: `daily`.


## Binários Externos
- `chrome_path` — caminho absoluto do binário Chrome ou Chromium. Padrão: nenhum.
- `lighthouse_path` — caminho absoluto da CLI `lighthouse`. Padrão: nenhum.
- `ffmpeg_path` — caminho absoluto do `ffmpeg`, opcional para codificar screencast e converter vídeo ou extrair MP3. Padrão: nenhum.
- `lighthouse_timeout_secs` — teto de tempo real da CLI `lighthouse` em segundos, na faixa de 1 até 3600. Padrão: `300`.
- `ffmpeg_timeout_secs` — teto de tempo real da codificação `ffmpeg` em segundos, na faixa de 1 até 3600. Padrão: `120`.


## LLM e Webhooks
- `openrouter_api_key` — chave de API do provedor de LLM, armazenada com permissão `0600`. Padrão: nenhum.
- `llm_base_url` — URL base compatível com a API OpenAI. Padrão: nenhum.
- `llm_model` — identificador do modelo de LLM usado por padrão. Padrão: nenhum.
- `llm_http_timeout_secs` — timeout HTTP bloqueante para chamadas de LLM e webhook em segundos. Padrão: `60`.
- `webhook_post_timeout_secs` — timeout do POST de webhook operacional em segundos. Padrão: `15`.
- `webhook_retry_base_delay_ms` — atraso base de retentativa de webhook em milissegundos, dobrando a cada tentativa. Padrão: `50`.
- `webhook_max_attempts` — número máximo de tentativas de webhook, incluindo a primeira. Padrão: `3`.


## Cache e Redis
- `cache_backend` — backend de cache, aceitando `sqlite`, `memory` ou `redis`. Padrão: `sqlite`.
- `cache_redis_url` — URL do Redis quando o backend é `redis`. Padrão: nenhum.
- `redis_allow_remote` — permite hosts Redis fora do loopback, desligado por padrão. Padrão: nenhum.
- `redis_connect_timeout_secs` — timeout de conexão TCP com o Redis em segundos. Padrão: `2`.
- `redis_io_timeout_secs` — timeout de entrada e saída do stream RESP do Redis em segundos. Padrão: `3`.
- `cache_max_resp_bulk_bytes` — teto de tamanho da bulk string RESP do Redis em bytes. Padrão: `16777216`.
- `cache_max_resp_line_bytes` — teto de tamanho da linha RESP do Redis em bytes. Padrão: `16777216`.
- `scrape_http_cache_ttl_secs` — período de validade do cache L2 de respostas HTTP de scrape em segundos. Padrão: `3600`.
- `file_parse_cache_ttl_secs` — período de validade do cache L2 de parse de arquivo local em segundos. Padrão: `86400`.


## HTTP e Segurança de Rede
- `http_ssrf_mode` — política HTTP contra SSRF, aceitando `strict`, `allow_loopback` ou `off`. Padrão: `strict`.
- `http_timeout_secs` — timeout total do cliente HTTP compartilhado em segundos. Padrão: `30`.
- `http_connect_timeout_secs` — timeout da fase de conexão HTTP em segundos. Padrão: `10`.
- `http_redirect_max` — número máximo de redirecionamentos HTTP seguidos pelos clientes do produto. Padrão: `10`.
- `http_pool_max_idle_per_host` — número máximo de conexões ociosas do pool `reqwest` por host. Padrão: `4`.
- `search_base_url` — URL base do endpoint HTML de busca, ao qual `?q=` é anexado. Padrão: `https://html.duckduckgo.com/html/`.


## Robots e Polidez
- `robots_loopback_exempt` — hosts de loopback pulam o `robots.txt`, e o valor `false` passa a exigir conformidade contra `localhost`. Padrão: `true`.
- `robots_probe_timeout_secs` — timeout da sondagem HEAD do `robots.txt` em segundos. Padrão: `5`.
- `robots_max_body_bytes` — limite de bytes do corpo do `robots.txt`, como proteção contra estouro de memória. Padrão: `524288`.
- `robots_fetch_timeout_secs` — timeout do download completo do `robots.txt` em segundos. Padrão: `30`.
- `scrape_min_delay_ms` — atraso mínimo entre requisições GET de mesma origem em milissegundos. Padrão: `0`.
- `scrape_delay_jitter_ratio` — razão de variação aleatória do atraso de polidez, de 0.0 até 1.0, com `0` desligando. Padrão: `0.2`.
- `scrape_honor_meta_robots` — respeita as diretivas `meta robots` e `X-Robots-Tag` do tipo `noindex`. Padrão: `true`.
- `scrape_honor_nofollow` — pula links com `rel=nofollow` durante a descoberta do crawl. Padrão: `true`.


## Scrape e Crawl
- `scrape_max_body_bytes` — número máximo de bytes do corpo em scrape HTTP. Padrão: `5000000`.
- `browser_scrape_max_body_bytes` — número máximo de bytes do corpo nos auxiliares de scrape do motor de navegador. Padrão: `2000000`.
- `scrape_max_text_chars` — número máximo de caracteres de texto ou markdown nos envelopes de scrape, com `0` removendo o teto. Padrão: `32768`.
- `scrape_use_sitemap` — prefere o `sitemap.xml` ao mapear um site. Padrão: `true`.
- `scrape_default_engine` — motor de scrape usado quando a CLI omite `--engine`, aceitando `http` ou `browser`. Padrão: `http`.
- `scrape_summary_chars` — número máximo de caracteres do formato `summary` de scrape. Padrão: `400`.
- `scrape_feed_max_entries` — número máximo de entradas mantidas pelo formato `feed` de scrape, cobrindo RSS, Atom e JSON Feed. Padrão: `50`.
- `scrape_follow_rel_next` — segue links de paginação com `rel=next` durante o crawl. Padrão: nenhum.
- `scrape_dedup_similar` — colapsa páginas quase duplicadas por similaridade de conteúdo em crawl e batch-scrape. Padrão: nenhum.
- `scrape_dedup_similar_distance` — distância de Hamming do SimHash, de 0 até 64, abaixo da qual as páginas são quase duplicadas. Padrão: `3`.
- `scrape_sitemap_max_bytes` — número máximo de bytes do corpo do sitemap. Padrão: `524288`.
- `scrape_charset_peek_bytes` — janela de inspeção usada para detectar o charset, em bytes. Padrão: `4096`.
- `scrape_crawl_limit_max` — orçamento máximo de páginas do crawl, atuando como teto contra abuso para `--limit`. Padrão: `500`.
- `scrape_crawl_max_depth` — profundidade máxima da busca em largura para crawl e map. Padrão: `10`.
- `scrape_search_limit_max` — orçamento máximo de resultados de busca, atuando como teto contra abuso. Padrão: `50`.
- `scrape_max_parse_bytes` — tamanho máximo de arquivo local aceito para parse antes da rejeição, em bytes. Padrão: `50000000`.
- `max_urls_file_bytes` — número máximo de bytes da lista informada em `batch-scrape --urls-file`. Padrão: `8388608`.


## Imagem
- `image_max_input_bytes` — número máximo de bytes de entrada para decodificar, converter ou redimensionar imagem local. Padrão: `32000000`.
- `image_max_pixels` — produto máximo de largura por altura na decodificação de imagem, como proteção contra bomba de descompressão. Padrão: `64000000`.
- `image_default_format` — formato padrão da conversão de imagem, aceitando `png`, `jpeg`, `webp` ou `gif`. Padrão: `png`.
- `image_default_quality` — qualidade padrão com perda, de 1 até 100, para conversão e redimensionamento de imagem. Padrão: `85`.
- `image_download_max_bytes` — número máximo de bytes do corpo HTTP no download de imagem. Padrão: `32000000`.
- `image_avif_speed` — velocidade do codificador AVIF, de 1 até 10, sendo 1 a mais lenta e de melhor resultado, exigindo a feature `image-avif`. Padrão: `6`.
- `default_jpeg_quality` — qualidade JPEG de 1 até 100 quando `grab` omite `--quality`. Padrão: `80`.


## Vídeo e Áudio
- `video_max_input_bytes` — número máximo de bytes na materialização de vídeo vindo do stdin ou na verificação prévia do caminho. Padrão: `512000000`.
- `video_download_max_bytes` — número máximo de bytes do corpo HTTP no download de vídeo. Padrão: `512000000`.
- `video_default_container` — contêiner padrão da conversão de vídeo, aceitando `mp4`, `webm`, `mkv`, `mov`, `avi` ou `m4v`. Padrão: `mp4`.
- `video_default_crf` — valor CRF padrão, de 1 até 51, para recodificação de vídeo com perda. Padrão: `23`.
- `video_default_audio_bitrate` — taxa de bits padrão na conversão de vídeo para MP3, por exemplo `192k`. Padrão: `192k`.
- `audio_max_input_bytes` — número máximo de bytes na materialização de áudio vindo do stdin ou na verificação prévia do caminho. Padrão: `256000000`.
- `audio_download_max_bytes` — número máximo de bytes do corpo HTTP no download de áudio. Padrão: `256000000`.
- `audio_default_format` — formato padrão da conversão de áudio, aceitando `mp3`, `m4a`, `ogg`, `opus`, `flac`, `wav` ou `aac`. Padrão: `mp3`.
- `audio_default_bitrate` — taxa de bits padrão na codificação de áudio com perda, por exemplo `192k`. Padrão: `192k`.


## SVG, GIF e Manifestos
- `svg_max_bytes` — número máximo de bytes do código-fonte SVG aceito antes da rasterização. Padrão: `4000000`.
- `svg_max_depth` — profundidade máxima de aninhamento XML aceita em um código-fonte SVG. Padrão: `128`.
- `svg_max_entities` — número máximo de declarações `<!ENTITY>` toleradas na DTD do SVG, com `0` rejeitando qualquer uma. Padrão: `0`.
- `gif_max_frames` — número máximo de quadros de animação decodificados de um GIF. Padrão: `2000`.
- `manifest_max_bytes` — número máximo de bytes aceitos no corpo de um manifesto HLS ou DASH. Padrão: `8000000`.
- `manifest_max_variants` — número máximo de entradas de variante ou representação emitidas por envelope de manifesto. Padrão: `500`.


## Motor Chrome e Ciclo de Vida
- `chrome_search_paths` — caminhos ordenados de descoberta do Chrome ou Chromium, separados pelo separador da plataforma, com o valor vazio usando o layout embutido de cada sistema. Padrão: nenhum.
- `chrome_legacy_oxide_launch` — inicia o Chrome via `chromiumoxide` em vez do caminho de auto-spawn, servindo como recuo de estabilização e perdendo o alvo de encerramento residual. Padrão: nenhum.
- `chrome_startup_timeout_secs` — espera pela prontidão do CDP no auto-spawn do Chrome em segundos. Padrão: `20`.
- `chrome_default_timeout_ms` — timeout padrão por operação do motor Chrome em milissegundos. Padrão: `25000`.
- `browser_close_wait_secs` — orçamento de espera por `Browser.close` e pelo término do processo durante a fase FINALIZE, em segundos. Padrão: `5`.
- `residual_orphan_min_age_secs` — idade mínima antes que um perfil marcador de dono morto se torne coletável, em segundos. Padrão: `60`.
- `platform_child_wait_secs` — prazo de espera pelo processo filho da plataforma em segundos. Padrão: `5`.
- `shutdown_poll_ms` — intervalo de sondagem cooperativa durante o desligamento em milissegundos. Padrão: `5`.
- `shutdown_deadline_secs` — prazo rígido de desligamento aguardando a saída do navegador, em segundos. Padrão: `30`.


## CDP e Eventos
- `cdp_connection_probe_timeout_secs` — timeout da sondagem de vitalidade `Browser.getVersion` do CDP em segundos. Padrão: `3`.
- `cdp_discovery_max_body_bytes` — número máximo de bytes do corpo HTTP de descoberta CDP em `/json/version` e `/json/list`. Padrão: `1048576`.
- `cdp_event_broadcast_capacity` — capacidade do canal local de difusão de eventos CDP dentro do processo. Padrão: `4096`.
- `cdp_event_drain_poll_ms` — fatia de sondagem no esvaziamento de eventos CDP durante a espera por navegação, em milissegundos. Padrão: `100`.
- `cdp_network_idle_settle_ms` — janela de estabilização de rede ociosa do CDP em milissegundos. Padrão: `500`.
- `cdp_target_event_wait_ms` — espera curta por evento de target do CDP em milissegundos. Padrão: `600`.
- `cdp_discovery_timeout_secs` — timeout da descoberta HTTP do CDP nas sondagens de `/json/version`, em segundos. Padrão: `2`.
- `event_tracker_max_entries` — tamanho do anel em memória do rastreador de console e rede por sessão de página. Padrão: `1000`.
- `event_pump_slice_ms` — fatia da bomba de eventos usada em `wait` e `eval`, em milissegundos. Padrão: `50`.
- `eval_drain_slice_ms` — fatia de esvaziamento durante a espera pelos resultados de `Runtime.evaluate`, em milissegundos. Padrão: `40`.


## Motor Lightpanda
- `lightpanda_startup_timeout_secs` — espera pela inicialização do processo Lightpanda em segundos. Padrão: `10`.
- `lightpanda_session_timeout_secs` — duração máxima da sessão informada em `--timeout` do Lightpanda, em segundos, na faixa de 1 até 604800. Padrão: `604800`.
- `lightpanda_poll_interval_ms` — intervalo de sondagem da prontidão do CDP do Lightpanda em milissegundos. Padrão: `100`.
- `lightpanda_discovery_timeout_ms` — timeout por sondagem de descoberta CDP durante a espera pelo Lightpanda, em milissegundos. Padrão: `500`.
- `lightpanda_max_log_lines` — anel limitado de log da inicialização do Lightpanda, contado em linhas por fluxo. Padrão: `40`.
- `lightpanda_ready_slice_ms` — fatia de esvaziamento após a saída do processo filho Lightpanda, antes de capturar os logs, em milissegundos. Padrão: `25`.
- `lightpanda_cdp_connect_timeout_secs` — timeout da tentativa de conexão CDP com o Lightpanda em segundos. Padrão: `5`.
- `lightpanda_target_init_timeout_secs` — espera pela inicialização do target do Lightpanda após a conexão, em segundos. Padrão: `10`.


## Interação e Esperas
- `interact_settle_ms` — atraso de estabilização da interface após clique, digitação ou ação de extensão, em milissegundos. Padrão: `200`.
- `dialog_settle_ms` — espera máxima após responder um diálogo JavaScript até o evento `javascriptDialogClosed`, em milissegundos. Padrão: `2000`.
- `network_idle_window_ms` — janela de silêncio usada por `wait --network-idle` em milissegundos. Padrão: `500`.
- `dom_stable_window_ms` — janela de silêncio usada por `wait --dom-stable-ms` em milissegundos. Padrão: `500`.
- `drag_move_steps` — número de posições intermediárias do mouse sintetizadas em um arrasto HTML5. Padrão: `6`.
- `drag_move_gap_ms` — atraso entre as posições sintetizadas de arrasto em milissegundos. Padrão: `16`.
- `support_settle_ms` — estabilização da thread de suporte para os auxiliares síncronos, em milissegundos. Padrão: `80`.
- `nav_micro_settle_ms` — microestabilização de navegação após transições de página, em milissegundos. Padrão: `100`.
- `extension_attach_poll_ms` — fatia de sondagem ao anexar uma extensão, em milissegundos. Padrão: `150`.


## Screencast e Perf
- `screencast_jpeg_quality` — qualidade JPEG do screencast via CDP, de 1 até 100. Padrão: `60`.
- `screencast_ffmpeg_framerate` — taxa de quadros de entrada do `ffmpeg` no screencast, em quadros por segundo. Padrão: `10`.
- `screencast_start_pump_iters` — iterações imediatas de bombeamento logo após `Page.startScreencast`. Padrão: `15`.
- `screencast_stop_pump_iters` — iterações de esvaziamento antes de `Page.stopScreencast`. Padrão: `40`.
- `perf_autostop_settle_ms` — estabilização da parada automática de perf após carga ou recarga, em milissegundos. Padrão: `500`.
- `perf_trace_inner_slice_ms` — fatia interna de sondagem do trace de perf em milissegundos. Padrão: `20`.
- `perf_trace_outer_slice_ms` — intervalo externo de sondagem do trace de perf em milissegundos. Padrão: `50`.
- `perf_trace_outer_iters` — número máximo de iterações da sondagem externa do trace de perf. Padrão: `100`.
- `perf_trace_inner_iters` — iterações internas de esvaziamento do trace de perf após a conclusão. Padrão: `5`.


## Heap
- `heap_snapshot_max_bytes` — teto de tamanho do arquivo de snapshot de heap analisado offline, em bytes. Padrão: `536870912`.
- `heap_max_retainers` — número máximo de retentores devolvidos por operação de nó do heap. Padrão: `200`.
- `heap_max_edges` — número máximo de arestas devolvidas por operação de nó do heap. Padrão: `200`.
- `heap_max_paths` — número máximo de caminhos na enumeração de caminhos do heap. Padrão: `32`.
- `heap_max_path_depth` — profundidade máxima dos caminhos do heap. Padrão: `8`.
- `heap_max_class_nodes` — teto da lista `class_nodes` do heap. Padrão: `500`.
- `heap_dominator_max_states` — teto de estados visitados no cálculo de dominadores, como proteção contra grafos patológicos. Padrão: `50000`.
- `heap_outer_iters` — número máximo de iterações da sondagem externa do snapshot de heap. Padrão: `200`.
- `heap_inner_iters` — iterações internas de esvaziamento do snapshot de heap após a conclusão. Padrão: `10`.
- `heap_final_iters` — iterações finais de esvaziamento do snapshot de heap. Padrão: `20`.


## MITM
- `mitm_list_limit_max` — teto de itens nas operações de listagem e consulta do MITM. Padrão: `10000`.
- `mitm_proxy_seconds_max` — janela máxima do proxy MITM em execução única, em segundos. Padrão: `600`.
- `mitm_chrome_settle_ms` — estabilização após a inicialização do Chrome no MITM, antes de navegar, em milissegundos. Padrão: `150`.
- `mitm_capture_wait_min_ms` — piso da espera de captura MITM após navegar, em milissegundos. Padrão: `800`.
- `mitm_capture_wait_max_ms` — teto da espera de captura MITM após navegar, em milissegundos. Padrão: `8000`.
- `mitm_ws_frames_cap` — teto de quadros WebSocket mantidos em memória por processo de captura. Padrão: `500`.
- `mitm_ws_preview_chars` — truncamento da prévia de texto WebSocket, contado em caracteres Unicode. Padrão: `256`.
- `mitm_ca_cache_size` — tamanho do cache de certificados dinâmicos do MITM, contado em hosts. Padrão: `1000`.
- `mitm_rebind_attempts` — número de novas tentativas de bind do proxy MITM quando a porta está temporariamente em uso. Padrão: `3`.


## Arquivos Locais e Raízes
- `allowed_roots` — raízes adicionais permitidas para leituras locais e escrita de artefatos, separadas pelo separador da plataforma, sendo que os padrões já cobrem o diretório atual, os diretórios XDG e o temporário. Padrão: nenhum.
- `max_json_file_bytes` — número máximo de bytes de arquivos JSON ou NDJSON usados como script ou manifesto. Padrão: `33554432`.
- `max_ndjson_line_bytes` — número máximo de bytes de uma única linha NDJSON em scripts de `run` e em traces. Padrão: `1048576`.
- `max_cli_json_payload_bytes` — número máximo de bytes das cargas JSON passadas por flag da CLI. Padrão: `4194304`.
- `max_sg_file_bytes` — número máximo de bytes de um arquivo-fonte lido por `sg-scan` e `sg-rewrite`. Padrão: `16777216`.
- `run_max_include_depth` — profundidade máxima de aninhamento nas cadeias de inclusão de `run --script`. Padrão: `16`.


## Orçamentos de Retentativa
- `retry_default_max_attempts` — número máximo padrão de tentativas, incluindo a primeira. Padrão: `3`.
- `retry_base_delay_ms` — atraso base padrão de retentativa em milissegundos. Padrão: `50`.
- `retry_max_delay_secs` — atraso máximo padrão de retentativa em segundos. Padrão: `2`.
- `retry_budget_secs` — orçamento padrão de tempo real das retentativas em segundos. Padrão: `10`.
- `retry_cdp_max_attempts` — número máximo de tentativas nas retentativas do CDP. Padrão: `4`.
- `retry_cdp_base_delay_ms` — atraso base das retentativas do CDP em milissegundos. Padrão: `100`.
- `retry_cdp_max_delay_secs` — atraso máximo das retentativas do CDP em segundos. Padrão: `3`.
- `retry_cdp_budget_secs` — orçamento de tempo real das retentativas do CDP em segundos. Padrão: `15`.
- `retry_http_max_attempts` — número máximo de tentativas nas retentativas de scrape HTTP. Padrão: `3`.
- `retry_http_base_delay_ms` — atraso base das retentativas de scrape HTTP em milissegundos. Padrão: `75`.
- `retry_http_max_delay_secs` — atraso máximo das retentativas de scrape HTTP em segundos. Padrão: `2`.
- `retry_http_budget_secs` — orçamento de tempo real das retentativas de scrape HTTP em segundos. Padrão: `12`.
- `retry_llm_max_attempts` — número máximo de tentativas nas retentativas HTTP de LLM. Padrão: `2`.
- `retry_llm_base_delay_ms` — atraso base das retentativas HTTP de LLM em milissegundos. Padrão: `200`.
- `retry_llm_max_delay_secs` — atraso máximo das retentativas HTTP de LLM em segundos. Padrão: `4`.
- `retry_llm_budget_secs` — orçamento de tempo real das retentativas HTTP de LLM em segundos. Padrão: `20`.


## Viewport e Estado
- `default_viewport_width` — largura padrão da janela do Chrome headless quando as opções de inicialização omitem o viewport. Padrão: `1280`.
- `default_viewport_height` — altura padrão da janela do Chrome headless quando as opções de inicialização omitem o viewport. Padrão: `720`.
- `state_collect_deadline_secs` — prazo externo da coleta de storage via CDP em segundos. Padrão: `5`.
- `state_event_recv_secs` — fatia de recebimento de eventos de storage do CDP em segundos. Padrão: `2`.
- `state_load_settle_ms` — atraso de estabilização após a navegação de `load_state`, em milissegundos. Padrão: `500`.


## Descobrindo Chaves em Tempo de Execução
- Liste toda a superfície de chaves com `browser-automation-cli --json config list-keys`
- Cada entrada devolve o nome da chave, o padrão embutido e a descrição
- Inspecione a configuração já resolvida com `browser-automation-cli --json config show`
- Leia uma chave isolada com `browser-automation-cli --json config get <key>`
- Confirme o arquivo vencedor com `browser-automation-cli --json config path`
- Trate a saída de `config list-keys` como fonte de verdade viva do produto
- Prefira a descoberta em tempo de execução a qualquer lista memorizada


## Veja Também
- [README](../README.pt-BR.md) para a visão geral do produto
- [Como Usar](HOW_TO_USE.pt-BR.md) para o guia prático de uso
- [Agentes](AGENTS.pt-BR.md) para o contrato de integração com agentes
