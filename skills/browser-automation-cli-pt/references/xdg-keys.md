# Chaves de Configuração XDG


## Contrato de Configuração
- DEVE tratar este arquivo como o inventário completo das 176 chaves XDG do produto
- O produto NUNCA lê variável de ambiente de produto; NUNCA ensine `export` como configuração
- DEVE configurar SOMENTE por flags CLI e por `config init|path|show|get|set|list-keys`
- DEVE respeitar a precedência flag CLI, depois valor XDG, depois padrão embutido
- DEVE descobrir a superfície viva legível por máquina com `browser-automation-cli --json config list-keys`
- DEVE resolver o caminho real do arquivo com `browser-automation-cli --json config path`
- DEVE ler `Padrão: nenhum` como ausência de padrão embutido, não como valor vazio
- NUNCA invente chave fora desta lista; NUNCA recuse chave desta lista por memória


## Núcleo e Identidade
- `lang` — override de idioma das mensagens (`en|pt-BR`; `pt` puro é rejeitado). Padrão: nenhum.
- `timeout` — timeout global em segundos. Padrão: `0`.
- `artifacts_dir` — diretório de saída de artefatos. Padrão: nenhum.
- `ignore_robots` — ignorar robots por padrão (as flags continuam obrigatórias). Padrão: nenhum.
- `namespace` — namespace isolado de estado. Padrão: nenhum.
- `encryption_key` — material da chave de criptografia de sessão. Padrão: nenhum.
- `color` — cores ANSI no stderr humano. Padrão: nenhum.


## Logging Local
- `log_level` — `EnvFilter` de tracing quando as flags de argv silenciam (sem `RUST_LOG`). Padrão: `error`.
- `log_to_file` — logs JSON locais rotacionados sob XDG state (nunca remotos). Padrão: nenhum.
- `max_log_files` — número de arquivos de log rotacionados retidos (1..=90). Padrão: `14`.
- `log_rotation` — política de rotação: `daily|hourly|never`. Padrão: `daily`.


## Binários Externos
- `chrome_path` — caminho absoluto do Chrome ou Chromium. Padrão: nenhum.
- `lighthouse_path` — caminho absoluto da CLI lighthouse. Padrão: nenhum.
- `ffmpeg_path` — caminho absoluto do ffmpeg (opcional para encode de screencast e conversão de vídeo ou to-mp3). Padrão: nenhum.
- `lighthouse_timeout_secs` — timeout de relógio da CLI lighthouse (segundos, 1..=3600). Padrão: `300`.
- `ffmpeg_timeout_secs` — timeout de relógio do encode ffmpeg (segundos, 1..=3600). Padrão: `120`.
- `chrome_search_paths` — caminhos ordenados de descoberta do Chrome ou Chromium (separados por plataforma); vazio usa o layout embutido por sistema. Padrão: nenhum.


## LLM
- `openrouter_api_key` — chave de API do LLM (armazenada com modo 0600). Padrão: nenhum.
- `llm_base_url` — URL base compatível com OpenAI. Padrão: nenhum.
- `llm_model` — identificador do modelo LLM padrão. Padrão: nenhum.
- `llm_http_timeout_secs` — timeout HTTP bloqueante de LLM e webhook (segundos). Padrão: `60`.


## Cache e Redis
- `cache_backend` — backend de cache: `sqlite|memory|redis`. Padrão: `sqlite`.
- `cache_redis_url` — URL do Redis quando o backend é `redis`. Padrão: nenhum.
- `redis_allow_remote` — permitir hosts Redis fora de loopback (padrão falso). Padrão: nenhum.
- `redis_connect_timeout_secs` — timeout de conexão TCP com o Redis (segundos). Padrão: `2`.
- `redis_io_timeout_secs` — timeout de I/O do stream Redis ou RESP (segundos). Padrão: `3`.
- `cache_max_resp_bulk_bytes` — teto de tamanho da bulk string RESP do Redis (bytes). Padrão: `16777216`.
- `cache_max_resp_line_bytes` — teto de tamanho da linha RESP do Redis (bytes). Padrão: `16777216`.
- `scrape_http_cache_ttl_secs` — TTL do cache L2 de resposta de scrape HTTP (segundos). Padrão: `3600`.
- `file_parse_cache_ttl_secs` — TTL do cache L2 de parse de arquivo local (segundos). Padrão: `86400`.


