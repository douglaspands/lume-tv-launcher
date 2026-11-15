## ADDED Requirements

### Requirement: Tela Receber imagem na régua
A tela Receber imagem SHALL ser secundária sem subtítulo: caminho "Configurações › Tela Início › Imagem do celular", título "Imagem do celular" e conteúdo em y=152. À esquerda, em x=64, um quadrado branco de 320x320 (raio `--shape-md`) com o QR e zona de silêncio de pelo menos 4 módulos. À direita, em x=432, um `.panel` até x=1216 com os passos, o endereço, o código e o estado. Abaixo do QR fica "Cancelar" (`.button-secondary`).

#### Scenario: Layout em 1280x720
- **WHEN** a tela Receber imagem é desenhada em 1280x720 com rede
- **THEN** o QR ocupa x 64 a 384 e y 152 a 472 sobre branco, e o painel ocupa x 432 a 1216 a partir de y=152, na régua de 4 px

#### Scenario: QR nítido
- **WHEN** a tela é desenhada em 1280x720 ou 1920x1080
- **THEN** cada módulo do QR tem um número inteiro de pixels do buffer, sem suavização entre módulos

#### Scenario: Painel do endereço
- **WHEN** a tela mostra o endereço e o código
- **THEN** os passos numerados usam 18/500 `--text-muted`, o endereço usa 24/700 `--text`, o código usa 40/800 `--primary` com espaçamento entre letras, e o estado fica na base do painel em 18/700

#### Scenario: Celular conectado
- **WHEN** a página está aberta e sendo usada no celular
- **THEN** o estado na base do painel diz "Celular conectado · editando…" em 18/700 `--primary`, sem relógio

#### Scenario: Último minuto
- **WHEN** falta menos de um minuto para o código expirar
- **THEN** o estado diz "O código expira em 0:45" em 18/700 `--text`, e só então o relógio aparece

### Requirement: Tela Confirmar imagem na régua
A tela Confirmar SHALL mostrar a imagem recebida em tela cheia, sem barra superior, caminho nem escurecimento, e um cartão `.panel` preso à base da área segura, centrado, com 640 de largura: à esquerda, a pergunta "Usar como papel de parede?" em 18/700 `--text`; à direita, "Cancelar" (`.button-secondary`) e "Aplicar" (`.button`, o único primário), com 16 entre eles. O cartão não escurece a imagem fora dele.

#### Scenario: Cartão de confirmação
- **WHEN** a tela Confirmar é desenhada em 1280x720
- **THEN** o cartão ocupa x 320 a 960 e termina 18 acima da base, com a pergunta à esquerda e Cancelar e Aplicar à direita, na régua de 4 px

#### Scenario: Foco inicial
- **WHEN** a tela Confirmar abre
- **THEN** o foco está em Aplicar, com o anel azul de 3 px

### Requirement: Edição na página do celular
A página SHALL ter dois estados. Sem foto, só a escolha: o botão primário "Escolher foto" numa área tracejada (que aceita soltar o arquivo no computador), com os formatos e o tamanho ideal embaixo. Com foto, a escolha some: a prévia 16:9 (raio `--radius` sobre `#000`) vem primeiro, com "Trocar foto" no canto dela, seguida dos modos e dos controles (ver "Controles da edição na página"). A prévia SHALL aceitar pinça e arraste sem rolar a página.

#### Scenario: Depois de escolher a foto
- **WHEN** o usuário escolhe uma foto
- **THEN** a área de escolha some, a prévia aparece no topo com "Trocar foto" no canto, e cada cartão de modo mostra a própria foto naquele modo

### Requirement: Controles da edição na página
Sob "Como a foto ocupa a TV", os cinco modos SHALL ser cartões com a miniatura da própria foto naquele modo e o nome (Inteira, Tela cheia, Esticar, Fundo suave e Mosaico), seguidos da frase do modo escolhido e dos controles dele: na Tela cheia, o slider de zoom e "Centralizar"; no Mosaico, "Iguais | Espelhadas" e o slider "Tamanho das cópias". "Enviar para a TV" SHALL ficar preso à base da tela no celular e virar "Enviar de novo" depois do envio.

#### Scenario: Modo ativo na página
- **WHEN** o modo Tela cheia (Preencher) está ativo na página
- **THEN** o cartão dele tem contorno e nome em `--primary`, a frase "Cobre a tela, cortando as bordas" aparece abaixo dos cartões, e abaixo dela o slider de zoom com "100%", a dica "Arraste a prévia para mover" e "Centralizar"

