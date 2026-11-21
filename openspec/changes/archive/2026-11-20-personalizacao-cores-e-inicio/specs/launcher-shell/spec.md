## ADDED Requirements

### Requirement: Modos de navegação da tela inicial
A casca do launcher SHALL suportar cinco configurações de navegação persistidas em `files/config.v1`: `Com abas` com alinhamento `À esquerda` (padrão) ou `Centralizadas`, e `Sem abas` nos modos `Descer`, `Espiada` ou `No Dock`.

#### Scenario: Padrão preservado em instalações existentes
- **WHEN** o Lume inicia com um `files/config.v1` anterior sem as chaves de navegação
- **THEN** ele abre no modo `Com abas — À esquerda`, idêntico ao comportamento anterior

#### Scenario: Alternar para modo sem abas
- **WHEN** o usuário seleciona `Sem abas` e `Espiada` em `Configurações › Tela Início` e volta à tela inicial
- **THEN** a barra superior deixa de mostrar as abas e a tela inicial passa a operar no modo `Espiada`

## MODIFIED Requirements

### Requirement: Barra superior
A barra superior SHALL exibir o logo do Lume (na cor de destaque ativa), a marca "Lume", o botão de entradas, o botão de Configurações (ícone de engrenagem) e o relógio, sobre o papel de parede. Quando `Abas na barra superior` estiver em `Com abas` (padrão), a barra SHALL exibir também as abas "Início" e "Apps", alinhadas à esquerda ou centralizadas conforme `Posição das abas`. Quando estiver em `Sem abas`, a barra SHALL omitir as abas e não exibir sublinhado; ao subir o foco do conteúdo para a barra sem abas, o foco SHALL ir diretamente para o botão de entradas (ou para a engrenagem quando o botão de entradas não existir). O botão de entradas SHALL ficar à esquerda da engrenagem, com o mesmo ícone do menu de entradas do launcher do Google TV, e SHALL abrir o menu de entradas da TV (ação `com.android.tv.action.VIEW_INPUTS`) ao receber OK. Se o sistema não resolver essa ação, o botão SHALL NOT ser exibido nem receber foco. A engrenagem SHALL abrir diretamente as configurações do Android ao receber OK. Com abas visíveis, o indicador de uma aba SHALL ser um sublinhado sob o nome: na cor primária ativa (`--primary`) com o foco na barra superior, e em branco (`--text`) com o foco no conteúdo. A mudança de cor SHALL acompanhar a animação do foco. Mover o foco para uma aba (Esquerda ou Direita na barra superior) SHALL exibir essa aba imediatamente, sem exigir OK, e o sublinhado SHALL aparecer sob ela; o foco SHALL permanecer na aba. OK sobre uma aba SHALL não mudar nada além do que o foco já mudou. As abas SHALL NOT ter contorno em volta do nome, nem em foco. A aba que não está ativa nem em foco SHALL aparecer em cinza claro. Os botões de entradas e de engrenagem em foco SHALL ter um contorno na cor primária ativa (`--primary`).

#### Scenario: Aba ativa destacada
- **WHEN** o modo `Com abas` está ativo, a aba Apps está selecionada e o foco está na grade
- **THEN** o rótulo "Apps" aparece em destaque com o sublinhado branco, e "Início" aparece em cinza claro, sem contorno

#### Scenario: Sublinhado branco no conteúdo
- **WHEN** o modo `Com abas` está ativo e o foco está num item do dock ou da grade de Apps
- **THEN** o sublinhado da aba ativa é branco

#### Scenario: Sublinhado azul na barra
- **WHEN** o modo `Com abas` está ativo e o foco sobe para uma aba, para o botão de entradas ou para a engrenagem
- **THEN** o sublinhado fica na cor primária ativa (`--primary`), e volta a ficar branco quando o foco desce de novo

#### Scenario: Aba em foco sem contorno
- **WHEN** o foco chega a uma aba da barra superior
- **THEN** o nome da aba não tem contorno e o sublinhado na cor primária aparece sob ele, sem precisar pressionar OK

#### Scenario: Trocar de aba só com o foco
- **WHEN** a aba Início está ativa, o foco está nela e o usuário pressiona Direita, chegando a "Apps"
- **THEN** a tela passa a mostrar a grade da aba Apps sem pressionar OK, o sublinhado fica sob "Apps" e o foco continua na aba

#### Scenario: Voltar à aba Início
- **WHEN** o foco está em "Apps" e o usuário pressiona Esquerda
- **THEN** a tela volta a mostrar a tela Início, com o sublinhado sob "Início"

#### Scenario: Passar pela aba a caminho da engrenagem
- **WHEN** o usuário pressiona Direita duas vezes a partir da aba Início
- **THEN** a aba Apps é exibida ao passar por ela, e o foco chega ao botão de entradas (o vizinho da engrenagem) com o sublinhado sob "Apps"

