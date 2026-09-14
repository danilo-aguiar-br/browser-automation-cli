# Chaves XDG de browser-automation-cli


## Contrato de Configuração
- DEVE configurar o produto SOMENTE por flags da CLI e por `config`, e NUNCA por variável de ambiente, `export` ou `.env`
- DEVE aplicar a precedência flag da CLI, depois valor gravado no XDG, depois padrão embutido, porque a flag SEMPRE vence o XDG
- DEVE executar `browser-automation-cli --json config init` para criar o layout XDG e o `config.toml` padrão
- DEVE executar `browser-automation-cli --json config path` para resolver o arquivo de configuração e cada diretório XDG, e NUNCA inventar caminho
- DEVE executar `browser-automation-cli --json config list-keys` para descobrir toda chave viva com padrão e descrição
- DEVE executar `browser-automation-cli --json config show` para ler os valores gravados
- DEVE executar `browser-automation-cli --json config get <CHAVE>` para ler o valor GRAVADO de uma chave, e omitir a chave para despejar todas
- DEVE saber que `config get` de chave não gravada devolve `null`, e NUNCA tratar esse `null` como o valor em vigor
- DEVE ler o valor em vigor de chave não gravada no `default` que `config list-keys` reporta
- DEVE executar `browser-automation-cli --json config set <CHAVE> <VALOR>`, que o binário valida por chave antes de gravar
- DEVE executar `browser-automation-cli --json config unset <CHAVE>` para restaurar o padrão embutido, que também sai com sucesso quando a chave já está ausente
- NUNCA trate `config set <CHAVE> ""` como restauração, salvo em `user_data_dir`, onde a string vazia limpa o opt-in
- DEVE gravar credencial de proxy SOMENTE com `config set proxy_username` e `config set proxy_password`, e NUNCA passá-la em argv
- DEVE gravar segredos com `config set encryption_key` e `config set openrouter_api_key`, e NUNCA logar o valor
- DEVE ler padrão `ausente` como chave sem valor embutido, em que a lógica do comando decide o comportamento
- NUNCA invente chave fora de `config list-keys`, e NUNCA recuse chave desta lista por memória


## Núcleo e Identidade
- `lang` padrão `ausente` — DEVE fixar o idioma das mensagens em `en` ou `pt-BR`, e NUNCA gravar `pt` puro, que o binário recusa
- `timeout` padrão `0` — DEVE fixar o timeout global em segundos
- `artifacts_dir` padrão `ausente` — DEVE apontar o diretório padrão de saída de artefatos
- `ignore_robots` padrão `false` — NUNCA trate como contorno de robots, porque AMBAS as flags de robots continuam OBRIGATÓRIAS
- `namespace` padrão `ausente` — DEVE isolar o estado local sob um namespace próprio
- `encryption_key` padrão `ausente` — DEVE guardar o material da chave de criptografia de sessão SOMENTE no XDG, e NUNCA logar o valor
- `color` padrão `ausente` — DEVE ligar ou desligar cores ANSI no stderr humano


## Anti-Detecção e Janela
- `stealth` padrão `true` — DEVE manter ligado para mascarar marcadores de automação, e SEMPRE use `--no-stealth` para desligar só numa execução
- `stealth_profile` padrão `auto` — DEVE escolher `auto`, `chrome-linux`, `chrome-win` ou `chrome-mac`, e NUNCA declarar plataforma diferente do host
- `stealth_seed` padrão `ausente` — DEVE fixar a identidade de stealth entre processos, porque ausente ela é sorteada a cada processo
- `screen` padrão `ausente` — DEVE fixar a tela padrão `WxH` das métricas de dispositivo, porque ausente ela espelha o viewport
- `browser_mode` padrão `auto` — DEVE valer `auto`, `headed` ou `headless`, sabendo que `auto` resolve para headed no Xvfb privado SOMENTE no Linux com `Xvfb` no PATH e sem `--no-xvfb`, e para headless em todo o resto
- `input_profile` padrão `human` — DEVE valer `human` para ritmar ponteiro e teclado, ou `direct` para input sem modelagem


