# home-editor Specification

## Purpose
Define o modo edição da tela Início (mockup "Editar tela Início"), em que o usuário organiza os favoritos do dock, e a persistência dessa configuração.

## Requirements

### Requirement: Acesso ao modo edição
O modo edição SHALL ser aberto pelo item "Tela Início" da tela Configurações do Lume, ou pelo item "Adicionar favoritos" do dock quando não há favoritos. A tela SHALL mostrar o caminho "Configurações › Tela Início", o título "Tela Início" (sem subtítulo), o painel de aparência (seletor de papel de parede e linha "Favoritos redondos"), o painel de favoritos e, sob o painel, o botão secundário "Restaurar padrão" à esquerda. A saída da tela de edição com a retenção de todas as alterações feitas SHALL ser realizada exclusivamente pelo botão Voltar do controle remoto.

#### Scenario: Entrar e sair
- **WHEN** o usuário abre "Editar tela Início" e depois pressiona o botão Voltar
- **THEN** a tela anterior volta a ser exibida, mantendo todas as alterações feitas

#### Scenario: Voltar equivale a Concluído
- **WHEN** o usuário pressiona Voltar no nível principal do modo edição aberto pelas Configurações
- **THEN** as alterações já feitas são mantidas e a tela "Configurações" do Lume é exibida, com o foco em "Tela Início"

#### Scenario: Voltar de quem veio do dock
- **WHEN** o modo edição foi aberto por "Adicionar favoritos" do dock e o usuário pressiona Voltar
- **THEN** as alterações são mantidas e a tela Início é exibida

### Requirement: Configurar favoritos
A seção de favoritos SHALL listar os favoritos numerados, cada um com botões para mover à esquerda, mover à direita e remover, e uma área "Adicionar app" com os apps do catálogo que ainda não são favoritos. O limite SHALL ser de 12 favoritos.

#### Scenario: Reordenar favorito
- **WHEN** o usuário move o "Spotify" (posição 4) para a esquerda
- **THEN** o Spotify passa para a posição 3, e o app que estava em 3 passa para a 4

#### Scenario: Adicionar favorito
- **WHEN** o usuário seleciona "+ Chrome" em "Adicionar app"
- **THEN** o Chrome entra no fim da lista de favoritos e sai da área "Adicionar app"

#### Scenario: Limite atingido
- **WHEN** já existem 12 favoritos
- **THEN** a área "Adicionar app" fica desabilitada e mostra "Limite de 12 favoritos"

### Requirement: Favoritos redondos
Acima do painel de favoritos, o modo edição SHALL exibir uma linha "Favoritos redondos" com uma chave de liga e desliga. O padrão de primeiro uso SHALL ser ligado (favoritos circulares no dock). Pressionar OK com o foco nessa linha SHALL alternar entre ligado e desligado (favoritos em quadrado de cantos arredondados) e gravar a configuração imediatamente. Um arquivo de configuração sem a chave SHALL ser lido como ligado.

#### Scenario: Padrão desligado no primeiro uso
(O título do cenário é o da spec antiga, mantido para a validação; o padrão agora é ligado.)
- **WHEN** o lume é aberto pela primeira vez e o usuário entra em "Tela Início"
- **THEN** a chave "Favoritos redondos" aparece ligada

#### Scenario: Ligar e desligar
(Alterna nos dois sentidos; o padrão é ligado.)
- **WHEN** o foco está em "Favoritos redondos" e o usuário pressiona OK
- **THEN** a chave passa para desligada, a alteração é gravada e o dock passa a desenhar os favoritos em quadrado de cantos arredondados

### Requirement: Restaurar padrão
O editor SHALL oferecer "Restaurar padrão", que, depois de uma confirmação, devolve os favoritos e a opção "Favoritos redondos" à configuração de primeiro uso. O histórico de uso SHALL ser mantido.

#### Scenario: Restaurar com confirmação
- **WHEN** o usuário seleciona "Restaurar padrão" e confirma
- **THEN** a configuração volta ao padrão de primeiro uso, com "Favoritos redondos" ligado

#### Scenario: Cancelar restauração
- **WHEN** o usuário seleciona "Restaurar padrão" e cancela
- **THEN** nada muda

### Requirement: Persistência da configuração
Cada alteração feita no editor (reordenação de favoritos, inclusão/remoção de apps, alternância de favoritos redondos, seleção de papel de parede ou aplicação de "Minha imagem") SHALL ser gravada no armazenamento interno antes da próxima interação. A chave `wallpaper` SHALL ser gravada no arquivo `config.v1` com o identificador do papel de parede ativo, ou `custom` para "Minha imagem", que tem ainda a chave `custom_rev` (a revisão da imagem aplicada, que nomeia o cache); o enquadramento não é gravado, porque a TV recebe a imagem já enquadrada. Arquivos de versões anteriores ou que não contenham a chave `wallpaper` SHALL ser lidos normalmente, assumindo o papel de parede padrão Aurora.

#### Scenario: Reinício da TV
- **WHEN** o usuário reordena os favoritos, liga "Favoritos redondos" ou escolhe outro papel de parede e a TV é desligada da tomada e religada
- **THEN** os favoritos e o papel de parede aparecem na nova ordem e configuração escolhidas

#### Scenario: Arquivo de configuração ilegível
- **WHEN** o arquivo de configuração está corrompido ou é de uma versão desconhecida
- **THEN** o lume inicia com a configuração padrão sem travar e registra o problema no log

#### Scenario: Arquivo de uma versão com carrossel
- **WHEN** o arquivo de configuração contém seções, apps do carrossel e intervalo, além dos favoritos
- **THEN** o lume abre com os mesmos favoritos, na mesma ordem, com "Favoritos redondos" desligado, papel de parede padrão Aurora, e ignora as chaves do carrossel e das seções

#### Scenario: Versão anterior lê `custom`
- **WHEN** um `config.v1` com `wallpaper=custom` é lido por uma versão que não conhece a imagem própria
- **THEN** o identificador é desconhecido e o lume usa a Aurora sem travar

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