#### Scenario: Foco na engrenagem
- **WHEN** o foco chega ao botão de entradas ou ao de engrenagem
- **THEN** o botão ganha um contorno na cor primária ativa

#### Scenario: Abrir as entradas da TV
- **WHEN** o foco está no botão de entradas e o usuário pressiona OK numa TV que resolve `com.android.tv.action.VIEW_INPUTS`
- **THEN** o menu de entradas da TV é aberto

#### Scenario: TV sem menu de entradas
- **WHEN** o sistema não resolve `com.android.tv.action.VIEW_INPUTS`
- **THEN** a barra superior não exibe o botão de entradas, e a engrenagem fica na mesma posição

#### Scenario: Engrenagem abre o Android
- **WHEN** o foco está na engrenagem e o usuário pressiona OK
- **THEN** a tela de configurações do Android é aberta, e não as configurações do Lume

#### Scenario: Foco na barra superior sem abas
- **WHEN** o modo `Sem abas` está ativo, o foco está no dock e o usuário pressiona Cima
- **THEN** o foco vai diretamente para o botão de entradas (ou para a engrenagem se o sistema não resolver `VIEW_INPUTS`), sem exibir abas nem sublinhado

### Requirement: Botão Voltar
Na raiz da tela Início (com o foco no dock), o botão Voltar SHALL devolver o foco ao primeiro item do dock, sem fechar o lume. Na grade de Apps, Voltar SHALL retornar ao nível anterior: à aba Início no modo `Com abas`, ao dock (retornando ao topo) nos modos `Descer` e `Espiada`, e à tela Início com o foco no botão "Todos os apps" do dock no modo `No Dock`. Em telas secundárias (Configurações, edição, Mapeamento de controle, Informações da TV, Ferramentas, Sobre), Voltar SHALL retornar ao nível anterior, devolvendo o foco ao tile "Lume" na grade de Apps ao sair de Configurações.

#### Scenario: Voltar na raiz
- **WHEN** o usuário está na aba Início com o foco num item do dock e pressiona Voltar
- **THEN** o foco vai para o primeiro item do dock e o lume continua aberto

#### Scenario: Voltar na aba Apps
- **WHEN** o usuário está na aba Apps (modo `Com abas`) e pressiona Voltar
- **THEN** a aba Início volta a ficar ativa

#### Scenario: Voltar na grade no modo Descer ou Espiada
- **WHEN** o usuário está na grade de Apps no modo `Descer` ou `Espiada` e pressiona Voltar
- **THEN** a tela retorna ao dock com foco no favorito anterior

#### Scenario: Voltar na grade no modo No Dock
- **WHEN** o usuário abriu a grade pelo botão "Todos os apps" no modo `No Dock` e pressiona Voltar
- **THEN** a grade fecha com fade e a tela Início reaparece com o foco no botão "Todos os apps" do dock

#### Scenario: Voltar das Configurações em modo sem abas
- **WHEN** o usuário abriu as Configurações do Lume pelo tile "Lume" no modo `Sem abas` e pressiona Voltar
- **THEN** a grade "Todos os apps" volta a ser exibida com o foco no tile "Lume"

### Requirement: Retorno depois de abrir um app
Quando o usuário abre um app a partir do lume (OK num item do dock, num tile da grade de Apps ou nos itens que abrem as Configurações do Android) e volta ao lume com o botão Home, o lume SHALL reaparecer na mesma tela e estágio (dock ou grade de Apps, em qualquer um dos cinco modos de navegação), com o foco no mesmo elemento e com a rolagem de antes, em vez de ir à tela Início. Esse retorno SHALL valer uma única vez e SHALL ser descartado assim que o usuário pressionar qualquer outra tecla no lume, ou quando o app não puder ser aberto. O botão Home pressionado com o lume já visível na tela (ou mais de 2 segundos depois de ele voltar) SHALL sempre levar à tela Início no topo, com o foco no primeiro item do dock. O estado SHALL ficar só na memória: se o sistema encerrar o processo do lume, ele abre na tela Início.

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
- **WHEN** o lume está na tela, em qualquer aba, estágio ou tela, mesmo depois de o usuário ter aberto um app, e o usuário pressiona Home
- **THEN** a tela Início é exibida no topo, com o foco no primeiro item do dock

#### Scenario: Retorno usado uma vez
- **WHEN** o usuário volta de um app pelo Home e pressiona Home de novo com o lume aberto
- **THEN** o segundo Home leva à tela Início, com o foco no primeiro item do dock

#### Scenario: Tecla pressionada antes do Home
- **WHEN** o usuário abre um app, volta ao lume pelo botão Voltar do app, move o foco e depois pressiona Home
- **THEN** o lume vai à tela Início, com o foco no primeiro item do dock

#### Scenario: App que não abre
- **WHEN** o lume não consegue abrir o app escolhido (por exemplo, foi desinstalado)
- **THEN** nada é guardado para o retorno, e o próximo Home leva à tela Início

