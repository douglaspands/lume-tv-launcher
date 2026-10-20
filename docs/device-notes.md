# Notas de dispositivo e ambiente

Registro de medições e descobertas feitas durante o desenvolvimento do lume v0.1.0.

## Build em container: validação do `wslc` (tarefa 1.6)

Ambiente: `wslc` 3.0.3.0 (`C:\Program Files\WSL\wslc.exe`), chamado do WSL2 (Ubuntu).

| Recurso | Resultado |
|---|---|
| Dockerfile em estágios (`FROM ... AS`, `--target`) | Funciona |
| `RUN --mount=type=cache` | Funciona, e o cache persiste entre builds (contador foi de 1 para 2 quando a camada foi invalidada) |
| `--secret id=...,src=<arquivo>` | Funciona (o segredo ficou visível em `/run/secrets/<id>` só durante o `RUN`) |
| Cache de camadas | Funciona (segunda execução idêntica: 0,8 s, todas as camadas `CACHED`) |
| Contexto convertido com `wslpath -w` | Funciona (caminhos `\\wsl.localhost\...`) |
| `-o type=local,dest=<pasta>` | **Não suportado**: `directory exporters are not supported` |
| `-o type=tar,dest=<arquivo>` | Funciona; o arquivo aparece na pasta do WSL |

**Decisão:** como o `wslc` só exporta um arquivo, o `scripts/container-build.sh` usa `-o type=tar,dest=dist/artifacts.tar` e extrai com `tar -xf`. O mesmo formato funciona com `docker buildx`, então o script usa um único caminho para os dois runtimes. O estágio `artifacts` (`FROM scratch`) não muda.

## TCL 55C715: dispositivo de referência (tarefas 2.1 a 2.3)

Medido em 2026-10-19 com o APK de debug (`dist/lume-0.1.0-debug.apk`), via `adb` por rede.

| Item | Valor |
|---|---|
| Modelo / fabricante | `UnionTV` / TCL (SoC Realtek `rtd284x`) |
| Android | 9 (API 28), build `release-keys`, `ro.config.low_ram=true` |
| ABIs | `armeabi-v7a`, `armeabi` (somente 32 bits) |
| GPU | ARM Mali-470 MP, **OpenGL ES 2.0** (`ro.opengles.version=131072`), driver r9p0 |
| Tela | `wm size` 1280×720, `wm density` 213; a janela recebida do lume é 1280×720 |
| RAM | `MemTotal` 1,4 GB (`MemAvailable` ~700 MB com a TV ociosa) |
| Fontes do sistema | 236 arquivos em `/system/fonts` (Noto por escrita, `NotoSansCJK-Regular.ttc`, `DroidSans*`), mais `/system/etc/fonts.xml` |

### 2.1 Hello GL / renderer

- O primeiro build falhou na TV com `program failed to link: L0010 Uniform 'u_view' differ on precision`: o driver Mali exige a mesma precisão para um `uniform` declarado nos dois estágios, e o fragment shader do retângulo usava `highp` condicional enquanto o vertex shader usava o padrão. **Correção:** o `u_view` saiu do fragment shader, e a unidade de pixel (`1 / escala`) passa do vertex shader para o fragment shader pelo varying `v_local.z`.
- Depois disso, o lume desenha a interface inteira na TV (barra superior, carrossel, favoritos, textos em pt-BR). Screenshot por `adb exec-out screencap -p`.
- O build de debug do container usa uma chave de debug diferente a cada imagem, então `adb install -r` falha com `INSTALL_FAILED_UPDATE_INCOMPATIBLE`. Em desenvolvimento, desinstale antes (`adb uninstall app.lume.tv`).

### 2.2 Botão Home

| Tentativa | Resultado |
|---|---|
| `cmd package set-home-activity app.lume.tv/.LumeActivity` | Aceito, **mas o Home continua abrindo o `com.google.android.tvlauncher`** (a resolução de HOME ignora a escolha) |
| `pm disable-user --user 0 com.google.android.tvlauncher` | **Funciona**: HOME resolve para `app.lume.tv/.LumeActivity`, e `input keyevent KEYCODE_HOME` volta ao lume a partir de Configurações e do YouTube |

Reverter: `adb shell pm enable com.google.android.tvlauncher`.
Limitação: o teste usou `input keyevent KEYCODE_HOME`. A conferência com o botão físico do controle da TCL continua pendente (tarefa 14.1).

### 2.3 Memória e comportamento em segundo plano (build de debug)

| Medida | Resultado |
|---|---|
| Lume em primeiro plano, ocioso (TOTAL PSS) | ~20,4 MB (Native 5,2 MB, Java 3,3 MB, Code 2,3 MB, System ~8,5 MB, Graphics 0) |
| Lume dormindo atrás de outro app, ~12 s (TOTAL PSS) | **18,6 MB**, Graphics 0 (`window gone: renderer, glyph atlas and fonts released (dormant)`) |
| Launcher de fábrica (`com.google.android.tvlauncher`), ocioso | **48,7 MB** (Native 21 MB, Code 10 MB) |

