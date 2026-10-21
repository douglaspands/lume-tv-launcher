# design-system Specification

## Purpose
Fixa o design system do frontend do Lume (tokens, tipografia, medidas, componentes, foco, movimento e regras de desenho) para que toda tela nova ou alterada siga as mesmas regras. A fonte dos valores é `themes/lume/theme.css` (cores, formas, texto e estados) e as constantes de layout do código; esta capacidade registra as regras que os dois devem obedecer.

## Requirements

### Requirement: Fonte única dos valores visuais
Todo valor visual de cor, forma, texto e estado SHALL vir de `themes/lume/theme.css` (variáveis e componentes), e nenhuma tela SHALL fixar no código uma cor, raio, peso ou tamanho de fonte que o tema já nomeia. Medidas de layout (posições, tamanhos de tile, dock, margens) SHALL ficar em constantes nomeadas no módulo da tela. Uma tela nova SHALL reaproveitar os componentes existentes (`.tile`, `.settings-item`, `.panel`, `.menu`, `.menu-item`, `.menu-note`, `.chip`, `.button`, `.button-secondary`, `.button-danger`, `.icon-button`, `.breadcrumb`, `.toast`) antes de criar outro; um componente novo SHALL ser registrado no tema e nesta spec no mesmo change.

#### Scenario: Cor nova
- **WHEN** uma tela precisa de uma cor que o tema não tem
- **THEN** a cor entra como variável em `:root` do tema, com o papel dela, e a tela usa a variável

#### Scenario: Componente existente
- **WHEN** uma tela precisa de um cartão, um botão redondo ou uma linha de lista
- **THEN** ela usa `.panel`, `.icon-button` ou `.settings-item`, em vez de desenhar caixas com valores próprios

#### Scenario: Lista de ações sobre um item
- **WHEN** uma tela precisa oferecer ações sobre o item em foco
- **THEN** ela usa o menu de contexto (`.menu`, `.menu-item`), em vez de criar outro painel de opções

### Requirement: Tokens de cor por papel
O design system SHALL nomear as cores por papel. Superfícies: `--bg` `#0F171E`, `--surface` `#121C25`, `--surface-row` `#17222C`, `--surface-focus` `#1F3040`, `--surface-icon` `#1A252F`, `--control` `#2A3A48`, `--control-disabled` `#1D2A35`, `--border` `#2F4252`. Texto: `--text` (igual a `--secondary`, `#F2F5F7`), `--text-soft` `#DDE3E8`, `--text-muted` `#AEB9C2`, `--text-disabled` `#59626A`. Papéis: `--primary` `#4DB3FF` (foco e controle), `--on-primary` `#0F171E`, `--secondary` `#F2F5F7` (neutra); `--accent` e `--on-accent` são apelidos de compatibilidade. Perigo: `--danger-bg` `#4A2630`, `--danger-fg` `#FFB4BE`. Menu de contexto (o único cartão claro): `--menu-surface` `#F2F5F7`, `--menu-text` `#1B2630`, `--menu-text-muted` `#5E6A75` (contraste de 5,0:1 sobre o cartão; `--menu-text`, 14:1). Cores de marca dos tiles (`--brand-*`) valem só para apps conhecidos. O texto sobre o papel de parede SHALL manter contraste de pelo menos 4,5:1.

#### Scenario: Cor de foco
- **WHEN** qualquer elemento recebe foco
- **THEN** o contorno, o sublinhado da barra ou o preenchimento de destaque usa `--primary`, e o nome do app em foco fica em `--text`

#### Scenario: Contraste
- **WHEN** o texto mudo (`--text-muted`) é desenhado sobre o ponto mais claro do papel de parede de uma tela
- **THEN** a razão de contraste é de pelo menos 4,5:1

