## MODIFIED Requirements

### Requirement: Tipografia
O texto SHALL usar a família Plus Jakarta Sans embutida, nos pesos 500, 700 e 800, e nenhum outro. O cirílico, que a Plus Jakarta não tem, SHALL usar a Manrope embutida no mesmo peso; as métricas verticais (linha-base, ascendente, altura das maiúsculas) SHALL continuar as da Plus Jakarta. A escala de referência no canvas lógico SHALL ter 6 níveis, todos de tamanho inteiro e par (para cair em pixel inteiro a 1,5x), e cada nível SHALL ser um pacote de tamanho, altura de linha e espaçamento de letras: 40/48, espaçamento -0,8 (título de tela, peso 800); 28/36, -0,5 (marca na barra, peso 800; relógio, peso 500 e espaçamento 0, exceção do autor); 22/28, 0 (abas, títulos de seção, itens de Configurações, texto dos tiles, linhas do editor, títulos da tela Sobre); 20/28, 0 (texto base e corpo da tela Sobre); 18/24, 0 (botões, balão do dock, aviso, subtítulo, caminho de navegação, versão, rótulo de tile, chip, painel e valores do painel de estatísticas) e 16/24, 0 (licenças de terceiros e cabeçalhos de cartão, estes em caixa alta com espaçamento de 1,2, `--tracking-caps`). Cada papel SHALL usar um só nível. Os tamanhos, pesos, alturas de linha e espaçamentos SHALL ser tokens do tema (`--font-title`, `--font-brand`, `--font-primary`, `--font-body`, `--font-label`, `--font-caption`, `--line-*` e `--tracking-*`), e nenhuma tela SHALL fixá-los no código. A altura de linha SHALL valer para texto de várias linhas; texto de uma linha continua posicionado pela linha-base da régua. O texto SHALL NOT usar tamanho menor que 16 (`MIN_FONT_SIZE`). O nome do app sob um tile da grade de Apps SHALL seguir o comportamento da aba: sem foco, cor `--text-muted` e peso 500; com o foco, cor `--text` e peso 700, com a transição de 150 ms. Texto que não cabe SHALL terminar em reticências, exceto o nome no balão do dock, que SHALL aparecer inteiro enquanto couber na área segura.

#### Scenario: Peso fora do conjunto
- **WHEN** um componente pede peso 600
- **THEN** o build do tema avisa, e a tela não usa esse peso

#### Scenario: Tamanho fora da escala
- **WHEN** um componente do tema pede um tamanho que não é um dos 6 níveis (por exemplo 17)
- **THEN** o build do tema avisa, e a tela usa o nível mais próximo

#### Scenario: Texto de várias linhas
- **WHEN** o corpo da tela Sobre quebra em duas linhas
- **THEN** a segunda linha-base fica 28 abaixo da primeira (`--line-body`)

#### Scenario: Cirílico ao lado do latim
- **WHEN** um título em peso 800 mistura cirílico e latim ("Overscan и геометрия")
- **THEN** o cirílico vem da Manrope 800 e o latim da Plus Jakarta Sans 800, na mesma linha-base