#### Scenario: Modo desabilitado na página
- **WHEN** a foto é pequena e só o Mosaico (Repetir) está disponível
- **THEN** os outros cartões aparecem esmaecidos com um cadeado, e tocar num deles mostra o motivo, sem trocar de modo

#### Scenario: Enviar sempre à mão
- **WHEN** o usuário ajusta a foto num celular de 360 px
- **THEN** "Enviar para a TV" fica preso à base da tela enquanto ele rola, e depois de enviar o botão diz "Enviar de novo"

#### Scenario: Computador
- **WHEN** a página está em edição num computador de 1440 px
- **THEN** a prévia fica à esquerda e, à direita, os modos em três colunas, os controles e "Enviar para a TV"

### Requirement: Mini-card do celular no editor
O card do celular SHALL seguir as medidas dos outros mini-cards (178x100, raio 12, mesmo foco e selo de ativo). Vazio, ele mostra fundo `--surface` com contorno tracejado de 2 `--border`, o ícone de celular (Lucide, 32 `--text-muted`) centrado e o rótulo "+ Do celular". Com imagem, mostra a miniatura da imagem aplicada e o rótulo "Minha imagem".

#### Scenario: Card vazio
- **WHEN** não há imagem própria e o editor é exibido
- **THEN** o primeiro mini-card mostra o ícone de celular sobre `--surface` com contorno tracejado e o rótulo "+ Do celular"

#### Scenario: Card com imagem ativa
- **WHEN** "Minha imagem" é o papel de parede ativo
- **THEN** o card mostra a miniatura da imagem aplicada, o anel `--primary` de 2 e o selo de ativo, como os outros

### Requirement: Página do celular segue a marca
A página servida ao celular SHALL usar as cores e as formas do tema Lume (`--bg`, `--surface`, `--primary`, `--text`, raio `--radius`), em fundo escuro, com a fonte do sistema do aparelho (sem baixar fonte), o logo do Lume (as mesmas proporções do logo da TV) e o nome "Lume" no topo, sem imagens externas. As cores e formas SHALL vir de `themes/lume/theme.css` na compilação, para não divergir da TV.

#### Scenario: Cores da marca
- **WHEN** a página é aberta no celular
- **THEN** o fundo usa `--bg`, o botão "Enviar para a TV" usa `--primary` com texto `--on-primary`, e o logo aparece ao lado do nome "Lume"

#### Scenario: Tema mudou
- **WHEN** um valor de cor muda em `themes/lume/theme.css` e o app é recompilado
- **THEN** a página usa o novo valor sem edição manual dela

## MODIFIED Requirements

### Requirement: Canvas, margens e grade
O layout SHALL ser desenhado no canvas lógico de 1280x720 (ver `display-adaptation`), com margem lateral de 64 e área segura de 32 na horizontal e 18 na vertical. A barra superior segue o requisito "Barra superior". Telas secundárias começam em x=64 com largura de 1152; nelas o caminho de navegação fica na linha-base 52 (a tela raiz, Configurações, mostra só "Configurações"), o título (40, peso 800) na 116 (64 abaixo do caminho) e, quando a tela possui subtítulo (18, linha-base 152), o conteúdo começa em y=212 (`page::CONTENT_TOP`). Nas telas secundárias sem subtítulo, o conteúdo começa em y=152 no editor e nas Ferramentas (`page::PLAIN_CONTENT_TOP`, 36 abaixo da linha-base do título) ou usa máscara de corte em y=152 com o texto começando em y=192 e passo de rolagem de 96 nas telas Sobre o Lume e Informações da TV. A versão instalada, quando mostrada, fica no canto inferior direito da área segura, em texto mudo de 18. A grade de Apps tem 6 colunas, espaço de 24 e tiles 16:9. A tela de Configurações divide a largura de 1152 em duas colunas iguais de 552, com 48 de vão: os itens (72 de altura, passo 88) na esquerda e o painel de estatísticas ("Recursos da TV", 312 de altura), alinhado ao primeiro item, na direita (x=664, cabeçalho `.card-heading` em y+24, três linhas de métricas em y+72+i·80 com rótulo 18/500 `--text-muted` à esquerda, valor 18/700 `--text` à direita e barra de 8 px de altura e raio 4 em top+36). No editor, o conteúdo em y=152 abre com o painel de aparência (`.panel`, enchimento lateral de 32 e vertical de 24, largura útil interna de 1088 em x=96), que reúne o cabeçalho "APARÊNCIA" em `--primary` à esquerda e o indicador `"1 / 7 ›"` (16/700, `--text-muted`) à direita, 32 acima da faixa horizontal de 7 mini-cards de papel de parede (primeiro o do celular, depois os 6 do catálogo; 178x100, vão 16), um divisor de 1 px (`--surface-row`) 24 abaixo dos mini-cards e, 16 abaixo do divisor, a linha "Favoritos redondos" (`.editor-row`, 64 de altura, largura 1088, chave Liga/Desliga de 48x28 à direita com bolinha de 20). A 24 abaixo do painel de aparência fica o painel de favoritos (`.panel`, enchimento lateral de 32 e vertical de 24): cabeçalho "ORDEM DOS FAVORITOS" em `--primary` à esquerda e dica (16/500, `--text-muted`) à direita, linha de descrição (48 de altura, texto 16/500 `--text-muted`), cartões de 168x116 (56 de faixa de nome colorida + 60 de faixa de controles em `--surface-row` com 3 botões redondos de 44x44) com 16 de espaço (6 por linha, preenchendo os 1088 úteis), 20 entre os cartões e "Adicionar app", chips de 40 de altura (raio 20) com 16 de espaço entre linhas e 24 abaixo do painel até o botão secundário "Restaurar padrão" (48 de altura, em x=64, precedido por uma linha divisória de 2 px em y-12). O dock tem ícones de 64, espaço de 16, enchimento de 16 e pico de ampliação de 1,6. Valores que mudam SHALL ser alterados na constante do módulo, e esta spec SHALL ser atualizada junto.

