## ADDED Requirements

### Requirement: Paleta de cores de destaque
O design system SHALL oferecer 10 presets de `--primary` com fundo Splash T7 correspondente: `blue` (`#4DB3FF`, padrão), `aurora` (`#00C0B2`), `emerald` (`#57C071`), `amber` (`#EC9A00`), `coral` (`#FF8163`), `rose` (`#FA7ABC`), `violet` (`#AC96FF`), `red` (`#FF5449`), `white` (`#F1F4F9`) e `black` (`#11151A`, `--on-primary` `#F1F4F9`; nos demais, `#0C171F`), acompanhados pelo logo do Lume.

#### Scenario: Contraste da cor Branco no cartão claro
- **WHEN** a cor de destaque `Branco` está ativa e o foco está num item do menu de contexto ou diálogo claro
- **THEN** o item focado usa preenchimento escuro (`--menu-text`, `#1D2730`) e texto claro (`--menu-surface`, `#F1F4F9`), mantendo contraste de pelo menos 4,5:1

#### Scenario: Visibilidade da cor Preto sobre superfícies escuras
- **WHEN** a cor de destaque `Preto` está ativa e o foco está num tile da grade, botão ou item de Configurações
- **THEN** o texto usa `--on-primary` claro (`#F1F4F9`) e o anel/preenchimento exibe uma borda externa sutil de contraste visível sobre o fundo escuro

#### Scenario: Logo acompanha a cor de destaque com L e ponto vazados
- **WHEN** o usuário escolhe `Esmeralda` como cor de destaque
- **THEN** o quadrado arredondado do logo do Lume na barra superior, no tile "Lume" da grade, no breadcrumb e no Splash é desenhado em `Esmeralda`, com o "L" e o ponto transparentes (vazados), revelando o fundo ou papel de parede por trás

### Requirement: Layout dos modos sem abas na régua
Sem abas na barra superior, o canvas lógico de 1280x720 SHALL seguir a régua de 4 e 8 px nos três modos: `Descer` (dock em `y = 568..664` + dica `v Todos os apps` em `y = 676..700`, transição vertical de `250 ms`), `Espiada` (dock em `y = 524..620`, título em `y = 656`, 1ª linha de apps em `y = 672` com ~48 px visíveis e escurecimento progressivo no próprio `Painter::wallpaper`) e `No Dock` (item `"Todos os apps"` no dock, abrindo a grade com `--tab-fade` de `180 ms`).

#### Scenario: Medidas do modo Espiada em 1280x720
- **WHEN** a tela Início é exibida no modo `Espiada` em 1280x720 com o foco no dock
- **THEN** o dock ocupa `y = 524..620`, a primeira linha de tiles começa em `y = 672` com cerca de 48 px visíveis e todas as medidas são múltiplas de 4 px

#### Scenario: Escurecimento progressivo sem camada translúcida
- **WHEN** a tela rola do dock para a grade no modo `Espiada`
- **THEN** o escurecimento do papel de parede é interpolado no próprio desenho do papel sobre `--bg`, sem desenhar camada translúcida de tela cheia

#### Scenario: Dica do modo Descer dentro da área segura
- **WHEN** a tela Início é exibida no modo `Descer` em 1280x720
- **THEN** o dock ocupa `y = 568..664` e a dica não focável `v Todos os apps` (16/500 `--text-muted`) fica centralizada em `y = 676..700`, dentro da área segura

### Requirement: Seletor de cor, navegação e formato do dock no painel Aparência
No painel `APARÊNCIA` (`x = 96..1184`), abaixo dos mini-cards e do divisor, o layout SHALL incluir a linha `"Cor de destaque"` (`72 px`, 10 swatches), duas linhas `.editor-row` (`64 px`, passo `72 px`) para a navegação condicional em dois níveis e a linha `"Formato dos favoritos"` (`64 px`, chips `Redondo`, `Quadrado` e `Retangular` de `40 px`), todos alinhados em `x = 1160` na régua de 4 px.

#### Scenario: Alinhamento do painel Aparência na régua
- **WHEN** a tela `Configurações › Tela Início` é medida em 1280x720
- **THEN** a linha de cor de destaque, as duas linhas condicionais de navegação e a linha "Formato dos favoritos" ocupam `x = 96..1184`, alinham seus controles em `x = 1160` e usam apenas múltiplos de 4 px

#### Scenario: Foco e selo nos swatches de cor
- **WHEN** o usuário foca um swatch de cor no painel `APARÊNCIA`
- **THEN** o swatch amplia `1,08x` com anel de `3 px` afastado de `4 px` (com borda externa sutil no swatch `Preto`), e o swatch salvo exibe ícone check em `--on-primary`

#### Scenario: Métricas do dock retangular 16:9
- **WHEN** o formato dos favoritos está em `Retangular`
- **THEN** cada item parte de `112×64` (16:9 na régua de 4 px) com cantos arredondados proporcionais e reduz proporcionalmente quando necessário para manter até 13 itens (12 favoritos + "Todos os apps") e a ampliação dentro da área segura (`1152 px`)

