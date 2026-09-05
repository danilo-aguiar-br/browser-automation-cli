# Matriz de Paridade de Stealth


## O que este documento é
- Este documento é um CRITÉRIO DE COMPLETUDE para defesas anti-detecção, e nada além disso
- Ele responde uma pergunta por linha: esta CLI cobre o vetor que uma implementação de referência nomeada já cobre
- Ele NÃO é um plano de porte, e nunca diz qual lacuna fechar primeiro como ordem
- Ele NÃO é promessa de paridade, e uma linha `COBERTO` diz que o código existe, nunca que um detector foi vencido
- Ele existe porque o Defeito 25 registrou que, sem uma referência canônica, não há critério para separar defesa pronta de defesa inacabada
- O Defeito 25 registrou o custo dessa ausência: a injeção de WebGL parecia pronta e vazava em 14 por cento dos launches
- Toda medição aqui está datada, porque medição é snapshot e nunca contrato
- Todas as medições da CLI neste documento foram feitas em 2026-09-04
- Todas as leituras de referência neste documento foram feitas em 2026-09-04


## Como ler a matriz
### Os três valores de estado
- `COBERTO` significa que a CLI implementa o vetor e um arquivo e uma linha reais provam isso
- `PARCIAL` significa que a CLI implementa parte do vetor e o restante está nomeado na mesma linha
- `AUSENTE` significa que a CLI não implementa o vetor de forma alguma
### A regra de evidência
- Toda linha `COBERTO` carrega arquivo e linha obtidos por `rg`, nunca por leitura de comentário
- Comentário não é evidência, porque comentário envelhece e este repositório tem histórico documentado disso
- Linha sem evidência é `AUSENTE` até que a evidência apareça
- Toda linha `PARCIAL` e `AUSENTE` carrega um `Como verificar`, para que a lacuna seja acionável em vez de aspiracional
- Célula de referência que não pôde ser lida diz `não acessado` com a data
### O que a matriz não decide
- Ela não ordena as lacunas por valor de negócio
- Ela não autoriza copiar código de nenhuma referência para dentro deste repositório
- Ela não afirma que fechar toda lacuna vence algum detector específico


## As referências e o que foi lido de cada uma
### Referências lidas nesta medição
- `patchright` foi lido em `https://raw.githubusercontent.com/Kaliiiiiiiiii-Vinyzu/patchright/main/README.md` em 2026-09-04
- `patchright-python` foi lido em `https://raw.githubusercontent.com/Kaliiiiiiiiii-Vinyzu/patchright-python/main/README.md` em 2026-09-04
- `rebrowser-patches` foi lido em `https://raw.githubusercontent.com/rebrowser/rebrowser-patches/main/README.md` em 2026-09-04
- `nodriver` foi lido em `https://raw.githubusercontent.com/ultrafunkamsterdam/nodriver/main/README.md` em 2026-09-04
- `zendriver-rs` foi lido em `https://raw.githubusercontent.com/TurtIeSocks/zendriver-rs/main/README.md` em 2026-09-04
- `wreq` foi lido em `https://raw.githubusercontent.com/0x676e67/wreq/main/README.md` em 2026-09-04
- `guise` foi lido em `https://raw.githubusercontent.com/santhreal/guise/main/README.md` em 2026-09-04
- `eoka` foi lido em `https://raw.githubusercontent.com/shrimp-software/eoka/main/README.md` em 2026-09-04
### O que não foi acessado
- Nenhum arquivo de código de nenhuma referência foi aberto, apenas o README publicado de cada repositório
- Portanto toda célula de referência nomeia um comportamento documentado, nunca um símbolo interno que esta medição não viu
- As duas exceções são `plan_keystrokes` no `guise` e `StealthConfig` no `eoka`, que os READMEs nomeiam literalmente
- O `guise` também nomeia `src/human/keystroke.rs`, a tabela `HOT_BIGRAMS` e os braços de match `hold_envelope` no próprio README
- `CDP-Patches` não foi lido nesta medição e não aparece em nenhuma linha


