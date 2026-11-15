## MODIFIED Requirements

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
