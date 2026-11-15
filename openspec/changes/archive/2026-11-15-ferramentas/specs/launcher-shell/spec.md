## MODIFIED Requirements

### Requirement: Tela de Configurações do Lume
A tela "Configurações" do Lume SHALL ser aberta pelo tile "Lume" da grade de Apps, com os itens "Tela Início", "Mapeamento de controle", "Informações da TV", "Ferramentas" e "Sobre o Lume", sem item do Android e com o painel de estatísticas à direita (`device-stats`), caminho "Configurações" no topo e versão instalada (`platform-compatibility`) no canto inferior direito. Voltar SHALL retraçar o caminho de entrada passo a passo até o tile "Lume".

#### Scenario: Abrir pelo tile
- **WHEN** o foco está no tile "Lume" da grade de Apps e o usuário pressiona OK
- **THEN** a tela "Configurações" do Lume é exibida com o foco no primeiro item

#### Scenario: Painel de estatísticas visível
- **WHEN** a tela "Configurações" do Lume é exibida
- **THEN** o painel de estatísticas aparece à direita dos itens, sem receber foco

#### Scenario: Caminho e versão
- **WHEN** a tela "Configurações" do Lume é exibida
- **THEN** o caminho "Configurações" aparece no topo, e a versão instalada no canto inferior direito

#### Scenario: Versão de release com "v"
- **WHEN** o build instalado veio da tag `v0.3.0` e a tela "Configurações" ou a tela Sobre é exibida
- **THEN** a versão aparece como "v0.3.0", com um "v" só

#### Scenario: Versão de build local
- **WHEN** o build instalado foi gerado localmente no commit `4d8438b` com mudanças não commitadas
- **THEN** a versão aparece como "4d8438b-dirty", sem "v"

#### Scenario: Sem item do Android
- **WHEN** a tela "Configurações" do Lume é exibida
- **THEN** ela lista apenas "Tela Início", "Mapeamento de controle", "Informações da TV", "Ferramentas" e "Sobre o Lume"

#### Scenario: Voltar ao tile
- **WHEN** o usuário pressiona Voltar na tela "Configurações" do Lume
- **THEN** a aba Apps é exibida com o foco no tile "Lume"

#### Scenario: Ir para a edição da tela Início
- **WHEN** o usuário seleciona "Tela Início" em Configurações
- **THEN** o modo edição é aberto com o caminho "Configurações › Tela Início" e o título "Tela Início"

#### Scenario: Ir para o Mapeamento de controle
- **WHEN** o usuário seleciona "Mapeamento de controle" em Configurações
- **THEN** a tela de mapeamento é aberta com o caminho "Configurações › Mapeamento de controle" e o título "Mapeamento de controle"

#### Scenario: Ir para as Ferramentas
- **WHEN** o usuário seleciona "Ferramentas" em Configurações
- **THEN** o menu Ferramentas é aberto com o caminho "Configurações › Ferramentas" (`diagnostic-tools`)

#### Scenario: Voltar passo a passo
- **WHEN** o usuário está no modo edição, no mapeamento de controle ou no menu Ferramentas aberto pelas Configurações e pressiona Voltar duas vezes
- **THEN** a primeira volta à tela "Configurações" com o foco no item de origem ("Tela Início", "Mapeamento de controle" ou "Ferramentas"), e a segunda à aba Apps com o foco no tile "Lume"

#### Scenario: Tela Sobre
- **WHEN** o usuário escolhe "Sobre o Lume"
- **THEN** é exibida uma tela com a versão do build (a mesma do canto da tela "Configurações"), a licença (GPL-3.0-or-later), o endereço do código-fonte `https://github.com/douglaspands/lume-tv-launcher` e as licenças de terceiros (fonte, ícones, papéis de parede e bibliotecas)