Descobertas:
- O Android da TCL **encerra o processo do lume pouco depois de ele ir para segundo plano** (`ActivityManager: Killing ... app.lume.tv (adj 600): stop app.lume.tv`; há também uma lista `persist.tcl.standby.processName`). Nos testes isso aconteceu quando outro app pesado abriu (Play Store). Nesses casos o custo em segundo plano é zero, e todo retorno é uma abertura a frio.
- Configurações da TV (`com.android.tv.settings/.MainSettings`) é translúcida: o lume só pausa, não para, e o ciclo DORMINDO não dispara. Para testar o DORMINDO, use um app de tela cheia (Plex, Play Store).
- O YouTube, nesta TV, fecha sozinho ao ser aberto por `am start` e devolve a tela ao lume (provavelmente não está logado), então não serve para as medições.

### Build de release na TCL (APK de 1,9 MB, assinado com chave de debug)

| Medida | Resultado | Orçamento |
|---|---|---|
| Primeiro plano, ocioso 30 s (TOTAL PSS) | **13,8 MB** (Native 4,5 MB, Java 1,2 MB, Code 3,1 MB, System 4,4 MB, Graphics 0) | ≤ 45 MB |
| Dormindo atrás do Plex, ~12 s (TOTAL PSS) | **13,6 MB**, Graphics 0 | ≤ 20 MB |
| Retorno a quente (Home → `window 1280x720`, 3 execuções) | 0,48 a 0,57 s | ≤ 300 ms até o primeiro quadro |
| Abertura a frio (Home → `window 1280x720`, 3 execuções) | 0,87 a 0,88 s (`Displayed` +470 a 490 ms) | ≤ 800 ms até o primeiro quadro |

Observações:
- Os tempos vão do `START u0 {HOME}` do `ActivityManager` até o log `window` do lume, que **não inclui a criação do renderer nem o primeiro quadro**. Boa parte do intervalo é a transição de janelas do sistema, que o lume não controla. Falta um log de "primeiro quadro desenhado" para fechar a medição (tarefa 14.1).
- Mesmo só até a janela, o retorno a quente (~0,5 s) já passa de 300 ms. Parte disso é a transição do sistema, mas é o principal ponto de atenção da tarefa 14.1 e pode exigir recalibrar esse orçamento na spec.
- A release usa metade da memória do build de debug (o debug carrega símbolos e DEX sem otimização).

### Bug encontrado só na release: o R8 removia a ponte JNI

Com `minifyEnabled true` e sem regras, o R8 removia `LumeActivity.bridge(String, String)`, chamado apenas pelo Rust. A TV mostrou o lume em inglês, sem catálogo (só o tile "Settings"), com os logs `bridge command '...' failed: Method not found`. **Correção:** `android/app/proguard-rules.pro` mantém `LumeActivity.bridge`, `nativeHomePressed` e `LumeBridge`. Depois disso, a TV lista os apps reais, em pt-BR (idioma do sistema vindo da ponte).

### Aba Apps: serviços de sistema e ícones (2026-10-19)

- **O OK nos tiles funciona**: testei os 22 tiles da grade antigos por `adb input`, e cada OK abriu a activity certa. O "volta para o início" e o "vai para configurações" vinham de apps de sistema da TCL que estavam na grade: `copyDataToTV` e `TclHomePassive` fecham na hora (a TV volta ao HOME, que é o lume), e o Android TV Remote Service, `Config.` e `Configuração Inicial` abrem telas de configuração.
- **Origem**: eram atividades com a categoria `LAUNCHER` comum (não `LEANBACK_LAUNCHER`) de apps de sistema. O launcher original só lista `LEANBACK_LAUNCHER`. O lume agora descarta as só-`LAUNCHER` de apps de sistema, e a grade passa de 25 para 18 tiles (apps de TV e o tile "Configurações").
- **Ícones**: aparecem na grade (sem o nome dentro do tile) e nos favoritos (ícone mais o nome). Medido na release com ícones carregados: **18,3 MB** em primeiro plano e **16,9 MB** dormindo (Graphics 0), contra 13,8 MB e 13,6 MB sem ícones. Ainda dentro dos orçamentos de 45 e 20 MB.
- **Banners de TV** (como o Android TV Launcher): os apps de TV mostram o banner 16:9 cobrindo o tile, com os cantos arredondados do tile, e os demais mostram o ícone sobre a cor do tile. Memória na release: **16,4 MB** em primeiro plano e **15,5 MB** dormindo (Graphics 0), dentro dos orçamentos de 45 e 20 MB.

