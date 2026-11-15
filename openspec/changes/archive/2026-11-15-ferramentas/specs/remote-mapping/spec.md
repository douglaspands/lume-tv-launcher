## ADDED Requirements

### Requirement: Teste do controle remoto
A ferramenta "Controle remoto" do menu Ferramentas (`diagnostic-tools`) SHALL mostrar, a cada botão pressionado, o nome da tecla, o código Android (`KEYCODE_*` e número) e o que o Lume faz com ela pelo mapeamento atual (comando, atalho, aplicativo, tecla reservada ou "não usada"). Com o botão mantido pressionado, ela SHALL mostrar o número de repetições e o intervalo médio entre elas em ms. Um histórico das últimas 8 teclas SHALL ficar visível.

#### Scenario: Tecla desconhecida
- **WHEN** o usuário pressiona um botão cujo código não tem nome conhecido
- **THEN** a tela mostra o código numérico e "Não usada"

#### Scenario: Repetição
- **WHEN** o usuário mantém a seta Baixo pressionada por 2 segundos
- **THEN** a tela mostra o número de repetições e o intervalo médio entre elas

### Requirement: Teclas sem ação durante o teste do controle
Enquanto o teste do controle estiver aberto, nenhuma tecla SHALL executar a sua ação no Lume: setas, OK e teclas mapeadas só são registradas. As exceções são Home, que segue o sistema, e Voltar, que é registrado na primeira vez e fecha o teste se for pressionado de novo em até 2 segundos. A tela SHALL mostrar a dica "Pressione Voltar duas vezes para sair".

#### Scenario: Tecla mapeada
- **WHEN** o teste está aberto e o usuário pressiona um botão mapeado para abrir um aplicativo
- **THEN** a tela mostra o nome da tecla, o código e "Abre <aplicativo>", e o aplicativo não abre

#### Scenario: Sair com Voltar duas vezes
- **WHEN** o usuário pressiona Voltar uma vez e outra vez 1 segundo depois
- **THEN** o primeiro Voltar aparece no histórico, e o segundo fecha o teste e volta ao menu Ferramentas com o foco em "Controle remoto"

#### Scenario: Voltar isolado
- **WHEN** o usuário pressiona Voltar uma vez e espera mais de 2 segundos
- **THEN** o teste continua aberto, com Voltar registrado no histórico
