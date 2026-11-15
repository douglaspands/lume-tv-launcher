# resource-efficiency Specification

## Purpose
Torna mensurável o requisito central do lume, que é consumir o mínimo de memória e de processamento em TVs com 2 GB de RAM, sobretudo quando outro app (ex.: um player de vídeo) está em primeiro plano.

## Requirements

### Requirement: Orçamento de memória em primeiro plano
Com o lume visível e ocioso na tela Início, a memória privada suja do processo (`Private Dirty` da linha `TOTAL` de `dumpsys meminfo`), descontada a da GPU (`GL mtrack` e `EGL mtrack`, que só alguns drivers informam), SHALL ser de no máximo 32 MB em cada TV de teste (TCL 55C715 e AIWA). O PSS não é o orçamento: ele inclui páginas do driver gráfico e do sistema que, num aparelho `low_ram`, caem quase todas na conta do Lume (na AIWA com Android 14, uns 34 MB que o Lume não controla). Ele continua medido e anotado em `docs/device-notes.md`, por aparelho.

#### Scenario: Medição em primeiro plano
- **WHEN** o lume está na tela Início há 30 segundos sem interação
- **THEN** `adb shell dumpsys meminfo <pacote>` reporta `Private Dirty` do `TOTAL`, menos `GL mtrack` e `EGL mtrack`, de no máximo 32 MB

#### Scenario: Driver que informa a GPU
- **WHEN** a mesma medição é feita na AIWA, cujo driver informa 14 MB de `GL mtrack`, e na TCL, cujo driver não informa
- **THEN** o valor comparado é o mesmo nas duas: a memória privada sem a GPU

### Requirement: Orçamento de memória em segundo plano
Quando outro app passa ao primeiro plano, o lume SHALL liberar superfície, contexto gráfico, texturas e atlas, e a memória privada suja do processo (`Private Dirty` do `TOTAL`) SHALL cair para no máximo 10 MB em até 10 segundos, em qualquer TV.

#### Scenario: Assistindo a outro app
- **WHEN** o usuário abre o YouTube pelo lume e espera 10 segundos
- **THEN** `dumpsys meminfo` do lume reporta `Private Dirty` ≤ 10 MB, sem memória de GL/Graphics atribuída

#### Scenario: Pressão de memória
- **WHEN** o sistema envia um aviso de pouca memória (trim memory) com o lume em segundo plano
- **THEN** o lume libera todos os caches recriáveis, mantendo só a configuração e o catálogo em memória

### Requirement: Retorno rápido
Ao pressionar Home, a parte do retorno sob controle do Lume (a partir da entrega da janela nativa pelo sistema até o primeiro quadro do splash aparecer e o Lume estar respondendo ao controle) SHALL levar no máximo 150 ms quando o processo do lume ainda existe (retorno a quente) e no máximo 800 ms de ponta a ponta quando o processo foi encerrado (abertura a frio), no dispositivo de referência. A interface completa (fim da revelação do splash, ver `launcher-shell`) SHALL estar na tela no máximo 600 ms mais a duração da revelação depois do primeiro quadro. O Lume SHALL registrar no log o tempo do primeiro quadro e o da interface pronta.

#### Scenario: Retorno a quente
- **WHEN** o lume está em segundo plano, com recursos gráficos liberados, o usuário pressiona Home e o sistema entrega a janela nativa
- **THEN** o primeiro quadro (o splash) aparece em até 150 ms após a criação da janela, e a tela Início completa em até 960 ms depois dele

#### Scenario: Abertura a frio
- **WHEN** o processo do lume foi encerrado e o usuário pressiona Home
- **THEN** o primeiro quadro aparece em até 800 ms, usando o catálogo em cache sem esperar pela consulta completa de apps, e a tela Início completa em até 1200 ms depois dele

#### Scenario: Log da interface pronta
- **WHEN** a revelação do splash termina
- **THEN** o log do Lume registra o tempo desde a janela e desde o início do processo

### Requirement: Renderização sob demanda
O lume SHALL desenhar quadros apenas quando algo visível muda (entrada do usuário, animação em curso, virada de minuto do relógio, mudança no catálogo, progresso de um teste das Ferramentas). Ocioso, SHALL NOT desenhar quadros nem manter threads acordadas em laço. O papel de parede e o dock SHALL NOT manter animação contínua. A única exceção são os modos das Ferramentas que animam por natureza (ciclo de cores da limpeza de retenção, destravamento de pixel, teste de movimento, quadrado do teste de sincronia), que SHALL pedir quadros contínuos só enquanto estão em andamento. O progresso de um teste em segundo plano (armazenamento) SHALL pedir no máximo 4 quadros por segundo.

