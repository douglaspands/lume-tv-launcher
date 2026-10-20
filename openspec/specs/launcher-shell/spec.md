# launcher-shell Specification

## Purpose
Define a casca do launcher lume: registro como tela inicial, barra superior, navegação entre abas, tela de Configurações do lume e comportamento dos botões do controle remoto.

## Requirements

### Requirement: Registro como tela inicial
O lume SHALL se declarar como atividade de tela inicial (HOME) e como app de TV (LEANBACK_LAUNCHER), para poder ser escolhido como launcher padrão e aparecer na lista de apps do sistema.

#### Scenario: Escolha como launcher padrão
- **WHEN** o usuário (ou um comando adb) define o lume como atividade HOME padrão
- **THEN** pressionar o botão Home do controle abre o lume na tela Início

#### Scenario: Instância única
- **WHEN** o botão Home é pressionado com o lume já aberto
- **THEN** o lume não cria uma segunda instância e volta para a aba Início, com o foco no primeiro elemento do conteúdo

### Requirement: Barra superior
A barra superior SHALL exibir o logo do Lume, as abas "Início" e "Apps", o botão de entradas, o botão de Configurações (ícone de engrenagem) e o relógio, sobre o papel de parede. O botão de entradas SHALL ficar à esquerda da engrenagem, com o mesmo ícone do menu de entradas do launcher do Google TV, e SHALL abrir o menu de entradas da TV (ação `com.android.tv.action.VIEW_INPUTS`) ao receber OK. Se o sistema não resolver essa ação, o botão SHALL NOT ser exibido nem receber foco. A engrenagem SHALL abrir diretamente as configurações do Android ao receber OK. O indicador de uma aba SHALL ser um sublinhado sob o nome. Com o foco na barra superior (abas, entradas ou engrenagem), o sublinhado SHALL ser azul (a cor de destaque do tema); com o foco no conteúdo (dock ou grade), ele SHALL aparecer na aba ativa em branco, para indicar que o usuário não está controlando as abas. A mudança de cor SHALL acompanhar a animação do foco. Mover o foco para uma aba (Esquerda ou Direita na barra superior) SHALL exibir essa aba imediatamente, sem exigir OK, e o sublinhado SHALL aparecer sob ela; o foco SHALL permanecer na aba. OK sobre uma aba SHALL não mudar nada além do que o foco já mudou. As abas SHALL NOT ter contorno em volta do nome, nem em foco. A aba que não está ativa nem em foco SHALL aparecer em cinza claro. Os botões de entradas e de engrenagem em foco SHALL ter um contorno azul (a cor de destaque do tema).

#### Scenario: Aba ativa destacada
- **WHEN** a aba Apps está selecionada e o foco está na grade
- **THEN** o rótulo "Apps" aparece em destaque com o sublinhado azul, e "Início" aparece em cinza claro, sem contorno

#### Scenario: Sublinhado branco no conteúdo
- **WHEN** o foco está num item do dock ou da grade de Apps
- **THEN** o sublinhado da aba ativa é branco

#### Scenario: Sublinhado azul na barra
- **WHEN** o foco sobe para uma aba, para o botão de entradas ou para a engrenagem
- **THEN** o sublinhado fica azul, e volta a ficar branco quando o foco desce de novo

#### Scenario: Aba em foco sem contorno
- **WHEN** o foco chega a uma aba da barra superior
- **THEN** o nome da aba não tem contorno e o sublinhado azul aparece sob ele, sem precisar pressionar OK

#### Scenario: Trocar de aba só com o foco
- **WHEN** a aba Início está ativa, o foco está nela e o usuário pressiona Direita, chegando a "Apps"
- **THEN** a tela passa a mostrar a grade da aba Apps sem pressionar OK, o sublinhado azul fica sob "Apps" e o foco continua na aba

#### Scenario: Voltar à aba Início
- **WHEN** o foco está em "Apps" e o usuário pressiona Esquerda
- **THEN** a tela volta a mostrar a tela Início, com o sublinhado sob "Início"

#### Scenario: Passar pela aba a caminho da engrenagem
- **WHEN** o usuário pressiona Direita duas vezes a partir da aba Início
- **THEN** a aba Apps é exibida ao passar por ela, e o foco chega ao botão de entradas (o vizinho da engrenagem) com o sublinhado sob "Apps"

#### Scenario: Foco na engrenagem
- **WHEN** o foco chega ao botão de entradas ou ao de engrenagem
- **THEN** o botão ganha um contorno azul

#### Scenario: Abrir as entradas da TV
- **WHEN** o foco está no botão de entradas e o usuário pressiona OK numa TV que resolve `com.android.tv.action.VIEW_INPUTS`
- **THEN** o menu de entradas da TV é aberto

#### Scenario: TV sem menu de entradas
- **WHEN** o sistema não resolve `com.android.tv.action.VIEW_INPUTS`
- **THEN** a barra superior não exibe o botão de entradas, e a engrenagem fica na mesma posição

