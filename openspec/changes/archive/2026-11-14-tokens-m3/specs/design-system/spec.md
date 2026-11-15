# Spec Delta

## ADDED Requirements

### Requirement: Paletas tonais
Toda cor neutra e de perigo do tema SHALL ser um tom (HCT) de uma paleta declarada no tema: `surface` (matiz 245, croma 12), `surface-variant` (246, 18), `neutral` (240, 6) e `error` (25, 84). Cada token SHALL trazer, ao lado do hex, a paleta e o tom de origem. `--primary` SHALL continuar `#4DB3FF`, como semente fixa, e as cores de marca dos tiles ficam fora das paletas. A cor SHALL NOT variar com o papel de parede.

#### Scenario: Token gerado
- **WHEN** o hex de um token neutro ou de perigo é comparado ao tom declarado da sua paleta
- **THEN** ele é igual ao tom calculado, conferido pelo script de geração

#### Scenario: Papel de parede trocado
- **WHEN** o usuário troca o papel de parede no editor
- **THEN** nenhuma cor de superfície, texto ou foco muda

### Requirement: Contraste por diferença de tom
Um par de texto e fundo SHALL ter pelo menos 50 tons de diferença (4,5:1). Um contorno ou componente gráfico sobre o fundo SHALL ter pelo menos 40 (3:1). Texto desativado (`--text-disabled`) fica fora da regra. A exceção já registrada da barra superior sobre o papel de parede continua valendo, porque o papel de parede não é uma paleta.

#### Scenario: Texto mudo sobre superfície
- **WHEN** um teste do tema compara `--text-muted` (tom 75) com `--surface` (tom 10)
- **THEN** a diferença é de pelo menos 50 tons e a razão de contraste, de pelo menos 4,5:1

#### Scenario: Token novo sem contraste
- **WHEN** um token de texto novo fica a menos de 50 tons do fundo em que é usado
- **THEN** o teste de contraste do tema falha

### Requirement: Escala de forma
Os raios SHALL vir da escala de forma do tema: `--shape-md` 12 (cartões, botões, itens, tiles; `--radius` é apelido dele), `--shape-lg` 20 (cartões claros), `--shape-xl` 28 (dock) e `--shape-full`, metade da altura (chip, keycap, botão redondo). Um componente SHALL NOT usar raio fora da escala.

#### Scenario: Raio do menu
- **WHEN** o menu de contexto é desenhado
- **THEN** o raio é `--shape-lg` (20), e o item dentro dele usa `--shape-md` (12)

### Requirement: Tokens de movimento
Toda transição SHALL usar uma curva com nome (`--easing-standard`, `--easing-standard-decelerate`, `--easing-standard-accelerate`, `--easing-emphasized-decelerate`, `--easing-emphasized-accelerate`, com os valores do M3) e uma duração da escala (`--duration-short1..4`, `--duration-medium1..4`, `--duration-long1..4`, de 50 a 600 ms em passos de 50). Ficam fora da escala, como exceções do autor, `--tab-fade` (180 ms) e `--dock-reorder` (240 ms).

#### Scenario: Elemento que entra
- **WHEN** o menu de contexto, um ícone carregado ou o logo do splash aparecem
- **THEN** a transição usa `--easing-emphasized-decelerate`

#### Scenario: Magnification do dock
- **WHEN** o foco anda pelo dock
- **THEN** os ícones seguem a animação própria da Magnification (210 ms, ease-out, no código), que fica fora da escala e não muda

#### Scenario: Duração fora da escala
- **WHEN** um componente do tema pede 170 ms
- **THEN** o build do tema avisa, e o componente passa a usar um degrau da escala ou uma exceção registrada

### Requirement: Níveis de elevação
Sombra SHALL existir só em camadas que flutuam sobre outras, por três tokens: `--elevation-1` (0 4 24, preto a 45%: menu de contexto, modal e diálogo claros), `--elevation-2` (0 8 24, 40%: aviso) e `--elevation-3` (0 16 40, 45%: dock). Elemento que não flutua (tile, mini-card, item, painel) SHALL NOT ter sombra, e nenhuma tela SHALL desenhar sombra com valores no código.