#### Scenario: Tela ociosa
- **WHEN** a tela Início está visível, sem interação
- **THEN** o lume desenha no máximo 1 quadro por minuto (relógio), e o uso de CPU do processo fica abaixo de 1% em média

#### Scenario: Outras telas ociosas
- **WHEN** a aba Apps, as Configurações, a tela Sobre, o editor, o menu Ferramentas ou uma ferramenta sem modo em andamento estão visíveis, sem interação
- **THEN** o lume desenha no máximo 1 quadro por minuto (relógio), além das leituras periódicas que a própria tela exibe

#### Scenario: Magnification terminada
- **WHEN** a animação de Magnification do dock termina
- **THEN** o lume deixa de desenhar quadros até a próxima mudança visível

#### Scenario: Padrão estático das Ferramentas
- **WHEN** um padrão de diagnóstico sólido ou a limpeza "Branco" está na tela e o texto de dica já sumiu
- **THEN** o lume não desenha quadros até a próxima tecla ou o fim do tempo

#### Scenario: Modo animado das Ferramentas termina
- **WHEN** o usuário sai do teste de movimento ou o destravamento de pixel termina
- **THEN** o lume volta a desenhar só sob demanda

### Requirement: Fluidez da navegação
Animações de foco, rolagem e Magnification do dock SHALL rodar a 60 quadros por segundo no dispositivo de referência, com no máximo 1% de quadros perdidos durante a navegação contínua, mesmo com o papel de parede desenhado atrás do conteúdo. Na grade de Apps, o desenho dos tiles SHALL agrupar todas as caixas, todas as imagens e todos os textos em chamadas de desenho contíguas (sem partir o lote de texto no meio por causa do tile do próprio launcher), e as telas com rolagem (grade de Apps, editor, informações da TV e Sobre) SHALL descartar o desenho de itens fora da área visível (`viewport culling`).

#### Scenario: Rolagem da linha de favoritos
- **WHEN** o usuário mantém a seta Direita pressionada sobre os 12 favoritos do dock
- **THEN** o `dumpsys gfxinfo`/medição de quadros mostra ≤ 1% de quadros perdidos

#### Scenario: Rolagem da grade de Apps
- **WHEN** o usuário mantém a seta Baixo pressionada na grade de Apps com 30 apps
- **THEN** a medição de quadros mostra ≤ 1% de quadros perdidos

#### Scenario: Lote contíguo de desenho na grade de Apps
- **WHEN** a grade de Apps é desenhada com apps antes e depois do tile "Lume"
- **THEN** o lote de texto dos tiles não é interrompido por comandos de retângulo do logo do tile "Lume"

### Requirement: Boas práticas de loop engineering (engenharia de laços)
Caminhos quentes de layout e renderização por quadro SHALL NOT realizar alocações dinâmicas na heap. Algoritmos de elipsização de texto SHALL operar em passo único sem strings intermediárias. Coleções com rolagem SHALL aplicar descarte antecipado de itens fora da janela visível (viewport culling) antes de medições e despacho. Varreduras de catálogo SHALL suportar interrupção antecipada (short-circuit) com cota e deduplicação de chaves.

#### Scenario: Zero alocações em laço quente de texto da tela Sobre
- **WHEN** a tela Sobre é desenhada a cada quadro exibindo 340+ linhas de licença
- **THEN** as quebras de linha e textos de licença vêm de cache estático pré-computado, com zero alocações na heap por quadro de renderização

#### Scenario: Elipsização em passo único
- **WHEN** um título ou rótulo longo de aplicativo necessita de elipsização com reticências
- **THEN** a largura e o ponto de corte são calculados em passo único em tempo linear sem alocar strings intermediárias durante a busca da quebra

#### Scenario: Culling vertical em laços de renderização de listas e grades
- **WHEN** uma lista ou grade com dezenas de itens sofre rolagem vertical
- **THEN** itens com limites superiores ou inferiores completamente externos à janela visível são descartados antes de qualquer cálculo de texto ou despacho para o renderizador

#### Scenario: Interrupção antecipada em varredura de catálogo
- **WHEN** o launcher solicita um lote restrito de ícones ou aplicativos prioritários
- **THEN** o laço de varredura interrompe o processamento imediatamente ao atingir o limite estipulado, sem percorrer nem calcular hashes para o restante da coleção