#### Scenario: Visual do item "Todos os apps" no Dock nos três formatos
- **WHEN** o modo `No Dock` está ativo em qualquer um dos três formatos (`Redondo`, `Quadrado` ou `Retangular`)
- **THEN** o item `"Todos os apps"` exibe fundo sólido liso na cor de destaque ativa `--primary` (sem gradiente, com borda sutil de contraste quando `Preto` está ativo) e uma grade 3×3 de pontos circulares em `--on-primary` (centralizada em `Redondo`/`Quadrado` e à esquerda do rótulo traduzido `"Apps"` em `Retangular` 16:9), desenhada em SDF sem textura extra

## MODIFIED Requirements

### Requirement: Tokens de cor por papel
O design system SHALL nomear as cores por papel, cada uma vinda de um tom de paleta (ver "Paletas tonais"). Superfícies (paleta `surface`): `--bg` `#0C171F` (T7), `--surface` `#121D25` (T10), `--surface-row` `#18232C` (T13), `--surface-icon` `#1B252E` (T14), `--control-disabled` `#1F2932` (T16). Superfícies de destaque (paleta `surface-variant`): `--surface-focus` `#1E303F` (T19), `--control` `#293B4B` (T24), `--border` `#304252` (T27). Texto (paleta `neutral`): `--text` (igual a `--secondary`, `#F1F4F9`, T96), `--text-soft` `#DFE3E8` (T90), `--text-muted` `#B5B9BE` (T75), `--text-disabled` `#5D6165` (T41). Papéis: `--primary` (padrão `#4DB3FF`, configurável entre os 10 presets da paleta de cores de destaque), `--on-primary` (padrão `--bg`, ou `#F1F4F9` quando `--primary` é `Preto`), `--secondary` `#F1F4F9` (neutra); `--accent` e `--on-accent` são apelidos de compatibilidade. Perigo (paleta `error`): `--danger-bg` `#690005` (T20), `--danger-fg` `#FFB4AB` (T80) (só sobre superfícies escuras). Cartões claros de sobreposição (o menu de contexto e, no mapeamento de controle, o modal de escuta de tecla e o diálogo de confirmação): `--menu-surface` `#F1F4F9` (`neutral` T96), `--menu-text` `#1D2730` (`surface` T15, 13,8:1), `--menu-text-muted` `#64696D` (`neutral` T44, 5,0:1 sobre o cartão) e `--menu-danger` `#BA1A1A` (`error` T40, 5,9:1). O tema SHALL ter também os apelidos de papel do M3 (`--surface-container-lowest`, `--surface-container-low`, `--surface-container`, `--surface-container-high`, `--on-surface`, `--on-surface-variant`, `--outline`, `--error`, `--error-container`, `--on-error-container`, `--inverse-surface`, `--inverse-on-surface`) apontando para os tokens acima. Cores de marca dos tiles (`--brand-*`) valem só para apps conhecidos. O texto sobre o papel de parede SHALL manter contraste de pelo menos 4,5:1.

#### Scenario: Cor de foco
- **WHEN** qualquer elemento recebe foco
- **THEN** o contorno, o sublinhado da barra ou o preenchimento de destaque usa `--primary` da cor de destaque ativa (ou o par de contraste definido para `Branco` no cartão claro e `Preto` em superfícies escuras), e o nome do app em foco fica em `--text`

#### Scenario: Contraste
- **WHEN** o texto mudo (`--text-muted`) é desenhado sobre o ponto mais claro do papel de parede de uma tela
- **THEN** a razão de contraste é de pelo menos 4,5:1

#### Scenario: Perigo sobre o cartão claro
- **WHEN** o aviso de tecla reservada (`--menu-danger`) ou o botão "Restaurar" do diálogo de confirmação é desenhado sobre `--menu-surface`
- **THEN** a razão de contraste é de pelo menos 4,5:1, conferida por teste do tema

#### Scenario: Apelido de papel
- **WHEN** um componente novo usa `--surface-container-high`
- **THEN** ele recebe a mesma cor de `--surface-focus`

### Requirement: Barra superior
A barra superior SHALL seguir estas métricas (canvas lógico; "tinta" é o desenho visível, medido na captura, e "caixa" é o retângulo de layout):

| Item | Caixa | Tinta medida | Regra |
|---|---|---|---|
| Barra | 0 a 88, margens de 64 | — | eixo central em y=44 para todos os itens |
| Logo | 40x40 em x=64, y de 24 a 64 | 64 a 103, altura 40 | âncora da ponta esquerda; cor acompanha `--primary` |
| Marca "Lume" | x=118, 28 peso 800, linha-base 56 | começa a 16 do logo | logo e marca formam um bloco só |
| Abas (`À esquerda`, padrão) | a primeira em x=236, 32 entre os textos; 22, linha-base 52 | 48 da marca à primeira aba | menores que as pontas: o sublinhado as marca |
| Abas (`Centralizadas`) | grupo `Início` + 32 + `Apps` centralizado em `x = 640`, 22, linha-base 52 | centro geométrico do par em `x = 640` | mesmas fontes, peso e sublinhado; sem colidir com marca nem botões |
| Barra `Sem abas` | sem itens de aba | centro livre | apenas logo+marca à esquerda e botões+relógio à direita |
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

