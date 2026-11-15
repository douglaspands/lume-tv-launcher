## MODIFIED Requirements

### Requirement: Canvas, margens e grade
O layout SHALL ser desenhado no canvas lógico de 1280x720 (ver `display-adaptation`), com margem lateral de 64 e área segura de 32 na horizontal e 18 na vertical. A barra superior segue o requisito "Barra superior". Telas secundárias começam em x=64 com largura de 1152; nelas o caminho de navegação fica na linha-base 52 (a tela raiz, Configurações, mostra só "Configurações"), o título (40, peso 800) na 116 (64 abaixo do caminho) e, quando a tela possui subtítulo (18, linha-base 152), o conteúdo começa em y=212 (`page::CONTENT_TOP`). Nas telas secundárias sem subtítulo, o conteúdo começa em y=152 no editor e nas Ferramentas (`page::PLAIN_CONTENT_TOP`, 36 abaixo da linha-base do título) ou usa máscara de corte em y=152 com o texto começando em y=192 e passo de rolagem de 96 nas telas Sobre o Lume e Informações da TV. A versão instalada, quando mostrada, fica no canto inferior direito da área segura, em texto mudo de 18. A grade de Apps tem 6 colunas, espaço de 24 e tiles 16:9. A tela de Configurações divide a largura de 1152 em duas colunas iguais de 552, com 48 de vão: os itens (72 de altura, passo 88) na esquerda e o painel de estatísticas ("Recursos da TV", 312 de altura), alinhado ao primeiro item, na direita (x=664, cabeçalho `.card-heading` em y+24, três linhas de métricas em y+72+i·80 com rótulo 18/500 `--text-muted` à esquerda, valor 18/700 `--text` à direita e barra de 8 px de altura e raio 4 em top+36). No editor, o conteúdo em y=152 abre com o painel de aparência (`.panel`, enchimento lateral de 32 e vertical de 24, largura útil interna de 1088 em x=96), que reúne o cabeçalho "APARÊNCIA" em `--primary` à esquerda e o indicador `"1 / 6 ›"` (16/700, `--text-muted`) à direita, 32 acima da faixa horizontal de mini-cards de papel de parede (178x100, vão 16), um divisor de 1 px (`--surface-row`) 24 abaixo dos mini-cards e, 16 abaixo do divisor, a linha "Favoritos redondos" (`.editor-row`, 64 de altura, largura 1088, chave Liga/Desliga de 48x28 à direita com bolinha de 20). A 24 abaixo do painel de aparência fica o painel de favoritos (`.panel`, enchimento lateral de 32 e vertical de 24): cabeçalho "ORDEM DOS FAVORITOS" em `--primary` à esquerda e dica (16/500, `--text-muted`) à direita, linha de descrição (48 de altura, texto 16/500 `--text-muted`), cartões de 168x116 (56 de faixa de nome colorida + 60 de faixa de controles em `--surface-row` com 3 botões redondos de 44x44) com 16 de espaço (6 por linha, preenchendo os 1088 úteis), 20 entre os cartões e "Adicionar app", chips de 40 de altura (raio 20) com 16 de espaço entre linhas e 24 abaixo do painel até o botão secundário "Restaurar padrão" (48 de altura, em x=64, precedido por uma linha divisória de 2 px em y-12). O dock tem ícones de 64, espaço de 16, enchimento de 16 e pico de ampliação de 1,6. Valores que mudam SHALL ser alterados na constante do módulo, e esta spec SHALL ser atualizada junto.

#### Scenario: Botão novo na barra
- **WHEN** um botão é adicionado à barra superior
- **THEN** ele tem 48 de diâmetro, glifo de 36, fica 64 à esquerda do vizinho (16 de vão) e segue a tabela do requisito "Barra superior"

#### Scenario: Painel ao lado dos itens
- **WHEN** a tela de Configurações é exibida em 1280x720
- **THEN** a lista ocupa x de 64 a 616 e o painel de estatísticas ocupa x de 664 a 1216 com 312 de altura, em colunas iguais

#### Scenario: Seis cartões por linha no editor
- **WHEN** a tela Editar tela Início é exibida em 1280x720 com 6 favoritos
- **THEN** os 6 cartões de 168 com vão de 16 cabem na mesma linha, preenchendo os 1088 de largura útil do painel sem sobra à direita

### Requirement: Item de Informações da TV nas Configurações
A tela de Configurações SHALL incluir, na coluna esquerda de itens, as opções "Tela Início", "Mapeamento de controle", "Informações da TV", "Ferramentas" e "Sobre o Lume", respeitando a altura de 72 px e o passo de 88 px na régua.

#### Scenario: Terceiro item de configurações
- **WHEN** a tela de Configurações é desenhada
- **THEN** a lista exibe os itens "Tela Início", "Mapeamento de controle", "Informações da TV", "Ferramentas" e "Sobre o Lume" alinhados à régua de 4 px, sem invadir a área da versão no canto inferior direito

