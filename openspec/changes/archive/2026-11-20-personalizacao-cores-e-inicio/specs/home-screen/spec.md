## ADDED Requirements

### Requirement: Modo Descer sem abas
No modo `Sem abas` + `Descer`, a tela Início SHALL exibir o dock limpo com a dica discreta não focável `v Todos os apps` abaixo dele; pressionar `Baixo` no dock SHALL transitar verticalmente em `250 ms` para a grade `"Todos os apps"` escurecida, com foco no primeiro tile.

#### Scenario: Descer do dock para a grade
- **WHEN** o modo `Descer` está ativo, o foco está num favorito do dock e o usuário pressiona Baixo
- **THEN** a tela transita verticalmente em 250 ms para a grade "Todos os apps" com o foco no primeiro tile da primeira linha

#### Scenario: Dica não recebe foco
- **WHEN** o usuário navega no dock no modo `Descer`
- **THEN** a dica `v Todos os apps` permanece visível abaixo do dock sem receber foco intermediário

### Requirement: Modo Espiada sem abas
No modo `Sem abas` + `Espiada`, a tela Início SHALL exibir o dock em `y = 524..620` e a borda superior (~48 px) da primeira linha de apps em `y = 672`; pressionar `Baixo` SHALL rolar suavemente (`250 ms`) para a grade `"Todos os apps"`, interpolando o escurecimento do papel de parede em `Painter::wallpaper`, sem camada translúcida de tela cheia.

#### Scenario: Espiada visível no topo
- **WHEN** o modo `Espiada` está ativo e o foco está no dock
- **THEN** a parte superior da primeira linha de apps aparece abaixo do dock, indicando visualmente a lista de apps

#### Scenario: Rolagem para a grade com escurecimento progressivo
- **WHEN** o foco está no dock no modo `Espiada` e o usuário pressiona Baixo
- **THEN** a página rola para a grade "Todos os apps" enquanto o papel de parede escurece progressivamente no próprio desenho do papel, sem camada translúcida extra

### Requirement: Botão "Todos os apps" no Dock
No modo `Sem abas` + `No Dock`, o dock SHALL incluir o item fixo `"Todos os apps"` (slot extra além dos até 12 favoritos, reordenável e não removível), acompanhando o formato configurado (`Redondo`, `Quadrado` ou `Retangular` 16:9), em fundo sólido liso `--primary` com grade 3×3 de pontos circulares em `--on-primary`, Magnification e nome no balão; `OK` SHALL abrir a grade `"Todos os apps"` com `--tab-fade` (`180 ms`).

#### Scenario: Visual do item nos três formatos
- **WHEN** o modo `No Dock` está ativo em `Redondo`, `Quadrado` ou `Retangular`
- **THEN** o item usa fundo sólido liso `--primary` (com borda sutil de contraste em `Preto`) e grade 3×3 de pontos circulares em `--on-primary`, centralizada em `Redondo`/`Quadrado` e à esquerda do rótulo traduzido `"Apps"` em `Retangular` 16:9

#### Scenario: Abrir grade pelo Dock
- **WHEN** o modo `No Dock` está ativo, o foco está no item "Todos os apps" do dock e o usuário pressiona OK
- **THEN** a grade "Todos os apps" é exibida com fade de 180 ms e o foco vai para o primeiro tile

#### Scenario: Slot extra com 12 favoritos
- **WHEN** o usuário já possui 12 favoritos e ativa o modo `No Dock`
- **THEN** o dock exibe os 12 favoritos e o item "Todos os apps" (13 itens ao todo) dentro da área segura, sem rolagem horizontal, em qualquer um dos três formatos (`Redondo`, `Quadrado` ou `Retangular`)

#### Scenario: Menu de contexto do item "Todos os apps"
- **WHEN** o foco está no item "Todos os apps" do dock (com pelo menos um favorito) e o usuário pressiona Menu
- **THEN** o menu de contexto exibe apenas "Mover", sem "Remover do Início" nem "Informações do app"

## MODIFIED Requirements

### Requirement: Dock de favoritos
Os favoritos configurados SHALL ser exibidos na ordem configurada num dock: uma barra translúcida de cantos arredondados, flutuante, centralizada horizontalmente e acima da borda inferior da área segura. Cada favorito SHALL aparecer no formato configurado em `"Formato dos favoritos"`: `Redondo` (circular, padrão), `Quadrado` (quadrado de cantos arredondados) ou `Retangular` (banner retangular 16:9 de TV, igual aos tiles da grade de Apps), sobre a cor do app quando a imagem tem transparência ou está carregando. O dock SHALL mostrar todos os favoritos (até 12, ou 13 com `"Todos os apps"` no modo `No Dock`) lado a lado, sem rolagem horizontal, e SHALL ficar dentro da área segura em qualquer formato. OK sobre um favorito SHALL abrir o app.

