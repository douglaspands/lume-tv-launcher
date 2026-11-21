## MODIFIED Requirements

### Requirement: Navegação na grade
Na grade, as setas SHALL mover o foco entre células vizinhas. Direita na última coluna SHALL NOT pular para a linha seguinte. OK SHALL abrir o app focado. Pressionar Cima na primeira linha SHALL levar o foco:
- à barra superior (aba `Apps`) no modo `Com abas`;
- à barra superior (botão de entradas ou engrenagem) no modo `Sem abas` + `No Dock`;
- de volta ao dock de favoritos (retornando ao estágio/topo da tela Início) nos modos `Sem abas` + `Descer` e `Sem abas` + `Espiada`.

#### Scenario: Limite da linha
- **WHEN** o foco está na 6ª coluna e o usuário pressiona Direita
- **THEN** o foco permanece no mesmo tile

#### Scenario: Última linha incompleta
- **WHEN** o foco está na penúltima linha, na coluna 5, e a última linha tem só 2 itens
- **THEN** pressionar Baixo leva o foco ao último item da última linha

#### Scenario: Subir da primeira linha nos modos Descer e Espiada
- **WHEN** o foco está na primeira linha da grade nos modos `Descer` ou `Espiada` e o usuário pressiona Cima
- **THEN** a tela retorna ao topo/dock e o foco volta ao favorito anterior do dock

#### Scenario: Subir da primeira linha no modo No Dock
- **WHEN** o foco está na primeira linha da grade no modo `No Dock` e o usuário pressiona Cima
- **THEN** o foco vai para o botão de entradas (ou para a engrenagem se não houver entradas) na barra superior

### Requirement: Tile do lume na grade
A grade SHALL incluir um tile do próprio lume, tratado como mais um app: ele SHALL ter o mesmo formato retangular dos demais, ocupar o seu lugar na ordem alfabética (o nome é "Lume" em qualquer idioma), mostrar o logo do lume (o L e o ponto, acompanhando a cor de destaque ativa) e exibir o nome "Lume" sob o tile. OK sobre ele SHALL abrir a tela de Configurações do lume; Voltar das Configurações SHALL retornar à grade de Apps com o foco no tile "Lume" em todos os modos de navegação (com abas ou sem abas). As Configurações do Android continuam acessíveis pela barra superior e pelo app de configurações do sistema, quando ele aparece na grade.

#### Scenario: Tile do lume
- **WHEN** a aba ou grade de Apps é exibida
- **THEN** o tile "Lume" aparece entre os apps, em ordem alfabética, retangular como os outros, com o logo do lume no centro

#### Scenario: Abrir as configurações do lume
- **WHEN** o usuário seleciona o tile "Lume"
- **THEN** a tela de Configurações do lume é aberta e Voltar leva de volta à grade de Apps

#### Scenario: Tile do lume com cor personalizada
- **WHEN** a cor de destaque é `Aurora` e a grade de Apps é exibida
- **THEN** o tile "Lume" aparece em ordem alfabética com o logo do lume na cor `Aurora`

#### Scenario: Abrir e voltar das configurações do lume em modo sem abas
- **WHEN** o usuário seleciona o tile "Lume" no modo `Descer`, `Espiada` ou `No Dock` e depois pressiona Voltar em Configurações
- **THEN** a tela de Configurações fecha e a grade "Todos os apps" reaparece com o foco no tile "Lume"

### Requirement: Foco nos tiles da grade
O tile em foco SHALL ter um contorno na cor primária ativa do tema (`--primary`, padrão azul `#4DB3FF`, com a borda externa sutil de contraste quando a cor ativa for `Preto`) e leve aumento de escala, em vez do contorno branco da versão anterior.

#### Scenario: Contorno azul
- **WHEN** a cor padrão `Azul Lume` está ativa e o foco está num tile da grade, inclusive de um app azul como o Prime Video
- **THEN** o tile mostra um contorno azul claro, visível sobre a cor do tile, e fica um pouco maior

#### Scenario: Contorno na cor de destaque ativa
- **WHEN** o usuário escolhe `Âmbar` como cor de destaque e foca um tile da grade
- **THEN** o tile mostra um contorno `Âmbar` de 4 px e amplia 1,05x
