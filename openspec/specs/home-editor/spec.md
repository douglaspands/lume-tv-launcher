# home-editor Specification

## Purpose
Define o modo edição da tela Início (mockup "Editar tela Início"), em que o usuário organiza os favoritos do dock, e a persistência dessa configuração.

## Requirements

### Requirement: Acesso ao modo edição
O modo edição SHALL ser aberto pelo item "Tela Início" da tela Configurações do Lume, ou pelo item "Adicionar favoritos" do dock quando não há favoritos. A tela SHALL mostrar o caminho "Configurações › Tela Início", o título "Tela Início" (sem subtítulo), o painel de aparência (seletor de papel de parede, seletor de cor de destaque, seletor condicional de navegação em dois níveis e seletor "Formato dos favoritos"), o painel de favoritos e, sob o painel, o botão secundário "Restaurar padrão" à esquerda. A saída da tela de edição com a retenção de todas as alterações feitas SHALL ser realizada exclusivamente pelo botão Voltar do controle remoto.

#### Scenario: Entrar e sair
- **WHEN** o usuário abre "Tela Início" e depois pressiona o botão Voltar
- **THEN** a tela anterior volta a ser exibida, mantendo todas as alterações confirmadas

#### Scenario: Voltar equivale a Concluído
- **WHEN** o usuário pressiona Voltar no nível principal do modo edição aberto pelas Configurações
- **THEN** as alterações já confirmadas são mantidas e a tela "Configurações" do Lume é exibida, com o foco em "Tela Início"

#### Scenario: Voltar de quem veio do dock
- **WHEN** o modo edição foi aberto por "Adicionar favoritos" do dock e o usuário pressiona Voltar
- **THEN** as alterações são mantidas e a tela Início é exibida

### Requirement: Configurar favoritos
A seção de favoritos SHALL listar os favoritos numerados, cada um com botões para mover à esquerda, mover à direita e remover, e uma área "Adicionar app" com os apps do catálogo que ainda não são favoritos. O limite SHALL ser de 12 favoritos. Quando o modo `Sem abas` + `No Dock` estiver ativo, a seção SHALL exibir também o cartão `"Todos os apps"` na posição configurada no dock, com botões para mover à esquerda e à direita, sem opção de remoção e sem consumir uma das 12 vagas de favoritos.

#### Scenario: Reordenar favorito
- **WHEN** o usuário move o "Spotify" (posição 4) para a esquerda
- **THEN** o Spotify passa para a posição 3, e o app que estava em 3 passa para a 4

#### Scenario: Adicionar favorito
- **WHEN** o usuário seleciona "+ Chrome" em "Adicionar app"
- **THEN** o Chrome entra no fim da lista de favoritos e sai da área "Adicionar app"

#### Scenario: Limite atingido
- **WHEN** já existem 12 favoritos
- **THEN** a área "Adicionar app" fica desabilitada e mostra "Limite de 12 favoritos"

#### Scenario: Reordenar "Todos os apps" no modo No Dock
- **WHEN** o modo `No Dock` está ativo e o usuário move o cartão "Todos os apps" para a esquerda na seção de favoritos
- **THEN** a nova posição do botão "Todos os apps" no dock é gravada imediatamente e o cartão não oferece opção de remoção

#### Scenario: Limite de 12 favoritos preservado no modo No Dock
- **WHEN** o modo `No Dock` está ativo e existem 11 favoritos mais o item "Todos os apps"
- **THEN** o usuário ainda consegue adicionar um 12º favorito antes de atingir "Limite de 12 favoritos"

### Requirement: Restaurar padrão
O editor SHALL oferecer "Restaurar padrão", que, depois de uma confirmação, devolve os favoritos, o formato dos favoritos (`Redondo`), a cor de destaque (`Azul Lume`) e a navegação da tela inicial (`Com abas`, `À esquerda`) à configuração de primeiro uso. O histórico de uso SHALL ser mantido.

#### Scenario: Restaurar com confirmação
- **WHEN** o usuário personalizou a cor para `Coral`, ativou `Sem abas` + `Espiada`, mudou o formato dos favoritos para `Retangular`, seleciona "Restaurar padrão" e confirma
- **THEN** a configuração volta ao padrão de primeiro uso (`Azul Lume`, `Com abas`, `À esquerda`, formato `Redondo` e favoritos padrão)

#### Scenario: Cancelar restauração
- **WHEN** o usuário seleciona "Restaurar padrão" e cancela
- **THEN** nada muda

### Requirement: Persistência da configuração
Cada alteração confirmada no editor (reordenação de favoritos, posição do botão "Todos os apps", inclusão/remoção de apps, formato dos favoritos no dock, seleção de papel de parede ou aplicação de "Minha imagem", cor de destaque e modo de navegação da tela inicial) SHALL ser gravada no armazenamento interno (`files/config.v1`) antes da próxima interação. A chave `wallpaper` SHALL ser gravada com o identificador do papel de parede ativo, ou `custom` para "Minha imagem" (com `custom_rev`). As novas chaves opcionais (`accent_color`, `tabs_visible`, `tabs_align`, `tabless_mode`, `dock_shape` e posição do botão "Todos os apps") SHALL ser retrocompatíveis: arquivos de versões anteriores sem essas chaves SHALL assumir `blue`, `Com abas`, `À esquerda`, `Descer` e derivar `dock_shape` da chave legada `round_favorites` (`1` ou ausente → `Redondo`, `0` → `Quadrado`), e valores desconhecidos SHALL cair no padrão correspondente sem travar.