## A matriz de paridade
### Camada de marcadores JavaScript
| Defesa | Referência que cobre | Estado na CLI | Evidência na CLI | Como verificar |
| --- | --- | --- | --- | --- |
| `navigator.webdriver` presente e `false` | patchright, seção Command Flags Leaks: adiciona `--disable-blink-features=AutomationControlled` e remove `--enable-automation` | COBERTO | `src/native/stealth/mod.rs:217` declara o invariante e `src/native/cdp/chrome/args.rs:236` empurra o switch de feature | `rg -n "AutomationControlled" src/native/cdp/chrome/args.rs` e depois `browser-automation-cli --json doctor --fingerprint` lendo `webdriver_value` |
| `--enable-automation` nunca passado | patchright, seção Command Flags Leaks, listado como removido | COBERTO | `src/native/cdp/chrome/args.rs:282` registra a flag como REJECT na tabela de auditoria de argv | `rg -n "enable-automation" src/native/cdp/chrome/args.rs` e confirmar que nenhum `args.push` a carrega |
| Vendor e renderer de WebGL na thread principal | zendriver-rs, matriz de features: spoof de parâmetro de WebGL de superfície completa resolvido por tiers medidos de capacidade de GPU | COBERTO | `src/native/stealth/webgl.rs:135` envolve `getParameter` e `src/native/stealth/mod.rs:238` chama `webgl::coherence_patch` | `rg -n "getParameter" src/native/stealth/webgl.rs` e depois ler `webgl_renderer` de `doctor --fingerprint` |
| WebGL dentro de Worker e `OffscreenCanvas` | patchright, Init Script Shenanigans: injetar no stream do HTML coloca o override em todo contexto; spoof de superfície completa do zendriver-rs | COBERTO | `src/native/stealth/webgl.rs:161` prefixa o mesmo override no escopo do Worker por `self[k]` e `src/native/stealth/webgl.rs:145` monta o par a partir de `OffscreenCanvas` | `rg -n "self\[k\]" src/native/stealth/webgl.rs` e depois `eval` de um Worker que lê `getParameter(37446)` comparando com a thread principal |
| GPU relatada igual à GPU que renderiza | zendriver-rs, opt-in `gpu_backend` para renderizar WebGL e WebGPU na GPU real do host em vez do fallback por software | AUSENTE | nenhuma; `src/native/cdp/chrome/args.rs:350` lança com `--use-vulkan=swiftshader` enquanto o patch relata um par de hardware | `rg -n "swiftshader" src/native/cdp/chrome/args.rs` e cronometrar um draw pesado de WebGL contra a string de renderer que a página lê |
| Closed shadow roots alcançáveis por locator | patchright, seção Closed Shadow Roots, incluindo XPath dentro de raiz fechada | AUSENTE | nenhuma; `src/native/snapshot/take/iframe.rs:54` apenas lê o array `shadowRoots` que o CDP já devolve | `rg -n "attachShadow" src/ -g '*.rs'` não devolve nada, e então tentar pressionar um elemento dentro de raiz fechada |
### Camada de protocolo CDP
| Defesa | Referência que cobre | Estado na CLI | Evidência na CLI | Como verificar |
| --- | --- | --- | --- | --- |
| `contextId` obtido sem `Runtime.enable` | rebrowser-patches, três técnicas: binding no main world, `Page.createIsolatedWorld` e enable seguido de disable; patchright evita o comando avaliando em contextos isolados | COBERTO | `src/browser_policy/runtime_events.rs:20` registra que nenhum call site mira avaliação por `executionContextId` e `src/browser_policy/mod.rs:261` publica `runtime_enable_used` | `rg -n "runtime_enable_used" src/browser_policy/mod.rs` e depois ler esse campo em qualquer envelope de browser |
| Vazamento de `Console.enable` e `consoleAPICalled` | patchright, seção Console.enable Leak: a API de console é desligada por inteiro | PARCIAL | `src/browser/session/launch/ingest.rs:64` assina `Runtime.consoleAPICalled` somente sob `--capture-console`, e `src/browser_policy/runtime_events.rs:25` nomeia esse como o único alcance | `rg -n "consoleAPICalled" src/browser/session/launch/ingest.rs` e depois rodar com e sem `--capture-console` comparando `runtime_enable_used` |
| Init script injetado antes do parse do HTML | patchright, Init Script Shenanigans: Playwright Routes injetam JavaScript nas requisições de HTML, então `Runtime.enable` nunca é necessário | AUSENTE | nenhuma; `src/native/stealth/mod.rs:19` declara que os patches andam sobre `Page.addScriptToEvaluateOnNewDocument`, e o único `Fetch.enable` é `src/native/state/collect.rs:153`, usado para servir conteúdo substituto | `rg -n "addScriptToEvaluateOnNewDocument" src/native/stealth/mod.rs` e confirmar que nenhum `Fetch.fulfillRequest` reescreve corpo de documento com o payload de stealth |
| CSP do script injetado | patchright cobre isso implicitamente ao injetar no corpo da resposta, então nenhuma CSP de página alcança o payload | COBERTO | nenhuma necessária, e esse é o achado: medido em 2026-09-04 contra uma página servida com `Content-Security-Policy: script-src 'none'`, o payload ainda aplicou — `webdriver:false`, `platform: Linux x86_64` e o par mascarado `ANGLE (NVIDIA, NVIDIA GeForce GTX 1070, OpenGL 4.6)` num host macOS — porque script de init do CDP não é script de página e nenhuma CSP de página o governa | `cargo test --test csp_init_script_gate` |
| Iframes cross-origin alcançáveis | nodriver, conexão em flat mode, declarada como incluindo iframes na maioria das operações | COBERTO | `src/native/interaction/element_ops.rs:30` passa um mapa `iframe_sessions` por toda resolução de elemento e `src/native/interaction/pointer.rs:39` trata diálogo no escopo do OOPIF | `rg -n "iframe_sessions" src/native/interaction/element_ops.rs` e depois `view --detailed` numa página com frame cross-origin |
| `screenX` sintético não pode igualar `pageX` | patchright passa no Brotector somente com CDP-Patches, que existe por causa deste vetor | COBERTO | `src/native/cdp/types/input.rs:78` registra o crbug, a medição e por que ambos os campos ficam `None` para o Chrome derivá-los sozinho | `rg -n "screen_x" src/native/cdp/types/input.rs` e depois rodar a checagem de cinco linhas do Brotector num clique despachado |
### Camada de rede e transporte
| Defesa | Referência que cobre | Estado na CLI | Evidência na CLI | Como verificar |
| --- | --- | --- | --- | --- |
| Fingerprint TLS JA3 e JA4 | wreq, declarado como controle fino de extensões TLS em vez de strings de fingerprint, com mais de 100 perfis de dispositivo em `wreq-util` | AUSENTE | nenhuma; `Cargo.toml:296` constrói `reqwest` sobre `rustls` com `webpki-roots`, o que emite um ClientHello de rustls | `rg -n "rustls" Cargo.toml` e depois mandar `--engine http` num endpoint que ecoa JA3, comparando com um Chrome real |
| Frame `SETTINGS` de HTTP/2 igual ao do Chrome | wreq, paridade de HTTP/2 sobre TLS por extensões e settings por perfil | PARCIAL | `src/xdg/config_model.rs:112` até `:127` expõem seis knobs `http2_*`, mas eles são valores de ajuste do hyper e nenhum perfil de Chrome os define | `browser-automation-cli --json config list-keys` e confirmar que nenhuma chave nomeia perfil de navegador |
| QUIC e HTTP/3 negociados com a origem | não acessado em 2026-09-04; nenhuma referência lida nesta medição documenta este vetor | AUSENTE | nenhuma; `src/native/cdp/chrome/args.rs:326` empurra `--disable-quic` e `:332` declara que a recusa é decisão de segurança de proxy | `rg -n "disable-quic" src/native/cdp/chrome/args.rs` e depois capturar o tráfego confirmando que não há UDP 443 para o alvo |
| Perfil persistente entre processos one-shot | nodriver `Config.user_data_dir`, documentado como não limpo quando especificado; `StealthConfig` do eoka; `browser_cookies_persist` do zendriver-rs | COBERTO | `src/native/cdp/chrome/args.rs:409` prefere `--profile` e depois a chave XDG `user_data_dir`, e `src/native/cdp/chrome/args.rs:104` restringe o perfil nomeado ao modo `0700` | `browser-automation-cli --json config set user_data_dir /tmp/p` e depois lançar duas vezes confirmando que o cookie jar sobrevive |
### Camada de comportamento humano
| Defesa | Referência que cobre | Estado na CLI | Evidência na CLI | Como verificar |
| --- | --- | --- | --- | --- |
| Caminho do ponteiro como curva amostrada | eoka, declarado como simulação de input humano com curvas de Bézier; guise, timing de teclado e mouse | COBERTO | `src/native/interaction/kinematics/geometry.rs:87` arqueia os pontos de controle e `:100` amostra `cubic_bezier` sob uma suavização | `rg -n "cubic_bezier" src/native/interaction/kinematics/geometry.rs` e depois despachar um move conferindo que o caminho não é reta |
| Número de passos amostrado em vez de fixo | mouse humano do eoka; timing determinístico do guise sob RNG semeado | COBERTO | `src/constants/timing.rs:131` define `INPUT_MOVE_STEPS_STDDEV` e `src/xdg/policy/knobs/table.rs:185` o expõe como knob | `browser-automation-cli --json config get input_move_steps_stddev` e confirmar padrão diferente de zero |
| Delta de rolagem amostrado por tick | `stealth::human::Human` scroll do eoka | COBERTO | `src/constants/timing.rs:138` define `INPUT_SCROLL_TICK_STDDEV_PX` e `src/xdg/policy/knobs/table.rs:195` o expõe | `rg -n "INPUT_SCROLL_TICK_STDDEV_PX" src/constants/timing.rs` e depois medir deltas consecutivos de roda |
| Dwell e gap de tecla dispersos | `plan_keystrokes` do guise, que devolve hold e gap por tecla a partir de envelopes calibrados | COBERTO | `src/constants/timing.rs:117` define `INPUT_KEY_DWELL_STDDEV_MS` e `:122` define `INPUT_TYPE_DELAY_STDDEV_MS` | `browser-automation-cli --json config get input_key_dwell_stddev_ms` e depois medir intervalos de `keydown` numa página viva |
| Timing de tecla por bigrama | guise, tabela `HOT_BIGRAMS` e braços de match `hold_envelope` em `src/human/keystroke.rs`, nomeados no próprio README | COBERTO | `src/native/interaction/kinematics/qwerty.rs:57` escala o intervalo pelo par de dedos do QWERTY e `src/native/interaction/kinematics/mod.rs:219` aplica essa escala ao atraso sorteado; o layout é derivado em vez de a tabela da referência ser copiada, porque uma tabela de digramas mede um corpus enquanto o par de dedos é a causa por trás dela | `cargo test --lib the_pair_and_not_the_character_sets_the_gap`, que exige que a média de `th` vença a de `qz` |
| Injeção de typo com correção por backspace | guise, declarado como injeção de typo com correção por backspace e pausas aleatórias de pensamento | COBERTO | `src/native/interaction/kinematics/mod.rs:241` sorteia a tecla errada em `qwerty::neighbour` sob `input_typo_permille`, e `src/native/interaction/keyboard.rs:158` a digita, manda `Backspace` e redigita o caractere pretendido; a chave é `0` por PADRÃO porque esta é a única humanização que muda o que a página lê, e não quando ela lê | `cargo test --test typo_correction_gate`, que digita com a taxa fixada em 1000 e exige 6 eventos `Backspace`, 18 keydowns e o campo ainda com o texto pedido |
| Pausa longa entre palavras | pausas aleatórias de pensamento do guise | COBERTO | `src/native/interaction/kinematics/mod.rs:229` implementa `maybe_long_pause` sob o gate `input_word_pause_permille` | `browser-automation-cli --json config get input_word_pause_permille` e depois digitar uma frase procurando o intervalo destoante |


