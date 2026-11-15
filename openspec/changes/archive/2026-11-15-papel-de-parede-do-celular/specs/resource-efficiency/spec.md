## MODIFIED Requirements

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

### Requirement: Memória das Ferramentas
As Ferramentas SHALL caber no orçamento de memória em primeiro plano (memória privada sem a GPU até 32 MB) com qualquer teste em andamento, e SHALL devolver a memória ao sair: ao voltar do menu Ferramentas às Configurações, a memória privada suja SHALL ficar a até 2 MB do valor medido antes de entrar. Nenhum teste SHALL manter mais que 1 MB de dados de teste em memória.

#### Scenario: Teste de armazenamento na TCL
- **WHEN** o teste de armazenamento está em andamento na TCL
- **THEN** o `dumpsys meminfo app.lume.tv` mostra memória privada sem a GPU abaixo de 32 MB

#### Scenario: Memória devolvida
- **WHEN** o usuário roda todos os testes e volta às Configurações
- **THEN** a memória privada suja fica a até 2 MB do valor medido antes de abrir as Ferramentas

## ADDED Requirements

### Requirement: Memória devolvida ao sistema
Buffers grandes (a partir de 512 KB: imagens de papel de parede, o corpo de um envio do celular, arquivos lidos dos assets) SHALL ter memória própria, devolvida ao sistema assim que deixam de ser usados, sem depender do alocador do aparelho. Depois de soltar um papel de parede, o lume SHALL também pedir ao alocador que devolva as páginas livres do heap.

#### Scenario: Imagem solta
- **WHEN** o usuário recebe uma foto do celular, cancela e volta ao editor
- **THEN** a memória privada suja volta a até 2 MB do valor de antes, também na AIWA (Android 14), cujo alocador guarda blocos livres

#### Scenario: Trocas seguidas
- **WHEN** o usuário troca de papel de parede várias vezes no editor
- **THEN** a memória privada suja não cresce a cada troca