#### Scenario: Reinício da TV
- **WHEN** o usuário reordena os favoritos, liga "Favoritos redondos", escolhe outro papel de parede, outra cor de destaque ou outro modo de navegação e a TV é desligada da tomada e religada
- **THEN** os favoritos, o papel de parede, a cor de destaque e a navegação aparecem na configuração escolhida

#### Scenario: Arquivo de configuração ilegível
- **WHEN** o arquivo de configuração está corrompido ou é de uma versão desconhecida
- **THEN** o lume inicia com a configuração padrão sem travar e registra o problema no log

#### Scenario: Arquivo de uma versão com carrossel
- **WHEN** o arquivo de configuração contém seções, apps do carrossel e intervalo, além dos favoritos
- **THEN** o lume abre com os mesmos favoritos, na mesma ordem, com "Favoritos redondos" desligado, papel de parede padrão Aurora, cor `Azul Lume`, `Com abas` à esquerda, e ignora as chaves do carrossel e das seções

#### Scenario: Versão anterior lê `custom`
- **WHEN** um `config.v1` com `wallpaper=custom` é lido por uma versão que não conhece a imagem própria
- **THEN** o identificador é desconhecido e o lume usa a Aurora sem travar

#### Scenario: Arquivo de versão anterior sem novas chaves
- **WHEN** o Lume lê um `files/config.v1` gerado antes desta mudança
- **THEN** ele mantém os favoritos e o papel de parede existentes e assume `Azul Lume` e `Com abas` à esquerda sem erro

#### Scenario: Valor desconhecido na configuração
- **WHEN** `files/config.v1` contém `accent_color=unknown`
- **THEN** o Lume usa `Azul Lume` (`blue`) sem travar

### Requirement: Botões do modo edição
O modo edição SHALL exibir no rodapé sob o painel de favoritos apenas o botão secundário "Restaurar padrão", posicionado na margem esquerda com 48 de altura. O botão "Concluído" SHALL NOT existir, pois todas as alterações são persistidas no ato e o botão Voltar do controle remoto é o meio de retorno. Pressionar Baixo a partir da última linha de apps a adicionar SHALL mover o foco para "Restaurar padrão".

#### Scenario: Baixo vai para Concluído
- **WHEN** o foco está nos apps a adicionar e o usuário pressiona Baixo na última linha
- **THEN** o foco vai para "Restaurar padrão", já que o botão "Concluído" não existe

#### Scenario: Restaurar pela esquerda
- **WHEN** o foco está no rodapé
- **THEN** "Restaurar padrão" é o único botão exibido, posicionado na margem esquerda

#### Scenario: Foco inicial
- **WHEN** o modo edição é aberto com favoritos
- **THEN** o foco começa na linha de favoritos redondos ou na seção de papéis de parede

### Requirement: Seleção de papel de parede no modo edição
Acima do painel de favoritos, o modo edição SHALL exibir uma seção "Papel de parede" composta por uma linha de mini-cards retangulares (16:9). Cada mini-card exibe a miniatura e o nome do papel de parede. O primeiro mini-card SHALL ser o slot do celular (capacidade `custom-wallpaper`): "+ Do celular", com ícone de celular, enquanto não há imagem própria, e "Minha imagem", com a miniatura dela, depois. Os 6 do catálogo vêm em seguida. Ao abrir, o foco SHALL ficar no papel de parede ativo (a Aurora no primeiro uso). Navegar com as setas Esquerda e Direita percorre os mini-cards, e pressionar OK ativa o papel de parede selecionado; no card "+ Do celular", OK abre a tela Receber imagem; em "Minha imagem" já ativa, OK abre o menu dela.

#### Scenario: Foco no mini-card de papel de parede
- **WHEN** o foco está em um mini-card de papel de parede
- **THEN** o mini-card recebe o contorno de foco do tema e o rótulo ganha destaque

#### Scenario: Seleção de novo papel de parede
- **WHEN** o usuário navega até o mini-card "Floresta Noturna" e pressiona OK
- **THEN** o fundo da tela muda imediatamente para a Floresta Noturna, a opção torna-se a ativa e a configuração é gravada

#### Scenario: Navegação vertical a partir dos mini-cards
- **WHEN** o foco está nos mini-cards de papel de parede e o usuário pressiona Baixo
- **THEN** o foco passa para a seção de favoritos (ou primeiro botão de reordenação)

#### Scenario: Card vazio do celular
- **WHEN** não há imagem própria e o usuário pressiona OK em "+ Do celular"
- **THEN** a tela Receber imagem é aberta

