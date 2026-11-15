## 1. Base: menu Ferramentas e ciclo de vida

- [x] 1.1 Adicionar `Screen::Tools` e `Screen::Tool(ToolKind)`, os `FocusId` do menu e das ferramentas em `ids`, e o módulo `app/tools/` com `ToolState` em `Option<Box<_>>` (D1); teste em `app/tests.rs` que confirma `tool == None` depois de voltar às Configurações
- [x] 1.2 Adicionar o item "Ferramentas" às Configurações (entre "Informações da TV" e "Sobre o Lume") e o menu com as seis ferramentas, com a descrição do item em foco no `.panel` à direita e rolagem da lista (com o conteúdo em y=152, `page::PLAIN_CONTENT_TOP`, os seis itens no passo de 88 cabem sem rolar); testes de navegação (abrir, Voltar passo a passo com o foco no item de origem) e `the_layout_is_on_the_ruler` cobrindo os 5 itens
- [x] 1.3 Criar os `Effect` `KeepScreenOn`, `Audio`, `StartTest`/`CancelTest` (D2) e emitir `KeepScreenOn(true/false)` ao entrar e sair do menu; garantir `Stop`/`CancelTest` em `Shell::sleep`; testes unitários dos efeitos emitidos
- [x] 1.4 Na plataforma Android, tratar `KeepScreenOn` com `AndroidApp::set_window_flags`, e criar o canal `mpsc` e a thread `lume-test` com cancelamento por `AtomicBool`, lidos no laço com `try_recv` e no máximo 4 redesenhos por segundo; verificar com `cargo clippy --workspace --all-targets` limpo e, no emulador, que o protetor de tela não aparece com o menu aberto
- [x] 1.5 Criar o aviso de flashes (cartão `.menu` com "Iniciar"/"Cancelar", foco em "Cancelar") e o `.toast` de rodapé com tempo de 3 s, reaproveitáveis pelas ferramentas; testes de que Voltar no aviso não inicia o modo
- [x] 1.6 Adicionar as strings novas em `values`, `values-pt-rBR`, `values-es`, `values-id` e `values-ru`, seguindo `docs/translations.md`; verificar que os testes de `lume-strings` passam e que o texto mais longo (ru) não quebra o layout numa captura `cargo run -p lume-desktop -- --locale ru`

## 2. Ferramenta Tela

- [x] 2.1 Criar o `ToolPainter` em pixel físico (D6): `glClear` para as cores sólidas, `glScissor`+`glClear` para as linhas de 1 px, quad com cor por vértice e dithering para as rampas; teste de captura no desktop em 1920x1080 confirmando a grade com linhas de 1 px exatas
- [x] 2.2 Implementar o modo Diagnóstico (13 padrões em ciclo com Esquerda/Direita, toast do nome por 3 s, sem quadros parado); testes de ordem, ciclo e de `next_deadline` sem quadros depois do toast
- [x] 2.3 Implementar "Limpar retenção" (escolha de 10, 30 ou 60 min e do padrão Branco/Cores, transições de pelo menos 1 s, cores de pelo menos 2 s, fim automático com "Limpeza concluída", tempo restante a cada tecla); testes da taxa de troca (no máximo 3 por segundo) e do fim do tempo com relógio simulado
- [x] 2.4 Implementar "Destravar pixel" (janela de 64 px físicos, passos de 16/8, limites da tela, aviso antes, ciclo R/G/B/branco/preto por quadro, 5, 10 ou 20 min, Voltar reposiciona); testes de posição, limites e de que o ciclo só começa após "Iniciar"
- [x] 2.5 Mostrar a descrição honesta de cada modo; verificar numa captura `--screenshot` da ferramenta Tela

## 3. Ferramenta Imagem

- [x] 3.1 Implementar "Overscan e geometria" (borda de 1 px, marcas de 2,5% e 5%, círculo e quadrado centrais, diagonais) no `ToolPainter`; captura em 1920x1080 e 1280x720 conferida com `scripts/measure-ink.py`
- [x] 3.2 Implementar "Movimento" (bloco e texto em velocidade constante, três velocidades com Cima/Baixo, contador de intervalos de 33 ms ou mais, quadros contínuos só com o teste aberto, D7); testes do contador com tempos simulados

## 4. Ferramenta Controle remoto

- [x] 4.1 Desviar `Shell::raw_key` para a ferramenta quando ela estiver aberta (D10): nome, `KEYCODE_*`, ação mapeada sem executar, repetições e intervalo médio, histórico das últimas 8 e saída com Voltar duas vezes em até 2 s; testes para tecla mapeada a app (não abre), código desconhecido ("Não usada"), repetição e Voltar isolado e duplo
- [ ] 4.2 Verificar no emulador com `scripts/emulator.sh key ...` e na TCL e na AIWA (Menu 82 e Bookmark 174 aparecem com os nomes certos)

