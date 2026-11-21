## ADDED Requirements

### Requirement: Sobrescrita dinâmica da cor de destaque em tempo de execução
O sistema de tema SHALL sobrescrever em tempo de execução `--primary`, `--on-primary`, foco sobre cartão claro, borda de contraste para foco escuro e fundo T7 do Splash a partir dos 10 presets curados (`blue`..`black`), sem recompilar o CSS, carregar arquivos externos ou alocar texturas extras na GPU.

#### Scenario: Troca imediata de cor em tempo de execução
- **WHEN** o usuário seleciona o preset `aurora` (`#00C0B2`) no editor
- **THEN** todos os componentes que leem `--primary` / `--accent`, o logo do Lume e o fundo T7 do Splash passam a usar os valores de `aurora` no quadro seguinte, sem alocar novas texturas

#### Scenario: Pré-visualização reversível
- **WHEN** o usuário move o foco para o preset `coral` no seletor de cores sem pressionar OK e sai do seletor
- **THEN** o tema aplica `coral` enquanto o foco está no swatch e restaura o preset salvo assim que o foco deixa o seletor

#### Scenario: Contraste específico de Branco e Preto
- **WHEN** o preset ativo é `white` ou `black`
- **THEN** o tema expõe foco escuro sobre `--menu-surface` para `white`, e `--on-primary` claro com borda externa sutil de contraste em superfícies escuras para `black`

## MODIFIED Requirements

### Requirement: Tema padrão lume
O app SHALL incluir o tema "lume", derivado do mockup: destaque padrão azul `#4DB3FF` (`Azul Lume`, personalizável em tempo de execução pelos 10 presets de cor de destaque), tiles com cantos arredondados e cores de marca, tipografia sans-serif geométrica, o dock translúcido de cantos arredondados e, atrás de tudo, o papel de parede de aurora boreal (ver `wallpaper`), que substitui o fundo liso azul-petróleo, mantido como cor de reserva. Nas abas, o tema não define contorno de foco: o foco é indicado pelo sublinhado, na cor primária com o foco na barra e branco com o foco no conteúdo. O tema SHALL nomear as cores por papel: `--primary` (a cor de foco e de controle, padrão `#4DB3FF`), `--on-primary` e `--secondary` (branco `#F1F4F9`, a cor neutra e do texto), mantendo `--accent` e `--on-accent` como apelidos para temas antigos. Todo contorno de foco (tiles da grade de Apps, itens de Configurações, linhas do editor, chips, botões da barra superior) SHALL usar a cor primária ativa, com o leve aumento de escala dos tiles.

#### Scenario: Comparação visual com o mockup
- **WHEN** as telas Apps e Editar são renderizadas a 1280x720 lógico com a cor padrão `Azul Lume` e os mesmos dados do mockup
- **THEN** cores, espaçamentos, raios e tamanhos de fonte coincidem com os de `mockup/` dentro de uma tolerância de 2 px lógicos e de diferenças imperceptíveis de cor, exceto onde mudanças posteriores redefiniram o visual

#### Scenario: Início com dock
- **WHEN** a tela Início é renderizada com cinco favoritos
- **THEN** ela mostra o papel de parede, a barra superior e o dock translúcido embaixo, sem carrossel