#### Scenario: Engrenagem abre o Android
- **WHEN** o foco está na engrenagem e o usuário pressiona OK
- **THEN** a tela de configurações do Android é aberta, e não as configurações do Lume

### Requirement: Relógio
O relógio SHALL mostrar a hora local do dispositivo, respeitar a preferência de 12 h ou 24 h do sistema e se atualizar na virada de cada minuto.

#### Scenario: Virada de minuto
- **WHEN** o horário do sistema passa de 21:45:59 para 21:46:00 com o lume visível
- **THEN** o relógio exibe 21:46 em até 1 segundo

#### Scenario: Formato 12 h
- **WHEN** o sistema está configurado para 12 horas
- **THEN** o relógio exibe a hora no formato de 12 horas

### Requirement: Navegação por controle remoto
Toda a interface SHALL ser operável apenas com D-pad (cima, baixo, esquerda, direita), OK/Enter e Voltar, sem exigir toque ou mouse. Exatamente um elemento SHALL estar focado por vez, indicado pelo estilo de foco do tema.

#### Scenario: Ir do conteúdo para a barra superior
- **WHEN** o foco está na primeira linha de conteúdo e o usuário pressiona Cima
- **THEN** o foco vai para a aba ativa na barra superior

#### Scenario: Trocar de aba
- **WHEN** o foco está na aba "Início" e o usuário pressiona Direita e depois OK
- **THEN** a aba "Apps" é ativada e a grade de apps é exibida

### Requirement: Botão Voltar
Na raiz da tela Início, o botão Voltar SHALL devolver o foco ao primeiro item do dock, sem fechar o lume. Em telas secundárias (Apps, Configurações, edição, Sobre), Voltar SHALL retornar ao nível anterior.

#### Scenario: Voltar na raiz
- **WHEN** o usuário está na aba Início com o foco num item do dock e pressiona Voltar
- **THEN** o foco vai para o primeiro item do dock e o lume continua aberto

#### Scenario: Voltar na aba Apps
- **WHEN** o usuário está na aba Apps e pressiona Voltar
- **THEN** a aba Início volta a ficar ativa

### Requirement: Retorno depois de abrir um app
Quando o usuário abre um app a partir do lume (OK num item do dock, num tile da grade de Apps ou nos itens que abrem as Configurações do Android) e volta ao lume com o botão Home, o lume SHALL reaparecer na mesma tela, com o foco no mesmo elemento e com a rolagem de antes, em vez de ir à tela Início. Esse retorno SHALL valer uma única vez e SHALL ser descartado assim que o usuário pressionar qualquer outra tecla no lume, ou quando o app não puder ser aberto. O botão Home pressionado com o lume já visível na tela (ou mais de 2 segundos depois de ele voltar) SHALL sempre levar à aba Início, com o foco no primeiro item do dock. O estado SHALL ficar só na memória: se o sistema encerrar o processo do lume, ele abre na tela Início.

#### Scenario: Voltar à aba Apps no mesmo ícone
- **WHEN** o usuário está na aba Apps, move o foco até o YouTube, pressiona OK e depois volta ao lume pelo botão Home
- **THEN** a aba Apps é exibida com o foco no ícone do YouTube

#### Scenario: Voltar ao dock no mesmo item
- **WHEN** o usuário abre o quarto item do dock e volta ao lume pelo botão Home
- **THEN** a tela Início é exibida com o foco no quarto item, e não no primeiro

#### Scenario: Grade rolada
- **WHEN** a grade de Apps estava rolada até uma linha distante, e o app dessa linha foi aberto
- **THEN** ao voltar, a grade mostra a mesma linha, com o foco no mesmo app

#### Scenario: Home com o lume visível
- **WHEN** o lume está na tela, em qualquer aba ou tela, mesmo depois de o usuário ter aberto um app, e o usuário pressiona Home
- **THEN** a aba Início é exibida, com o foco no primeiro item do dock

#### Scenario: Retorno usado uma vez
- **WHEN** o usuário volta de um app pelo Home e pressiona Home de novo com o lume aberto
- **THEN** o segundo Home leva à aba Início, com o foco no primeiro item do dock

#### Scenario: Tecla pressionada antes do Home
- **WHEN** o usuário abre um app, volta ao lume pelo botão Voltar do app, move o foco e depois pressiona Home
- **THEN** o lume vai à aba Início, com o foco no primeiro item do dock

#### Scenario: App que não abre
- **WHEN** o lume não consegue abrir o app escolhido (por exemplo, foi desinstalado)
- **THEN** nada é guardado para o retorno, e o próximo Home leva à aba Início