#### Scenario: Tile em foco
- **WHEN** um tile da grade de Apps recebe o foco
- **THEN** ele ganha o anel azul de 4 px e a escala 1,05, sem sombra

#### Scenario: Aviso
- **WHEN** um aviso (`.toast`) aparece
- **THEN** ele usa `--elevation-2`

### Requirement: Referências do Material Design 3
O design system SHALL seguir as regras de sistema do Material Design 3 (paleta tonal, papéis de cor, contraste por tom, escala de forma, movimento, elevação e escala tipográfica) e do Design for TV do Google, adaptadas ao Lume e registradas nesta spec. Componentes, abas em pílula, carrossel, menu escuro, camadas de estado, cor dinâmica e a fonte do M3 SHALL NOT entrar sem decisão do autor.

#### Scenario: Componente novo
- **WHEN** uma tela precisa de uma cor, um raio, uma curva ou uma sombra
- **THEN** ela usa o token do papel correspondente do M3 já registrado no tema, e a página do M3 de onde a regra vem está entre as referências abaixo

Referências:
- Cor: https://m3.material.io/styles/color/system/how-the-system-works (paletas tonais, HCT, papéis, contraste por tom)
- Papéis de cor: https://m3.material.io/styles/color/roles
- Forma: https://m3.material.io/styles/shape/corner-radius-scale
- Movimento: https://m3.material.io/styles/motion/easing-and-duration/tokens-specs
- Elevação: https://m3.material.io/styles/elevation/applying-elevation
- Tipografia: https://m3.material.io/styles/typography/type-scale-tokens
- TV: https://developer.android.com/design/ui/tv (foco, distância de leitura, área segura)

Adaptações do Lume: superfícies com croma maior que o M3 (12 e 18, contra 4 e 8 no M3); textos numa paleta à parte (`neutral`, croma 6); `--primary` fixo, sem cor dinâmica; 6 níveis de texto, em vez de 15; Plus Jakarta Sans, em vez de Roboto; foco por anel azul e escala, em vez de camada de estado; menu de contexto claro (`inverse-surface`); exceções de duração de 180 e 240 ms.

## MODIFIED Requirements

### Requirement: Tokens de cor por papel
O design system SHALL nomear as cores por papel, cada uma vinda de um tom de paleta (ver "Paletas tonais"). Superfícies (paleta `surface`): `--bg` `#0C171F` (T7), `--surface` `#121D25` (T10), `--surface-row` `#18232C` (T13), `--surface-icon` `#1B252E` (T14), `--control-disabled` `#1F2932` (T16). Superfícies de destaque (paleta `surface-variant`): `--surface-focus` `#1E303F` (T19), `--control` `#293B4B` (T24), `--border` `#304252` (T27). Texto (paleta `neutral`): `--text` (igual a `--secondary`, `#F1F4F9`, T96), `--text-soft` `#DFE3E8` (T90), `--text-muted` `#B5B9BE` (T75), `--text-disabled` `#5D6165` (T41). Papéis: `--primary` `#4DB3FF` (foco e controle, semente fixa), `--on-primary` igual a `--bg`, `--secondary` `#F1F4F9` (neutra); `--accent` e `--on-accent` são apelidos de compatibilidade. Perigo (paleta `error`): `--danger-bg` `#690005` (T20), `--danger-fg` `#FFB4AB` (T80) (só sobre superfícies escuras). Cartões claros de sobreposição (o menu de contexto e, no mapeamento de controle, o modal de escuta de tecla e o diálogo de confirmação): `--menu-surface` `#F1F4F9` (`neutral` T96), `--menu-text` `#1D2730` (`surface` T15, 13,8:1), `--menu-text-muted` `#64696D` (`neutral` T44, 5,0:1 sobre o cartão) e `--menu-danger` `#BA1A1A` (`error` T40, 5,9:1). O tema SHALL ter também os apelidos de papel do M3 (`--surface-container-lowest`, `--surface-container-low`, `--surface-container`, `--surface-container-high`, `--on-surface`, `--on-surface-variant`, `--outline`, `--error`, `--error-container`, `--on-error-container`, `--inverse-surface`, `--inverse-on-surface`) apontando para os tokens acima. Cores de marca dos tiles (`--brand-*`) valem só para apps conhecidos. O texto sobre o papel de parede SHALL manter contraste de pelo menos 4,5:1.