## Busca Web
- `search_base_url` — base do endpoint de busca HTML (`?q=` é anexado). Padrão: `https://html.duckduckgo.com/html/`.


## Limites de Payload e Raízes
- `max_json_file_bytes` — máximo de bytes para arquivos de script ou manifesto JSON e NDJSON. Padrão: `33554432`.
- `max_ndjson_line_bytes` — máximo de bytes de uma linha NDJSON (scripts de `run` e traces). Padrão: `1048576`.
- `max_cli_json_payload_bytes` — máximo de bytes para payloads JSON passados em flags da CLI. Padrão: `4194304`.
- `max_sg_file_bytes` — máximo de bytes de um arquivo-fonte lido por `sg scan` ou `sg rewrite`. Padrão: `16777216`.
- `max_urls_file_bytes` — máximo de bytes da lista `--urls-file` do `batch-scrape`. Padrão: `8388608`.
- `run_max_include_depth` — profundidade máxima de aninhamento das cadeias de include em `run --script`. Padrão: `16`.
- `allowed_roots` — raízes extras permitidas para leituras locais e escrita de artefatos (separadas por plataforma); os padrões cobrem o diretório atual, os diretórios XDG e o temporário. Padrão: nenhum.


## Captura Visual e Screencast
- `default_jpeg_quality` — qualidade JPEG 1..=100 quando `grab` omite `--quality`. Padrão: `80`.
- `screencast_jpeg_quality` — qualidade JPEG do screencast via CDP 1..=100. Padrão: `60`.
- `screencast_ffmpeg_framerate` — taxa de quadros de entrada do ffmpeg no screencast (quadros por segundo). Padrão: `10`.
- `screencast_start_pump_iters` — iterações imediatas de bombeamento após `Page.startScreencast`. Padrão: `15`.
- `screencast_stop_pump_iters` — iterações de drenagem antes de `Page.stopScreencast`. Padrão: `40`.


## Interação e Espera
- `event_pump_slice_ms` — fatia de bombeamento de eventos em wait e eval (milissegundos). Padrão: `50`.
- `interact_settle_ms` — atraso de acomodação da UI após click, type ou extension (ms). Padrão: `200`.
- `dialog_settle_ms` — espera máxima após responder diálogo JS pelo `javascriptDialogClosed` (ms, GAP-054). Padrão: `2000`.
- `network_idle_window_ms` — janela de silêncio para `wait --network-idle` (milissegundos). Padrão: `500`.
- `dom_stable_window_ms` — janela de silêncio para `wait --dom-stable-ms` (milissegundos). Padrão: `500`.
- `drag_move_steps` — posições intermediárias de mouse sintetizadas em um drag HTML5. Padrão: `6`.
- `drag_move_gap_ms` — atraso entre posições sintetizadas de drag (milissegundos). Padrão: `16`.
- `eval_drain_slice_ms` — fatia de drenagem enquanto aguarda resultados de `Runtime.evaluate` (milissegundos). Padrão: `40`.
- `support_settle_ms` — acomodação da thread de suporte para helpers síncronos (milissegundos). Padrão: `80`.
- `nav_micro_settle_ms` — microacomodação de navegação após transições de página (milissegundos). Padrão: `100`.