## Log Local
- `log_level` padrão `error` — DEVE fixar o nível de tracing usado quando nenhuma flag de verbosidade vier no argv
- `log_to_file` padrão `false` — DEVE ligar logs JSON locais rotacionados sob o estado XDG, que NUNCA saem da máquina
- `max_log_files` padrão `14` — DEVE limitar os arquivos de log retidos entre 1 e 90
- `log_rotation` padrão `daily` — DEVE escolher a rotação `daily`, `hourly` ou `never`


## Binários Externos
- `chrome_path` padrão `ausente` — DEVE apontar o caminho absoluto do Chrome ou Chromium de sistema
- `chrome_search_paths` padrão `ausente` — DEVE listar caminhos de descoberta do Chrome em ordem, com separador da plataforma, e ausente usa o layout embutido do sistema
- `lighthouse_path` padrão `ausente` — DEVE apontar o caminho absoluto da CLI lighthouse
- `lighthouse_timeout_secs` padrão `300` — DEVE limitar o relógio da CLI lighthouse entre 1 e 3600 segundos
- `ffmpeg_path` padrão `ausente` — DEVE apontar o caminho absoluto do ffmpeg usado por screencast, conversão de vídeo e `to-mp3`
- `ffmpeg_timeout_secs` padrão `120` — DEVE limitar o relógio do encode ffmpeg entre 1 e 3600 segundos


## LLM
- `openrouter_api_key` padrão `ausente` — DEVE guardar a chave de API do LLM SOMENTE no XDG, gravada em modo 0600, e NUNCA logar o valor
- `llm_base_url` padrão `ausente` — DEVE apontar a URL base compatível com a API da OpenAI
- `llm_model` padrão `ausente` — DEVE fixar o identificador do modelo LLM padrão
- `llm_http_timeout_secs` padrão `60` — DEVE limitar em segundos o HTTP bloqueante de LLM e de webhook


## Cache e Redis
- `cache_backend` padrão `sqlite` — DEVE escolher `sqlite`, `memory` ou `redis`, e NUNCA escolher `redis` sem `cache_redis_url`
- `cache_redis_url` padrão `ausente` — DEVE apontar a URL `redis://` quando o backend for `redis`, e NUNCA usar `rediss://`
- `redis_allow_remote` padrão `false` — NUNCA ligue sem decisão explícita, porque libera host Redis fora de loopback
- `redis_connect_timeout_secs` padrão `2` — DEVE limitar em segundos a conexão TCP com o Redis
- `redis_io_timeout_secs` padrão `3` — DEVE limitar em segundos o I/O do stream Redis
- `cache_max_resp_bulk_bytes` padrão `16777216` — DEVE limitar em bytes a bulk string RESP do Redis
- `cache_max_resp_line_bytes` padrão `16777216` — DEVE limitar em bytes a linha RESP do Redis
- `scrape_http_cache_ttl_secs` padrão `3600` — DEVE fixar em segundos a validade do cache de resposta de scrape HTTP
- `file_parse_cache_ttl_secs` padrão `86400` — DEVE fixar em segundos a validade do cache de parse de arquivo local


## Endpoint de Busca
- `search_base_url` padrão `https://html.duckduckgo.com/html/` — DEVE apontar a base do endpoint de busca HTML, à qual o binário anexa `?q=`