#### Scenario: Cor de foco
- **WHEN** qualquer elemento recebe foco
- **THEN** o contorno, o sublinhado da barra ou o preenchimento de destaque usa `--primary`, e o nome do app em foco fica em `--text`

#### Scenario: Contraste
- **WHEN** o texto mudo (`--text-muted`) é desenhado sobre o ponto mais claro do papel de parede de uma tela
- **THEN** a razão de contraste é de pelo menos 4,5:1

#### Scenario: Perigo sobre o cartão claro
- **WHEN** o aviso de tecla reservada (`--menu-danger`) ou o botão "Restaurar" do diálogo de confirmação é desenhado sobre `--menu-surface`
- **THEN** a razão de contraste é de pelo menos 4,5:1, conferida por teste do tema

#### Scenario: Apelido de papel
- **WHEN** um componente novo usa `--surface-container-high`
- **THEN** ele recebe a mesma cor de `--surface-focus`

### Requirement: Tipografia
O texto SHALL usar a família Plus Jakarta Sans embutida, nos pesos 500, 700 e 800, e nenhum outro. A escala de referência no canvas lógico SHALL ter 6 níveis, todos de tamanho inteiro e par (para cair em pixel inteiro a 1,5x), e cada nível SHALL ser um pacote de tamanho, altura de linha e espaçamento de letras: 40/48, espaçamento -0,8 (título de tela, peso 800); 28/36, -0,5 (marca na barra, peso 800; relógio, peso 500 e espaçamento 0, exceção do autor); 22/28, 0 (abas, títulos de seção, itens de Configurações, texto dos tiles, linhas do editor, títulos da tela Sobre); 20/28, 0 (texto base e corpo da tela Sobre); 18/24, 0 (botões, balão do dock, aviso, subtítulo, caminho de navegação, versão, rótulo de tile, chip, painel e valores do painel de estatísticas) e 16/24, 0 (licenças de terceiros e cabeçalhos de cartão, estes em caixa alta com espaçamento de 1,2, `--tracking-caps`). Cada papel SHALL usar um só nível. Os tamanhos, pesos, alturas de linha e espaçamentos SHALL ser tokens do tema (`--font-title`, `--font-brand`, `--font-primary`, `--font-body`, `--font-label`, `--font-caption`, `--line-*` e `--tracking-*`), e nenhuma tela SHALL fixá-los no código. A altura de linha SHALL valer para texto de várias linhas; texto de uma linha continua posicionado pela linha-base da régua. O texto SHALL NOT usar tamanho menor que 16 (`MIN_FONT_SIZE`). O nome do app sob um tile da grade de Apps SHALL seguir o comportamento da aba: sem foco, cor `--text-muted` e peso 500; com o foco, cor `--text` e peso 700, com a transição de 150 ms. Texto que não cabe SHALL terminar em reticências, exceto o nome no balão do dock, que SHALL aparecer inteiro enquanto couber na área segura.

#### Scenario: Peso fora do conjunto
- **WHEN** um componente pede peso 600
- **THEN** o build do tema avisa, e a tela não usa esse peso

#### Scenario: Tamanho fora da escala
- **WHEN** um componente do tema pede um tamanho que não é um dos 6 níveis (por exemplo 17)
- **THEN** o build do tema avisa, e a tela usa o nível mais próximo

#### Scenario: Texto de várias linhas
- **WHEN** o corpo da tela Sobre quebra em duas linhas
- **THEN** a segunda linha-base fica 28 abaixo da primeira (`--line-body`)