## 5. Ferramenta Áudio

- [x] 5.1 Escrever o sintetizador puro em `lume-core` (senoide por canal, rampa de 20 ms (o bipe de sincronia usa 2 ms, ver `audio-diagnostics`), varredura logarítmica de 20 Hz a 20 kHz em 30 s, bipe de 1 kHz por 50 ms, pico de no máximo -12 dBFS); testes de amplitude máxima, rampa e frequência no meio da varredura
- [ ] 5.2 Ativar a feature `audio` do `ndk` e abrir e fechar o stream AAudio pelo `Effect::Audio` (D2), com "Áudio indisponível" se falhar; verificar no emulador que o som toca no canal certo e que o stream fecha ao sair e com Home
- [x] 5.3 Implementar a tela Áudio (Esquerdo, Direito, Os dois, Varredura com a frequência atual e Sincronia com o quadrado de 50 ms, ajuste de ±300 ms em passos de 10 e o texto "atrasado/adiantado", D8); testes do ajuste e de que o ajuste é descartado ao sair
- [ ] 5.4 Verificar a sincronia na TCL pelo alto-falante da TV e, se houver, por uma soundbar; registrar o atraso percebido em `docs/device-notes.md`

## 6. Ferramenta Rede

- [x] 6.1 Reaproveitar `network()` para o tipo e o IP, e ler o gateway de `/proc/net/route` com "—" quando for negado (D5); leitura a cada 2 s só com a tela visível e a interface parada; teste do parser de `/proc/net/route`
- [x] 6.2 (Java autorizado) Adicionar o comando `wifi-signal` em `LumeBridge.java` e a permissão `ACCESS_WIFI_STATE` no manifesto (D4), mais o parser em `shell/protocol.rs` com testes; verificar no build de release (ProGuard) que o comando responde na TCL
- [x] 6.3 Mostrar o sinal (dBm, barra, classificação pelas faixas da spec, link e faixa de frequência), só com Wi-Fi; testes de classificação nos limites -55, -67 e -75
- [ ] 6.4 Adicionar `ureq` + `rustls` + `webpki-roots` só no alvo Android e implementar o teste de velocidade na thread `lume-test` (latência como mediana de 5, download até 25 MB ou 10 s, upload até 10 MB ou 10 s, tempos limite, cancelamento, D3); no desktop o teste responde "indisponível"; verificar na TCL que os valores batem, com até 20% de diferença, com outro medidor na mesma rede
- [x] 6.5 Mostrar o aviso de serviço externo, as fases com valor parcial e os resultados, e as mensagens de falha; testes da máquina de estados (iniciar, cancelar, falha, repetir) com progresso simulado

## 7. Ferramenta Armazenamento

- [x] 7.1 Implementar o teste (D9): espaço livre e total, mínimo de 256 MB livres, escrita de 64 MB em blocos de 1 MB com `fsync`, `posix_fadvise(DONTNEED)`, leitura, MB/s, apagar no fim, ao cancelar e ao abrir; testes da máquina de estados e da limpeza do arquivo esquecido
- [x] 7.2 Medir na TCL e na AIWA; se a AIWA passar de 20 s, reduzir para 32 MB (Open Question do design) e registrar em `docs/device-notes.md`

## 8. Specs, orçamentos e verificação nas TVs

- [x] 8.1 Rodar `cargo test --workspace`, `cargo clippy --workspace --all-targets` e `cargo fmt --all`, tudo limpo
- [x] 8.2 Gerar as capturas no desktop (menu e cada ferramenta, `--locale en` e `pt-BR`) e conferir a régua com `scripts/measure-ink.py`
- [x] 8.3 Gerar o APK de release (`scripts/container-build.sh release`) e verificar com `uv run scripts/verify-apk.py` (até 40 MB) e `aapt dump permissions` (só as três permissões); registrar o aumento do APK em `docs/device-notes.md`
- [ ] 8.4 Medir na TCL e na AIWA: PSS antes de entrar, durante o teste de velocidade e depois de voltar (até 2 MB de diferença), quadros perdidos no teste de movimento e ausência de quadros com padrão estático (`dumpsys SurfaceFlinger --latency`); registrar em `docs/device-notes.md`
- [ ] 8.5 Confirmar na TCL e na AIWA que a tela não apaga numa limpeza de 30 minutos e que Voltar e Home interrompem cada modo e liberam os recursos
- [x] 8.6 Validar com `openspec validate ferramentas --strict`
- [x] 8.7 Revisar a tela Sobre (Configurações do Lume › Sobre o Lume): incluir `ureq`, `rustls`, `ring` e `webpki-roots` com as licenças em `licenses.rs`; verificar com `cargo run -p lume-desktop -- --screenshot about`

## Workflow follow-up

- Commits por tema só quando o autor pedir.
- Fechamento com `/opsx:archive` e merge squash na `main`.