## Limites de Payload e Raízes
- `max_json_file_bytes` padrão `33554432` — DEVE limitar em bytes o arquivo JSON ou NDJSON de script e de manifesto
- `max_ndjson_line_bytes` padrão `1048576` — DEVE limitar em bytes uma linha NDJSON de script de `run` ou de trace
- `max_cli_json_payload_bytes` padrão `4194304` — DEVE limitar em bytes o payload JSON passado em flag
- `max_sg_file_bytes` padrão `16777216` — DEVE limitar em bytes o arquivo-fonte lido por `sg-scan` e `sg-rewrite`
- `max_urls_file_bytes` padrão `8388608` — DEVE limitar em bytes a lista `--urls-file` do `batch-scrape`
- `run_max_include_depth` padrão `16` — DEVE limitar a profundidade de include em `run --script`
- `allowed_roots` padrão `ausente` — DEVE somar raízes permitidas de leitura e escrita com separador da plataforma, e SEMPRE preferir esta chave a `--allow-outside-roots`


## Captura Visual e Screencast
- `default_jpeg_quality` padrão `80` — DEVE fixar a qualidade JPEG entre 1 e 100 quando `grab` omitir `--quality`
- `screencast_jpeg_quality` padrão `60` — DEVE fixar a qualidade JPEG do screencast entre 1 e 100
- `screencast_ffmpeg_framerate` padrão `10` — DEVE fixar em quadros por segundo a entrada do ffmpeg no screencast
- `screencast_start_pump_iters` padrão `15` — DEVE fixar as iterações de bombeamento logo após iniciar o screencast
- `screencast_stop_pump_iters` padrão `40` — DEVE fixar as iterações de drenagem antes de parar o screencast


## Interação e Espera
- `event_pump_slice_ms` padrão `50` — DEVE fixar em milissegundos a fatia de bombeamento de eventos em `wait` e `eval`
- `interact_settle_ms` padrão `200` — DEVE fixar em milissegundos a acomodação da página após clique, digitação ou extensão
- `dialog_settle_ms` padrão `2000` — DEVE fixar em milissegundos a espera máxima pelo fechamento do diálogo JS respondido, e NUNCA somar wait artificial
- `network_idle_window_ms` padrão `500` — DEVE fixar em milissegundos a janela de silêncio de `wait --network-idle`
- `dom_stable_window_ms` padrão `500` — DEVE fixar em milissegundos a janela de silêncio de `wait --dom-stable-ms`
- `drag_move_steps` padrão `6` — DEVE fixar as posições intermediárias do mouse num drag HTML5
- `drag_move_gap_ms` padrão `16` — DEVE fixar em milissegundos o intervalo entre posições do drag
- `eval_drain_slice_ms` padrão `40` — DEVE fixar em milissegundos a fatia de drenagem enquanto `eval` aguarda o resultado
- `support_settle_ms` padrão `80` — DEVE fixar em milissegundos a acomodação dos auxiliares síncronos
- `nav_micro_settle_ms` padrão `100` — DEVE fixar em milissegundos a microacomodação após transição de página


## Cinemática de Input
- `input_timing_distribution` padrão `lognormal` — DEVE escolher `lognormal`, `normal` ou `uniform` para o ritmo rápido, sabendo que a cauda de pausas longas é `input_word_pause_permille`
- `input_move_steps` padrão `24` — DEVE fixar as posições intermediárias do ponteiro por movimento no perfil `human`
- `input_move_gap_ms` padrão `12` — DEVE fixar em milissegundos o intervalo entre posições do ponteiro
- `input_click_dwell_ms` padrão `65` — DEVE fixar em milissegundos a retenção entre pressionar e soltar o botão
- `input_key_dwell_ms` padrão `45` — DEVE fixar em milissegundos a retenção entre descer e subir a tecla
- `input_type_delay_ms` padrão `95` — DEVE fixar em milissegundos o atraso entre caracteres digitados
- `input_scroll_tick_px` padrão `100` — DEVE fixar em pixels CSS a distância de um tick de roda
- `input_scroll_max_ticks` padrão `40` — DEVE limitar os ticks de roda por gesto de rolagem
- `input_target_jitter_px` padrão `3` — DEVE fixar em pixels CSS o raio do deslocamento aleatório do alvo do clique
- `input_scroll_settle_rounds` padrão `3` — DEVE fixar as rodadas extras para entregar um delta de roda descartado
- `input_move_steps_stddev` padrão `6` — DEVE fixar o desvio padrão das amostras de ponteiro por gesto
- `input_move_gap_stddev_ms` padrão `5` — DEVE fixar em milissegundos o desvio padrão do intervalo entre posições do ponteiro
- `input_click_dwell_stddev_ms` padrão `26` — DEVE fixar em milissegundos o desvio padrão da retenção do clique
- `input_key_dwell_stddev_ms` padrão `18` — DEVE fixar em milissegundos o desvio padrão da retenção da tecla
- `input_type_delay_stddev_ms` padrão `40` — DEVE fixar em milissegundos o desvio padrão do atraso entre caracteres
- `input_scroll_tick_stddev_px` padrão `25` — DEVE fixar em pixels CSS o desvio padrão da distância do tick de roda
- `input_word_pause_ms` padrão `320` — DEVE fixar em milissegundos a média da pausa extra em limite de palavra ou frase
- `input_word_pause_permille` padrão `120` — DEVE fixar a chance em mil de um limite de palavra ganhar pausa longa
- `input_typo_permille` padrão `0` — NUNCA ligue sem decisão explícita, porque o erro de digitação corrigido com `Backspace` muda o fluxo de caracteres que a página lê