### Requirement: Foco e movimento
O foco SHALL ser sempre visível e azul primário: contorno de 4 px (`--focus-ring`) em tiles, itens e linhas; 3 px em chips, botões e botões redondos, com `outline-offset` de 2 a 3; os botões preenchidos afastam o anel do fundo. A única exceção ao anel é o item do menu de contexto, em que o preenchimento `--primary` sobre o cartão claro basta (decisão do autor). Tiles ampliam 1,05 e botões redondos 1,08, sem sombra (ver "Níveis de elevação"). As abas não têm contorno: o foco é o sublinhado de 2 px. As transições de foco SHALL durar 150 ms (`--duration-short3`) com a curva `--easing-standard-decelerate` (`--focus-time`); a troca de aba SHALL ter fade de 180 ms (`--tab-fade`); o dock SHALL deslizar com a animação de Magnification; quando a ordem dos favoritos muda (mover, remover, desinstalar), cada ícone SHALL deslizar do lugar e do tamanho antigos para os novos em 240 ms (`--dock-reorder`, ease-in-out), e o removido encolhe e some no lugar; o menu de contexto SHALL aparecer com fade de 150 ms e subida de 8, com `--easing-emphasized-decelerate`. Um elemento sem ação (como o painel de estatísticas) SHALL NOT receber foco nem ter animação. Uma animação cujo valor de repouso é 0 SHALL receber o valor inicial em `animate_to`.

#### Scenario: Foco em botão redondo
- **WHEN** o foco chega ao botão de entradas ou à engrenagem
- **THEN** o botão ganha contorno azul de 3 px e cresce 1,08 em 150 ms

#### Scenario: Cartão sem foco
- **WHEN** o usuário navega pela tela de Configurações
- **THEN** o foco passa só pelos itens da lista, nunca pelo cartão de estatísticas

#### Scenario: Tile em foco sem sombra
- **WHEN** o foco chega a um tile da grade de Apps
- **THEN** o tile cresce 1,05 com o anel de 4 px `--primary` em 150 ms, e nenhuma sombra é desenhada em volta dele

### Requirement: Superfícies e papel de parede
O fundo de toda tela SHALL ser o papel de parede de aurora (ver `wallpaper`) sobre `--bg`; as telas de texto o desenham escurecido no próprio desenho do papel (`Painter::wallpaper(true)`). Cartões e painéis SHALL usar `.panel` (`--surface`, raio `--shape-md` de 12), sem sombra. A exceção é o menu de contexto (`.menu`): um cartão claro (`--menu-surface`, raio `--shape-lg` de 20, sombra `--elevation-1` (0 4 24, preto a 45%), que o separa de um tile branco), o único elemento claro da interface, para se destacar como camada temporária sobre as telas escuras. O dock é translúcido (`rgba(15,23,30,.55)`, raio `--shape-xl` de 28, contorno de 1 px a 14% de branco, sombra `--elevation-3`), e o nome do favorito em foco, acima dele, tem fundo transparente: só o texto (`--text`, 18, peso 700), sem balão nem sombra. O aviso (`.toast`) usa `--elevation-2`. O desenho SHALL NOT usar nenhuma camada translúcida de tela cheia, exceto o escurecimento do diálogo de confirmação, que só existe enquanto o diálogo está aberto.

#### Scenario: Escurecimento
- **WHEN** a tela de Configurações, Sobre ou Editar é desenhada
- **THEN** o escurecimento vem do desenho do papel de parede, sem camada translúcida por cima

#### Scenario: Camadas com sombra
- **WHEN** a tela Início é desenhada com o menu de contexto aberto
- **THEN** só o dock (`--elevation-3`) e o menu (`--elevation-1`) têm sombra