### Requirement: Tipografia
O texto SHALL usar a família Plus Jakarta Sans embutida, nos pesos 500, 700 e 800, e nenhum outro. A escala de referência no canvas lógico SHALL ter 6 níveis, todos de tamanho inteiro e par (para cair em pixel inteiro a 1,5x): 40 (título de tela, peso 800), 28 (marca na barra, peso 800; relógio, peso 500), 22 (abas, títulos de seção, itens de Configurações, texto dos tiles, linhas do editor, títulos da tela Sobre), 20 (texto base e corpo da tela Sobre), 18 (botões, balão do dock, aviso, subtítulo, caminho de navegação, versão, rótulo de tile, chip, painel e valores do painel de estatísticas) e 16 (licenças de terceiros e cabeçalhos de cartão, estes em caixa alta com espaçamento de 1,2). Cada papel SHALL usar um só nível. Os tamanhos, pesos e espaçamentos de letras SHALL ser tokens do tema (`--font-title`, `--font-brand`, `--font-primary`, `--font-body`, `--font-label`, `--font-caption`) e nenhuma tela SHALL fixá-los no código. O texto SHALL NOT usar tamanho menor que 16 (`MIN_FONT_SIZE`). O nome do app sob um tile da grade de Apps SHALL seguir o comportamento da aba: sem foco, cor `--text-muted` e peso 500; com o foco, cor `--text` e peso 700, com a transição de 150 ms. Texto que não cabe SHALL terminar em reticências, exceto o nome no balão do dock, que SHALL aparecer inteiro enquanto couber na área segura.

#### Scenario: Peso fora do conjunto
- **WHEN** um componente pede peso 600
- **THEN** o build do tema avisa, e a tela não usa esse peso

#### Scenario: Tamanho fora da escala
- **WHEN** um componente do tema pede um tamanho que não é um dos 6 níveis (por exemplo 17)
- **THEN** o build do tema avisa, e a tela usa o nível mais próximo

### Requirement: Canvas, margens e grade
O layout SHALL ser desenhado no canvas lógico de 1280x720 (ver `display-adaptation`), com margem lateral de 64 e área segura de 32 na horizontal e 18 na vertical. A barra superior segue o requisito "Barra superior". Telas secundárias começam em x=64 com largura de 1152; nelas o caminho de navegação fica na linha-base 52 (a tela raiz, Configurações, mostra só "Configurações"), o título (40, peso 800) na 116 (64 abaixo do caminho), o subtítulo (18) na 152 e o conteúdo começa em y=212 (`page::CONTENT_TOP`), ou em y=184 no editor (32 abaixo do subtítulo). A versão instalada, quando mostrada, fica no canto inferior direito da área segura, em texto mudo de 18. A grade de Apps tem 6 colunas, espaço de 24 e tiles 16:9. A tela de Configurações divide a largura de 1152 em duas colunas iguais de 552, com 48 de vão: os itens (72 de altura, passo 88) na esquerda e o painel de estatísticas, alinhado ao primeiro item, na direita (x=664). No editor, a linha "Favoritos redondos" (`.editor-row`, 64 de altura, chave Liga/Desliga de 48x28 à direita com bolinha de 20) começa em y=184 e fica 16 acima do painel de favoritos; o painel tem enchimento lateral de 32 e vertical de 24, 32 entre o cabeçalho e os cartões, cartões de 168 com 16 de espaço (6 por linha, preenchendo os 1088 úteis), 20 entre os cartões e "Adicionar app", chips de 40 de altura (raio 20) com 16 de espaço entre linhas e 24 entre o painel e "Restaurar padrão". O dock tem ícones de 64, espaço de 16, enchimento de 16 e pico de ampliação de 1,6. Valores que mudam SHALL ser alterados na constante do módulo, e esta spec SHALL ser atualizada junto.

#### Scenario: Botão novo na barra
- **WHEN** um botão é adicionado à barra superior
- **THEN** ele tem 48 de diâmetro, glifo de 36, fica 64 à esquerda do vizinho (16 de vão) e segue a tabela do requisito "Barra superior"

#### Scenario: Painel ao lado dos itens
- **WHEN** a tela de Configurações é exibida em 1280x720
- **THEN** a lista ocupa x de 64 a 616 e o painel de 664 a 1216, em colunas iguais

#### Scenario: Seis cartões por linha no editor
- **WHEN** a tela Editar tela Início é exibida em 1280x720 com 6 favoritos
- **THEN** os 6 cartões de 168 com vão de 16 cabem na mesma linha, preenchendo os 1088 de largura útil do painel sem sobra à direita

### Requirement: Barra superior
A barra superior SHALL seguir estas métricas (canvas lógico; "tinta" é o desenho visível, medido na captura, e "caixa" é o retângulo de layout):