## CDP e Sessão Chrome
- `cdp_connection_probe_timeout_secs` padrão `3` — DEVE limitar em segundos a sonda de vida do CDP
- `cdp_discovery_max_body_bytes` padrão `1048576` — DEVE limitar em bytes o corpo de descoberta CDP, sabendo que só a prontidão do Lightpanda lê esta chave
- `cdp_discovery_timeout_secs` padrão `2` — NUNCA espere efeito no lançamento do Chrome, que fala CDP por pipe, e o Lightpanda usa `lightpanda_discovery_timeout_ms`
- `cdp_event_broadcast_capacity` padrão `4096` — DEVE fixar a capacidade do canal local de eventos CDP
- `cdp_event_drain_poll_ms` padrão `100` — DEVE fixar em milissegundos a fatia de drenagem de eventos durante a espera de navegação
- `cdp_network_idle_settle_ms` padrão `500` — DEVE fixar em milissegundos a acomodação de rede ociosa no CDP
- `cdp_target_event_wait_ms` padrão `600` — DEVE fixar em milissegundos a espera curta por evento de target
- `event_tracker_max_entries` padrão `1000` — DEVE mover SOMENTE por esta chave o teto do buffer de console e rede, e SEMPRE ler `dropped_oldest`
- `capture_preserved_rings` padrão `3` — DEVE fixar quantas fronteiras de navegação `--include-preserved` mantém em console e rede
- `chrome_default_timeout_ms` padrão `25000` — DEVE fixar em milissegundos o timeout padrão por operação do Chrome
- `extension_attach_poll_ms` padrão `150` — DEVE fixar em milissegundos a fatia de sondagem do attach de extensão
- `extension_attach_poll_iters` padrão `20` — DEVE fixar as iterações do attach de extensão, sabendo que fatia vezes iterações é a espera total


## HTTP e Segurança de Rede
- `http_ssrf_mode` padrão `strict` — DEVE manter `strict`, usar `allow_loopback` só para alvo local, e NUNCA gravar `off` sem decisão explícita
- `http_timeout_secs` padrão `30` — DEVE limitar em segundos o tempo total do cliente HTTP
- `http_connect_timeout_secs` padrão `10` — DEVE limitar em segundos a fase de conexão HTTP
- `http_redirect_max` padrão `10` — DEVE limitar os redirecionamentos HTTP seguidos
- `http_pool_max_idle_per_host` padrão `4` — DEVE limitar as conexões HTTP ociosas por host