### Requirement: Mini-cards de papel de parede no editor
Os mini-cards de papel de parede no modo edição SHALL ser desenhados dentro do painel de aparência (`.panel` em `x = 64`, largura `1152 px`, largura útil interna `1088 px` a partir de `inner_left = 96 px`) no canvas lógico 1280x720, com proporção aproximada de 16:9, medindo `178 px` de largura por `100 px` de altura, raio de cantos de `12 px` (`--shape-md`) e vão horizontal de `16 px` entre eles. A faixa SHALL rolar horizontalmente (`Anim::WallpaperScrollX`, transição `--focus-time`) para manter o mini-card focado visível dentro dos `1088 px` úteis do painel, exibindo scrims laterais de `48 px` e setas (`20 px` em `--text-muted`) quando houver conteúdo fora da vista, além do indicador `"<i> / <total>"` no cabeçalho do painel. Cada mini-card SHALL desenhar a miniatura real com scrim escuro nos `36 px` inferiores para legibilidade do título (16 px em `x + 12`, `bottom - 12`); quando ativo, SHALL exibir anel de `2 px` `--primary` e um selo circular de `18x18 px` (`--primary` com ponto branco de `8x8 px`) no canto superior direito. O mini-card SHALL NOT ter sombra.

#### Scenario: Mini-card em repouso
- **WHEN** o mini-card está sem foco e não é o papel de parede ativo
- **THEN** ele exibe a miniatura com anel de 1 px `--control` e o nome em `--text-muted` na escala `--font-caption` (16 px, peso 500)

#### Scenario: Mini-card em foco
- **WHEN** um mini-card de papel de parede recebe o foco do D-Pad
- **THEN** ele é ampliado em `1.05x`, sem sombra, contornado pelo anel de foco azul `--primary` de `3 px`, e exibe o nome em `--text` com 16 px e peso 700

#### Scenario: Alinhamento horizontal na régua
- **WHEN** o painel de aparência é exibido em 1280x720
- **THEN** os mini-cards começam em `x = 96 px` com passo horizontal de `194 px` (`178 + 16`) dentro da janela útil de `1088 px` do painel e rolam suavemente ao navegar até o último item

### Requirement: Régua de layout
Toda tela, existente ou nova, SHALL ser montada sobre uma régua única no canvas lógico de 1280x720, para que espaços, alinhamentos e tamanhos sejam os mesmos em todas as telas e uma tela nova saia alinhada de primeira:

- **Unidade de 4.** Toda posição, tamanho, enchimento, espaço, raio e linha-base SHALL ser múltiplo de 4. O espaço entre blocos de uma tela (seções, cartões, grupos de botões) SHALL ser múltiplo de 8. Série de espaços: 4, 8, 12, 16, 24, 32, 40, 48, 64. Exceções, registradas nesta spec: medida derivada de proporção ou de animação (tile 16:9 de 172x96,75, desenhado com 97; ampliação do dock), espessuras de linha (sublinhado de 2, divisor de 1 e 2, anéis de foco de 3 e 4 e seus afastamentos) e correção ótica pela tinta (marca em x=118, para a tinta ficar a 16 do logo). Os testes `the_layout_is_on_the_ruler` (um por módulo de tela) e `the_component_boxes_are_on_the_ruler` (tema) conferem a unidade.
- **Colunas.** O conteúdo vai de x=64 a x=1216 (1152 de largura). Divisões permitidas: 6 colunas de 172 com vão de 24 (grade de Apps), 2 colunas de 552 com vão de 48 (Configurações) e 2 colunas de ficha técnica em Informações da TV (rótulos de x=64 a x=416 e valores de x=448 a x=1216). Todo bloco SHALL começar e terminar numa borda dessas colunas; o texto e o ícone dentro de um bloco alinham à borda do bloco mais o enchimento do componente.
- **Linhas fixas.** Barra superior de 0 a 88, eixo 44, linha-base 52 (requisito "Barra superior"). Tela secundária com subtítulo: caminho na linha-base 52, título na 116, subtítulo na 152, conteúdo a partir de y=212. Tela secundária sem subtítulo: caminho na 52, título na 116, conteúdo a partir de y=152 (Editar tela Início) ou janela de corte em y=152 com texto a partir de y=192 (Sobre o Lume e Informações da TV). Tela principal sob a barra (Apps): título de seção na linha-base 116 e conteúdo a partir de y=144. O conteúdo termina pelo menos 40 acima do fim da área segura.
- **Tamanhos de componente.** Alturas: 40 (chip e keycap), 48 (botões e botões redondos), 64 (linha do editor `.editor-row`, passo 72), 72 (item de Configurações `.settings-item`, passo 88); ícones: glifo de 24 (setas e ícones dentro de componentes), 36 (barra superior) e 64 (dock); raio `--shape-md` de 12 (cartões, botões, itens) ou `--shape-full`, metade da altura (chip, keycap, botão redondo); enchimento lateral de 24 a 32 em componentes de texto e de 32 em cartões e painéis. Espaço entre ícone e texto num componente: 8; entre itens de um grupo de botões, botões redondos, keycaps ou chips: 16.
- **Texto corrido.** O passo de um parágrafo ou de uma linha de lista SHALL ser múltiplo de 4: 32 para a legenda (16), 40 para corpo e rótulo (18 e 20), 44 para o principal (22). As linhas em que um parágrafo quebra ficam a uma altura de linha do seu nível (`--line-*`: 24 para legenda e rótulo, 28 para corpo e principal), e o parágrafo seguinte começa um passo depois da última.
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