## Lacunas por custo de fechamento
- 7 linhas não são `COBERTO`: 2 são `PARCIAL` e 5 são `AUSENTE`, de 23 linhas em `A matriz de paridade`
- Cada uma dessas linhas tem bullet próprio abaixo, então a contagem e a lista concordam por construção
- `JA3 e JA4` é integração, não porte, porque `wreq` já é crate Rust com o catálogo de perfis em `wreq-util`
- `SETTINGS de HTTP/2` fecha com a mesma integração, porque o `wreq` trata TLS e HTTP/2 como um perfil único
- `GPU relatada igual à GPU que renderiza` exige abandonar o bundle SwiftShader, que `src/native/cdp/chrome/args.rs:338` documenta como quatro flags que só fazem sentido juntas
- `Init script antes do parse` é a maior mudança, porque tira o payload de `Page.addScriptToEvaluateOnNewDocument` e o coloca no corpo da resposta
- `Closed shadow roots` é independente de toda linha acima e cai na resolução de elemento, não no payload de stealth
- `Console.enable` é independente de toda linha acima e o custo dele é a superfície `--capture-console`
- QUIC está listado como `AUSENTE` e está assim de propósito: `src/native/cdp/chrome/args.rs:332` registra a recusa como decisão de segurança de proxy, que vence fidelidade de fingerprint