## Proxy de Saída
- `proxy_url` padrão `ausente` — DEVE apontar o proxy `http`, `https` ou `socks5` de saída do Chrome e do motor HTTP
- `proxy_bypass` padrão `ausente` — DEVE listar os hosts que pulam o proxy na sintaxe de bypass-list do Chrome
- `proxy_username` padrão `ausente` — DEVE guardar o usuário do proxy SOMENTE no XDG, e NUNCA em argv, que a tabela de processos expõe
- `proxy_password` padrão `ausente` — DEVE guardar a senha do proxy SOMENTE no XDG, NUNCA em argv, e NUNCA logar o valor
- `cdp_proxy_bypass_loopback` padrão `true` — DEVE manter ligado para o canal de controle CDP em loopback sobreviver sob `--proxy`


## Fingerprint HTTP/2
- `http2_enabled` padrão `true` — DEVE manter ligado para o motor HTTP negociar HTTP/2 como o Chrome
- `http2_initial_stream_window_size` padrão `6291456` — DEVE fixar o `SETTINGS_INITIAL_WINDOW_SIZE` anunciado ao par
- `http2_initial_connection_window_size` padrão `15663105` — DEVE fixar a janela de controle de fluxo da conexão HTTP/2
- `http2_max_header_list_size` padrão `262144` — DEVE fixar o `SETTINGS_MAX_HEADER_LIST_SIZE` do HTTP/2
- `http2_max_frame_size` padrão `16384` — DEVE fixar o `SETTINGS_MAX_FRAME_SIZE` entre 16384 e 16777215
- `http2_adaptive_window` padrão `false` — DEVE manter desligado para o fingerprint HTTP/2 ficar constante


## Robots
- `robots_loopback_exempt` padrão `true` — DEVE gravar `false` para impor o robots.txt também contra localhost
- `robots_user_agent` padrão `ausente` — DEVE fixar o token de user-agent casado contra as regras do robots.txt
- `robots_probe_timeout_secs` padrão `5` — DEVE limitar em segundos a requisição do robots.txt
- `robots_max_body_bytes` padrão `524288` — DEVE limitar em bytes o corpo do robots.txt


## Imagem e SVG
- `image_max_input_bytes` padrão `32000000` — DEVE limitar em bytes a imagem local a decodificar, converter ou redimensionar
- `image_max_pixels` padrão `64000000` — DEVE limitar largura vezes altura na decodificação contra bomba de imagem
- `image_default_format` padrão `png` — DEVE escolher `png`, `jpeg`, `webp` ou `gif` como formato padrão de conversão
- `image_default_quality` padrão `85` — DEVE fixar a qualidade com perdas entre 1 e 100 em conversão e redimensionamento
- `image_download_max_bytes` padrão `32000000` — DEVE limitar em bytes o corpo HTTP de `image download`
- `image_avif_speed` padrão `6` — NUNCA espere efeito sem codificação AVIF no binário, e NUNCA peça saída AVIF
- `svg_max_bytes` padrão `4000000` — DEVE limitar em bytes a fonte SVG aceita
- `svg_max_depth` padrão `128` — DEVE limitar a profundidade de aninhamento XML de uma fonte SVG
- `svg_max_entities` padrão `0` — DEVE manter `0` para recusar toda declaração `<!ENTITY>` em SVG
- `gif_max_frames` padrão `2000` — DEVE limitar os quadros decodificados de um GIF


## Vídeo e Manifestos
- `video_max_input_bytes` padrão `512000000` — DEVE limitar em bytes o vídeo lido por stdin ou por caminho
- `video_download_max_bytes` padrão `512000000` — DEVE limitar em bytes o corpo HTTP de `video download`
- `video_default_container` padrão `mp4` — DEVE escolher `mp4`, `webm`, `mkv`, `mov`, `avi` ou `m4v` como contêiner padrão
- `video_default_crf` padrão `23` — DEVE fixar o CRF entre 1 e 51 na recodificação com perdas
- `video_default_audio_bitrate` padrão `192k` — DEVE fixar o bitrate padrão de `video to-mp3`
- `manifest_max_bytes` padrão `8000000` — DEVE limitar em bytes o corpo de manifesto HLS ou DASH
- `manifest_max_variants` padrão `500` — DEVE limitar as variantes emitidas por envelope de `video manifest`


