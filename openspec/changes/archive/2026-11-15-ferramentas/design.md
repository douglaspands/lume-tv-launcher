## Context

A motivação e o escopo estão no proposal.md. Pontos do estado atual que moldam o desenho:

- **Telas**: as telas vivem em `Screen` (`crates/lume-core/src/app/mod.rs`), e as subtelas das Configurações (`TvInfo`, `KeyMap`, `About`) seguem o mesmo padrão: `open_*`, `draw_*`, `back_from_*` e um intervalo de `FocusId` em `ids`. As Configurações têm 4 itens de 72 px, no passo de 88 (`settings.rs`).
- **Plataforma**: o app pede coisas à plataforma por `Effect`. A plataforma faz leituras periódicas pelo padrão `stats_due`/`stats_timeout`/`set_stats` no laço de `lume-app/src/android/mod.rs`, sempre com a interface parada (`is_busy`). O laço dorme até `next_timeout`.
- **Rede local já lida em Rust**: `network()` em `lume-app/src/android/device_info.rs` lista as interfaces IPv4 por um socket (o `getifaddrs` é bloqueado no Android 11). É por isso que a permissão `INTERNET` já está no manifesto, embora a spec `platform-compatibility` não a cite. Este change corrige a spec.
- **Threads**: a única thread de trabalho é a de decodificação do papel de parede (`wallpaper/mod.rs`), com `std::thread::Builder` e resultado entregue ao laço.
- **Bibliotecas disponíveis**: `android-activity` 0.6 (NativeActivity) expõe `AndroidApp::set_window_flags`. O crate `ndk` 0.9 tem a feature `audio` (AAudio, API 26 ou mais, e o `minSdk` é 28).
- **Desenho**: o desenho trabalha num canvas lógico de 1280x720, escalado para o painel. A TCL é 1920x1080, a AIWA 1280x720.

## Goals / Non-Goals

**Goals:**
- Custo zero fora das Ferramentas: nenhum estado, thread, stream ou arquivo vivo.
- Tudo em Rust, sem Java.
- Padrões de tela em pixel físico, sem escala nem suavização.
- Testes que rodam em thread nunca travam o laço de desenho.

**Non-Goals:**
- Ajustar brilho, backlight ou o áudio do sistema. Seria API de fabricante ou Java não autorizado.
- Persistir resultados de testes ou histórico entre sessões.
- Teste de velocidade da internet (ver D3).
- Separar o código das ferramentas em uma biblioteca nativa carregada sob demanda.

## Decisions

### D1. Estado por ferramenta em `Option<Box<ToolState>>`
O `LumeApp` ganha `tool: Option<Box<ToolState>>`, onde `ToolState` é um enum com uma variante por ferramenta (`Display`, `Picture`, `Remote`, `Audio`, `Storage`). Ele é criado em `open_tool` e vira `None` em `back_from_tool`. O `Box` mantém o `LumeApp` pequeno quando não há ferramenta aberta. O código fica em `crates/lume-core/src/app/tools/`, com um arquivo por ferramenta. Na `Screen` entram duas variantes: `Tools` (o menu) e `Tool(ToolKind)`.
- *Alternativa*: estados sempre alocados em campos do `LumeApp`. Foi descartada porque contradiz o pedido de ativação sob demanda e deixa buffers vivos o tempo todo.
- *Alternativa*: bibliotecas `.so` separadas carregadas com `dlopen`. Foi descartada porque o código que não roda já não entra no PSS (as páginas de código são mapeadas do arquivo e só sobem ao ser tocadas), e isso complicaria o build e a verificação do APK.

### D2. Recursos pesados vivem no lado da plataforma, ligados a um `Effect`
O `lume-core` só descreve o que quer, e a plataforma (`lume-app`) cria e destrói o recurso:
- `Effect::KeepScreenOn(bool)`: vira `set_window_flags(KEEP_SCREEN_ON)` ao entrar no menu Ferramentas e o inverso ao sair dele. Assim não precisa de Java.
- `Effect::Audio(AudioCommand)`, com `Tone { channel, hz }`, `Sweep`, `SyncBeep { period_ms }` e `Stop`: a plataforma abre o stream AAudio (estéreo, float, `PerformanceMode::LowLatency`) no primeiro comando e o fecha em `Stop`. O sintetizador (senoide com rampa de 20 ms) fica em `lume-core` como função pura testável, e o callback do AAudio só chama a função.
- `Effect::StartTest(TestKind)` e `Effect::CancelTest`: a plataforma cria uma thread nomeada (`lume-test`) com um `Arc<AtomicBool>` de cancelamento. A thread envia o progresso por um canal `mpsc`, e o laço o lê em cada volta com `try_recv`, como o papel de parede. O progresso marca a tela como suja no máximo 4 vezes por segundo.
- O Lume ir para segundo plano (`sleep()` do `Shell`) gera `Stop` e `CancelTest` sempre.
- *Alternativa*: threads e streams criados dentro do `lume-core`. Foi descartada porque o `lume-core` roda no desktop e nos testes sem Android, e a fronteira por `Effect` já é o padrão do projeto.

### D3. Teste de velocidade da internet (retirado)
O teste contra `speed.cloudflare.com` com `ureq` + `rustls` + `webpki-roots` chegou a ser implementado e medido na TCL (48 ms, 92/80 Mbps), e foi retirado por decisão do autor em 2026-11-15, junto com o cliente HTTPS (cerca de 2,6 MB no APK). A ferramenta Rede ficou só com dados locais, e o Lume segue sem abrir conexões.