#### Scenario: Voltar à grade no modo sem abas
- **WHEN** o usuário está na grade de Apps no modo `Espiada` ou `No Dock`, abre o YouTube e volta ao lume pelo botão Home
- **THEN** a grade de Apps reaparece na mesma rolagem com o foco no tile do YouTube

### Requirement: Splash na abertura e no retorno
O objetivo do splash é a experiência do usuário: a abertura do Lume e a passagem entre outro app e o Lume SHALL ser suaves e breves. Ao ser aberto (a frio) ou trazido de volta ao primeiro plano (a quente), o Lume SHALL mostrar o splash: o logo do Lume na cor de destaque ativa (com o L e o ponto transparentes/vazados), centralizado diretamente sobre o fundo escuro T7 correspondente à cor de destaque ativa desde o primeiro quadro, sem exibir o azul padrão `#0C171F` antes da cor escolhida. O splash SHALL ser desenhado pelo próprio Lume, e a janela inicial do sistema (`windowBackground`) usa um fundo escuro neutro (`#0A0D10`). O Lume SHALL manter o splash até que as imagens da tela inicial e da primeira tela da grade de Apps estejam carregadas (os ícones do dock e os tiles visíveis na rolagem restaurada ou da primeira tela da grade, inclusive nos modos sem abas e no peek do modo `Espiada`), por no máximo 600 ms a partir do primeiro quadro, e SHALL então revelar a interface. Um ícone que chegar com a interface já na tela (depois do teto, ou abaixo da primeira tela da grade) SHALL entrar com fade de 150 ms sobre a cor e a inicial do app, que somem sob ele, e não aparecer de uma vez. O Lume SHALL manter o logo à vista por um tempo mínimo desde o primeiro quadro, para que o usuário o veja: 600 ms na abertura a frio e 300 ms no retorno a quente. A revelação só começa depois desse tempo e de as imagens estarem prontas (ou do teto de 600 ms). O splash SHALL NOT mostrar a versão. Nesta versão, o splash SHALL NOT mostrar barra, porcentagem ou outro indicador de carregamento. O Lume SHALL, porém, acompanhar durante o splash quantas das imagens esperadas já estão prontas, de modo que um indicador futuro possa usar essa informação sem mudar o restante do splash. Uma tecla pressionada durante o splash, exceto Home, SHALL encerrá-lo imediatamente (a interface aparece sem a revelação) e SHALL ser aplicada à interface. Home SHALL seguir as regras de Home e de retorno sem encerrar o splash, porque é a tecla que traz o Lume de volta: ela só define a tela que o splash revela. A tela de destino SHALL ser a definida pelas regras de Home e de retorno depois de abrir um app.

#### Scenario: Abertura a frio
- **WHEN** o processo do Lume não existe e o usuário pressiona Home
- **THEN** a tela passa do fundo escuro neutro da janela do sistema diretamente para o fundo T7 e o logo da cor ativa (sem exibir o azul padrão), e a tela Início aparece já com os ícones do dock, sem que eles surjam depois

#### Scenario: Retorno a quente
- **WHEN** o Lume está em segundo plano e o usuário pressiona Home
- **THEN** o logo aparece e a tela Início é revelada quando os ícones do dock estão carregados e o tempo mínimo de 300 ms do logo passou

#### Scenario: Retorno para a grade
- **WHEN** o usuário volta ao Lume depois de abrir um app pela grade de Apps
- **THEN** o splash segura até os tiles visíveis estarem carregados e revela a grade com o foco e a rolagem restaurados

#### Scenario: Ícone que não carrega
- **WHEN** um ícone do dock não termina de carregar em 600 ms
- **THEN** a interface é revelada mesmo assim, e o item mostra a cor e a inicial até o ícone chegar, quando ele entra com fade

#### Scenario: Troca para Apps logo depois da revelação
- **WHEN** o Lume volta para a tela Início e o usuário vai para a grade de Apps assim que a interface aparece
- **THEN** os tiles da primeira tela da grade já estão carregados

#### Scenario: Sem indicador de carregamento
- **WHEN** o splash está na tela esperando os ícones
- **THEN** só o logo aparece, sem barra nem porcentagem, embora o Lume saiba quantas imagens faltam

#### Scenario: Home durante o splash
- **WHEN** o Home que trouxe o Lume de volta chega enquanto o splash está na tela
- **THEN** o splash continua e revela a tela definida pelas regras de Home e de retorno

#### Scenario: Tecla durante o splash
- **WHEN** o usuário pressiona Direita enquanto o splash está na tela
- **THEN** o splash acaba na hora e o foco se move para a direita na interface

#### Scenario: Pré-carga no modo Espiada
- **WHEN** o Lume inicia no modo `Espiada`
- **THEN** o splash aguarda o carregamento dos ícones do dock e da primeira linha de tiles visível na espiada (até o teto de 600 ms) antes da revelação