## Áudio
- `audio_max_input_bytes` padrão `256000000` — DEVE limitar em bytes o áudio lido por stdin ou por caminho
- `audio_download_max_bytes` padrão `256000000` — DEVE limitar em bytes o corpo HTTP de `audio download`
- `audio_default_format` padrão `mp3` — DEVE escolher `mp3`, `m4a`, `ogg`, `opus`, `flac`, `wav` ou `aac` como formato padrão
- `audio_default_bitrate` padrão `192k` — DEVE fixar o bitrate padrão do encode de áudio com perdas


## Scrape Crawl e Map
- `scrape_default_engine` padrão `http` — DEVE manter `http` como motor barato, e SEMPRE passar `--engine browser` só na página que depende de JavaScript
- `scrape_max_body_bytes` padrão `5000000` — DEVE limitar em bytes o corpo do scrape HTTP
- `browser_scrape_max_body_bytes` padrão `2000000` — DEVE limitar em bytes o corpo dos auxiliares de scrape do motor browser
- `scrape_max_text_chars` padrão `32768` — DEVE limitar os caracteres de texto e markdown no envelope, sabendo que `0` remove o teto
- `scrape_min_delay_ms` padrão `0` — DEVE fixar em milissegundos o piso de cortesia entre GETs da mesma origem
- `scrape_delay_jitter_ratio` padrão `0.2` — DEVE fixar entre 0.0 e 1.0 o jitter do atraso de cortesia, sabendo que `0` desliga
- `scrape_honor_meta_robots` padrão `true` — DEVE manter ligado para honrar meta robots e `X-Robots-Tag` noindex
- `scrape_honor_nofollow` padrão `true` — DEVE manter ligado para pular link `rel=nofollow` na descoberta do crawl
- `scrape_use_sitemap` padrão `true` — DEVE manter ligado para preferir o sitemap.xml ao mapear um site
- `scrape_follow_rel_next` padrão `false` — DEVE ligar para o crawl seguir paginação `rel=next`
- `scrape_dedup_similar` padrão `false` — DEVE ligar para colapsar páginas quase duplicadas em `crawl` e `batch-scrape`
- `scrape_dedup_similar_distance` padrão `3` — DEVE fixar entre 0 e 64 a distância SimHash abaixo da qual a página é quase duplicada
- `scrape_summary_chars` padrão `400` — DEVE limitar os caracteres do formato `summary`
- `scrape_feed_max_entries` padrão `50` — DEVE limitar as entradas do formato `feed` de RSS, Atom e JSON Feed
- `scrape_sitemap_max_bytes` padrão `2000000` — DEVE limitar em bytes o corpo do sitemap
- `scrape_charset_peek_bytes` padrão `4096` — DEVE fixar em bytes a janela de detecção de charset
- `scrape_crawl_limit_max` padrão `500` — DEVE limitar o orçamento de páginas que `--limit` do crawl aceita
- `scrape_crawl_max_depth` padrão `10` — DEVE limitar a profundidade de `crawl` e `map`
- `scrape_search_limit_max` padrão `50` — DEVE limitar o orçamento de resultados de `search`
- `scrape_max_parse_bytes` padrão `50000000` — DEVE limitar em bytes o arquivo local aceito por `parse`
- `scrape_no_cache` padrão `false` — DEVE ligar para ignorar o cache na leitura e SEMPRE buscar na origem
- `monitor_diff_max_bytes` padrão `65536` — DEVE limitar em bytes o payload de `monitor check --diff-mode`


## Webhook do Operador
- `webhook_post_timeout_secs` padrão `15` — DEVE limitar em segundos o POST do webhook do operador
- `webhook_retry_base_delay_ms` padrão `50` — DEVE fixar em milissegundos o atraso base de retry, que dobra a cada tentativa
- `webhook_max_attempts` padrão `3` — DEVE limitar as tentativas do webhook contando a primeira


