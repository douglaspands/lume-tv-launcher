## Purpose

Define as ferramentas Tela e Imagem: padrões de tela cheia para diagnosticar defeitos do painel e do backlight, limpeza de retenção de imagem, destravamento de pixel preso, padrão de overscan e geometria, e teste de movimento.

## ADDED Requirements

### Requirement: Modos da ferramenta Tela
A ferramenta Tela SHALL oferecer três modos, cada um com uma descrição: "Diagnóstico" (padrões para encontrar pixels mortos ou presos, manchas, vazamento de luz e falhas do backlight), "Limpar retenção" (remove imagem fantasma temporária) e "Destravar pixel" (tenta destravar um pixel preso aceso).

#### Scenario: Lista de modos
- **WHEN** a ferramenta Tela é aberta
- **THEN** ela lista "Diagnóstico", "Limpar retenção" e "Destravar pixel", cada um com a sua descrição

### Requirement: Padrões de diagnóstico
O modo Diagnóstico SHALL mostrar em tela cheia, nesta ordem: preto, branco, vermelho, verde, azul, cinza 5%, cinza 20%, cinza 50%, rampa de cinza, rampas de vermelho, verde e azul, e grade. As setas Esquerda e Direita SHALL trocar de padrão, em ciclo. O nome do padrão SHALL aparecer por 3 segundos ao entrar e a cada troca.

#### Scenario: Trocar de padrão
- **WHEN** o padrão branco está na tela e o usuário pressiona Direita
- **THEN** o padrão vermelho aparece, com o nome "Vermelho" visível por 3 segundos

#### Scenario: Ciclo
- **WHEN** o último padrão está na tela e o usuário pressiona Direita
- **THEN** o primeiro padrão (preto) aparece

### Requirement: Padrões limpos em pixel físico
Todo padrão de tela cheia das ferramentas Tela e Imagem SHALL ocupar a tela inteira sem papel de parede, barra superior nem camada translúcida, e sem texto depois que a dica some. Cores sólidas, linhas e grades SHALL ser desenhadas na resolução física do painel, sem escala nem suavização. As rampas SHALL seguir a regra de degradês sem faixas do `design-system`.

#### Scenario: Tela limpa
- **WHEN** um padrão sólido está na tela há mais de 3 segundos
- **THEN** a tela inteira tem a cor do padrão, sem texto, ícone nem borda

#### Scenario: Grade em pixel físico
- **WHEN** a grade é mostrada numa TV 1920x1080
- **THEN** as linhas têm exatamente 1 pixel físico de largura, sem suavização

### Requirement: Limpeza de retenção de imagem
O modo "Limpar retenção" SHALL deixar o usuário escolher a duração (10, 30 ou 60 minutos) e o padrão: "Branco" (tela branca fixa) ou "Cores" (ciclo de tela cheia por branco, vermelho, verde, azul e preto). No padrão "Cores", cada cor SHALL ficar pelo menos 2 segundos na tela, e cada transição SHALL durar pelo menos 1 segundo.

#### Scenario: Escolha antes de começar
- **WHEN** o usuário abre "Limpar retenção"
- **THEN** ele escolhe a duração e o padrão, e a limpeza só começa com OK em "Iniciar"

#### Scenario: Ciclo de cores lento
- **WHEN** a limpeza "Cores" está em andamento
- **THEN** cada cor fica pelo menos 2 segundos na tela, e a passagem para a seguinte dura pelo menos 1 segundo

### Requirement: Posicionamento da janela de destravamento
O modo "Destravar pixel" SHALL começar mostrando uma janela quadrada de 64 pixels físicos sobre fundo preto, no centro. As setas SHALL mover a janela em passos de 16 pixels físicos (8 com a tecla mantida pressionada), sem que ela saia da tela, e OK SHALL fixar a posição e passar ao aviso de flashes de `diagnostic-tools`.

#### Scenario: Posicionar a janela
- **WHEN** a janela está no centro e o usuário pressiona Direita
- **THEN** a janela anda 16 pixels físicos para a direita

#### Scenario: Limite da tela
- **WHEN** a janela encosta na borda direita e o usuário pressiona Direita
- **THEN** a janela não sai da tela

### Requirement: Ciclo de destravamento de pixel
Depois da confirmação, a janela SHALL alternar vermelho, verde, azul, branco e preto a cada quadro, pela duração escolhida (5, 10 ou 20 minutos), e o resto da tela SHALL ficar preto. Voltar durante o ciclo SHALL parar e devolver a janela ao posicionamento, na mesma posição.

#### Scenario: Ciclo restrito à janela
- **WHEN** o ciclo está em andamento
- **THEN** só a janela de 64 pixels físicos muda de cor, e o resto da tela fica preto

#### Scenario: Interromper e reposicionar
- **WHEN** o ciclo está em andamento e o usuário pressiona Voltar
- **THEN** o ciclo para, e a janela volta ao posicionamento, na mesma posição

### Requirement: Padrão de overscan e geometria
A ferramenta Imagem SHALL oferecer o padrão "Overscan e geometria", com uma borda de 1 pixel físico no limite exato da imagem, marcas de 2,5% e 5% em cada lado, um círculo e um quadrado centrais e as diagonais.

#### Scenario: Borda cortada
- **WHEN** o padrão de overscan está na tela e a TV corta 2% de cada lado
- **THEN** a borda de 1 pixel não aparece, mas a marca de 2,5% aparece, e o usuário sabe que a TV corta menos de 2,5%

#### Scenario: Círculo redondo
- **WHEN** o padrão de overscan é mostrado com a proporção certa
- **THEN** o círculo central aparece redondo, e o quadrado aparece com os lados iguais

### Requirement: Teste de movimento
A ferramenta Imagem SHALL oferecer o teste "Movimento": um bloco e um texto que atravessam a tela horizontalmente em velocidade constante, com velocidade ajustável pelas setas Cima e Baixo (lenta, média, rápida), e um contador de quadros perdidos (intervalos de 33 ms ou mais desde a entrada).

#### Scenario: Contador de quadros
- **WHEN** o teste de movimento roda e um quadro demora 50 ms
- **THEN** o contador de quadros perdidos aumenta em 1

#### Scenario: Velocidade
- **WHEN** o teste de movimento roda na velocidade média e o usuário pressiona Cima
- **THEN** o bloco e o texto passam a andar na velocidade rápida