### Requirement: Superfícies e papel de parede
O fundo de toda tela SHALL ser o papel de parede de aurora (ver `wallpaper`) sobre `--bg`; as telas de texto o desenham escurecido no próprio desenho do papel (`Painter::wallpaper(true)`). A exceção são os padrões de tela cheia das Ferramentas (diagnóstico, limpeza, destravamento de pixel, overscan, movimento, quadrado de sincronia sobre fundo preto), que SHALL ser desenhados sem papel de parede, só com a cor do padrão sobre o quadro limpo. Cartões e painéis SHALL usar `.panel` (`--surface`, raio `--shape-md` de 12), sem sombra. A exceção é o menu de contexto (`.menu`): um cartão claro (`--menu-surface`, raio `--shape-lg` de 20, sombra `--elevation-1` (0 4 24, preto a 45%), que o separa de um tile branco), o único elemento claro da interface, para se destacar como camada temporária sobre as telas escuras. O dock é translúcido (`rgba(15,23,30,.55)`, raio `--shape-xl` de 28, contorno de 1 px a 14% de branco, sombra `--elevation-3`), e o nome do favorito em foco, acima dele, tem fundo transparente: só o texto (`--text`, 18, peso 700), sem balão nem sombra. O aviso (`.toast`) usa `--elevation-2`. O desenho SHALL NOT usar nenhuma camada translúcida de tela cheia, exceto o escurecimento do diálogo de confirmação, que só existe enquanto o diálogo está aberto.

#### Scenario: Escurecimento
- **WHEN** a tela de Configurações, Sobre ou Editar é desenhada
- **THEN** o escurecimento vem do desenho do papel de parede, sem camada translúcida por cima

#### Scenario: Camadas com sombra
- **WHEN** a tela Início é desenhada com o menu de contexto aberto
- **THEN** só o dock (`--elevation-3`) e o menu (`--elevation-1`) têm sombra

#### Scenario: Padrão de diagnóstico
- **WHEN** um padrão sólido das Ferramentas é desenhado
- **THEN** o quadro é limpo com a cor do padrão, sem papel de parede nem camada translúcida

## ADDED Requirements

### Requirement: Menu Ferramentas na régua
O menu Ferramentas SHALL usar o layout da tela de Configurações: caminho no topo, itens `.settings-item` na coluna esquerda com altura de 72 px e passo de 88 px, e a descrição do item em foco num `.panel` na coluna direita, no lugar do painel de estatísticas, tudo alinhado à régua de 4 px. O conteúdo SHALL começar em y=152, como o editor (tela secundária sem subtítulo).

#### Scenario: Menu na régua
- **WHEN** o menu Ferramentas é desenhado em 1280x720
- **THEN** os itens ficam no passo de 88 px, e o painel à direita mostra a descrição do item em foco

#### Scenario: Topo do conteúdo
- **WHEN** o menu Ferramentas é desenhado
- **THEN** o primeiro item e o painel começam em y=152, 36 abaixo da linha-base do título

#### Scenario: Último item visível
- **WHEN** o foco desce até "Armazenamento", o sexto item
- **THEN** a lista não rola: os cinco itens já estão inteiros na área segura, e o painel à direita fica parado

### Requirement: Telas de ferramenta na régua
As telas de cada ferramenta SHALL usar o caminho completo no topo, o conteúdo começando em y=152 (tela secundária sem subtítulo), itens de ação `.settings-item` à esquerda e os resultados (valores, barras, histórico) num `.panel` à direita, com rótulos em `--text-muted` e valores em `--text`, como as linhas de Informações da TV. As barras (progresso de teste, espaço usado) SHALL usar as cores e a forma da barra de memória do painel de estatísticas.

#### Scenario: Resultados no painel
- **WHEN** a ferramenta Armazenamento é desenhada em 1280x720 depois de um teste
- **THEN** o item de ação fica à esquerda, e os rótulos, valores e as barras ficam no `.panel` à direita, na régua de 4 px

### Requirement: Aviso de flashes e textos sobre os padrões
O aviso de flashes SHALL usar o cartão claro de sobreposição (`.menu`), com o texto do aviso e os botões "Iniciar" e "Cancelar". Textos sobre os padrões de tela cheia (nome do padrão, dica, tempo restante) SHALL usar um `.toast` no rodapé da tela, que some depois de 3 segundos.

#### Scenario: Aviso de flashes
- **WHEN** o aviso do destravamento de pixel é exibido
- **THEN** ele aparece como cartão claro de sobreposição, com o foco em "Cancelar"

#### Scenario: Dica sobre o padrão
- **WHEN** um padrão de diagnóstico acaba de aparecer
- **THEN** o nome do padrão aparece num `.toast` no rodapé e some depois de 3 segundos