## Heap
- `heap_snapshot_max_bytes` padrão `536870912` — DEVE limitar em bytes o arquivo de heap snapshot lido offline
- `heap_max_retainers` padrão `200` — DEVE limitar os retainers devolvidos por operação de nó
- `heap_max_edges` padrão `200` — DEVE limitar as arestas devolvidas por operação de nó
- `heap_max_paths` padrão `32` — DEVE limitar os caminhos enumerados por `heap paths`
- `heap_max_path_depth` padrão `8` — DEVE limitar a profundidade de `heap paths`
- `heap_max_class_nodes` padrão `500` — DEVE limitar a lista de nós por classe
- `heap_dominator_max_states` padrão `50000` — DEVE limitar os estados visitados no cálculo de dominadores
- `heap_outer_iters` padrão `200` — DEVE limitar as iterações externas de sondagem do snapshot
- `heap_inner_iters` padrão `10` — DEVE fixar as iterações de drenagem interna após o snapshot concluir
- `heap_final_iters` padrão `20` — DEVE fixar as iterações de drenagem final do snapshot


## Ciclo de Vida e Residual
- `user_data_dir` padrão `ausente` — DEVE tratar como abrir mão EXPLÍCITA do residual-zero, porque o perfil nasce 0700 em Unix e persiste ao DIE, e SEMPRE voltar com `config unset user_data_dir`
- `chrome_legacy_oxide_launch` padrão `false` — NUNCA ligue, porque reabre porta DevTools sem autenticação, não sobe o Xvfb e desenha janela headed no display do operador
- `chrome_startup_timeout_secs` padrão `20` — DEVE limitar em segundos a espera de prontidão do Chrome lançado pela CLI
- `browser_close_wait_secs` padrão `5` — DEVE limitar em segundos a espera de fechamento do navegador no FINALIZE
- `shutdown_deadline_secs` padrão `30` — DEVE fixar em segundos o prazo duro de saída do navegador
- `platform_child_wait_secs` padrão `5` — DEVE fixar em segundos o prazo de espera do processo filho
- `platform_child_poll_ms` padrão `50` — DEVE fixar em milissegundos a sondagem de saída do filho no FINALIZE
- `residual_orphan_min_age_secs` padrão `60` — DEVE fixar em segundos a idade mínima para coletar perfil marcador de dono morto
- `default_viewport_width` padrão `1920` — DEVE fixar a largura padrão da janela quando o lançamento omitir viewport
- `default_viewport_height` padrão `1080` — DEVE fixar a altura padrão da janela quando o lançamento omitir viewport


## Lightpanda
- `lightpanda_startup_timeout_secs` padrão `10` — DEVE limitar em segundos a subida do processo Lightpanda
- `lightpanda_session_timeout_secs` padrão `604800` — DEVE limitar a sessão do Lightpanda entre 1 e 604800 segundos
- `lightpanda_poll_interval_ms` padrão `100` — DEVE fixar em milissegundos a sondagem de prontidão CDP do Lightpanda
- `lightpanda_discovery_timeout_ms` padrão `500` — DEVE limitar em milissegundos cada sonda de descoberta CDP do Lightpanda
- `lightpanda_max_log_lines` padrão `40` — DEVE limitar as linhas de log de lançamento do Lightpanda por stream
- `lightpanda_ready_slice_ms` padrão `25` — DEVE fixar em milissegundos a drenagem após a saída do filho Lightpanda
- `lightpanda_cdp_connect_timeout_secs` padrão `5` — DEVE limitar em segundos a conexão CDP do Lightpanda
- `lightpanda_target_init_timeout_secs` padrão `10` — DEVE limitar em segundos a inicialização do target Lightpanda