## Avaliação de desempenho na TCL (release com banners, 2026-10-19)

APK de release assinado com chave de debug, TCL 55C715 (4 núcleos, 1,4 GB de RAM, Mali-470, Android 9). O PSS vem de `dumpsys meminfo`, a CPU de `/proc/<pid>/stat` (ticks de 10 ms, relativos aos 4 núcleos) e os quadros do `dumpsys SurfaceFlinger --latency`.

### Resumo

| Item | lume | Launcher original | Orçamento |
|---|---|---|---|
| PSS em primeiro plano, estável (30 s a 90 s depois de abrir) | **28 a 32 MB** | 40,9 MB (48,7 MB em outra medição) | ≤ 45 MB |
| PSS em primeiro plano, pico (0 a 10 s depois de abrir) | **51 a 53 MB** | n/a | ≤ 45 MB (**acima**) |
| PSS dormindo atrás de outro app (5 a 12 s) | **19,6 a 22,3 MB** | n/a | ≤ 20 MB em 10 s (**no limite**) |
| Graphics (`dumpsys meminfo`) | 0 | 0 | 0 dormindo |
| CPU parada na Home, foco no botão do carrossel (60 s) | **0,00%** (1 quadro/min) | 1,26% | < 1% |
| CPU parada na Home, carrossel girando (60 s) | **0,05%** (13 quadros/min) | n/a | < 1% |
| CPU parada na aba Apps (60 s) | **0,00%** (1 quadro/min) | n/a | < 1% |
| CPU de 16 teclas de rolagem da grade | 0,14 s no total | n/a | n/a |
| CPU de 80 teclas de troca de abas | 0,76 s no total | n/a | n/a |
| Threads | 15 | 38 | n/a |
| APK | **1,9 MB** (`liblume.so` 750 KB em ARMv7 e 1092 KB em ARM64, `classes.dex` 9 KB) | n/a | ≤ 6 MB |

### Vazamento
Sem vazamento: em 90 ciclos de troca de abas, o heap Java ficou em 1,7 MB e o heap nativo em 5,9 MB. O PSS só oscilou por páginas de arquivo (ver abaixo).

### Abertura
- Cronologia de uma abertura a frio (release, logs): `START` → processo +0,1 s → `lume starting` +0,3 s → `Displayed` **+0,54 a +0,61 s** → `window` (início do renderer) **+1,0 s**. O primeiro quadro desenhado vem depois do `window` e ainda não é medido (falta o log de "primeiro quadro", tarefa 14.1). O orçamento é 800 ms até o primeiro quadro, então a abertura a frio provavelmente passa dele.
- Pelo botão Home, antes dos banners: 0,87 s até o `window`.
- **O `TotalTime` do `am start -W` não serve para este app**: dá 0,65 s a 17 s para a mesma abertura (o sistema espera o primeiro quadro de uma activity que desenha pelo próprio EGL). Usar o `Displayed` do log.
- Retorno a quente: 0,2 s pelo `am start -W` (mesma ressalva) e ~0,5 s da tecla Home até o `window`, parte disso é a transição do sistema.

### O que explica o PSS
- **Pico de 51 a 53 MB na abertura**: ~23 MB são páginas **limpas** de arquivo (`.apk mmap`, privadas e reclamáveis pelo kernel), que caem para ~7 MB sozinhas em 20 a 30 s. A explicação mais provável é que o `PackageManager` abre o APK de cada app instalado para ler o rótulo e o banner (Netflix, Disney+ e Prime têm dezenas de MB). O pico vem do custo de abrir esses recursos, não do lume. Para reduzir: guardar os banners num cache em disco (`cacheDir`, ~150 KB por app) e carregar só uma vez por versão do app.
- **Heap nativo de ~11 MB em primeiro plano** (contra 4,5 MB sem imagens): memória livre que o alocador ainda não devolveu depois de montar os banners. Reutilizar os `Bitmap` (já feito) não mudou o valor estável. Cai para ~6 MB dormindo.
- **Dormindo, ~6 MB de heap nativo** é o piso do processo; o lume em si usa pouco.
- A TCL encerra o processo do lume ~40 a 60 s depois de ir para segundo plano.

### Bug encontrado e corrigido
Na aba Apps (e em Configurações, Sobre e Editor), com o foco fora do botão do carrossel, o prazo do carrossel ficava vencido para sempre (só o desenho da Home o consumia) e o laço desenhava a **60 quadros por segundo sem parar, usando ~30% de um núcleo**. Corrigido: o carrossel só tem prazo na Home. Teste: `no_screen_but_home_keeps_the_carousel_timer_running`. Depois da correção, a aba Apps parada usa 0,00% de CPU.

### Não medido
Quadros perdidos na rolagem contínua (o `--latency` guarda só os últimos 128 quadros e as teclas pelo `adb` chegam a ~0,4 s uma da outra), YouTube em primeiro plano (fecha sozinho nesta TV) e o botão Home físico do controle.