#### Scenario: Favoritos padrão no primeiro uso
- **WHEN** o lume é aberto pela primeira vez com Netflix, YouTube e Spotify instalados
- **THEN** o dock mostra os apps conhecidos instalados, na ordem Netflix, YouTube, Prime Video, Spotify, Disney+, Max, Globoplay, limitados a 5, em formato circular (`Redondo`)

#### Scenario: Doze favoritos
- **WHEN** existem 12 favoritos em qualquer formato (`Redondo`, `Quadrado` ou `Retangular`)
- **THEN** todos aparecem no dock ao mesmo tempo, dentro da área segura e sem rolagem, mesmo com o item focado ampliado

#### Scenario: Abrir pelo dock
- **WHEN** o foco está num ícone do dock e o usuário pressiona OK
- **THEN** o app daquele ícone é aberto

#### Scenario: Favoritos em formato circular
(Título mantido da spec anterior para compatibilidade de validação; cobre o formato `Quadrado`.)
- **WHEN** o formato dos favoritos está em `Quadrado`
- **THEN** cada item do dock é desenhado como um quadrado de cantos arredondados com o ícone do app

#### Scenario: Favoritos em formato retangular de TV
- **WHEN** o formato dos favoritos está em `Retangular`
- **THEN** cada item do dock é desenhado como um retângulo 16:9 de cantos arredondados exibindo o banner de TV do app (ou cor e inicial como fallback), mantendo todos os itens dentro da área segura

### Requirement: Conteúdo da tela Início
A tela Início SHALL exibir, sobre o papel de parede, a barra superior e, na parte de baixo, o dock com os apps favoritos. A tela Início SHALL NOT exibir carrossel nem seções extras, exceto os elementos definidos pelos modos sem abas quando `Abas na barra superior` estiver em `Sem abas`: a dica discreta `v Todos os apps` no modo `Descer`, o título e a espiada da primeira linha da grade no modo `Espiada`, ou o item fixo `"Todos os apps"` no dock no modo `No Dock`.

#### Scenario: Primeiro uso
- **WHEN** o lume é aberto pela primeira vez
- **THEN** a tela Início mostra o papel de parede, a barra superior com abas à esquerda e, embaixo, o dock com os favoritos padrão

#### Scenario: Modo sem abas ativo
- **WHEN** o usuário desativa as abas na barra superior
- **THEN** a tela Início exibe apenas os elementos correspondentes ao modo escolhido (`Descer`, `Espiada` ou `No Dock`)

### Requirement: Navegação no dock
Com o foco no dock, Esquerda e Direita SHALL mover o foco entre os itens (incluindo o item `"Todos os apps"` no modo `No Dock`), sem avançar além do primeiro ou do último. Cima SHALL levar o foco à aba ativa da barra superior (no modo `Com abas`) ou ao primeiro botão disponível da barra superior — entradas ou engrenagem (no modo `Sem abas`). Baixo SHALL levar o foco para a grade `"Todos os apps"` nos modos `Descer` e `Espiada`, e SHALL NOT mudar o foco nos modos `Com abas` e `No Dock`. Ao descer da barra superior, o foco SHALL ir ao primeiro item do dock.

#### Scenario: Limite do dock
- **WHEN** o foco está no último item do dock e o usuário pressiona Direita
- **THEN** o foco permanece no último item

#### Scenario: Ir à barra superior
- **WHEN** o modo `Com abas` está ativo, o foco está no dock e o usuário pressiona Cima
- **THEN** o foco vai para a aba ativa da barra superior

#### Scenario: Ir à barra superior sem abas
- **WHEN** o modo `Sem abas` está ativo, o foco está no dock e o usuário pressiona Cima
- **THEN** o foco vai para o botão de entradas (ou para a engrenagem se não houver entradas)

#### Scenario: Descer da barra superior
- **WHEN** o foco está na barra superior e o usuário pressiona Baixo
- **THEN** o foco vai ao primeiro item do dock

#### Scenario: Descer do dock no modo No Dock
- **WHEN** o modo `No Dock` está ativo, o foco está no dock e o usuário pressiona Baixo
- **THEN** o foco permanece no dock