| Item | Caixa | Tinta medida | Regra |
|---|---|---|---|
| Barra | 0 a 88, margens de 64 | — | eixo central em y=44 para todos os itens |
| Logo | 40x40 em x=64, y de 24 a 64 | 64 a 103, altura 40 | âncora da ponta esquerda |
| Marca "Lume" | x=118, 28 peso 800, linha-base 56 | começa a 16 do logo | logo e marca formam um bloco só |
| Abas | a primeira em x=236, 32 entre os textos; 22, linha-base 52 | 48 da marca à primeira aba | menores que as pontas: o sublinhado as marca |
| Botões (entradas, engrenagem) | 48 de diâmetro, y de 20 a 68, 16 de vão | glifo de 36 (Lucide redesenhado com traço 1,75), desenho de 30 a 32 (cerca de 80% do logo), centro 44, traço de tinta igual ao das abas (4 px a 1,5x) | o ícone tem o peso do texto das abas; desenho de traço parece maior que o logo cheio, por isso fica menor que ele |
| Relógio | 28 peso 500, linha-base 56, termina em x=1216 | altura de tinta e linha-base da marca; traço igual ao do texto das abas (4 px a 1,5x) | algarismos da fonte padrão (escolha do autor) |
| Engrenagem → relógio | 24 da caixa da engrenagem ao relógio | 34 de tinta, igual ao vão entre os ícones, em qualquer hora | os botões acompanham a largura do relógio |

O relógio SHALL usar a fonte padrão com os algarismos dela, alinhado à direita em x=1216. Os botões SHALL ser posicionados a partir da largura da hora mostrada, para que o vão de tinta entre a engrenagem e o relógio seja sempre o mesmo que entre os dois ícones; eles se movem só quando a largura da hora muda, e nunca chegam às abas. O anel de foco de um botão (3 px, afastado 2, botão ampliado 1,08) SHALL caber na barra e não tocar a tinta do vizinho. Item novo na barra SHALL usar estas métricas, sem tamanho, espaço ou linha-base próprios; se precisar de um, esta tabela é atualizada no mesmo change.

#### Scenario: Mesma escala na barra
- **WHEN** a barra superior é exibida em 1280x720 e medida com `scripts/measure-ink.py`
- **THEN** logo, ícones, marca e relógio têm o centro de tinta em y=44 (1 px de tolerância), os ícones têm de 30 a 32 de altura e traço igual ao das abas, o relógio tem a altura e a linha-base da marca e o traço das abas, e os vãos de tinta são 16 (logo → marca), 48 (marca → abas, 2 px de tolerância pela folga das letras), 34 (ícone → ícone) e 34 (engrenagem → relógio, em qualquer hora)

#### Scenario: Hora mais larga
- **WHEN** o relógio passa de "11:11" para "00:00", ou o formato é de 12 h
- **THEN** o relógio cabe no espaço reservado, nada encosta na engrenagem, e os botões ficam no mesmo lugar

### Requirement: Régua de layout
Toda tela, existente ou nova, SHALL ser montada sobre uma régua única no canvas lógico de 1280x720, para que espaços, alinhamentos e tamanhos sejam os mesmos em todas as telas e uma tela nova saia alinhada de primeira:

- **Unidade de 4.** Toda posição, tamanho, enchimento, espaço, raio e linha-base SHALL ser múltiplo de 4. O espaço entre blocos de uma tela (seções, cartões, grupos de botões) SHALL ser múltiplo de 8. Série de espaços: 4, 8, 12, 16, 24, 32, 40, 48, 64. Exceções, registradas nesta spec: medida derivada de proporção ou de animação (tile 16:9 de 172x96,75, desenhado com 97; ampliação do dock), espessuras de linha (sublinhado de 2, anéis de foco de 3 e 4 e seus afastamentos) e correção ótica pela tinta (marca em x=118, para a tinta ficar a 16 do logo). Os testes `the_layout_is_on_the_ruler` (um por módulo de tela) e `the_component_boxes_are_on_the_ruler` (tema) conferem a unidade.
- **Colunas.** O conteúdo vai de x=64 a x=1216 (1152 de largura). Divisões permitidas: 6 colunas de 172 com vão de 24 (grade de Apps) e 2 colunas de 552 com vão de 48 (Configurações). Todo bloco SHALL começar e terminar numa borda dessas colunas; o texto e o ícone dentro de um bloco alinham à borda do bloco mais o enchimento do componente.
- **Linhas fixas.** Barra superior de 0 a 88, eixo 44, linha-base 52 (requisito "Barra superior"). Tela secundária: caminho na linha-base 52, título na 116, subtítulo na 152, conteúdo a partir de y=212. Tela principal sob a barra (Apps): título de seção na linha-base 116 e conteúdo a partir de y=144. O conteúdo termina pelo menos 40 acima do fim da área segura.
- **Tamanhos de componente.** Alturas: 40 (chip), 48 (botões e botões redondos), 72 (item de Configurações e linha do editor), 88 (passo da lista); ícones: glifo de 24 (setas e ícones dentro de componentes), 36 (barra superior) e 64 (dock); raio 12 (cartões, botões, itens) ou metade da altura (chip, botão redondo); enchimento lateral de 24 a 32 em componentes de texto e de 32 em cartões e painéis. Espaço entre ícone e texto num componente: 8; entre itens de um grupo de botões, botões redondos ou chips: 16.
- **Texto corrido.** A altura de linha SHALL ser múltipla de 4: 32 para a legenda (16), 40 para corpo e rótulo (18 e 20), 44 para o principal (22).
- **Alinhamento.** Itens lado a lado numa linha compartilham o eixo central (ícone, botão, chip) ou a linha-base (textos); textos empilhados numa coluna compartilham a borda esquerda; valores numéricos de uma lista alinham à direita; um cartão ao lado de uma lista alinha o topo ao do primeiro item.
- **Caixa e tinta.** Um espaço é especificado na caixa (o que o código posiciona), mas o que o olho vê é a tinta; quando a folga da fonte ou a margem do glifo faz vãos iguais parecerem diferentes, a caixa é corrigida para que a tinta fique igual (como os ícones da barra, que têm 16 de vão na caixa e 34 na tinta por causa da margem do glifo). Textos lado a lado alinham pela linha-base; ícones e blocos, pelo centro da tinta.
- **Verificação.** Toda tela nova ou alterada SHALL ser medida na captura de 1280x720 com `scripts/measure-ink.py` (`columns` dá os vãos de uma faixa, `rows` dá altura e centro de cada item), e os números conferidos contra esta régua e contra a tabela do requisito da tela.
- **Receita de tela nova.** Escolher o modelo (principal, sob a barra, ou secundária, com cabeçalho); dividir a largura com uma das divisões de colunas; empilhar os blocos a partir da linha de conteúdo com espaços da série; usar só componentes e tokens existentes. Medida nova SHALL entrar nesta spec e numa constante do módulo, no mesmo change, antes da implementação da tela.

#### Scenario: Medida fora da régua
- **WHEN** uma tela usa um espaço de 14, uma altura de 71 ou uma linha-base de 119
- **THEN** a revisão a recusa, e o valor vira o múltiplo de 4 da série mais próximo (16, 72, 116) ou é registrado aqui como exceção derivada

#### Scenario: Tela nova alinhada de primeira
- **WHEN** um change cria uma tela
- **THEN** o design do change escreve a tela em termos desta régua, numa tabela de métricas como a da barra superior (item, caixa, tinta esperada, regra), antes do código, e a medição da captura em 1280x720 confere com a tabela

#### Scenario: Mesma medida em telas diferentes
- **WHEN** o mesmo papel aparece em duas telas (título, conteúdo, cartão, botão)
- **THEN** ele tem a mesma posição, o mesmo tamanho e o mesmo espaço nas duas

### Requirement: Botões
O design system SHALL ter três botões de 48 de altura, raio `--radius`, texto 18 peso 700 e enchimento horizontal de 28. O **primário** (`.button`) é a ação principal da tela e se comporta como uma aba: sem foco fica apagado (fundo `--control`, texto `--text-muted`) e, com o foco, acende, preenchido com `--primary` (a cor do logo do Lume) e texto `--on-primary`; como todo foco do Lume, o dele SHALL ser um anel azul `--primary` de 3 px, afastado do preenchimento por `outline-offset` 4 (o vão escuro deixa o anel visível) e com escala 1,04. O **secundário** (`.button-secondary`) é a alternativa discreta, com fundo `--surface`, contorno de 2 px `--border` e texto `--text`; em foco, fundo `--surface-focus`, anel `--primary` de 3 px e escala 1,04. O **de perigo** (`.button-danger`) confirma o que não se desfaz, com fundo `--danger-bg` e texto `--danger-fg`, e em foco o mesmo anel azul afastado do primário. Cada tela SHALL ter no máximo um botão primário visível. "Concluído" SHALL ser primário, "Restaurar padrão" e "Cancelar" secundários e "Restaurar" (na confirmação) de perigo. Botões SHALL ser desenhados pelo componente do tema, nunca com cores no código.