## CDP e Sessão Chrome
- `cdp_connection_probe_timeout_secs` — timeout da sonda de vida `Browser.getVersion` do CDP (segundos). Padrão: `3`.
- `cdp_discovery_max_body_bytes` — máximo de bytes do corpo HTTP de descoberta CDP (`/json/version`, `/json/list`). Padrão: `1048576`.
- `cdp_event_broadcast_capacity` — capacidade do canal local de broadcast de eventos CDP. Padrão: `4096`.
- `cdp_event_drain_poll_ms` — fatia de polling de drenagem de eventos CDP durante espera de navegação (milissegundos). Padrão: `100`.
- `cdp_network_idle_settle_ms` — janela de acomodação de rede ociosa no CDP (milissegundos). Padrão: `500`.
- `cdp_target_event_wait_ms` — espera curta por evento de target no CDP (milissegundos). Padrão: `600`.
- `cdp_discovery_timeout_secs` — timeout de descoberta HTTP do CDP para sondas `/json/version` (segundos). Padrão: `2`.
- `event_tracker_max_entries` — tamanho do anel em memória do rastreador de console e rede por sessão de página. Padrão: `1000`.
- `chrome_default_timeout_ms` — timeout padrão por operação do motor Chrome (milissegundos). Padrão: `25000`.
- `extension_attach_poll_ms` — fatia de polling de attach de extensão (milissegundos). Padrão: `150`.


## HTTP e Segurança de Rede
- `http_ssrf_mode` — política SSRF do HTTP: `strict|allow_loopback|off`. Padrão: `strict`.
- `http_timeout_secs` — timeout total do cliente HTTP compartilhado (segundos). Padrão: `30`.
- `http_connect_timeout_secs` — timeout da fase de conexão HTTP (segundos). Padrão: `10`.
- `http_redirect_max` — máximo de redirecionamentos HTTP seguidos pelos clientes do produto. Padrão: `10`.
- `http_pool_max_idle_per_host` — máximo de conexões ociosas no pool reqwest por host. Padrão: `4`.


## Robots
- `robots_loopback_exempt` — hosts de loopback pulam o robots.txt (defina falso para impor contra localhost). Padrão: `true`.
- `robots_probe_timeout_secs` — timeout de HEAD ou sonda do robots.txt (segundos). Padrão: `5`.
- `robots_max_body_bytes` — máximo de bytes do corpo do robots.txt (anti-OOM). Padrão: `524288`.
- `robots_fetch_timeout_secs` — timeout para buscar o robots.txt (segundos). Padrão: `30`.


## Imagem e SVG
- `image_max_input_bytes` — máximo de bytes de entrada para decodificar, converter ou redimensionar imagem local. Padrão: `32000000`.
- `image_max_pixels` — máximo de largura vezes altura na decodificação de imagem (antibomba). Padrão: `64000000`.
- `image_default_format` — formato padrão de conversão de imagem: `png|jpeg|webp|gif`. Padrão: `png`.
- `image_default_quality` — qualidade com perdas padrão 1..=100 para converter ou redimensionar imagem. Padrão: `85`.
- `image_download_max_bytes` — máximo de bytes de corpo HTTP no download de imagem. Padrão: `32000000`.
- `image_avif_speed` — velocidade do codificador AVIF 1..=10 (1 é o mais lento e melhor); exige a feature `image-avif`. Padrão: `6`.
- `svg_max_bytes` — máximo de bytes de fonte SVG aceitos antes da rasterização. Padrão: `4000000`.
- `svg_max_depth` — profundidade máxima de aninhamento XML aceita em uma fonte SVG. Padrão: `128`.
- `svg_max_entities` — máximo de declarações `<!ENTITY>` toleradas em um DTD de SVG (0 rejeita qualquer uma). Padrão: `0`.
- `gif_max_frames` — máximo de quadros de animação decodificados de um GIF. Padrão: `2000`.


## Vídeo e Manifestos
- `video_max_input_bytes` — máximo de bytes na materialização de vídeo por stdin ou na checagem prévia de caminho. Padrão: `512000000`.
- `video_download_max_bytes` — máximo de bytes de corpo HTTP no download de vídeo. Padrão: `512000000`.
- `video_default_container` — contêiner padrão de conversão de vídeo: `mp4|webm|mkv|mov|avi|m4v`. Padrão: `mp4`.
- `video_default_crf` — CRF padrão 1..=51 na recodificação de vídeo com perdas. Padrão: `23`.
- `video_default_audio_bitrate` — bitrate padrão do `video to-mp3` (por exemplo `192k`). Padrão: `192k`.
- `manifest_max_bytes` — máximo de bytes aceitos no corpo de um manifesto HLS ou DASH. Padrão: `8000000`.
- `manifest_max_variants` — máximo de entradas de variante ou representação emitidas por envelope de manifesto. Padrão: `500`.