## Divergências entre o registro do Defeito 25 e o que as referências dizem
- O registro do Defeito 25 nomeia duas categorias do patchright, `Script Injection` e `Execution Context`
- Nenhuma das duas strings aparece no README do patchright lido em 2026-09-04
- O README publica cinco títulos sob `Patches`: `Runtime.enable Leak`, `Console.enable Leak`, `Command Flags Leaks`, `General Leaks` e `Closed Shadow Roots`
- O comportamento que o registro descreve é real e vive sob `Init Script Shenanigans`, um bloco recolhido fora da lista `Patches`
- O registro afirma que a CLI já bate com as categorias 1 e 3 do patchright, e pela ordem publicada essas são `Runtime.enable Leak` e `Command Flags Leaks`, o que esta matriz confirma
- O registro diz que o patchright deriva o `contextId` avaliando `globalThis` e parseando o `objectId`; o README do patchright diz apenas que ele avalia em contextos de execução isolados, e as três técnicas nomeadas pertencem ao README do rebrowser
- O registro afirma `21+ patches em cinco categorias`; o README publica cinco títulos e nenhuma contagem de patches, então o número não foi confirmado
- Duas referências lidas aqui cobrem vetores que o registro nunca nomeia: o patchright cobre closed shadow roots, e o zendriver-rs oferece `gpu_backend` para a divergência do SwiftShader