#### Scenario: Concluído
- **WHEN** a tela Editar tela Início é exibida
- **THEN** "Concluído" aparece apagado (cinza-azulado com texto mudo) enquanto o foco está em outro lugar e, quando recebe o foco, acende com o azul do logo (`#4DB3FF`), texto escuro e o anel azul afastado de 4 px

#### Scenario: Restaurar padrão
- **WHEN** a tela Editar tela Início é exibida
- **THEN** "Restaurar padrão" aparece como botão secundário contornado, e em foco ganha anel azul e fundo de foco

#### Scenario: Confirmação
- **WHEN** o diálogo "Restaurar a tela Início padrão?" é exibido
- **THEN** "Restaurar" é o botão de perigo e "Cancelar" o secundário, com o foco inicial em "Cancelar"

### Requirement: Itens de configuração
Os itens da tela de Configurações (`.settings-item`) SHALL seguir o comportamento do botão primário: sem foco ficam apagados (fundo `--surface-row`, texto e seta `--text-muted`) e, com o foco, acendem, preenchidos com `--primary`, texto e seta `--on-primary` e anel azul de 3 px afastado de 4, sem ampliar a escala. A seta à direita SHALL acompanhar a cor do texto.

#### Scenario: Item em foco
- **WHEN** o foco chega a "Tela Início" em Configurações
- **THEN** o item acende em azul com texto escuro, e "Sobre o Lume" permanece apagado

### Requirement: Menu de contexto
O menu de contexto (botão Menu, ver `launcher-shell`) SHALL ser um cartão claro (`.menu`: `--menu-surface`, raio 20, sombra próxima e em volta do cartão (0 4 24, preto a 45%), que o separa de um tile branco), como o do launcher do Android TV, para se destacar sobre as telas escuras. Ele SHALL ficar junto do item de origem, sem seta e sem escurecer a tela (sem camada translúcida de tela cheia): acima do favorito no dock e ao lado do tile na grade (à direita, ou à esquerda quando não couber), sempre dentro da área segura.

Os itens (`.menu-item`) SHALL formar uma lista contínua: texto `--menu-text` (18, peso 500, o mesmo tamanho e peso dos nomes dos apps; em 20 e peso 700 o texto escuro sobre o cartão claro parecia pesado) sem foco e, com o foco, preenchimento `--primary` e texto `--on-primary`, sem anel de contorno. Só um item SHALL aparecer em foco por vez. Um item desabilitado SHALL usar `--menu-text-muted` com opacidade 0,6, mostrar o motivo embaixo em `.menu-note` (16, peso 500) e SHALL NOT receber foco. O menu SHALL aparecer e sumir com fade de 150 ms (`--focus-time`) e uma subida de 8 ao aparecer, aplicados só à área do cartão. Todos os textos SHALL vir de `strings.xml`.

Medidas, na régua de 4:

| Medida | Valor | Por quê |
|---|---|---|
| Margem interna do cartão | 8 | com o raio 20 do cartão e o 12 do item, os cantos ficam concêntricos (20 = 12 + 8) |
| Item | 56 de altura, enchimento lateral de 16 | o texto (18) fica a 24 da borda do cartão (pedido do autor: laterais enxutas) |
| Vão entre itens | 0 | itens de uma lista só; sem anel, não há o que afastar |
| Item desabilitado | 72 | nome e motivo centrados pela tinta |
| Largura | a do texto mais longo + 4 de folga, mínimo 200 | a folga evita reticências por arredondamento em 1,5x; o mínimo só vale para menus muito curtos, para que o texto decida a largura e as duas laterais fiquem iguais |
| Distância ao tile (grade) | 8 do anel de foco, com o topo do cartão no topo do anel | o menu fica junto do tile como ele aparece com o foco (ampliado 1,05, com o anel de 4), perto o bastante para se ler como dele |
| Distância ao nome (dock) | 4 acima da caixa do nome do favorito (~18 até as letras) | perto do app, e o nome continua visível e diz a qual app o menu se refere |

No modo de mover um favorito (`home-screen`), os vizinhos se afastam 24 de cada lado e as setas (chevrons Lucide de 24, cor `--text`) ficam no centro desses vãos.

#### Scenario: Menu sobre o dock
- **WHEN** o menu de contexto de um favorito é aberto
- **THEN** o painel aparece acima do favorito, dentro da área segura, sem escurecer o resto da tela