## Áudio
- `audio_max_input_bytes` — máximo de bytes na materialização de áudio por stdin ou na checagem prévia de caminho. Padrão: `256000000`.
- `audio_download_max_bytes` — máximo de bytes de corpo HTTP no download de áudio. Padrão: `256000000`.
- `audio_default_format` — formato padrão de conversão de áudio: `mp3|m4a|ogg|opus|flac|wav|aac`. Padrão: `mp3`.
- `audio_default_bitrate` — bitrate padrão do encode de áudio com perdas (por exemplo `192k`). Padrão: `192k`.


## Scrape Crawl e Map
- `scrape_max_body_bytes` — máximo de bytes de corpo no scrape HTTP. Padrão: `5000000`.
- `browser_scrape_max_body_bytes` — máximo de bytes de corpo nos helpers de scrape do motor browser. Padrão: `2000000`.
- `scrape_max_text_chars` — máximo de caracteres de texto ou markdown nos envelopes de scrape (0 remove o teto). Padrão: `32768`.
- `scrape_min_delay_ms` — atraso mínimo entre GETs de mesma origem (ms). Padrão: `0`.
- `scrape_honor_meta_robots` — honrar meta robots e `X-Robots-Tag` noindex. Padrão: `true`.
- `scrape_honor_nofollow` — pular links `rel=nofollow` na descoberta do crawl. Padrão: `true`.
- `scrape_use_sitemap` — preferir o sitemap.xml ao mapear um site. Padrão: `true`.
- `scrape_default_engine` — motor de scrape padrão quando a CLI omite `--engine` (`http|browser`). Padrão: `http`.
- `scrape_delay_jitter_ratio` — razão de jitter do atraso de cortesia 0.0..=1.0 (0 desliga). Padrão: `0.2`.
- `scrape_summary_chars` — máximo de caracteres do formato summary de scrape. Padrão: `400`.
- `scrape_feed_max_entries` — máximo de entradas mantidas pelo formato feed de scrape (RSS, Atom, JSON Feed). Padrão: `50`.
- `scrape_follow_rel_next` — seguir links de paginação `rel=next` durante o crawl. Padrão: nenhum.
- `scrape_dedup_similar` — colapsar páginas quase duplicadas por similaridade de conteúdo em crawl e batch-scrape. Padrão: nenhum.
- `scrape_dedup_similar_distance` — distância de Hamming SimHash (0..=64) abaixo da qual as páginas são quase duplicadas. Padrão: `3`.
- `scrape_sitemap_max_bytes` — máximo de bytes do corpo do sitemap. Padrão: `524288`.
- `scrape_charset_peek_bytes` — janela de inspeção para detectar charset (bytes). Padrão: `4096`.
- `scrape_crawl_limit_max` — orçamento máximo de páginas no crawl (clamp anti-DoS para `--limit`). Padrão: `500`.
- `scrape_crawl_max_depth` — profundidade máxima de BFS em crawl e map. Padrão: `10`.
- `scrape_search_limit_max` — orçamento máximo de resultados de busca (clamp anti-DoS). Padrão: `50`.
- `scrape_max_parse_bytes` — tamanho máximo de parse de arquivo local antes da rejeição (bytes). Padrão: `50000000`.


## Webhook do Operador
- `webhook_post_timeout_secs` — timeout do POST de webhook do operador (segundos). Padrão: `15`.
- `webhook_retry_base_delay_ms` — atraso base de retry do webhook (milissegundos; dobra a cada tentativa). Padrão: `50`.
- `webhook_max_attempts` — máximo de tentativas do webhook (incluindo a primeira). Padrão: `3`.