#### Scenario: Abas centralizadas
- **WHEN** o usuário escolhe `Com abas` e `Centralizadas`
- **THEN** o bloco formado por "Início" e "Apps" (com 32 px entre si) fica centralizado em `x = 640`, na linha-base 52, sem sobrepor a marca nem os botões da direita

#### Scenario: Barra sem abas
- **WHEN** o usuário escolhe `Sem abas`
- **THEN** a barra superior exibe apenas o logo e "Lume" à esquerda e os botões (entradas, engrenagem) e o relógio à direita, deixando o centro livre

### Requirement: Splash
O splash SHALL ser só o logo do Lume (o quadrado arredondado na cor de destaque ativa com o L e o ponto transparentes/vazados, nas proporções de `logo.rs`) centralizado sobre o fundo escuro T7 correspondente à cor de destaque ativa desde o primeiro quadro, com tamanho definido no tema, sem texto. O splash SHALL NOT exibir o azul padrão `#0C171F` antes de assumir a cor escolhida; a janela inicial do sistema (`windowBackground`) usa um fundo escuro neutro (`#0A0D10`). O tema SHALL reservar abaixo do logo uma faixa para um indicador de carregamento futuro (posição e altura em tokens), que nesta versão fica vazia e não desloca o logo. O logo SHALL entrar com fade de 300 ms (ease-out) e crescimento de 0,96 a 1 no primeiro quadro do splash, e SHALL ficar à vista por no mínimo 600 ms (abertura a frio) ou 300 ms (retorno a quente) desde o primeiro quadro antes da revelação. A revelação SHALL ser um progresso linear dividido em etapas que se sobrepõem, cada uma com curva ease-in-out, de modo que logo e interface fiquem parcialmente visíveis ao mesmo tempo, sem um momento vazio entre eles: o papel de parede passa da intensidade 0 (só o fundo T7) à intensidade normal, pela própria cor com que é desenhado; o logo some nos primeiros 60% da revelação, com fade e encolhendo até 0,92 do tamanho, aplicado só à área do logo; e a interface entra de 30% a 100% da revelação, com fade pela opacidade de cada elemento (como o fade das abas), com o conteúdo (dock ou grade) subindo 16 unidades e a barra superior parada; o papel de parede sobe de 0 a 80%. A revelação SHALL durar 600 ms na abertura a frio e 360 ms no retorno a quente. Um ícone que chegar com a interface na tela SHALL entrar com fade de 150 ms, ease-out (`--icon-in`), enquanto a cor e a inicial do app saem sob ele. Nenhuma etapa SHALL usar camada translúcida de tela cheia.

#### Scenario: Entrada do logo
- **WHEN** a janela do sistema é substituída pelo primeiro quadro do Lume com a cor padrão
- **THEN** o fundo T7 de `Azul Lume` e o logo surgem com fade, com o L e o ponto vazados revelando o fundo T7

#### Scenario: Entrada do logo com cor personalizada
- **WHEN** o usuário usa `Esmeralda` e pressiona Home (abertura a frio ou retorno a quente)
- **THEN** o splash exibe imediatamente o fundo escuro T7 de `Esmeralda` (`#091A10`) e o logo em `Esmeralda` (com L e ponto transparentes) desde o primeiro quadro, sem passar pelo azul padrão `#0C171F` e sem camada translúcida de tela cheia

#### Scenario: Revelação sem camada de tela cheia
- **WHEN** a interface é revelada
- **THEN** o papel de parede ganha intensidade pelo próprio desenho, e só a área do logo é desenhada com transparência

### Requirement: Paletas tonais
Toda cor neutra e de perigo do tema SHALL ser um tom (HCT) de uma paleta declarada no tema: `surface` (matiz 245, croma 12), `surface-variant` (246, 18), `neutral` (240, 6) e `error` (25, 84). Cada token SHALL trazer, ao lado do hex, a paleta e o tom de origem. `--primary` SHALL ter `#4DB3FF` (`Azul Lume`) como padrão compilado e poder ser sobrescrito em tempo de execução por qualquer um dos 10 presets da paleta de cores de destaque, mantendo inalteradas as superfícies neutras e de perigo; apenas o fundo T7 do Splash acompanha o preset escolhido. As cores de marca dos tiles ficam fora das paletas. A cor SHALL NOT variar automaticamente com o papel de parede.

#### Scenario: Token gerado
- **WHEN** o hex de um token neutro ou de perigo é comparado ao tom declarado da sua paleta
- **THEN** ele é igual ao tom calculado, conferido pelo script de geração

#### Scenario: Papel de parede trocado
- **WHEN** o usuário troca o papel de parede no editor
- **THEN** nenhuma cor de superfície, texto ou foco muda automaticamente