#### Scenario: Menu na última coluna
- **WHEN** o menu de contexto é aberto sobre um app da sexta coluna da grade
- **THEN** o painel aparece à esquerda do tile

#### Scenario: Um item em foco por vez
- **WHEN** o usuário abre o menu, leva o foco ao último item, fecha e abre de novo
- **THEN** só o primeiro item aparece em foco

#### Scenario: Item desabilitado
- **WHEN** o menu mostra "Adicionar ao Início" com o limite atingido
- **THEN** o item aparece apagado, com "Limite de 12 favoritos" em texto menor, e o foco o pula

### Requirement: Foco e movimento
O foco SHALL ser sempre visível e azul primário: contorno de 4 px (`--focus-ring`) em tiles, itens e linhas; 3 px em chips, botões e botões redondos, com `outline-offset` de 2 a 3; os botões preenchidos afastam o anel do fundo. A única exceção ao anel é o item do menu de contexto, em que o preenchimento `--primary` sobre o cartão claro basta (decisão do autor). Tiles ampliam 1,05 e botões redondos 1,08, com `--focus-shadow` nos tiles. As abas não têm contorno: o foco é o sublinhado de 2 px. As transições de foco SHALL durar 150 ms com curva ease-out (`--focus-time`); a troca de aba SHALL ter fade de 180 ms; o dock SHALL deslizar com a animação de Magnification; quando a ordem dos favoritos muda (mover, remover, desinstalar), cada ícone SHALL deslizar do lugar e do tamanho antigos para os novos em 240 ms (ease-in-out), e o removido encolhe e some no lugar; o menu de contexto SHALL aparecer com fade de 150 ms e subida de 8. Um elemento sem ação (como o painel de estatísticas) SHALL NOT receber foco nem ter animação. Uma animação cujo valor de repouso é 0 SHALL receber o valor inicial em `animate_to`.

#### Scenario: Foco em botão redondo
- **WHEN** o foco chega ao botão de entradas ou à engrenagem
- **THEN** o botão ganha contorno azul de 3 px e cresce 1,08 em 150 ms

#### Scenario: Cartão sem foco
- **WHEN** o usuário navega pela tela de Configurações
- **THEN** o foco passa só pelos itens da lista, nunca pelo cartão de estatísticas

### Requirement: Superfícies e papel de parede
O fundo de toda tela SHALL ser o papel de parede de aurora (ver `wallpaper`) sobre `--bg`; as telas de texto o desenham escurecido no próprio desenho do papel (`Painter::wallpaper(true)`). Cartões e painéis SHALL usar `.panel` (`--surface`, raio `--radius` de 12). A exceção é o menu de contexto (`.menu`): um cartão claro (`--menu-surface`, raio 20, sombra próxima e em volta do cartão (0 4 24, preto a 45%), que o separa de um tile branco), o único elemento claro da interface, para se destacar como camada temporária sobre as telas escuras. O dock é translúcido (`rgba(15,23,30,.55)`, raio 28, contorno de 1 px a 14% de branco), e o nome do favorito em foco, acima dele, tem fundo transparente: só o texto (`--text`, 18, peso 700), sem balão nem sombra. O desenho SHALL NOT usar nenhuma camada translúcida de tela cheia, exceto o escurecimento do diálogo de confirmação, que só existe enquanto o diálogo está aberto.

#### Scenario: Escurecimento
- **WHEN** a tela de Configurações, Sobre ou Editar é desenhada
- **THEN** o escurecimento vem do desenho do papel de parede, sem camada translúcida por cima

### Requirement: Ícones, marca e texto de interface
Os ícones de interface SHALL vir do Lucide, desenhados pelo `scripts/subset-fonts.py` a partir dos SVGs com o traço `LUCIDE_STROKE` (1,75 na grade de 24, em vez dos 2 do Lucide), para que no glifo de 36 da barra o traço tenha a espessura do texto das abas e os ícones pareçam da mesma família da fonte; o tamanho é o do requisito "Régua de layout" (36 na barra superior, 24 dentro de componentes). Um ícone novo SHALL ser adicionado ao script e regerado com o mesmo traço, nunca desenhado à mão. O nome do produto SHALL ser escrito "Lume" em todo texto de interface, doc e spec; identificadores técnicos (pacote, crate, biblioteca) ficam em minúsculas. Todo texto de interface SHALL estar em `strings.xml` (en e pt-BR), nunca no código.