## MITM
- `mitm_list_limit_max` padrão `10000` — DEVE limitar os itens de list e query do MITM
- `mitm_proxy_seconds_max` padrão `600` — DEVE limitar em segundos a janela do proxy MITM
- `mitm_chrome_settle_ms` padrão `150` — DEVE fixar em milissegundos a acomodação do Chrome antes de navegar sob MITM
- `mitm_capture_wait_min_ms` padrão `800` — DEVE fixar em milissegundos o piso de espera da captura após navegar
- `mitm_capture_wait_max_ms` padrão `8000` — DEVE fixar em milissegundos o teto de espera da captura após navegar
- `mitm_ws_frames_cap` padrão `500` — DEVE limitar os quadros WebSocket em memória por captura
- `mitm_ws_preview_chars` padrão `256` — DEVE limitar os caracteres do preview de texto WebSocket
- `mitm_ca_cache_size` padrão `1000` — DEVE fixar em hosts o cache de certificados dinâmicos
- `mitm_rebind_attempts` padrão `3` — DEVE fixar as tentativas de rebind quando a porta estiver ocupada


## Perf
- `perf_autostop_settle_ms` padrão `500` — DEVE fixar em milissegundos a acomodação do auto-stop de perf após load
- `perf_trace_inner_slice_ms` padrão `20` — DEVE fixar em milissegundos a fatia interna de sondagem do trace
- `perf_trace_outer_slice_ms` padrão `50` — DEVE fixar em milissegundos o intervalo externo de sondagem do trace
- `perf_trace_outer_iters` padrão `100` — DEVE limitar as iterações externas de sondagem do trace
- `perf_trace_inner_iters` padrão `5` — DEVE fixar as iterações de drenagem interna após o trace concluir


## Estado de Storage
- `state_collect_deadline_secs` padrão `5` — DEVE limitar em segundos a coleta de storage
- `state_event_recv_secs` padrão `2` — DEVE fixar em segundos a fatia de recebimento de evento de storage
- `state_load_settle_ms` padrão `500` — DEVE fixar em milissegundos a acomodação após a navegação de importação de estado


## Retry
- `retry_default_max_attempts` padrão `3` — DEVE limitar as tentativas do retry padrão contando a primeira
- `retry_base_delay_ms` padrão `50` — DEVE fixar em milissegundos o atraso base do retry padrão
- `retry_max_delay_secs` padrão `2` — DEVE limitar em segundos o atraso do retry padrão
- `retry_budget_secs` padrão `10` — DEVE limitar em segundos o orçamento total do retry padrão
- `retry_cdp_max_attempts` padrão `4` — DEVE limitar as tentativas de retry no CDP
- `retry_cdp_base_delay_ms` padrão `100` — DEVE fixar em milissegundos o atraso base de retry no CDP
- `retry_cdp_max_delay_secs` padrão `3` — DEVE limitar em segundos o atraso de retry no CDP
- `retry_cdp_budget_secs` padrão `15` — DEVE limitar em segundos o orçamento total de retry no CDP
- `retry_http_max_attempts` padrão `3` — DEVE limitar as tentativas de retry no scrape HTTP
- `retry_http_base_delay_ms` padrão `75` — DEVE fixar em milissegundos o atraso base de retry no scrape HTTP
- `retry_http_max_delay_secs` padrão `2` — DEVE limitar em segundos o atraso de retry no scrape HTTP
- `retry_http_budget_secs` padrão `12` — DEVE limitar em segundos o orçamento total de retry no scrape HTTP
- `retry_llm_max_attempts` padrão `2` — DEVE limitar as tentativas de retry no HTTP do LLM
- `retry_llm_base_delay_ms` padrão `200` — DEVE fixar em milissegundos o atraso base de retry no HTTP do LLM
- `retry_llm_max_delay_secs` padrão `4` — DEVE limitar em segundos o atraso de retry no HTTP do LLM
- `retry_llm_budget_secs` padrão `20` — DEVE limitar em segundos o orçamento total de retry no HTTP do LLM
