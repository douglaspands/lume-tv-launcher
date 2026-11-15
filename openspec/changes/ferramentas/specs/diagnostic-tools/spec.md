## Purpose

Reúne as ferramentas de teste e recuperação da TV num menu "Ferramentas" das Configurações do Lume e fixa as regras comuns a todas: navegação, ativação sob demanda, tela ligada, saída, proteção contra flashes e honestidade sobre o que cada ferramenta consegue fazer.

## ADDED Requirements

### Requirement: Menu Ferramentas
O Lume SHALL oferecer, a partir do item "Ferramentas" das Configurações, uma tela "Ferramentas" com o caminho "Configurações › Ferramentas" que lista, nesta ordem, "Tela", "Imagem", "Controle remoto", "Áudio", "Rede" e "Armazenamento". Cada item SHALL ter um nome e uma descrição curta de uma linha. OK num item SHALL abrir a ferramenta com o caminho "Configurações › Ferramentas › <ferramenta>". O menu SHALL NOT oferecer ferramentas que prometem liberar memória, acelerar ou otimizar a TV.

#### Scenario: Abrir o menu
- **WHEN** o usuário seleciona "Ferramentas" nas Configurações do Lume
- **THEN** a tela "Ferramentas" aparece com o caminho "Configurações › Ferramentas", as seis ferramentas na ordem definida e o foco na primeira

#### Scenario: Abrir uma ferramenta
- **WHEN** o foco está em "Áudio" no menu Ferramentas e o usuário pressiona OK
- **THEN** a ferramenta Áudio abre com o caminho "Configurações › Ferramentas › Áudio"

#### Scenario: Voltar passo a passo
- **WHEN** o usuário está numa ferramenta aberta pelo menu e pressiona Voltar até sair das Configurações
- **THEN** a primeira volta leva ao menu Ferramentas com o foco na ferramenta de origem, a segunda às Configurações com o foco em "Ferramentas", e a terceira à aba Apps com o foco no tile "Lume"

### Requirement: Ativação e liberação sob demanda
Cada ferramenta SHALL criar o próprio estado só quando é aberta e SHALL liberá-lo ao ser fechada. Os recursos pesados (saída de áudio, a thread de teste, arquivo temporário e buffers de teste) SHALL ser criados só quando o teste que os usa começa, e SHALL ser liberados quando ele termina, é cancelado ou a ferramenta fecha. Fora do menu Ferramentas, o Lume SHALL NOT manter memória, threads ou arquivos das ferramentas.

#### Scenario: Nenhum custo fora das ferramentas
- **WHEN** o usuário nunca abriu as Ferramentas, ou abriu e voltou às Configurações
- **THEN** não há thread, stream de áudio, conexão nem arquivo temporário das ferramentas, e o PSS em primeiro plano fica dentro do orçamento de `resource-efficiency`

#### Scenario: Recurso só durante o teste
- **WHEN** a ferramenta Áudio está aberta e nenhum tom toca
- **THEN** não há stream de áudio aberto

#### Scenario: Saída durante um teste
- **WHEN** um teste está em andamento e o usuário sai da ferramenta por Voltar ou Home, ou o Lume vai para segundo plano
- **THEN** o teste é cancelado, e os recursos dele são liberados em até 1 segundo

### Requirement: Saída sempre disponível
Voltar SHALL interromper qualquer ferramenta ou modo de teste e voltar ao passo anterior (com um teste em segundo plano rodando, o passo anterior é a ferramenta parada, com o resultado parcial descartado), exceto no teste do controle remoto, que segue `remote-mapping`. Home SHALL seguir o comportamento normal do Lume. Nenhuma ferramenta SHALL capturar Home ou exigir uma sequência de teclas para sair de um modo de tela cheia.

#### Scenario: Voltar num padrão de tela cheia
- **WHEN** um padrão de diagnóstico ou uma limpeza ocupa a tela inteira e o usuário pressiona Voltar
- **THEN** o padrão para na hora, e a tela da ferramenta reaparece com o foco no item que o iniciou

### Requirement: Tela ligada durante as sessões
Enquanto uma ferramenta estiver aberta, o Lume SHALL pedir ao sistema que mantenha a tela ligada, para que o protetor de tela e o desligamento por inatividade não interrompam um teste. O pedido SHALL ser desfeito ao sair das Ferramentas.

#### Scenario: Limpeza longa
- **WHEN** a limpeza de retenção roda por 30 minutos sem toque no controle
- **THEN** o protetor de tela não aparece e a tela não apaga

#### Scenario: Pedido desfeito
- **WHEN** o usuário volta do menu Ferramentas às Configurações
- **THEN** o protetor de tela volta a aparecer após o tempo normal de inatividade

### Requirement: Proteção contra flashes
Antes de qualquer modo que alterne cores de alto contraste mais de 3 vezes por segundo, o Lume SHALL mostrar um aviso sobre epilepsia fotossensível e só começar depois da confirmação com OK. Esse modo SHALL ocupar no máximo uma janela pequena (até 5% da área da tela). Os modos de tela cheia SHALL NOT trocar de cor mais de 3 vezes por segundo e SHALL fazer as transições de cor de forma gradual.

#### Scenario: Aviso antes do ciclo rápido
- **WHEN** o usuário inicia o destravamento de pixel
- **THEN** o aviso aparece primeiro, e nada pisca até o usuário confirmar com OK

#### Scenario: Recusa
- **WHEN** o aviso está na tela e o usuário pressiona Voltar
- **THEN** o modo não começa, e a tela da ferramenta reaparece

#### Scenario: Tela cheia sem flashes
- **WHEN** a limpeza de retenção de tela cheia está em andamento
- **THEN** a cor da tela muda no máximo 3 vezes por segundo

### Requirement: Tempo restante visível
Todo modo com duração (limpeza de retenção, destravamento de pixel) SHALL deixar o usuário escolher a duração antes de começar, SHALL terminar sozinho ao fim do tempo, voltando à tela da ferramenta, e SHALL mostrar o tempo restante. Nos modos de tela cheia, o tempo restante SHALL aparecer por 3 segundos ao começar e a cada tecla pressionada, e SHALL ficar escondido no resto do tempo, para não marcar a tela.

#### Scenario: Fim do tempo
- **WHEN** a limpeza de 10 minutos chega ao fim
- **THEN** a tela da ferramenta Tela reaparece com a mensagem "Limpeza concluída"

#### Scenario: Consultar o tempo
- **WHEN** uma limpeza de tela cheia está em andamento e o usuário pressiona uma seta
- **THEN** o tempo restante aparece por 3 segundos e some

### Requirement: Texto honesto sobre o resultado
Cada ferramenta de recuperação SHALL dizer, na própria tela, o que pode e o que não pode recuperar: a limpeza de retenção resolve imagem fantasma temporária, o destravamento de pixel às vezes resolve um pixel preso aceso, e pixel morto (sempre preto), linhas e defeitos de backlight não têm conserto por software. Nesses casos, os padrões servem só para diagnóstico.

#### Scenario: Descrição na ferramenta Tela
- **WHEN** a ferramenta Tela é aberta
- **THEN** cada modo mostra uma descrição que diz o que ele consegue fazer, sem prometer consertar defeitos de hardware