### Requirement: Cartões claros de sobreposição
O modal de escuta de tecla e o diálogo de confirmação da tela "Mapeamento de controle" SHALL ser cartões claros como o menu de contexto (`--menu-surface`, raio `--shape-lg` de 20, sombra `--elevation-1` (`0 4 24` preto a 45%), texto `--menu-text` e `--menu-text-muted`), centrados no canvas, **sem camada escura de tela cheia** e sem animação contínua, com enchimento de 32 e medidas múltiplas de 4.

#### Scenario: Modal de escuta sobre a tela
- **WHEN** o modal de escuta de tecla é aberto
- **THEN** o cartão claro de 560x188 aparece no centro com sombra, a tela atrás não escurece, e não há botão nem item em foco

#### Scenario: Tecla reservada no modal
- **WHEN** o modal de escuta está aberto e o usuário pressiona uma tecla reservada
- **THEN** a faixa de 116 a 156 mostra o aviso em `--menu-danger`, e a altura e a posição do cartão não mudam

#### Scenario: Diálogo de confirmação claro
- **WHEN** o diálogo "Restaurar o padrão de fábrica?" é exibido
- **THEN** o cartão claro de 560x244 aparece sem camada escura, "Cancelar" está aceso em `--primary` com texto `--on-primary`, e "Restaurar" tem contorno e texto em `--menu-danger`

#### Scenario: Medidas do modal de escuta
- **WHEN** o modal de escuta de tecla é aberto
- **THEN** o cartão claro de 560x188 aparece em `x = 360, y = 264` com sombra, a tela atrás não escurece, e não há botão nem item em foco

#### Scenario: Faixa de dica e aviso do modal
- **WHEN** o modal está aberto
- **THEN** o título "Pressione uma tecla" (28/800) ocupa `y 32 a 76`, o subtítulo "Para: <alvo>" (18/500) `y 76 a 116` e a faixa de `y 116 a 156` mostra a dica "Voltar para cancelar" (18/500, `--menu-text-muted`), que ao pressionar uma tecla reservada troca para o aviso (18/700, `--menu-danger`) sem mudar a altura do cartão

#### Scenario: Medidas do diálogo de confirmação
- **WHEN** o diálogo "Restaurar o padrão de fábrica?" é exibido
- **THEN** o cartão claro de 560x244 aparece em `x = 360, y = 240` sem camada escura, com título (28/800, `y 32 a 76`), corpo de até 2 linhas a 24 uma da outra (18/500, `--line-label`, `y 76 a 140`) e dois botões de 48 (`y 164 a 212`, vão 16, alinhados à direita)

#### Scenario: Botões do diálogo claro
- **WHEN** os botões do diálogo de confirmação são desenhados
- **THEN** "Cancelar" tem contorno de 2 `--menu-text-muted` e texto `--menu-text`, "Restaurar" tem contorno de 2 e texto `--menu-danger`, e o botão em foco tem fundo `--primary`, texto `--on-primary` e nenhum anel, com o foco inicial em "Cancelar"
