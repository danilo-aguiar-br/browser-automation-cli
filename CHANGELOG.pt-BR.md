[English](CHANGELOG.md) | [Português Brasileiro](CHANGELOG.pt-BR.md)

# Changelog

Todas as mudanças notáveis deste projeto são documentadas neste arquivo.

O formato segue [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/),
e este projeto adere ao [Semantic Versioning](https://semver.org/lang/pt-BR/).

## [Unreleased]

## [0.1.8] - 2026-08-10

### Adicionado
- Patches anti-detecção aplicados antes da primeira navegação, ligados por padrão, mascarando os marcadores de automação que um Chrome real nunca expõe: `--no-stealth` desliga nesta execução, `--stealth-profile` escolhe a identidade personificada (`auto`, `chrome-linux`, `chrome-win`, `chrome-mac`, com `auto` seguindo o host) e `--stealth-seed` fixa essa identidade entre processos. Sem a semente cada execução sorteia identidade nova, então um crawl de 50 URLs distribuído em 50 processos one-shot se apresentava como 50 máquinas diferentes. Sustentado pelas chaves XDG `stealth`, `stealth_profile` e `stealth_seed`
- `browser_mode` (`auto`, `headed`, `headless`; `auto` resolve para headless e o `doctor` reporta qual foi usado) e `--no-xvfb`, que pula o display virtual privado no Linux e usa o display atual — o único caso em que isso faz sentido é o modo headed no Linux
- Proxy de saída para o motor Chrome E para o motor HTTP: `--proxy <URL>` aceitando `http`, `https` e `socks5`, e `--proxy-bypass <HOSTS>` na sintaxe de bypass-list do próprio Chrome. As chaves XDG `proxy_url` e `proxy_bypass` carregam os mesmos valores, enquanto `proxy_username` e `proxy_password` existem somente no XDG, porque argv é visível na tabela de processos. `cdp_proxy_bypass_loopback` tem padrão `true` para que o canal de controle CDP sobreviva a um proxy que de outra forma o engoliria
- Controle de fingerprint HTTP/2 no cliente HTTP compartilhado, para que o motor `http` pare de anunciar um frame de settings que navegador nenhum envia: `http2_enabled` (padrão `true`, porque o Chrome sempre oferece h2), `http2_initial_stream_window_size` (6291456), `http2_initial_connection_window_size` (15663105), `http2_max_header_list_size` (262144), `http2_max_frame_size` (16384, faixa 16384..=16777215) e `http2_adaptive_window` (padrão `false`, porque uma janela que redimensiona em tempo de execução move o fingerprint entre requisições)
- Cinemática humana de input, ligada por padrão: `--input-profile human|direct` interpola trajetórias do ponteiro, aplica dwell entre press e release e ritma a digitação, e `--input-seed` faz uma execução `human` reproduzir exatamente. Dez chaves XDG expõem o modelo: `input_profile`, `input_move_steps` (24), `input_move_gap_ms` (12), `input_click_dwell_ms` (65), `input_key_dwell_ms` (45), `input_type_delay_ms` (95), `input_scroll_tick_px` (100), `input_scroll_max_ticks` (40), `input_target_jitter_px` (3) e `input_scroll_settle_rounds` (3)
- `--warmup` visita a raiz da origem antes da URL alvo para que a sessão já carregue cookies quando a requisição que importa for feita, e `--warmup-url <URL>` aquece outra URL em vez dessa raiz
- `--expect <EXPR>` afirma que o payload emitido casa com `key=value`, `key!=value` ou `key~substring`, repetível e conjugado por AND, para que o chamador pare de reler o envelope inteiro só para decidir se a execução foi útil. `--expect-exit-code` é separada e desligada por padrão: transformar divergência de conteúdo de dado em exit **65** quebraria em silêncio todo chamador existente que só ramifica em falha de transporte
- `config unset <KEY>`, o inverso de `set`. `config set <key> ""` nunca foi um inverso — para chave string gravava um valor vazio que o caminho normal nunca produz, e para chave numérica era erro de parse. Desfazer chave já ausente tem sucesso, então um script nunca precisa saber o estado anterior
- `robots_user_agent`, que nomeia o token contra o qual as regras do robots.txt são casadas; `scrape_no_cache`, que ignora o cache de resposta na leitura e sempre busca da origem; e `monitor_diff_max_bytes` (65536), teto em bytes para o payload de `monitor check --diff-mode`
- `scripts/config-roundtrip-check.sh`, auto-descoberto pelo `ci-check`, exigindo cada chave de `CONFIG_KEYS` presente no gravador E no leitor. Dois controles em `scripts/verifier-controls-check.sh` provam que o gate acusa cada lado. O `every_declared_key_survives_being_set` já existente itera três chaves fixas e não teria pego nenhuma das seis chaves quebradas
- `tests/phantom_flag_gate.rs`, o `phantom_flag_scan.py` de 411 linhas portado para Rust e deletado, cobrindo 242 flags declaradas com as mesmas três propriedades. O piso do universo subiu de 20 para 200, porque 20 é satisfeito por um walk que falha em todo subcomando. `scripts/phantom-flag-gate.sh` é o adaptador que mantém o runner de controles apontado para um gate que ainda existe — mover uma verificação sem mover o controle é como um gate vira carimbo

### Modificado
- A superfície XDG cresceu de **176** chaves para **204**, em cinco famílias: anti-detecção, modo de janela, proxy de saída, fingerprint HTTP/2 e cinemática humana de input, mais `robots_user_agent`, `scrape_no_cache` e `monitor_diff_max_bytes`. O inventário de agente permanece em **69** comandos
- `src/xdg/config_write_optional.rs` extraído: as seis chaves reparadas empurraram `config_write.rs` para 301 linhas contra o limite de 300. A fronteira é semântica e não aritmética — renderizar o template e anexar o que o template não expressa são trabalhos distintos. O gate de round-trip varre os dois arquivos, senão reportaria toda chave opcional como ausente
- `json_escape` em `scripts/docs-check.sh` passou a usar `jaq -R -c .`. Medido: `bash scripts/ci-check.sh` FALHAVA em host sem `python3`, por duas cadeias — `agent-ops-check.sh:152` chegando em `phantom_flag_scan.py`, e `docs-check.sh:39` rodando `python3 -c` sob `set -euo pipefail`. As duas cadeias acabaram; restam cinco scripts `.py` somando 1030 linhas e `python3` inline em sete shells, nenhum deles quebra o `ci-check`, e todos seguem como dívida nomeada contra a regra rust-native
- `tests/v018_parity_gate.rs` resolve o binário com `env!("CARGO_BIN_EXE_browser-automation-cli")` em vez do caminho fixo `target/debug/browser-automation-cli`. O caminho de skip usava `eprintln!`, que o libtest engole, então os 14 testes reportariam `ok` sem medir nada. Outros oito gates herdam o padrão frágil e seguem como dívida nomeada
- Dois comentários que contradiziam o código que descreviam: `scrape_view.rs` afirmava "BODY SIGNALS ONLY" enquanto a chamada já passava corpo, URL e título, e o `Cargo.toml` enquadrava reconhecimento de texto como dependência adiada por MSRV quando a regra do produto o proíbe de forma permanente — o comentário ainda citava `src/image_local/ocr_rs.rs`, arquivo que não existe

### Corrigido
- Seis chaves XDG declaradas aceitavam `config set` com `ok:true` e nunca persistiam: `proxy_url`, `proxy_bypass`, `proxy_username`, `proxy_password`, `stealth_seed` e `robots_user_agent`. O processo seguinte lia `null`. Duas delas são credenciais que `docs/CONFIGURATION.md:204` manda o operador guardar no XDG justamente porque argv vaza, então o canal documentado como seguro descartava o valor e sobrava o canal que vaza. Reparadas nos dois lados, gravador e leitor, com `proxy_password` entrando nas varreduras de zeroize de `secrets.rs`. A rodada de 2026-08-09 havia corrigido 17 chaves sem corrigir o mecanismo: `write_config` seguia template manual e `apply_toml_kv` seguia match literal, sem nenhuma fonte única de verdade entre os dois
- O `scrape` respondia em duas formas diferentes conforme a quantidade de valores que `--format` carregava. Medido em 2026-08-10 contra a mesma URL: `--format markdown` devolvia vinte chaves no topo, conteúdo mais todo o diagnóstico (`status_code`, `http_error`, `cache_hit`, `robots_policy`, `charset`, `http_version`, `stealth`, `tls_impersonation`, `http2_profile`, `header_order_controlled`, `change_status`), enquanto `--format markdown,links` devolvia quatro (`engine`, `format_list`, `formats`, `source_url`) com todo campo de diagnóstico sumido. Pedir MAIS dado devolvia MENOS, e `--fields markdown` funcionava no primeiro caso mas devolvia `data` vazio com `ok: true`, sem `agent_ops` e com exit **0** no segundo — uma resposta errada e silenciosa. A forma agora é a união: `formats` e `format_list` sempre presentes, cada formato também espelhado no topo, e um campo do transporte vence sobre o derivado de mesmo nome para que o campo siga significando "o que voltou no fio"
- `--mitm-max-body-bytes`, `--mitm-no-media-bodies` e `--mitm-redact-secrets` eram declaradas na CLI e lidas por ninguém: o `--help` prometia teto de corpo, filtro de mídia e chave de redação, e a captura não aplicava nenhuma delas. A redação até acontecia, mas por acidente de call site — todo chamador passava `true` literal, então a flag não conseguia nem ligar nem desligar. `src/mitm_local/policy.rs` agora publica a política uma vez a partir do dispatch da CLI, com teto padrão de 65536 bytes por corpo, e resolve a contradição entre pedir mascaramento e pedir para desligá-lo mascarando

## [0.1.7] - 2026-08-04

### Adicionado
- Operações universais de dados no envelope de sucesso, aplicadas sobre `data` antes de chegar ao stdout e portanto cobrindo todos os **69** comandos com uma implementação: `--fields`, `--filter-rows`, `--limit-rows`, `--sort-rows`, `--dedupe-by`, `--count-only`, `--truncate-content`, `--max-output-bytes`. Os quatro nomes de escopo de linha levam o sufixo `-rows` porque `--select`, `--filter`, `--limit` e `--sort` já eram flags por comando em `scrape`, `crawl`, `map`, `search`, `batch-scrape` e nos verbos `info` de mídia; promover essas grafias ao escopo global colidiria com 32 declarações existentes. Antes só 8 de 69 comandos tinham alguma e elas discordavam — `crawl` tinha oito, `scrape` tinha uma, `doctor` não tinha nenhuma. Medido em `doctor --offline --quick`: 26.277 bytes → **80** bytes com `--fields residual.ghost_marker_processes`. O envelope ganha `agent_ops` (`total`, `matched`, `truncated`, `omitted_rows`) apenas quando alguma flag rodou, então envelopes intocados mantêm a forma anterior exata
- `scripts/natives-check.sh` Pass N trava a allowlist de crates `*-sys` / nativas e proíbe `openssl` e `nasm-rs`; também dispara quando `aws-lc-sys` SAI, para que `cmake` seja aposentado dos pré-requisitos documentados em vez de sobreviver à própria causa. Quatro controles novos em `scripts/verifier-controls-check.sh` provam que o gate detecta cada mutação
- `docs/CONFIGURATION.md` e `docs/CONFIGURATION.pt-BR.md`: a referência XDG completa, todas as **176** chaves com padrão e propósito. **132** delas não apareciam em nenhum documento público, então a única forma de conhecer a superfície era `config list-keys --json` — servível para um agente, invisível para quem compara o produto com alternativas
- `scripts/doc-coverage-check.sh`: lê o binário VIVO para chaves e comandos e reprova quando a prosa se afasta de qualquer um dos dois. `scripts/docs-check.sh` valida rustdoc e nunca abre o README; `scripts/inventory-flat-check.sh` trava a CONTAGEM de comandos sem verificar se cada nome está documentado em algum lugar, e nada diz sobre chaves de configuração. A asserção de escopo de flag é deliberadamente consciente de escopo: um check ingênuo do tipo "essa flag existe" passaria em `--select`, porque ela existe em `scrape`
- `PRIVACY.pt-BR.md`: a política existia apenas em inglês e era o único documento da raiz sem espelho bilíngue
- `agent_ops.unresolved_paths`, que nomeia a flag e o caminho exatamente como o chamador digitou sempre que uma chave pedida não resolve em nenhuma linha. Uma contagem pura não seria acionável
- `scripts/agent-ops-check.sh` e `tests/agent_ops_cli.rs`: dez asserções conduzidas por argv contra o binário compilado. A cobertura de integração das oito flags de envelope era zero absoluto — o único match de `--fields` sob `tests/` era `--fields-json` do `fill-form`
- Nove chaves XDG promovidas para fora de literais no código: `max_urls_file_bytes`, `run_max_include_depth`, `mitm_rebind_attempts`, `network_idle_window_ms`, `dom_stable_window_ms`, `chrome_default_timeout_ms`, `drag_move_steps`, `drag_move_gap_ms`, `robots_fetch_timeout_secs`. Os dois orçamentos de espera são os mais visíveis ao usuário no produto, e o motor Lightpanda já tinha chave de timeout de sessão enquanto o motor Chrome não tinha equivalente
- `scrape --format metadata` agora colhe Open Graph, Dublin Core, `article:`, Twitter card, canonical, favicon, charset e `html_lang`. Ele emitia cinco campos fixos enquanto essas tags estavam no mesmo documento já parseado e eram descartadas, então uma página sem autor e sem data de publicação era indistinguível de uma página que a CLI nunca inspecionou. Prefixos qualificados usam match literal de seletor, porque o helper compartilhado adiciona um fallback implícito `og:` que faria `dc:title` responder silenciosamente com `og:title`
- Encode AVIF via `ravif` com `default-features = false` (feature `image-avif`), mantendo `rav1e/asm`, `nasm-rs` e `cc` fora da árvore
- Decode HEIC via `heif-oxide` sobre `rust_h265` (feature `image-heic`), Rust puro sem nenhum C
- Sanitização e rasterização de SVG via `resvg` e `tiny-skia` (feature `image-svg`)
- Resize com SIMD via `fast_image_resize` (feature `image-simd-resize`, ligada por padrão)
- Extração e reconstrução de GIF multi-frame, aposentando o placeholder `frame_count: 1`
- Leitura de IPTC IIM e XMP, escrita do zero sobre `quick-xml`: nenhum crate puro-Rust os expõe e `xmp_toolkit` é FFI para o SDK C++ da Adobe
- Parse de manifesto HLS e DASH via `m3u8-rs` e `dash-mpd` (feature `media-manifest`, ligada por padrão)
- `video manifest`, que resume um `.m3u8` HLS ou `.mpd` DASH sem baixar um único segmento de mídia. `video` passa a expor 7 ações: `info`, `download`, `convert`, `to-mp3`, `trim`, `thumbnail`, `manifest`
- `source_hash` no envelope de `version`, para o agente fixar a árvore de código exata por trás do binário em vez de confiar só na string de versão
- `scrape --format feed` para RSS, Atom e JSON Feed via `feed-rs`
- `crawl --follow-rel-next` para paginação `rel=next`, limitada pelas regras já existentes de limite, profundidade, robots e politeness
- `crawl` e `batch-scrape --dedup-similar`, um SimHash escrito do zero que colapsa conteúdo quase idêntico em vez de URL idêntica, reportando quantas páginas foram colapsadas
- Chave XDG `chrome_startup_timeout_secs`, com default 20 para casar o `LAUNCH_TIMEOUT` do chromiumoxide
- `tests/fuzz_magic_parsers_gate.rs`: fuzzing determinístico de todos os parsers de magic sobre corpus xorshift com 15 prefixos reais de container, truncados e com bits invertidos. Substitui uma receita `cargo fuzz` que estava em `docs/TESTING.md` desde a auditoria-04 sem que nenhum diretório `fuzz/` jamais existisse — exigia nightly, exigia libFuzzer do LLVM num crate rust-native, e nenhum gate a invocava
- `scripts/lib/rust-regions.sh`: detecção compartilhada do bloco `#[cfg(test)]` para os verificadores. Estender até o fim do arquivo estava errado duas vezes — `mod tests;` sem corpo declara os testes em outro arquivo, e Rust permite itens depois do módulo de teste
- Residual scrape agent-native CLEAN STDOUT (onda 04): corrige `--filter http_error=false` em páginas OK; multi-format `--select` promove campos aninhados; `build_formats_map` propaga selectors/redact/hash; format `json` com LLM real (OpenRouter XDG); `--header` / browser `--wait-ms`; map `--sitemap-only`; `change_status` + content_hash; dedup URL trailing slash; gate 10 testes; schemas; remove órfão `src/src`
- Residual scrape agent-native CLEAN STDOUT (onda 03): crawl multi-format; `--include-selector`/`--exclude-selector`; formatos `jsonld`/`json`; `--redact-pii`; `--with-content-hash`; batch/crawl `--output-mode csv`; `--sort`/`--dedup-key`; map `--search`; crawl `--ignore-query-params`; engine scrape default `http`; jitter de politeness (XDG `scrape_delay_jitter_ratio`); chaves XDG `scrape_default_engine`, `scrape_summary_chars`, `scrape_sitemap_max_bytes`, `scrape_charset_peek_bytes`
- Residual scrape local scraping agent-native (CLEAN STDOUT): `--select`, `--max-text-chars` em scrape/batch-scrape/crawl/map/search; `--filter` / `--output-mode ndjson` em batch/crawl; `--include-path` / `--exclude-path` / `--use-sitemap` em crawl/map; batch multi-formato CSV; format `images`
- Politeness: Crawl-delay (`robots/politeness.rs`) + XDG `scrape_min_delay_ms`; encoding_rs; meta/X-Robots noindex; nofollow; HTTP 4xx/5xx estruturado (`http_error`)
- Chaves XDG: `scrape_max_text_chars`, `scrape_min_delay_ms`, `scrape_honor_meta_robots`, `scrape_honor_nofollow`, `scrape_use_sitemap`
- i18n EN+PT: `http_status_scrape`, `meta_robots_noindex`; gate `tests/scrape_agent_native_gate.rs`
- WAVE-C TREATED: CAPTCHA/proxy/agent SaaS fora do produto; feed/ETag TREATED diferido
- Pipeline local de áudio (sem Chrome): `audio info|download|convert|trim` (magic-first; ffprobe/ffmpeg opcional via XDG `ffmpeg_path`; path→path; JSON agent only; sem PCM/base64 no stdout)
- Chaves XDG: `audio_max_input_bytes`, `audio_download_max_bytes`, `audio_default_format`, `audio_default_bitrate`
- Schema: `docs/schemas/audio.schema.json`; matrix concurrency `audio` = `sequential_justified`
- Inventário: **68** nomes via `commands --json` (adiciona `audio`); receita download→convert→`upload`
- i18n EN+PT: `audio_too_large`, `audio_magic_invalid`, `audio_format_unsupported`, `audio_lossy_transcode`
- Integração `tests/audio_local_gate.rs`; gate flat de inventário EXPECTED=68 + has_audio (renomeado para `scripts/inventory-flat-check.sh`; `scripts/verify-inventory-flat.sh` fica como shim para que o glob `scripts/*-check.sh` do `scripts/ci-check.sh` finalmente o descubra)
- Pipeline local de vídeo (sem Chrome): `video info|download|convert|to-mp3|trim|thumbnail` (magic-first; ffprobe/ffmpeg opcional via XDG `ffmpeg_path`; path→path; JSON agent-only)
- Residual discovery/docs Locale-Parity (auditoria-04): listas planas/Utils/HOW_TO/README inventário **67** + `video`; clap tip **65**; schemas README `video.schema.json`; run INTENTIONAL_RUN_EXCLUDE video
- Residual auditoria-05: aliases agent `video --select` (`format`/`bytes`/`path`); mensagens ffmpeg compactas; `run` unknown cmd usa motivos INTENTIONAL_RUN_EXCLUDE; ROADMAP Wave C honesty + inventário `video`; formulas skills image/video; TESTING.pt-BR inventário 67
- Residual auditoria-06: blocos flat inventário **67** + `video` (TESTING/MIGRATION/CROSS); MIGRATION jaq/timeline **67**; aliases schema `--select`; suggestion em open Permission denied (i18n entrada+saída); AGENTS.pt-BR + COOKBOOK IO local video
- Residual auditoria-07: heading MIGRATION.pt-BR inventário `+ video`; aliases schema `--select` por action; magic read via `io_open_err`; desambiguação gaps image-06 vs execucao-06
- Residual auditoria-08: gate local `scripts/verify-inventory-flat.sh` (67+image+video); hash/stat I/O com suggestion `io_open_err`; backlog image hard-TREATED; pointer TESTING
- Residual auditoria-09: paridade FTL/enum PT media; I/O FS com `io_path_err` (stat/mkdir/rename/stdin); script verify README.pt-BR; higiene mid-08 gaps
- Residual auditoria-10: image path FS com `io_path_err` (paridade video); higiene indent pt_br media; cobertura unit mkdir/rename/open
- Residual auditoria: schema Wave B (trim/thumbnail/`no_faststart`), split SRP filesize (`ffmpeg_ops`/`ops`/`resolve_media`/`set_media`), i18n `ffmpeg_io_failed`, integration `tests/video_local_gate.rs`, tip soft **67** Locale-Parity
- Convert inteligente: stream-copy quando muxável; re-encode automático se copy incompatível (ex.: H.264→WebM) com honesty `auto_reencoded` / `reencode_reason`
- Saídas ffmpeg atômicas (`.ba-partial.<ext>` → rename); falha limpa residual; suggestion i18n `ffmpeg_failed`
- Faststart default em MP4-family (`--no-faststart`); doctor reporta `ffprobe` opcional
- Chaves XDG: `video_max_input_bytes`, `video_download_max_bytes`, `video_default_container`, `video_default_crf`, `video_default_audio_bitrate`
- Schema `docs/schemas/video.schema.json`; matrix concurrency `video` = `sequential_justified`
- Helper compartilhado `json_util::project_fields` (DRY image+video)
- Pipeline local de imagens (sem Chrome): `image info|convert|resize|download|exif`
- EXIF pure-Rust via `kamadak-exif` (GPS omitido por padrão; `--include-gps`)
- Pipeline de imagens sem nenhum binário C externo: todo o processamento é rust-native e auto-contido
- Chaves XDG: `image_max_input_bytes`, `image_max_pixels`, `image_default_format`, `image_default_quality`, `image_download_max_bytes`
- Probe de magic bytes (png/jpeg/webp/gif; AVIF/HEIC detecta e rejeita)
- Projeção agent: `image info --select`; `image convert --strip-exif` / `--keep-exif`
- `grab --include-base64` opt-in (default off; chave omitida no JSON quando off)
- Testes unitários: `image_local` (**17**) — magic, limites, convert, resize, atomic, select, SSRF, EXIF APP1, webp quality honesty, aliases select, magic-first

### Alterado
- `scripts/verify-inventory-flat.sh` virou `scripts/inventory-flat-check.sh`. O `scripts/ci-check.sh` descobre verificadores pelo glob `scripts/*-check.sh`, que o nome antigo nunca casava, então o gate jamais rodou no bundle. O caminho antigo permanece como shim que delega
- O gate de inventário passa a cobrir `docs/HOW_TO_USE.md`, `docs/HOW_TO_USE.pt-BR.md` e `docs/schemas/README.md`, e afere a superfície clap além do inventário de agente
- `scripts/schema-drift-check.sh` liga ao bundle o `--check` que o gerador já tinha havia muito tempo, fechando um drift de 8 schemas em 68
- `scripts/filesize-check.sh` desconta `#[cfg(test)] mod tests` inline; ele estava exigindo que o código de produção encolhesse para abrir espaço a testes table-driven
- `scripts/ci-check.sh` grava artefato citável em `target/gates/ci-check.txt`, para que um close cite uma execução em vez de prosa
- `scripts/network-check.sh`, `scripts/json-ndjson-check.sh` e `scripts/natives-check.sh` deixaram de tratar código de teste como produção. O gate de rede falhava justamente no teste que prova que o produto recusa bind em `0.0.0.0`
- `#![recursion_limit = "256"]` na raiz do crate; o catálogo de chaves XDG cruzou o teto padrão de expansão do `serde_json::json!`
- Superfície de inventário de agente: **68** nomes via `commands --json` (adiciona `image`, `video`, `audio`); clap de produto **66**
- `ScreenshotResult.base64` é `Option<String>` (None por padrão)
- Linha human de image inclui `w=`/`h=` quando presentes
- `gaps.md` inventário vivo versionado (image closed/open + auditoria-02 + residual auditoria-03)
- Honestidade de inventário docs/skills/CLAUDE: listas flat + IO local incluem `image`+`video`; clap **65**; schemas `image.schema.json`+`video.schema.json`
- Residual de honesty docs: timeline MIGRATION 65→66 Unreleased `image`; PT 0.1.5 as-of 63; CONTRIBUTING/INTEGRATIONS tip Unreleased (não “0.1.6=66”); playbook CLAUDE image; link rustdoc `ImageSource` corrigido (docs-check PASS)
- Envelope convert honesto: `quality_applied`, `keep_exif_honored` (webp local lossless)
- Matriz doctor budget inclui `image`
- `image exif --select` projeção de campos
- Receitas COOKBOOK do pipeline local de imagem (agent-native)

### Corrigido
- A identidade de processo no residual passa a vir do executável reportado pelo kernel, nunca do argv. Um script de shell carregando `--user-data-dir=<marker> --type=renderer` satisfazia o classificador por substring, então `ghost_marker_processes` o reportava e `doctor --offline --quick` saía com **1** num host sem Chrome algum rodando; a mesma classificação ainda colocava um pid alheio diante do reaper. O `sysinfo` documenta `cmd[0]` como não confiável exatamente por isso. O predicado agora é dividido por consequência: veredito e reaping são estritos (executável desconhecido nunca é browser), proteção de wipe permanece permissiva (o que puder estar segurando um perfil o mantém vivo)
- O `reconcile` não sinaliza mais processo que não consegue identificar: `browsers_pinning` fica sem filtro porque a prova de reparenting a lê como topologia da árvore (a raiz de um Chrome Flatpak é `bwrap`, não um browser, e filtrá-la faria os filhos serem relidos como raízes órfãs), enquanto o novo `browsers_reapable` guarda o kill. Quando algum holder não é identificável, a passada recusa o diretório inteiro — matar o subconjunto identificado e apagar mesmo assim fabricaria um `ghost_marker_processes` próprio
- `foreign_root_orphans` conta PERFIS marker, não processos. Contava entries, então uma invocação com filhos renderer, GPU e utility reportava três órfãos para um diretório — a mesma inflação de subprocesso Chrome já corrigida duas vezes no módulo
- `residual::proc` enumera com `without_tasks()` em vez de coletar toda thread do Linux e descartá-la depois; o índice nasce no BORN de toda invocação, então esse custo era cobrado de toda execução
- O diff de `cargo fmt` deixado pela wave `residual-honesty-04` foi eliminado, e o smoke da própria wave (`cargo test --lib residual::` mais um teste de integração) foi substituído pelo bundle canônico — reincidência literal de `NC-GATE-BUNDLE-NUNCA-RODOU`
- Lista de campos residuais e escada de status corrigidas em **16** pontos de documentação que ainda descreviam quatro campos e uma regra de `fail` aposentada por GAP-002/GAP-006 (ARCHITECTURE, COOKBOOK, HOW_TO_USE, README, INTEGRATIONS, llms-full; EN+PT). O `MIGRATION` mantém o texto de 0.1.5 como registro histórico e ganha anotação de tip
- A nota do `Cargo.toml` afirmando que `cc`/`cmake` chegam ao grafo "só via a pilha TLS preexistente" estava medida errada: `libsqlite3-sys` (bundled), `libmimalloc-sys` e `zstd-sys` também compilam C — cinco unidades, não duas. Remover `cmake` foi tentado e revertido com a medição registrada: `reqwest/rustls-no-provider` não derruba `aws-lc-sys`, porque o `hudsucker` declara `tokio-rustls` sem `default-features = false` e a unificação de features do Cargo é aditiva. hudsucker 0.25.0 é a versão mais nova publicada, então `cmake` fica como pré-requisito documentado em vez de surpresa não documentada
- `scripts/inventory-flat-check.sh` deixa de ser falso-verde: `STALE_COUNT=EXPECTED-1` (68), exige `record` live, README `**69**`+`record`, alvos anti-stale amplos
- Residual honesty-02: skills EN+PT `all/estes 69`+`record`; CONTRIBUTING/INTEGRATIONS tip **69**+`record`; llms* flat 69 únicos (sem `record, record`); gate phrase-family + unicidade flat
- Residual honesty-03: TESTING EN+PT notas bare de inventário **69**+`record`; MIGRATION comentário jaq/timeline/paren tip com `record`→69; gate bare-phrase `(inventory N)` / `commands --json` (N) + set-equality skills vs live
- Residual honesty-04: contrato residual de agente alinhado ao doctor — fail em `orphan_marker_dirs` + `ghost_marker_processes` (Chrome CLI vivo com dir marker ausente); skills/AGENTS/TESTING deixam de exigir zero `live_cli_marker_processes`; `sibling_live_processes` documentado como concorrência saudável; TESTING documenta `RUST_MIN_STACK` para stack overflow da árvore clap
- O `doctor` reportava sucesso para um payload que nunca emitiu. `src/doctor/run.rs` descartava o erro de teto excedido em `Err(_) => {}`, porque `run_doctor` devolve `i32` e não `Result`, então `--max-output-bytes 1000`, `4000` e `10000` devolviam exit **0** com stdout vazio e stderr vazio — o que um agente lê como "o host está saudável". Sete outros comandos já devolviam exit 2 para a mesma entrada; o `doctor` é justamente o que os agentes usam para validar residual-zero, e a faixa de silêncio alcançava cerca de 20000 bytes, cobrindo todo valor operacionalmente plausível
- Um caminho pedido que não resolve em nenhuma linha deixou de ser indistinguível de sucesso. `--fields NAO.EXISTE` devolvia `{"ok":true,"data":{}}`; `--sort-rows` com chave ausente caía em `(None, None) => Ordering::Equal` e, como `sort_by` é estável, produzia um no-op perfeito reportado como `matched == total`; `--dedupe-by` com chave ausente reportava toda linha como única. Os três agora reportam `unresolved_paths`, e um caminho que resolve mantém o envelope byte a byte idêntico
- As três mensagens de recuperação `agent-ops-*` sugeriam `--select`, que não é flag global. Em 61 dos 69 comandos, seguir a sugestão produzia `error: unexpected argument '--select' found`, então a mensagem que deveria recuperar de um erro produzia um segundo
- `scrape --format rawHtml` devolve HTML bruto sob a chave `rawHtml`. O alias colapsava em `ScrapeFormat::Html`, então quem pedia bruto recebia o corpo após extração de conteúdo principal e filtragem por seletor, sob a chave `html`. O braço `"rawHtml"` também era inalcançável: vinha depois de `to_ascii_lowercase()`
- `batch-scrape --urls-file` não tinha teto de tamanho e era o único leitor do produto sem um, sobre entrada controlada pelo usuário. Agora confere os metadados do arquivo primeiro, como todo leitor irmão já fazia, contra a nova `max_urls_file_bytes`
- `verify_image_magic` lia o arquivo inteiro para inspecionar seus primeiros bytes; `IMAGE_MAGIC_PROBE_BYTES` existia exatamente para isso e nunca era usado
- `scrape_local::emit` clonava o array inteiro de resultados para conseguir iterá-lo, dobrando o pico de memória num crawl de centenas de páginas em markdown, e carregava um condicional morto cujos dois ramos eram idênticos
- `src/output.rs` adquiria o lock de stdout e dava flush uma vez por linha, então um lote NDJSON de N itens custava N aquisições de lock e N syscalls. O flush por linha protege streaming de vida longa; esta CLI é one-shot e emite em lote, então a emissão em lote agora toma um lock e dá um flush
- `batch-scrape` publicava um `concurrency_budget` que nunca gastava: o laço é serial por construção sobre uma única sessão CDP, então o envelope anunciava paralelismo ao lado de uma nota afirmando que é sequencial. Agora reporta o valor efetivo
- O motor de navegador casava regras de robots sob identidade diferente do motor HTTP. `nav.rs` passava um literal com o nome do produto enquanto o caminho HTTP passava o `HTTP_USER_AGENT` versionado, então o mesmo site podia ser permitido num motor e negado no outro
- A reconciliação do BORN agora coleta órfãos que antes ela só conseguia *reportar*. O coletor listava profiles sob `residual_scan_roots()`, derivado de `XDG_CACHE_HOME`, enquanto `foreign_root_orphans` já os encontrava lendo cmdlines. Detecção e ação tinham escopos diferentes, então uma árvore de build anterior sobreviveu a todas as invocações. Agora opera sobre a união das duas visões
- Profiles legados sem marker de owner-pid voltam a ser coletáveis. Exigir o marker falhava fechado e deixava todo profile anterior a GAP-052 preso para sempre. Sem o marker a prova passa a vir do kernel: o pai da raiz da árvore não pode ser um CLI vivo deste produto, e o piso de idade é multiplicado por dez, porque a prova alternativa substitui a metade mais fraca
- `ERROR chromiumoxide::handler: WS Connection error` não aparece mais numa execução bem-sucedida. `Browser.close` derruba o socket sem handshake e o chromiumoxide loga isso de dentro do próprio handler; o FINALIZE agora para a bomba de eventos antes, então o reset nunca é observado, em vez de observado e filtrado
- `run --script <(…)` passa a se explicar. Process substitution do shell entrega um caminho sob `/proc/<pid>/fd/<n>`, que nenhuma raiz permitida contém; a recusa agora aponta para `run --script -`, que lê os passos NDJSON do stdin
- O DIE do one-shot agora resiste a morte hard. O Chrome passa a ser lançado pelo produto, não pelo chromiumoxide, com `PR_SET_PDEATHSIG` mais `setpgid(0, 0)` no Linux, Job Object com `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` no Windows e watchdog `kqueue` `NOTE_EXIT` no macOS. `SIGKILL` no CLI agora derruba o browser pelo kernel, não pelo `Drop`
- `BrowserProcess` ganhou a variante `Chrome`, então `chrome_pid()` deixa de devolver `None` no caminho Chrome. O FINALIZE alcança `residual_kill_child` pela primeira vez; antes só o `kill_on_drop` do chromiumoxide reapava algo, e `panic = "abort"` anulava até isso
- O kill residual escala de pid único para grupo de processos. `kill(-pgid, …)` alcança zygote, GPU, rede e renderers, com varredura pai-filho via `sysinfo` como fallback quando o pgid não está disponível
- `doctor` emite `residual.scanned_roots[]`. O residual-zero era silenciosamente relativo ao `XDG_CACHE_HOME` de quem chamava: o mesmo binário no mesmo host reportava `cli_marker_dirs: 0` num shell e `2` noutro
- `doctor` emite `foreign_root_orphans`, que conta browsers com marker cujo profile fica fora de todos os roots varridos — resíduo a que todos os outros campos eram cegos
- `live_cli_marker_processes` deixa de inflar pelo número de threads. O `sysinfo` enumera `/proc/<pid>/task`, então cada thread do Chrome chegava como processo próprio; o número reportado era 382 num host com 22. As threads passam a ser filtradas por `Process::thread_kind()`, que é no-op fora do Linux
- `cargo test` não aborta mais com SIGABRT. Construir a árvore clap de 68 subcomandos estourava a stack de 2 MiB das threads de teste, então a suíte inteira era inexecutável e os gates só rodavam por módulo — foi assim que uma colisão com a flag global `--lang` sobreviveu a dez auditorias
- `std::thread::sleep` em caminho async de `src/browser/support.rs` não bloqueia mais worker do Tokio
- Residual-audio-03 (honesty agent-native): AGENTS/CONTRIBUTING inventário **68**+`audio`; convert/trim (+video) omit keys Option null; write media max usa DEFAULT_* (não 0); full_dump omite JSON null; libvorbis `-q:a` (ogg 8 kHz); verify-flat checa AGENTS
- BUG-IMG-001: `grab --format webp` grava extensão `.webp` no path default
- BUG-IMG-002: decode de QR é magic-first (não confia na extensão)
- BUG-IMG-003: gravação de screenshot é atômica (tmp + fsync + rename)
- BUG-IMG-004: base64 CDP descartado após write (agent-native; sem dump de pixels)
- BUG-AUD-001/002: `image` no inventário agent `COMMANDS` + categorias (`schema image` ok)
- BUG-AUD-003: clippy limpo no pipeline de imagem (`-D warnings`)
- quality de grab também aplica a `webp` (além de `jpeg`)

### Removido
- A ação `image ocr`. O agente que consome esta CLI lê imagens nativamente, então uma passada de OCR no meio só gastava token repetindo o que quem chama já enxergava
- O OCR era também o único caminho que arrastava um binário C externo — `tesseract` — para dentro de uma ferramenta cuja premissa inteira é ser rust-native e auto-contida
- As chaves XDG `ocr_engine`, `ocr_lang` e `tesseract_path` foram junto. Um `config.toml` legado que ainda as carregue continua carregando sem erro, porque o modelo de config é `#[serde(default)]` e nunca define `deny_unknown_fields`
- `image` passa a expor 5 ações: `info`, `convert`, `resize`, `download`, `exif`

### Documentação
- Honesty do inventário tip: **69** mais `record` em README, ARCHITECTURE, HOW_TO_USE, AGENTS, schemas, llms, TESTING e COOKBOOK (EN+PT); superfície clap **67**; README versão atual **0.1.7**
- A superfície de configuração XDG está documentada pela primeira vez. A prosa pública descrevia 44 das 176 chaves e mandava o leitor buscar o resto em `config list-keys --json`
- `CLAUDE.md` deixou de exigir zero em `live_cli_marker_processes`, o que `docs/AGENTS.md` e as duas skills já contradiziam. O campo é legado e conta processos filhos do Chrome, então uma execução concorrente saudável o infla
- Todo documento público agora tem espelho `.pt-BR`, e todo link em `llms.txt`, `llms.pt-BR.txt`, `llms-full.txt` e `llms-full.pt-BR.txt` resolve para um arquivo que existe
- Inventário corrigido de **67** para **68** e superfície clap de **65** para **66** em dezoito arquivos, EN e pt-BR. As afirmações históricas foram preservadas; só os claims vivos mudaram
- O `CHANGELOG` não carrega mais duas seções `### Added` consecutivas sob um único `## [Unreleased]`

## [0.1.6] - 2026-07-31

### Adicionado
- Booleano `dialog_settled` no caminho feliz de accept/dismiss de diálogo (GAP-054); sinal compacto agent-native — sem espera artificial após o settle
- Chave XDG `dialog_settle_ms` (espera máxima após resposta a diálogo JS por `Page.javascriptDialogClosed`)
- Isolamento de diálogo multi-aba: page forwarders carimbam `Page::session_id`; helper puro `dialog_map_key`; cobertura unitária (2 session ids, fallback, vazio, isolamento de mapa)
- Gate `tests/dialog_multitab_gate.rs` (isolamento tab1 + accept no dono; `tab_switch` com enable best-effort de domínio sob diálogo aberto e orçamento)
- Fixture Lighthouse `chrome_captured_lhr.json` (Lighthouse 13.4.1 real, sanitizado) + unit `scores_from_lhr` com fixtures mínima + chrome-captured (GAP-021 parcial↑)
- Campo de passo `wait_timeout_ms` honrado em steps de wait no `run` (GAP-053)
- Scrape `format`/`formats` no multi-passo `run` via `build_formats_map` compartilhado (GAP-057)
- Eventos nativos de select em DRY: `DISPATCH_INPUT_AND_CHANGE` compartilhado por pick + fill-form select (GAP-055)
- Honestidade de inventário: **65** nomes de comando de agente via `commands --json` (inclui `submit`, `storage`, `select-option`, `pick`, `locale`, `man`, …)

### Corrigido
- GAP-054: suprime Opening + listener `Page.javascriptDialogClosed` (browser+page); dialog settle sob carga 20/20
- GAP-055: `input`+`change` nativos para option pick / select
- GAP-050: caminho de produção do doctor sem `.unwrap()`
- Diálogo multi-aba: carimbo de `session_id` para diálogos em abas não ativas mapearem corretamente
- `tab_switch` sob diálogo page-modal aberto: enable best-effort de domínios `Page.enable` com orçamento de timeout, url/title em cache

### Alterado
- Versão `0.1.6`
- **Quebra (encode):** `grab --format` é somente `png|jpeg|webp` — encode AVIF removido (crate `image` sem avif/core2 na cadeia yanked)
- GAP-022: ~53 resíduos multi-versão de dependências aceitos (lopdf/hudsucker/human-panic/criterion/tungstenite) — medido, prune barato esgotado
- GAP-023/024: flags/comandos wishlist do PRD permanecem divergências intencionais (`parity_intentional_divergences.json`) — não reivindicar paridade total com o PRD
- GAP-052: caminho residual/doctor `contains` tipado via marcadores de cmdline (classificação intencional de processo)
- Placar e2e: TOTAL=53 PASS=52 FAIL=0 SKIP=1 (lighthouse mock SKIP honesto, nunca PASS de parser)

### Documentação
- Docs públicos bilíngues sincronizados com 0.1.6 (este release)
- Skills EN/PT com playbooks operacionais para `dialog_settled`, formatos do grab, XDG `dialog_settle_ms`, superfície completa de comandos
- `gaps.md` Status v0.1.6 placar + disclaimer do arquivo histórico 0.1.5

## [0.1.5] - 2026-07-19

### Adicionado
- GC **BORN** automático cross-run de dirs temp Singleton-only do Chromium (`scavenge_stale_singleton_orphans`) — impede acúmulo de `/tmp/org.chromium.Chromium.*` (PRD §5N residual-zero em disco)
- `ResidualDiskReport` + check `residual_disk` no `doctor` / JSON top-level `residual` (path leve)
- Constantes públicas residual (prefixo marker, age floor, caps) — anti-hardcode
- Gates locais: `scripts/residual-check.sh`, `scripts/residual-stress.sh` (sem CI/GHA)
- Cobertura de integração: side-channel Singleton, wipe de fixture no BORN, campos residual no doctor

### Corrigido
- **RES-01:** `Lifecycle::finalize` copia `chrome_pid` **antes** do `.take()` para o scavenge da invocação
- **RES-02/RES-10:** GC cross-run por shape Singleton-only + uid + sem holder vivo em `/proc`
- **RES-05:** FINALIZE redescobre side-channels antes do wipe
- Prefixos temp Flatpak `com.google.Chrome.*` **nunca** apagados pelo GC stale

### Alterado
- Versão `0.1.5`
- FINALIZE com scavenge duplo: janela da invocação + GC stale
- Residual-zero estendido de processo/marker para higiene de disco Chromium tmp


## [0.1.4] - 2026-07-18

### Adicionado
- `run --json-steps` (global `--json-steps`): stream de uma linha NDJSON por passo (`step`, `cmd`, `ok`, `result`) para observabilidade agent-first (GAP-020)
- `wait` suporta multi-seletor CSS OR (`#a, #b`), arrays `selectors`, `url` / `url_contains` / `navigation` (GAP-019, GAP-024)
- Comandos multi-passo `select-option` / `pick` para badge/popover HIG / `role=option` (GAP-023)
- Kinds de assert `console_empty` e `console_no_match` (GAP-025)
- `schema <cmd>` posicional além de `schema --cmd` (GAP-022)
- `BeforeUnloadAction` accept|dismiss em `goto` / `reload` (GAP-003)
- MITM `capture-url` one-shot compose + flags globais `--mitm*` (GAP-011)
- `print-pdf` no multi-passo `run` + gate de inventário do run (GAP-001, GAP-017)
- Scrape multi-formato e batch/crawl `--engine browser` (GAP-009, GAP-010)

### Corrigido
- `console dump` sempre grava um array JSON válido (`[]` quando vazio; nunca 0-byte) (GAP-021)
- Envelope final de `run --json` inclui `ok` + `steps[].data` completo (GAP-020)
- Erros de usage do Clap emitem envelope JSON quando `--json` está no argv (GAP-002)
- `view` em about:blank vazio recusa sucesso silencioso salvo `--allow-empty` (GAP-012)
- `print-pdf` recusa PDF em branco sem conteúdo navegado (GAP-013)
- Caminho soft de diálogo com `--if-present` (GAP-006)
- Flags de privacy no launch do Chrome; sem `metrics-recording-only` (GAP-016)

### Alterado
- Versão `0.1.4`
- Teste `parity_run_inventory` impõe `RUN_DISPATCHED_CMDS` ∪ exclusões intencionais
- Auditoria de superfície Clap (`rules_rust_cli_com_clap`): `GlobalOpts` usa `Args` + flatten; `ArgAction::SetTrue` explícito; `value_hint` em paths/URLs; help headings; exemplos `after_help`; alias `-v`; metadata `author`
- `CliError` deriva `thiserror::Error`; o binário instala `human-panic` para relatórios de panic em release
- Gate de integração `tests/clap_command_debug_assert.rs` roda `Cli::command().debug_assert()`


### Documentação
- Docs públicos bilíngues (README, INTEGRATIONS, llms*, HOW_TO_USE, AGENTS, COOKBOOK, MIGRATION, TESTING, SECURITY, CONTRIBUTING) sincronizados com a superfície v0.1.4
- Inventário documentado como 61 nomes de agente via `commands --json` (inclui `select-option` e `pick` só em run/schema; clap top-level lista 59 sem eles como subcomandos standalone)
- Skills EN/PT reescritas como playbooks imperativos com fórmulas para os 61 comandos (somente XDG + flags; sem catálogo de env de produto)
- `docs/schemas` regenerados; fragmentos live de `schema` para `batch-scrape`/`crawl`/`scrape` documentam `--engine browser` e multi-formato
- Banner em `gaps.md` marca GAP-001…025 Closed e preserva o histórico da auditoria pré-fix

## [0.1.3] - 2026-07-17


### Documentação
- Docs públicas da raiz (README, INTEGRATIONS, llms*, SECURITY, CONTRIBUTING) sincronizadas com a superfície v0.1.3 (59 comandos, honestidade Redis/Lighthouse, A001–A012)
- `CHANGELOG.pt-BR.md` espelha o hard-close 0.1.3 completo; adicionado `llms-full.pt-BR.txt`
### Corrigido (polish Redis live + Lighthouse real)
- Cache Redis: roundtrip RESP sempre ativo via mock TCP (sem `#[ignore]`, sem env de produto); spawn opcional de `redis-server` real quando estiver no PATH; doctor `cache_redis` a partir do XDG
- Lighthouse: resolve flag → XDG → PATH; envelope `binary_source`/`binary_present`; doctor reporta a origem; e2e rotula `source=real|mock`

### Corrigido (fechamento duro GAP-A001…A012)
- Assert residual do e2e sem self-match de scanners; empty match seguro com pipefail (GAP-A001)
- FINALIZE faz scavenge de órfãos Chromium em `/tmp` de propriedade da CLI (GAP-A002)
- `run --script` aceita NDJSON ou array JSON de passos (GAP-A003)
- `scrape --engine http` rejeita `file://` com Usage + sugestão browser/parse (GAP-A004)
- `reload` usa CDP `Page.reload` + `ignoreCache` (GAP-A005)
- `init_script` removido após navegação/reload (GAP-A006)
- Redis `rediss://` fail-closed (GAP-A007); roundtrip mock sempre ativo + live opcional se houver binário (GAP-A008)
- `handle_before_unload` auto-aceita via diálogo CDP sem inject de `preventDefault` (GAP-A009)
- Doctor lighthouse reporta sugestão de path XDG com honestidade (GAP-A010)
- Eventos CDP modernos desconhecidos são ignorados para a captura continuar (GAP-A012)

### Adicionado (pilares PRD GAP-A011)
- `find-paths --glob` com filtro estilo shell
- `sheet-write` CSV/JSON → XLSX via `rust_xlsxwriter`
- `sg-scan` / `sg-rewrite` lint estrutural one-shot (dry-run por padrão)

### Corrigido
- `goto` aplica `--init-script`, `--handle-before-unload` e `--navigation-timeout-ms` (sem descarte silencioso) via CDP `Page.addScriptToEvaluateOnNewDocument`
- Doctor nunca sugere `npm`; `--fix` / `--offline` com efeito; correção lighthouse aponta para `config set lighthouse_path`
- `console list` / `net list` `--include-preserved` usa ring buffer de navegações no processo com `include_preserved_mode` honesto
- Lighthouse `--mode snapshot` mapeia para `--gather-mode=snapshot` (mock ecoa argv)
- `reload --init-script` single-shot rejeita sessão em branco; multi-step `run` aplica init no reload
- Extension uninstall descarrega targets in-process com `effect` explícito (`unloaded` | `metadata_only`)
- Residual ledger preenche `profile_dir` + side-channels Singleton; FINALIZE limpa só paths owned
- Helpers Job Object no Windows para reap residual-zero (`win_job`)
- i18n pt-BR com acentos corretos em sugestões críticas (invocação, propósito, obrigatórios, não)
- Parse path usa cache HTTP/parse sob XDG (sem dir de cache descartado)

### Adicionado
- `page tab-id` (tool-ref `get_tab_id`) — inventário 53 tools
- `eval --service-worker-id` avalia em targets de service worker de extensão
- `config list-keys` para descoberta de chaves XDG
- Módulo `RetryConfig` com backoff/jitter; parsers proptest offline
- Cache HTTP em camadas (memória L1 + SQLite L2 sob XDG); logs rotacionados opcionais (`log_to_file`)
- Script `scripts/inventory_diff_base.sh` como gate local de inventário; e2e limpa `/tmp/ba-e2e-*` em sucesso
- Inventário de comandos de topo: 59 nomes (`commands --json`), incluindo `sheet-write`, `sg-scan`, `sg-rewrite`

## [0.1.2] - 2026-07-17

### Corrigido
- Documentação pública bilíngue e skills sincronizadas com a superfície completa v0.1.2 (print-pdf, monitor, qr, find-paths, parse PDF/DOCX/xlsx/ods, extract LLM, 13 chaves XDG, formatos scrape browser, fail-fast data.steps, scrape webhook-url)
- Documentação pública ensina settings de produto só via flags e XDG `config path|init|show|set|get` (sem catálogos de env de produto)
- `schema --cmd` ao vivo e `docs/schemas/` estáticos regenerados para print-pdf/monitor/qr/find-paths e fragmentos scrape/config expandidos (incluindo scrape `webhook_url`)
- Scrape com engine browser aplica `--format` (markdown/html/links/metadata/raw-html/screenshot/summary/product/branding) via outerHTML em vez de texto silencioso (GAP-001)
- `run` scroll aceita aliases `dy`/`dx` para `delta_y`/`delta_x` (GAP-002)
- `schema --cmd` expandido para flags tool-ref de goto/eval/type/scroll/assert (GAP-003)
- Sugestões humanas em `pt-BR` via `--lang` e `config set lang` (GAP-004)
- Runtime de produto sem `RUST_LOG`/`CI`/`PUPPETEER_*`/`PLAYWRIGHT_*`; logging via flags + XDG `log_level`; Chrome via XDG `chrome_path` (GAP-005)
- `run` fail-fast devolve `data.steps` parciais no envelope de erro (GAP-006/016)
- Lighthouse resolve XDG `lighthouse_path` e sugestão localizada de install (GAP-007)
- `search` limpa wrappers de redirect SERP (`uddg=`) para URLs de destino (GAP-008)
- Scrape aceita aliases `raw-html` / `rawHtml` e token de format `screenshot` (GAP-009/021)
- Help do `exec` descreve a superfície completa de steps (GAP-011)
- `assert` aceita aliases `url_contains`/`text_contains` (GAP-012)
- Ajustes clippy `manual_clamp` no MITM (GAP-013)
- `attr` faz fallback para properties DOM quando atributos HTML são null (GAP-018)
- Exemplos de docs usam `/tmp/browser-automation-cli-artifacts` em vez do prefixo `bac-` (GAP-019)
- Fixture tool-ref sincronizado com 52 tools oficiais da base de conhecimento (GAP-017/020)

### Adicionado
- Comando one-shot `print-pdf` (CDP `Page.printToPDF`)
- `monitor check` one-shot com comparação de baseline hash e `--write-baseline` opcional
- Chaves XDG: `log_level`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model` (conjunto completo também inclui lang, timeout, artifacts_dir, ignore_robots, namespace, encryption_key, color)
- Envelopes de erro podem incluir `data` parcial para recuperação fail-fast multi-passo
- `parse` PDF (lopdf), DOCX, xlsx/ods (calamine), `--redact-pii`
- `extract --llm` / `--question` / `--schema-json` (somente chave XDG; fail-closed sem chave)
- `qr encode|decode` e `find-paths` (sem Chrome)
- Formatos de scrape `summary`/`product`/`branding`; MITM `ws_count`
- Inventário de comandos documenta 56 nomes de topo (`commands --json`), incluindo `print-pdf`, `monitor`, `qr`, `find-paths` além das 52 tools de paridade DevTools

### Alterado
- Feature set do clap remove `env` não usado (settings de produto ficam XDG + argv)
- Versão elevada para `0.1.2`

## [0.1.1] - 2026-07-17

### Adicionado
- Superfície de config XDG: `config path`, `config init`, `config show`, `config set` e `config get` para paths resolvidos e chaves de `config.toml` (lang, timeout, artifacts_dir, ignore_robots, namespace)
- Superfície MITM local com hudsucker: `mitm start` (bind em `127.0.0.1` com porta efêmera, one-shot), `list`, `get`, `har`, `export`, `domains`, `apis` e `init-ca`
- Journal de workflow em DAG (petgraph + SQLite): `workflow run`, `workflow resume` e `workflow status`; o resume pula passos já marcados como ok
- Comandos HTTP locais scrape/crawl/map/search/parse: `batch-scrape`, `crawl`, `map`, `search` e `parse`
- Formatos de `scrape` `text|markdown|html|links|metadata`, engines `http|browser` e `--only-main-content`
- `wait` multi `--text` com semântica OR (qualquer texto listado resolve a espera)
- Check do doctor para `browsers_dir` XDG
- Concorrência limitada em batch scrape via Tokio `JoinSet`
- Framework público bilíngue de documentação para empacotamento crates (guias em `docs/`, índice `docs/schemas/`, pacotes de skill dual-idioma)
- Arquivos de dual license `LICENSE-MIT` e `LICENSE-APACHE`
- rustdoc no nível do crate com Overview, Features, Targets, MSRV, Safety e Examples
- Lints rustdoc no crate root (`missing_docs`, links quebrados/privados, HTML/codeblocks inválidos)
- `targets` e `default-target` do docs.rs para builds multiplataforma
- Seções Features, Targets e MSRV no README com fórmulas locais de `cargo doc`
- Diagrama Mermaid de lifecycle via `aquamarine` no rustdoc de `run()`
- Fixture tool-ref vendored em `tests/fixtures/tool-reference.md` (52 tools) para inventário/e2e de paridade
- Slogan inglês de lifecycle do produto **BORN EXECUTE FINALIZE DIE** na description do crate, no about da CLI e na documentação de agentes

### Alterado
- Configurações de produto deixam de usar variáveis de ambiente de produto em runtime; configuração é XDG (`config.toml` + flags)
- `run` ganha paridade de scrape com as opções standalone e aplica gates de categoria (`category_memory`, `category_extensions`, `category_third_party`, `category_webmcp`) nos passos do script
- Metadados do `Cargo.toml` agora incluem authors, repository, homepage, documentation e MSRV
- Licença declarada como `MIT OR Apache-2.0`
- Ordem de badges do README começa com docs.rs e crates.io
- Docs da API pública expandidas para `error`, `envelope` e `lifecycle`
- Profile de release com LTO fat (`lto = "fat"`, `codegen-units = 1`, `strip = true`, `panic = "abort"`)
- Help do clap sem sugestões de env de produto (`BROWSER_AUTOMATION_CLI_*` não anunciado nas flags)
- Empacotamento crates liberado com remoção de `publish = false`

### Corrigido
- Bloqueios de build: wiring do campo `RunFlags.category_extensions` e lifetime de `Selector`
- Paridade `run` + scrape ponta a ponta; wait multi-text OR; gates de categoria no `run`
- Config/paths XDG sem env de produto para settings; doctor reporta `browsers_dir` XDG
- MITM hudsucker one-shot em bind `127.0.0.1` com porta efêmera
- Resume de workflow pula corretamente passos ok já concluídos
- Concorrência de batch amigável a shutdown via `JoinSet`
- Links intra-doc quebrados no help de `emulate --viewport`
- `tests/parity_inventory.rs` lê `tests/fixtures/tool-reference.md` vendored (52 tools)
- Drift de formatação sob `cargo fmt`

### Removido
- Workflows GitHub Actions em `.github/workflows/`
- Cargo `[profile.ci]` usado só pelo CI removido
- Orientação de CI hospedado e GitHub Actions da documentação pública
- Settings de produto amarrados a variáveis de ambiente `BROWSER_AUTOMATION_CLI_*` (settings ficam sob XDG + flags da CLI)

## [0.1.0] - 2025-07-16

### Adicionado
- Launch one-shot do Chrome via `chromiumoxide::Browser::launch`
- Flags de launch para proxy, webgpu, extensions e sandbox no path oxide
- Path FINALIZE com close, wait e kill fallback
- Comandos core: `doctor`, `open`/`goto`, `extract`, `scrape`, `run`, `grab`, `view`, `click`/`press`, `fill`/`write`, `robots`
- Captura opcional de console e network
- Política robots com dual-flag de aceite
- Superfície de paridade DevTools para navegação, input, snapshot, screenshot, eval, pages, wait, perf, lighthouse, screencast, heap, extensions
- Flags tool-ref como `--include-snapshot` em hover, drag, keys, upload e fill-form
- Filtros de `net` e `console` list com paginação
- `eval` com `--args`, `--dialog-action` e `--file-path`
- `perf start --auto-stop` e `perf insight`
- `screencast stop --path` com export webm ou mp4 via ffmpeg
- Análise profunda de heap sob `--category-memory`
- Gestão de páginas com `--background` e `--isolated-context`
- Descoberta de schema via `schema --cmd` e testes de inventário

### Alterado
- `src/install.rs` reduzido a descoberta local
- Stack CDP 100 por cento chromiumoxide Chrome

### Removido
- Monólito dual-spawn `launch_chrome` / `ChromeProcess`
- Branding residual e dumps não-produto da árvore pública

### Corrigido
- Histórico git público recriado sem commits de branding legado

### Notas
- Explicitamente fora **apenas de 0.1.0**: PRD superfície local scrape crawl/map/search, MITM e journal SQLite de workflow (esses itens entraram em 0.1.1)