#### Scenario: Botão novo na barra
- **WHEN** um botão é adicionado à barra superior
- **THEN** ele tem 48 de diâmetro, glifo de 36, fica 64 à esquerda do vizinho (16 de vão) e segue a tabela do requisito "Barra superior"

#### Scenario: Painel ao lado dos itens
- **WHEN** a tela de Configurações é exibida em 1280x720
- **THEN** a lista ocupa x de 64 a 616 e o painel de estatísticas ocupa x de 664 a 1216 com 312 de altura, em colunas iguais

#### Scenario: Seis cartões por linha no editor
- **WHEN** a tela Editar tela Início é exibida em 1280x720 com 6 favoritos
- **THEN** os 6 cartões de 168 com vão de 16 cabem na mesma linha, preenchendo os 1088 de largura útil do painel sem sobra à direita

### Requirement: Mini-cards de papel de parede no editor
Os 7 mini-cards de papel de parede no modo edição (primeiro o do celular, ver "Mini-card do celular no editor", depois os 6 do catálogo) SHALL ser desenhados dentro do painel de aparência (`.panel` em `x = 64`, largura `1152 px`, largura útil interna `1088 px` a partir de `inner_left = 96 px`) no canvas lógico 1280x720, com proporção aproximada de 16:9, medindo `178 px` de largura por `100 px` de altura, raio de cantos de `12 px` (`--shape-md`) e vão horizontal de `16 px` entre eles. A faixa SHALL rolar horizontalmente (`Anim::WallpaperScrollX`, transição `--focus-time`) para manter o mini-card focado visível dentro dos `1088 px` úteis do painel, exibindo scrims laterais de `48 px` e setas (`20 px` em `--text-muted`) quando houver conteúdo fora da vista, além do indicador `"<i> / <total>"` no cabeçalho do painel (total 7). Cada mini-card SHALL desenhar a miniatura real com scrim escuro nos `36 px` inferiores para legibilidade do título (16 px em `x + 12`, `bottom - 12`); quando ativo, SHALL exibir anel de `2 px` `--primary` e um selo circular de `18x18 px` (`--primary` com ponto branco de `8x8 px`) no canto superior direito. O mini-card SHALL NOT ter sombra.

#### Scenario: Mini-card em repouso
- **WHEN** o mini-card está sem foco e não é o papel de parede ativo
- **THEN** ele exibe a miniatura com anel de 1 px `--control` e o nome em `--text-muted` na escala `--font-caption` (16 px, peso 500)

#### Scenario: Mini-card em foco
- **WHEN** um mini-card de papel de parede recebe o foco do D-Pad
- **THEN** ele é ampliado em `1.05x`, sem sombra, contornado pelo anel de foco azul `--primary` de `3 px`, e exibe o nome em `--text` com 16 px e peso 700

#### Scenario: Alinhamento horizontal na régua
- **WHEN** o painel de aparência é exibido em 1280x720
- **THEN** os mini-cards começam em `x = 96 px` com passo horizontal de `194 px` (`178 + 16`) dentro da janela útil de `1088 px` do painel e rolam suavemente ao navegar até o último item