## Heap
- `heap_snapshot_max_bytes` — teto de tamanho do arquivo de heap snapshot offline (bytes). Padrão: `536870912`.
- `heap_max_retainers` — máximo de retainers retornados por operação de nó de heap. Padrão: `200`.
- `heap_max_edges` — máximo de arestas retornadas por operação de nó de heap. Padrão: `200`.
- `heap_max_paths` — máximo de caminhos na enumeração `heap paths`. Padrão: `32`.
- `heap_max_path_depth` — profundidade máxima em `heap paths`. Padrão: `8`.
- `heap_max_class_nodes` — teto da lista `heap class_nodes`. Padrão: `500`.
- `heap_dominator_max_states` — teto de estados visitados no cálculo de dominadores (grafos patológicos). Padrão: `50000`.
- `heap_outer_iters` — máximo de iterações do polling externo do heap snapshot. Padrão: `200`.
- `heap_inner_iters` — iterações de drenagem interna do heap snapshot após concluir. Padrão: `10`.
- `heap_final_iters` — iterações de drenagem final do heap snapshot. Padrão: `20`.


## Ciclo de Vida e Residual
- `browser_close_wait_secs` — orçamento de espera de `Browser.close` e do processo no FINALIZE (segundos). Padrão: `5`.
- `chrome_startup_timeout_secs` — espera de prontidão CDP no self-spawn do Chrome (segundos). Padrão: `20`.
- `residual_orphan_min_age_secs` — idade mínima antes que um perfil marcador de dono morto seja coletável (segundos). Padrão: `60`.
- `platform_child_wait_secs` — prazo de espera do processo filho da plataforma (segundos). Padrão: `5`.
- `shutdown_poll_ms` — intervalo de polling cooperativo de shutdown (milissegundos). Padrão: `5`.
- `shutdown_deadline_secs` — prazo duro de shutdown aguardando a saída do browser (segundos). Padrão: `30`.
- `chrome_legacy_oxide_launch` — lançar o Chrome via chromiumoxide em vez do caminho de self-spawn (fallback de estabilização; perde o alvo de kill residual). Padrão: nenhum.
- `default_viewport_width` — largura padrão da janela do Chrome headless quando as opções de launch omitem viewport. Padrão: `1280`.
- `default_viewport_height` — altura padrão da janela do Chrome headless quando as opções de launch omitem viewport. Padrão: `720`.


## Lightpanda
- `lightpanda_startup_timeout_secs` — espera de inicialização do processo Lightpanda (segundos). Padrão: `10`.
- `lightpanda_session_timeout_secs` — máximo de sessão do `--timeout` do Lightpanda (segundos, 1..=604800). Padrão: `604800`.
- `lightpanda_poll_interval_ms` — intervalo de polling de prontidão CDP do Lightpanda (milissegundos). Padrão: `100`.
- `lightpanda_discovery_timeout_ms` — timeout de descoberta CDP por sonda enquanto aguarda o Lightpanda (milissegundos). Padrão: `500`.
- `lightpanda_max_log_lines` — anel limitado de log de launch do Lightpanda (linhas por stream). Padrão: `40`.
- `lightpanda_ready_slice_ms` — fatia de drenagem após a saída do filho Lightpanda antes do snapshot dos logs (milissegundos). Padrão: `25`.
- `lightpanda_cdp_connect_timeout_secs` — timeout da tentativa de conexão CDP do Lightpanda (segundos). Padrão: `5`.
- `lightpanda_target_init_timeout_secs` — espera de inicialização do target Lightpanda após conectar (segundos). Padrão: `10`.