### D4 e D5. Sinal do Wi-Fi e gateway (retirados)
A ferramenta Rede lia o sinal do Wi-Fi por um comando Java `wifi-signal` no `LumeBridge` (com `ACCESS_WIFI_STATE`) e o gateway por um dump de rotas via netlink em Rust. Sem o teste de velocidade (D3), o autor achou que ela não trazia nada útil e a retirou em 2026-11-15, com o Java e a permissão.

### D6. Padrões em pixel físico
Os padrões não passam pelo canvas lógico. Um caminho de desenho próprio (`ToolPainter`) recebe o tamanho físico do viewport e usa `glClear` para as cores sólidas e as linhas da grade e da borda de overscan desenhadas como retângulos de 1 px físico, com `glScissor` + `glClear`. Assim não há shader nem suavização. As rampas usam um quad com cor por vértice em coordenadas físicas, e o dithering segue a regra de degradês sem faixas. O destravamento de pixel usa `glScissor` só na janela de 64x64 físicos, e a cor muda a cada quadro em sequência fixa (R, G, B, branco, preto). O texto (`.toast`) usa o painter normal por cima.
- *Alternativa*: desenhar no canvas 1280x720 com escala 1,5. Foi descartada porque as linhas de 1 px viram 1,5 px suavizados na TCL, e isso esconde justamente o que o teste quer mostrar.

### D7. Quadros contínuos só nos modos animados
`LumeApp::next_deadline` devolve "agora" enquanto houver um modo animado em andamento (ciclo de cores, destravamento, movimento, sincronia) e, nos estáticos, o prazo do próximo evento (sumiço do toast, fim do tempo). O teste de movimento mede o intervalo entre quadros a partir do `now_ms` de cada quadro e conta os de 33 ms ou mais, a mesma régua de `device-notes`.

### D8. Sincronia entre áudio e imagem
O bipe é gerado pelo sintetizador com uma referência de tempo comum. A plataforma informa o instante em que o primeiro quadro de áudio do bipe sai, a partir de `AAudioStream_getTimestamp`, e o app desenha o quadrado com o atraso escolhido em relação a esse instante. Sem timestamp, o desenho usa o instante em que o bipe foi enfileirado, o que adiciona a latência do buffer (cerca de 20 a 40 ms). O valor mostrado é relativo, porque o usuário ajusta até perceber o bipe e o quadrado juntos, então um erro constante não muda o resultado.

### D9. Teste de armazenamento sem cache
O teste escreve 64 MB em blocos de 1 MB (um único buffer) em `files/lume-storage-test.<n>.tmp` (um arquivo por execução, para que uma execução cancelada que ainda está terminando não apague o arquivo da seguinte), com `fsync` no fim. Depois chama `posix_fadvise(POSIX_FADV_DONTNEED)` e lê o arquivo de volta, para tirar a leitura do cache de página. Ele apaga o arquivo no fim, ao cancelar e ao abrir a ferramenta (todos os `lume-storage-test*`, para limpar uma execução interrompida). O espaço livre vem do `statvfs`, que o `device-stats` já usa.

### D10. Teste do controle remoto pelo `raw_key`
Com a ferramenta Controle remoto aberta, `Shell::raw_key` desvia todo código para o estado da ferramenta antes do mapeamento. Ela consulta a tabela de `keys.rs` para saber o nome e a ação mapeada, sem executar nada. A repetição é detectada por códigos iguais em sequência, com o intervalo entre eventos. Voltar em dobro (menos de 2 s) fecha a ferramenta. Home nunca chega ao Lume como tecla de app.

## Risks / Trade-offs

- [Flashes do destravamento de pixel podem afetar pessoas com epilepsia fotossensível] → Aviso obrigatório com o foco em "Cancelar", janela restrita a 64x64 px físicos (menos de 0,3% da tela em 1080p) e tela cheia sempre abaixo de 3 Hz com transições graduais.
- [A AIWA (`low_ram`, cerca de 913 MB) pode encerrar o Lume durante um teste longo] → Os recursos são liberados no `sleep()`, e o arquivo temporário é apagado na próxima abertura.
- [`/proc/net/route` bloqueado] → Mostrar "—", sem regressão.
- [`AAudio` indisponível ou com falha no firmware da TV] → "Áudio indisponível" no item, sem travar.
- [Quadros contínuos esquentam a TV ou gastam CPU numa limpeza de 60 minutos] → A limpeza "Branco" é estática (sem quadros), e a "Cores" desenha só `glClear`, o desenho mais barato possível.
- [O teste de movimento pode mostrar quadros perdidos causados pelo próprio Lume] → É desejável, porque o teste também serve para medir o Lume.

## Migration Plan

O change é aditivo, sem migração de dados. O rollback é reverter o merge. Instalação nas TVs: o fluxo de sempre (`adb uninstall`, `adb install`, `set-home-activity`). Depois de instalar, conferir em `aapt dump permissions` as três permissões esperadas.

## Open Questions

- ~~O tamanho do arquivo de teste de armazenamento pode cair para 32 MB se a AIWA ficar lenta demais.~~ Resolvido: ficam 64 MB, por decisão do autor (a TCL termina em cerca de 2 s).