#### Scenario: Ativar "Minha imagem"
- **WHEN** existe imagem própria, outro papel de parede está ativo e o usuário pressiona OK em "Minha imagem"
- **THEN** o fundo passa para a imagem própria, como foi aplicada, e a configuração é gravada

#### Scenario: Primeiro uso
- **WHEN** o editor é aberto pela primeira vez, com a Aurora como papel de parede
- **THEN** o card do celular é o primeiro da linha e o foco está no card da Aurora, marcado como ativo

#### Scenario: OK em "Minha imagem" ativa
- **WHEN** "Minha imagem" já é o papel de parede ativo e o usuário pressiona OK nela
- **THEN** abre o menu do card, com "Trocar imagem" entre as opções

#### Scenario: Menu do card "Minha imagem"
- **WHEN** o foco está em "Minha imagem" e o usuário pressiona Menu
- **THEN** abre o menu de contexto com "Usar esta imagem", "Trocar imagem" e "Remover", em que "Trocar imagem" abre a tela Receber imagem (para enviar outra foto ou outro ajuste) e "Remover" pede confirmação

#### Scenario: Voltar da tela Receber imagem ou Confirmar
- **WHEN** o usuário sai da tela Receber imagem, ou aplica na tela Confirmar
- **THEN** o modo edição volta com o foco no card do celular

### Requirement: Seletor de cor de destaque no modo edição
No painel `APARÊNCIA` de `Configurações › Tela Início`, o editor SHALL exibir a linha `"Cor de destaque"` com os 10 swatches circulares da paleta curada. Navegar com Esquerda/Direita SHALL aplicar pré-visualização ao vivo em toda a tela; `OK` SHALL gravar em `config.v1`; sair sem `OK` SHALL reverter para a cor salva.

#### Scenario: Pré-visualização ao vivo no D-pad
- **WHEN** a cor salva é `Azul Lume`, o foco está no seletor de cor e o usuário pressiona Direita até `Esmeralda` sem pressionar OK
- **THEN** os destaques da tela de edição (títulos em `--primary`, anel de foco e nome da cor) mudam imediatamente para `Esmeralda`

#### Scenario: Sair sem confirmar reverte a cor
- **WHEN** o usuário está pré-visualizando `Esmeralda` sem pressionar OK e pressiona Baixo para ir à próxima linha
- **THEN** os destaques voltam para `Azul Lume` e `config.v1` mantém `blue`

#### Scenario: Confirmar nova cor com OK
- **WHEN** o foco está no swatch `Esmeralda` e o usuário pressiona OK
- **THEN** o ícone de ativo (check) passa para `Esmeralda`, a cor `emerald` é gravada em `config.v1` e passa a valer em todo o launcher

### Requirement: Seletor condicional de navegação em dois níveis
No painel `APARÊNCIA`, o editor SHALL exibir duas linhas condicionais com rótulos limpos de produto: `"Abas na barra superior"` (`Com abas` | `Sem abas`), seguida por `"Posição das abas"` (`À esquerda` | `Centralizadas`) quando `Com abas`, ou `"Acesso aos apps"` (`Descer` | `Espiada` | `No Dock`) quando `Sem abas`, gravando imediatamente ao pressionar `OK`.

#### Scenario: Subopções quando Com abas está ativo
- **WHEN** `"Abas na barra superior"` está em `"Com abas"`
- **THEN** a linha seguinte exibe `"Posição das abas"` com as opções `"À esquerda"` e `"Centralizadas"`

#### Scenario: Subopções quando Sem abas está ativo
- **WHEN** o usuário seleciona `"Sem abas"` em `"Abas na barra superior"`
- **THEN** a linha seguinte passa imediatamente a exibir `"Acesso aos apps"` com as opções `"Descer"`, `"Espiada"` e `"No Dock"`

#### Scenario: Ativar modo No Dock atualiza a lista de favoritos
- **WHEN** o usuário seleciona `"No Dock"` em `"Acesso aos apps"`
- **THEN** o cartão reordenável `"Todos os apps"` passa a aparecer na seção `"ORDEM DOS FAVORITOS"` abaixo do painel de aparência

### Requirement: Seletor de formato dos favoritos no dock
No painel `APARÊNCIA`, o editor SHALL exibir a linha `"Formato dos favoritos"` com três opções em chips: `"Redondo"` (padrão de primeiro uso), `"Quadrado"` e `"Retangular"` (banners 16:9 de TV). Pressionar `OK` em uma opção SHALL aplicar o formato imediatamente e gravar `dock_shape` em `files/config.v1`.

#### Scenario: Padrão Redondo no primeiro uso
- **WHEN** o lume é aberto pela primeira vez e o usuário entra em "Tela Início"
- **THEN** o chip `"Redondo"` aparece ativo na linha `"Formato dos favoritos"`

#### Scenario: Trocar para Quadrado ou Retangular
- **WHEN** o foco está no chip `"Retangular"` em `"Formato dos favoritos"` e o usuário pressiona OK
- **THEN** o chip `"Retangular"` fica ativo, `dock_shape=banner` é gravado imediatamente e o dock passa a desenhar os favoritos em formato retangular 16:9 com banners de TV
