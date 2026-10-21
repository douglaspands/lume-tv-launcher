# resource-efficiency Specification

## Purpose
Torna mensurável o requisito central do lume, que é consumir o mínimo de memória e de processamento em TVs com 2 GB de RAM, sobretudo quando outro app (ex.: um player de vídeo) está em primeiro plano.

## Requirements

### Requirement: Orçamento de memória em primeiro plano
Com o lume visível e ocioso na tela Início, o TOTAL PSS do processo (medido por `dumpsys meminfo`, incluindo Graphics/GL) SHALL ser de no máximo 64 MB no dispositivo de referência (TCL 55C715) com renderização em 1080p e 30 apps instalados. O limite subiu de 45 MB porque o papel de parede passou a ter 1920x1080 (textura de 6,2 MB, contra 0,7 MB) e o catálogo de 10 papéis.

#### Scenario: Medição em primeiro plano
- **WHEN** o lume está na tela Início há 30 segundos sem interação
- **THEN** `adb shell dumpsys meminfo <pacote>` reporta TOTAL PSS ≤ 64 MB

### Requirement: Orçamento de memória em segundo plano
Quando outro app passa ao primeiro plano, o lume SHALL liberar superfície, contexto gráfico, texturas e atlas, e o TOTAL PSS do processo SHALL cair para no máximo 20 MB em até 10 segundos.

#### Scenario: Assistindo a outro app
- **WHEN** o usuário abre o YouTube pelo lume e espera 10 segundos
- **THEN** `dumpsys meminfo` do lume reporta TOTAL PSS ≤ 20 MB, sem memória de GL/Graphics atribuída

#### Scenario: Pressão de memória
- **WHEN** o sistema envia um aviso de pouca memória (trim memory) com o lume em segundo plano
- **THEN** o lume libera todos os caches recriáveis, mantendo só a configuração e o catálogo em memória

### Requirement: Retorno rápido
Ao pressionar Home, o primeiro quadro do Lume (o splash) SHALL aparecer, e o Lume SHALL estar respondendo ao controle, em até 300 ms quando o processo do lume ainda existe (retorno a quente) e em até 800 ms quando o processo foi encerrado (abertura a frio), no dispositivo de referência. A interface completa (fim da revelação do splash, ver `launcher-shell`) SHALL estar na tela no máximo 600 ms mais a duração da revelação depois do primeiro quadro. O Lume SHALL registrar no log o tempo do primeiro quadro e o da interface pronta.

#### Scenario: Retorno a quente
- **WHEN** o lume está em segundo plano, com recursos gráficos liberados, e o usuário pressiona Home
- **THEN** o primeiro quadro (o splash) aparece em até 300 ms, e a tela Início completa em até 960 ms depois dele

#### Scenario: Abertura a frio
- **WHEN** o processo do lume foi encerrado e o usuário pressiona Home
- **THEN** o primeiro quadro aparece em até 800 ms, usando o catálogo em cache sem esperar pela consulta completa de apps, e a tela Início completa em até 1200 ms depois dele

#### Scenario: Log da interface pronta
- **WHEN** a revelação do splash termina
- **THEN** o log do Lume registra o tempo desde a janela e desde o início do processo

### Requirement: Renderização sob demanda
O lume SHALL desenhar quadros apenas quando algo visível muda (entrada do usuário, animação em curso, virada de minuto do relógio, mudança no catálogo). Ocioso, SHALL NOT desenhar quadros nem manter threads acordadas em laço. O papel de parede e o dock SHALL NOT manter animação contínua.

#### Scenario: Tela ociosa
- **WHEN** a tela Início está visível, sem interação
- **THEN** o lume desenha no máximo 1 quadro por minuto (relógio), e o uso de CPU do processo fica abaixo de 1% em média

#### Scenario: Outras telas ociosas
- **WHEN** a aba Apps, as Configurações, a tela Sobre ou o editor estão visíveis, sem interação
- **THEN** o lume desenha no máximo 1 quadro por minuto (relógio)

#### Scenario: Magnification terminada
- **WHEN** a animação de Magnification do dock termina
- **THEN** o lume deixa de desenhar quadros até a próxima mudança visível

### Requirement: Fluidez da navegação
Animações de foco, rolagem e Magnification do dock SHALL rodar a 60 quadros por segundo no dispositivo de referência, com no máximo 1% de quadros perdidos durante a navegação contínua, mesmo com o papel de parede desenhado atrás do conteúdo.

#### Scenario: Rolagem da linha de favoritos
- **WHEN** o usuário mantém a seta Direita pressionada sobre os 12 favoritos do dock
- **THEN** o `dumpsys gfxinfo`/medição de quadros mostra ≤ 1% de quadros perdidos

#### Scenario: Rolagem da grade de Apps
- **WHEN** o usuário mantém a seta Baixo pressionada na grade de Apps com 30 apps
- **THEN** a medição de quadros mostra ≤ 1% de quadros perdidos

### Requirement: Tamanho do pacote
O APK de release SHALL ter no máximo 40 MB, contendo as duas arquiteturas suportadas. O limite subiu de 6 MB para acomodar os 10 papéis de parede de alta definição embutidos.

#### Scenario: Verificação no CI
- **WHEN** o CI gera o APK de release
- **THEN** o build falha se o arquivo passar de 40 MB