### Requirement: Suavização entre as abas
Ao trocar de aba (Início e Apps), o conteúdo da aba nova SHALL aparecer com um fade de cerca de 180 ms, em vez de surgir de uma vez; a barra superior e o papel de parede SHALL permanecer fixos. Terminado o fade, o laço SHALL deixar de pedir quadros.

#### Scenario: Trocar de aba
- **WHEN** o usuário move o foco de "Início" para "Apps"
- **THEN** a grade de Apps surge gradualmente em cerca de 180 ms, enquanto a barra superior e o papel de parede ficam parados

#### Scenario: Depois do fade
- **WHEN** o fade termina
- **THEN** a tela volta a pedir quadros só para o relógio

### Requirement: Tela de Configurações do Lume
A tela "Configurações" do Lume SHALL ser aberta pelo tile "Lume" da grade de Apps, com os itens "Tela Início" (abre o modo edição) e "Sobre o Lume". Ela SHALL NOT ter item para as configurações do Android, nem ser aberta pela engrenagem. A tela SHALL exibir, à direita dos itens, o painel de estatísticas da TV (capacidade `device-stats`). A tela SHALL exibir o caminho "Configurações" e, no canto inferior direito, a versão do build instalado (capacidade `platform-compatibility`, requisito "Versão do build"). Uma versão de release SHALL aparecer com "v" na frente (ex.: "Versão v0.3.0"); uma versão de build local ou `dev` SHALL aparecer como é (ex.: "Versão 4d8438b-dirty"). As telas abaixo dela SHALL exibir o caminho de navegação a partir de "Configurações" (ex.: "Configurações › Tela Início"). Voltar SHALL retraçar o caminho de entrada, um passo por vez: da tela de edição para a tela "Configurações" (com o foco em "Tela Início") e dela para o tile "Lume" da aba Apps. Voltar da tela SHALL devolver o foco ao tile "Lume" na aba Apps.

#### Scenario: Abrir pelo tile
- **WHEN** o foco está no tile "Lume" da grade de Apps e o usuário pressiona OK
- **THEN** a tela "Configurações" do Lume é exibida com o foco no primeiro item

#### Scenario: Painel de estatísticas visível
- **WHEN** a tela "Configurações" do Lume é exibida
- **THEN** o painel de estatísticas aparece à direita dos itens, sem receber foco

#### Scenario: Caminho e versão
- **WHEN** a tela "Configurações" do Lume é exibida
- **THEN** o caminho "Configurações" aparece no topo, e a versão instalada no canto inferior direito

#### Scenario: Versão de release com "v"
- **WHEN** o build instalado veio da tag `v0.3.0` e a tela "Configurações" ou a tela Sobre é exibida
- **THEN** a versão aparece como "v0.3.0", com um "v" só

#### Scenario: Versão de build local
- **WHEN** o build instalado foi gerado localmente no commit `4d8438b` com mudanças não commitadas
- **THEN** a versão aparece como "4d8438b-dirty", sem "v"

#### Scenario: Sem item do Android
- **WHEN** a tela "Configurações" do Lume é exibida
- **THEN** ela lista apenas "Tela Início" e "Sobre o Lume"

#### Scenario: Voltar ao tile
- **WHEN** o usuário pressiona Voltar na tela "Configurações" do Lume
- **THEN** a aba Apps é exibida com o foco no tile "Lume"

#### Scenario: Ir para a edição da tela Início
- **WHEN** o usuário seleciona "Tela Início" em Configurações
- **THEN** o modo edição é aberto com o caminho "Configurações › Tela Início" e o título "Tela Início"

#### Scenario: Voltar passo a passo
- **WHEN** o usuário está no modo edição aberto pelas Configurações e pressiona Voltar duas vezes
- **THEN** a primeira volta à tela "Configurações" com o foco em "Tela Início", e a segunda à aba Apps com o foco no tile "Lume"

#### Scenario: Tela Sobre
- **WHEN** o usuário escolhe "Sobre o Lume"
- **THEN** é exibida uma tela com a versão do build (a mesma do canto da tela "Configurações"), a licença (GPL-3.0-or-later), o endereço do código-fonte `https://github.com/douglaspands/lume-tv-launcher` e as licenças de terceiros (fonte, ícones e bibliotecas)

### Requirement: Nome do app com L maiúsculo
O nome do app SHALL ser escrito "Lume" em todo texto que o usuário lê: o rótulo do app no sistema, a barra superior, o tile da grade, a tela Sobre e as telas de configurações, em todos os idiomas fornecidos. Identificadores técnicos (pacote, biblioteca nativa, tag de log) SHALL NOT mudar.

#### Scenario: Rótulo no sistema
- **WHEN** o usuário vê o Lume na lista de apps ou nas configurações de aplicativos do Android
- **THEN** o nome exibido é "Lume"

#### Scenario: Tela Sobre
- **WHEN** a tela Sobre é exibida em português ou em inglês
- **THEN** o título é "Sobre o Lume" ou "About Lume"
