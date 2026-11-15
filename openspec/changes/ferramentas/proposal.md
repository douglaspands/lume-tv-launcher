## Why

Quem tem uma TV com retenção de imagem, um pixel preso, o som fora de sincronia ou uma internet que trava não tem onde testar isso no próprio aparelho. Os apps de teste da loja são pesados, cheios de anúncios ou nem existem para Android TV. O Lume já roda sempre e desenha a tela inteira com OpenGL, então pode oferecer um conjunto pequeno de ferramentas honestas: elas recuperam o que software consegue recuperar (retenção de imagem, às vezes um pixel preso) e, para o resto, mostram o defeito com clareza para o usuário saber o que tem.

## What Changes

- Novo item **"Ferramentas"** nas Configurações do Lume, que abre um menu com seis ferramentas:
  - **Tela**: padrões de diagnóstico (cores sólidas, cinzas, rampas, grade), limpeza de retenção de imagem com tempo definido e destravamento de pixel preso com uma janela pequena que o usuário posiciona pelo controle.
  - **Imagem**: padrão de overscan e geometria, e teste de movimento com a contagem de quadros perdidos.
  - **Controle remoto**: mostra cada botão apertado, com nome, código e intervalo de repetição.
  - **Áudio**: tom no canal esquerdo, no direito e nos dois, varredura de frequência, e teste de sincronia entre áudio e imagem com ajuste de atraso.
  - **Rede**: tipo de conexão, IP e gateway, sinal do Wi-Fi (dBm, velocidade do link, faixa) e teste de velocidade da internet (latência, download e upload) contra `speed.cloudflare.com`, só quando o usuário pede.
  - **Armazenamento**: velocidade de escrita e leitura sequencial, com um arquivo temporário apagado ao fim.
- **Ativação sob demanda**: cada ferramenta cria o próprio estado e os recursos pesados (stream de áudio, cliente HTTPS, thread de teste, arquivo temporário) ao entrar ou ao começar o teste, e libera tudo ao sair. Fora das Ferramentas, o custo de memória é zero.
- **Segurança contra flashes**: aviso e confirmação antes de qualquer ciclo rápido de cores. O ciclo rápido fica restrito à janela pequena, e os modos de tela cheia mudam a menos de 3 vezes por segundo.
- Tela ligada durante as sessões longas, Voltar e Home sempre interrompem, e o tempo restante aparece na tela.
- **Rede**: o Lume passa a abrir conexões de rede, mas só durante o teste de velocidade iniciado pelo usuário. A permissão `INTERNET`, que já está no manifesto para listar as interfaces, passa a constar na spec, e entra a permissão `ACCESS_WIFI_STATE`.
- **Java autorizado pelo autor** só para ler o sinal do Wi-Fi: um comando novo no `LumeBridge`, porque o `WifiManager` não tem equivalente no NDK.
- Fica de fora: "liberar memória" (no Android piora o uso e engana o usuário) e o timer de desligar (um app comum não consegue desligar a TV).

## Capabilities

### New Capabilities
- `diagnostic-tools`: o menu Ferramentas e as regras comuns a todas: navegação, ativação e liberação sob demanda, tela ligada, saída, aviso de flashes e tempo restante.
- `display-diagnostics`: as ferramentas Tela e Imagem, com padrões de diagnóstico, limpeza de retenção, destravamento de pixel, overscan e movimento.
- `audio-diagnostics`: a ferramenta Áudio, com canais, varredura e sincronia entre áudio e imagem.
- `network-diagnostics`: a ferramenta Rede, com conexão, sinal do Wi-Fi e teste de velocidade.
- `storage-diagnostics`: a ferramenta Armazenamento, com a velocidade de escrita e leitura.

### Modified Capabilities
- `launcher-shell`: a tela Configurações ganha o item "Ferramentas", e Voltar passo a passo passa a incluir o caminho pelas ferramentas.
- `remote-mapping`: entra o teste do controle remoto, que mostra o nome, o código e a repetição de cada botão, e que reaproveita a tabela de teclas.
- `resource-efficiency`: exceção à renderização sob demanda enquanto uma ferramenta anima, e memória das ferramentas liberada ao sair.
- `platform-compatibility`: as permissões passam a ser `REQUEST_DELETE_PACKAGES`, `INTERNET` e `ACCESS_WIFI_STATE`, e o tráfego de rede fica restrito ao teste de velocidade iniciado pelo usuário.
- `design-system`: layout do menu Ferramentas e das telas de ferramenta na régua, e regra das telas de padrão de tela cheia (sem papel de parede, sem camada translúcida).

## Impact

- **Rust (`lume-core`)**: `Screen::Tools` e uma tela por ferramenta, um módulo `app/tools/` com o estado de cada ferramenta (`Option<ToolState>`), novos `Effect` (manter a tela ligada, tocar e parar tons, iniciar e cancelar testes), strings nos 5 idiomas (`values`, `-pt-rBR`, `-es`, `-id`, `-ru`).
- **Rust (`lume-app`)**: o stream AAudio (feature `audio` do crate `ndk`), `KEEP_SCREEN_ON` via `AndroidApp::set_window_flags`, a thread do teste de velocidade, o teste de armazenamento e a leitura do gateway, além do novo comando de bridge.
- **Dependências novas**: um cliente HTTPS em Rust (`ureq` com `rustls` e raízes `webpki-roots`), que aumenta o APK, entra na tela Sobre com as licenças e fica dentro do limite de 40 MB.
- **Java**: o comando `wifi-signal` em `LumeBridge.java`, a permissão `ACCESS_WIFI_STATE` no manifesto e, se precisar, uma regra `-keep` no ProGuard.
- **`lume-desktop`**: renderiza as telas para captura. Áudio, Wi-Fi e rede ficam sem efeito no desktop.
- **Docs**: `docs/device-notes.md` ganha as medidas nas duas TVs. A tela Sobre é revisada.