## MITM
- `mitm_list_limit_max` — clamp máximo de itens em list e query do MITM. Padrão: `10000`.
- `mitm_proxy_seconds_max` — janela máxima do proxy MITM em execução one-shot (segundos). Padrão: `600`.
- `mitm_chrome_settle_ms` — acomodação do launch do Chrome no MITM antes de navegar (milissegundos). Padrão: `150`.
- `mitm_capture_wait_min_ms` — piso de espera de captura MITM após navegar (milissegundos). Padrão: `800`.
- `mitm_capture_wait_max_ms` — teto de espera de captura MITM após navegar (milissegundos). Padrão: `8000`.
- `mitm_ws_frames_cap` — teto de quadros WebSocket em memória por processo de captura. Padrão: `500`.
- `mitm_ws_preview_chars` — truncamento do preview de texto WebSocket (caracteres Unicode). Padrão: `256`.
- `mitm_ca_cache_size` — tamanho do cache dinâmico de certificados do MITM (hosts). Padrão: `1000`.
- `mitm_rebind_attempts` — tentativas de rebind do proxy MITM quando a porta está transitoriamente em uso. Padrão: `3`.


## Perf
- `perf_autostop_settle_ms` — acomodação do auto-stop de perf após load ou reload (milissegundos). Padrão: `500`.
- `perf_trace_inner_slice_ms` — fatia interna de polling do trace de perf (milissegundos). Padrão: `20`.
- `perf_trace_outer_slice_ms` — intervalo externo de polling do trace de perf (milissegundos). Padrão: `50`.
- `perf_trace_outer_iters` — máximo de iterações do polling externo do trace de perf. Padrão: `100`.
- `perf_trace_inner_iters` — iterações de drenagem interna do trace de perf após concluir. Padrão: `5`.


## Estado de Storage
- `state_collect_deadline_secs` — prazo externo da coleta de storage via CDP (segundos). Padrão: `5`.
- `state_event_recv_secs` — fatia de recebimento de evento de storage via CDP (segundos). Padrão: `2`.
- `state_load_settle_ms` — atraso de acomodação após a navegação de `load_state` (milissegundos). Padrão: `500`.


## Retry
- `retry_default_max_attempts` — máximo de tentativas de retry padrão (incluindo a primeira). Padrão: `3`.
- `retry_base_delay_ms` — atraso base de retry padrão (milissegundos). Padrão: `50`.
- `retry_max_delay_secs` — atraso máximo de retry padrão (segundos). Padrão: `2`.
- `retry_budget_secs` — orçamento de relógio do retry padrão (segundos). Padrão: `10`.
- `retry_cdp_max_attempts` — máximo de tentativas de retry no CDP. Padrão: `4`.
- `retry_cdp_base_delay_ms` — atraso base de retry no CDP (milissegundos). Padrão: `100`.
- `retry_cdp_max_delay_secs` — atraso máximo de retry no CDP (segundos). Padrão: `3`.
- `retry_cdp_budget_secs` — orçamento de relógio do retry no CDP (segundos). Padrão: `15`.
- `retry_http_max_attempts` — máximo de tentativas de retry no scrape HTTP. Padrão: `3`.
- `retry_http_base_delay_ms` — atraso base de retry no scrape HTTP (milissegundos). Padrão: `75`.
- `retry_http_max_delay_secs` — atraso máximo de retry no scrape HTTP (segundos). Padrão: `2`.
- `retry_http_budget_secs` — orçamento de relógio do retry no scrape HTTP (segundos). Padrão: `12`.
- `retry_llm_max_attempts` — máximo de tentativas de retry no HTTP do LLM. Padrão: `2`.
- `retry_llm_base_delay_ms` — atraso base de retry no HTTP do LLM (milissegundos). Padrão: `200`.
- `retry_llm_max_delay_secs` — atraso máximo de retry no HTTP do LLM (segundos). Padrão: `4`.
- `retry_llm_budget_secs` — orçamento de relógio do retry no HTTP do LLM (segundos). Padrão: `20`.


## Referência Canônica
- DEVE tratar `docs/CONFIGURATION.pt-BR.md` como a referência canônica do produto para configuração
- DEVE usar este arquivo como índice operacional da skill e o documento canônico para detalhe normativo
- DEVE reconciliar qualquer divergência a favor de `docs/CONFIGURATION.pt-BR.md` e de `config list-keys --json`