## Como reproduzir esta medição
- Leia o defeito primeiro com `rg -n "Defeito 25" gaps.md` e abra o bloco que ele aponta
- Rebaixe cada README de referência com `browser-automation-cli -q --json --ignore-robots --i-accept-robots-risk scrape <URL_RAW> --format text --engine http`
- Reexecute cada comando de `Como verificar` da matriz, na linha a que ele pertence
- Trate como `AUSENTE` toda linha `COBERTO` cujo `rg` não devolva nada, e corrija a linha
- Redate o documento, porque arquivo e linha se movem: `src/native/stealth/webgl.rs` não existia mais cedo em 2026-09-04 e a evidência de WebGL migrou para lá durante esta medição
- Nunca rode `cargo` para reproduzir este documento, porque nenhuma linha depende de build


## O que este documento não prova
- Ele não prova que alguma linha `COBERTO` sobrevive a detector vivo, porque nenhuma linha foi testada contra anti-bot comercial nesta medição
- Ele não prova que as linhas `AUSENTE` são as únicas lacunas, porque 23 linhas em `A matriz de paridade` foram comparadas e 8 READMEs em `Referências lidas nesta medição` foram lidos
- Ele não prova que alguma célula de referência bate com o código atual daquela referência, porque só READMEs foram lidos e nenhum arquivo de código foi aberto
- Ele não prova que a CLI vence o Google Search, e o Defeito 25 já registra que a camada de atestação fica acima do fingerprint por inteiro
- Ele não mede a taxa de 14 por cento de vazamento de WebGL que motivou o Defeito 25, porque essa medição exige uma bateria de launches que este documento não rodou