### Requirement: Boas práticas de graph engineering (grafo de renderização e recursos GPU)
O grafo de renderização SHALL agrupar primitivas contíguas do mesmo tipo e textura para minimizar trocas de programa e chamadas de desenho na GPU. Texturas de alta resolução SHALL ter upload fatiado no tempo em linhas por quadro. VBOs de redução e buffers frequentes SHALL ser reutilizados na memória de vídeo, operações de leitura de pixels da GPU SHALL inverter linhas in-place e atlas de texturas SHALL reciclar slots via lista de reuso.

#### Scenario: Batching contíguo de comandos no grafo de renderização
- **WHEN** uma tela composta por múltiplos elementos visuais (fundos, ícones e textos) é desenhada
- **THEN** o grafo de renderização consolida os comandos emitindo primeiramente todos os retângulos, em seguida todas as texturas e por fim todos os glifos de texto, evitando alternância repetida entre programas de shader

#### Scenario: Upload particionado de papel de parede no tempo
- **WHEN** uma nova imagem de papel de parede é enviada à GPU
- **THEN** as linhas de pixels são transferidas em fatias sucessivas de no máximo 90 linhas por quadro, distribuindo o upload ao longo de múltiplos quadros para não bloquear a thread principal

#### Scenario: Reutilização de buffers e locations de shader
- **WHEN** operações de redução de escala de imagem ou desenho em lote ocorrem a cada quadro
- **THEN** os VBOs compartilhados e os uniform locations cacheados são reutilizados diretamente sem invocar chamadas de criação de buffer ou busca dinâmica de uniforms na GPU

#### Scenario: Reciclagem de slots no atlas de texturas
- **WHEN** um ícone de aplicativo tem seu conteúdo liberado ou atualizado no atlas
- **THEN** seu slot de coordenadas é adicionado à lista de reuso para alocação subsequente, sem necessidade de reconstrução ou redimensionamento do atlas

### Requirement: Tamanho do pacote
O APK de release SHALL ter no máximo 40 MB, contendo as duas arquiteturas suportadas. Os papéis de parede de alta definição SHALL entrar no APK uma única vez, fora das bibliotecas nativas de cada arquitetura, para que o limite não cresça com o número de arquiteturas.

#### Scenario: Verificação no CI
- **WHEN** o CI gera o APK de release
- **THEN** o build falha se o arquivo passar de 40 MB

#### Scenario: Papéis de parede fora das bibliotecas nativas
- **WHEN** o APK de release é verificado pelo script do projeto
- **THEN** a verificação falha se um papel de parede aparecer dentro de uma biblioteca nativa ou mais de uma vez no pacote

### Requirement: Memória das Ferramentas
As Ferramentas SHALL caber no orçamento de memória em primeiro plano (memória privada sem a GPU até 32 MB) com qualquer teste em andamento, e SHALL devolver a memória ao sair: ao voltar do menu Ferramentas às Configurações, a memória privada suja SHALL ficar a até 2 MB do valor medido antes de entrar. Nenhum teste SHALL manter mais que 1 MB de dados de teste em memória.

#### Scenario: Teste de armazenamento na TCL
- **WHEN** o teste de armazenamento está em andamento na TCL
- **THEN** o `dumpsys meminfo app.lume.tv` mostra memória privada sem a GPU abaixo de 32 MB

#### Scenario: Memória devolvida
- **WHEN** o usuário roda todos os testes e volta às Configurações
- **THEN** a memória privada suja fica a até 2 MB do valor medido antes de abrir as Ferramentas

### Requirement: Memória devolvida ao sistema
Buffers grandes (a partir de 512 KB: imagens de papel de parede, o corpo de um envio do celular, arquivos lidos dos assets) SHALL ter memória própria, devolvida ao sistema assim que deixam de ser usados, sem depender do alocador do aparelho. Depois de soltar um papel de parede, o lume SHALL também pedir ao alocador que devolva as páginas livres do heap.

#### Scenario: Imagem solta
- **WHEN** o usuário recebe uma foto do celular, cancela e volta ao editor
- **THEN** a memória privada suja volta a até 2 MB do valor de antes, também na AIWA (Android 14), cujo alocador guarda blocos livres

#### Scenario: Trocas seguidas
- **WHEN** o usuário troca de papel de parede várias vezes no editor
- **THEN** a memória privada suja não cresce a cada troca