#### Scenario: Ícone novo
- **WHEN** uma tela precisa de um ícone que o subconjunto não tem
- **THEN** o nome do ícone do Lucide entra em `ICONS` de `subset-fonts.py`, o script é rodado (`--icons-only` para não regerar a fonte de texto) e a constante nasce em `icons.rs`

#### Scenario: Peso do ícone
- **WHEN** a barra superior é medida na captura em 1920x1080
- **THEN** o traço dos ícones e o do relógio têm a mesma espessura de tinta que o texto das abas (4 px), com 1 px de tolerância

#### Scenario: Texto novo
- **WHEN** uma tela ganha um rótulo
- **THEN** ele entra em `values/strings.xml` e `values-pt-rBR/strings.xml`, e a tela o lê pelo tradutor

### Requirement: Mudança visual segue o design system
Todo change que cria ou altera tela, componente ou token SHALL ler esta spec antes de implementar, SHALL respeitar os requisitos acima e, se precisar divergir ou acrescentar, SHALL incluir a atualização desta spec no mesmo change. Uma captura da tela por `lume-desktop --screenshot` em 1280x720 SHALL ser conferida contra estas regras antes de dar a tarefa visual por concluída.

#### Scenario: Tela nova
- **WHEN** um change adiciona uma tela
- **THEN** as tarefas do change citam os componentes e tokens desta spec, e a captura de 1280x720 é anexada à verificação

#### Scenario: Divergência
- **WHEN** a implementação precisa de um valor que contraria esta spec
- **THEN** o change atualiza esta spec com a decisão, em vez de deixar o código divergir em silêncio

### Requirement: Splash
O splash SHALL ser só o logo do Lume (o quadrado arredondado com o L e o ponto, nas proporções de `logo.rs`) centralizado sobre `--bg`, com tamanho definido no tema, sem texto. O tema SHALL reservar abaixo do logo uma faixa para um indicador de carregamento futuro (posição e altura em tokens), que nesta versão fica vazia e não desloca o logo. O logo SHALL entrar com fade de 120 ms (ease-out) no primeiro quadro do splash, sobre o mesmo `--bg` da janela inicial do sistema. A revelação SHALL acontecer ao mesmo tempo em três partes: o papel de parede passa da intensidade 0 (só `--bg`) à intensidade normal, pela própria cor com que é desenhado; o logo some com fade, aplicado só à área do logo; e a interface entra com fade, pela opacidade de cada elemento (como o fade das abas), com o conteúdo (dock ou grade) subindo 24 unidades e a barra superior parada. A revelação SHALL durar 300 ms na abertura a frio e 220 ms no retorno a quente, com curva ease-out. Um ícone que chegar com a interface na tela SHALL entrar com fade de 150 ms, ease-out (`--icon-in`), enquanto a cor e a inicial do app saem sob ele. Nenhuma etapa SHALL usar camada translúcida de tela cheia.

#### Scenario: Entrada do logo
- **WHEN** a janela do sistema é substituída pelo primeiro quadro do Lume
- **THEN** o fundo continua da mesma cor e o logo surge com fade, sem saltar

#### Scenario: Revelação sem camada de tela cheia
- **WHEN** a interface é revelada
- **THEN** o papel de parede ganha intensidade pelo próprio desenho, e só a área do logo é desenhada com transparência

### Requirement: Item de Informações da TV nas Configurações
A tela de Configurações SHALL incluir, na coluna esquerda de itens, a opção "Informações da TV" entre "Tela Início" e "Sobre o Lume", respeitando a altura de 72 px e o passo de 88 px na régua.

#### Scenario: Terceiro item de configurações
- **WHEN** a tela de Configurações é desenhada
- **THEN** a lista exibe três itens: "Tela Início", "Informações da TV" e "Sobre o Lume"

### Requirement: Layout vertical de Informações da TV
A tela Informações da TV SHALL utilizar a largura útil de 1152 px no canvas 1280x720, com títulos de seção na escala de 16 px e linhas de chave/valor na escala de 18 px, alinhadas à régua de 4 px, com máscara de corte abaixo do cabeçalho.

#### Scenario: Rótulo e valor na régua
- **WHEN** as especificações são renderizadas
- **THEN** as linhas mantêm altura uniforme, com rótulos em `--text-muted` e valores em `--text`