### Depois da otimização de memória (cache de banners e sequência salva), 2026-10-19

A explicação do pico dada acima estava incompleta. O pico de 51 a 53 MB vinha, na verdade, de um **escaneamento completo do catálogo a cada início de processo**: o número de sequência do `getChangedPackages` ficava só em memória, então todo processo novo lia o rótulo de todos os apps abrindo o APK de cada um (`.apk mmap` de ~22 MB, páginas limpas). O design previa só aplicar as diferenças. Duas correções no `LumeBridge`:

1. **Sequência salva em disco** (`filesDir/change-sequence.v1`, junto com o `BOOT_COUNT` do aparelho, porque os números recomeçam a cada boot). Com ela, um processo novo só consulta as diferenças desde a última vez. Verificado na TV: desativar o Plex (`pm disable-user`) o tira da grade no processo seguinte, e reativá-lo o traz de volta.
2. **Cache em disco dos banners e ícones** (`cacheDir/tiles/`, ~147 KB por app, chave com a data de atualização do app). Um app atualizado ganha outro arquivo, e os de apps removidos são apagados.

| Item (release, TCL) | Antes | Depois | Orçamento |
|---|---|---|---|
| Pico de PSS na abertura (0 a 10 s) | 51 a 53 MB | **18 MB** | ≤ 45 MB |
| PSS estável em primeiro plano | 28 a 32 MB | **15,6 a 18 MB** | ≤ 45 MB |
| PSS dormindo (11 s) | 19,6 a 22 MB | **14,3 a 14,6 MB** (Graphics 0) | ≤ 20 MB |
| Heap nativo em primeiro plano | 11 MB | 4,9 MB | n/a |
| `.apk mmap` | até 24 MB | 1,2 MB | n/a |
| `Displayed` na abertura a frio | +540 a +610 ms | **+310 a +350 ms** | n/a |
| Início do renderer (`window`) a partir do `START` | ~1,0 s | ~0,5 s | n/a |
| CPU parada na aba Apps (45 s) | 0,00% | 0,01% | < 1% |

O primeiro uso, com o cache vazio, continua custando o escaneamento e a renderização dos banners (pico de 48 MB e ~13 MB de heap nativo), só uma vez.

## Primeiro quadro, animações e quadros perdidos (2026-10-19)

### Primeiro quadro (log `first frame` do lume, release, TCL)
- **A frio** (processo morto, `START` → primeiro quadro): **725 a 904 ms** (mediana ~745 ms), contra o orçamento de 800 ms. O quadro sai 80 a 90 ms depois da janela e ~430 a 510 ms depois do início do processo.
- **A quente** (processo vivo, outro app na frente): **503 a 714 ms** (mediana ~520 ms), contra 300 ms. A parte do lume (janela → primeiro quadro) é de **75 a 110 ms**. Cerca de 400 ms vão da camada do lume aparecer no SurfaceFlinger até a janela chegar ao lume, e esse tempo é da transição de janelas da TCL, fora do controle do lume.
- Um tema próprio (`LumeTheme`, sem animação de janela e com o fundo do lume) melhorou só a abertura a frio (mediana de ~745 para ~690 ms) e não mudou a quente.
- O primeiro quadro é desenhado sem as imagens dos tiles, que entram em seguida.

### Bug: as animações não rodavam na TV
`Animations::next_deadline` devolvia "agora + 16 ms", e o laço recalculava isso a cada espera, então o prazo nunca vencia: cada tecla gerava **um único quadro**, sem os quadros intermediários da animação de foco, da rolagem e da troca de slide. Corrigido em `LumeApp::next_deadline` (o próximo quadro é o do último quadro desenhado mais 16 ms, e o quadro que encontra tudo parado é o último). Testes: `frames_keep_coming_while_an_animation_runs` e os ajustes de `frames_are_built_only_when_something_changed` e `the_clock_only_asks_for_a_frame_when_the_minute_changes`. Na TV, cada tecla passou a gerar ~10 quadros em ~200 ms.

### Quadros perdidos (ainda acima do orçamento)
Com as animações rodando, cada animação de ~200 ms tem **1 a 3 quadros de 33 ms** (um vsync perdido) em ~10, ou seja, 10 a 30%, contra o orçamento de 1%. O custo por quadro medido é de ~8,5 ms de CPU. Já reordenei o desenho dos tiles em passes (caixas, imagens, texto) para reduzir as chamadas de GPU, e há um teste que fixa o número de lotes. Falta descobrir se o limite é a CPU (driver Mali) ou a GPU.

### Carrossel
Os 78 quadros em 30 s com o foco no botão do carrossel ficaram sem investigação: o usuário decidiu não seguir com o carrossel, e as melhorias sobre ele foram descartadas.
