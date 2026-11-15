## MODIFIED Requirements

### Requirement: Renderização sob demanda
O lume SHALL desenhar quadros apenas quando algo visível muda (entrada do usuário, animação em curso, virada de minuto do relógio, mudança no catálogo, progresso de um teste das Ferramentas). Ocioso, SHALL NOT desenhar quadros nem manter threads acordadas em laço. O papel de parede e o dock SHALL NOT manter animação contínua. A única exceção são os modos das Ferramentas que animam por natureza (ciclo de cores da limpeza de retenção, destravamento de pixel, teste de movimento, quadrado do teste de sincronia), que SHALL pedir quadros contínuos só enquanto estão em andamento. O progresso de um teste em segundo plano (velocidade da internet, armazenamento) SHALL pedir no máximo 4 quadros por segundo.

#### Scenario: Tela ociosa
- **WHEN** a tela Início está visível, sem interação
- **THEN** o lume desenha no máximo 1 quadro por minuto (relógio), e o uso de CPU do processo fica abaixo de 1% em média

#### Scenario: Outras telas ociosas
- **WHEN** a aba Apps, as Configurações, a tela Sobre, o editor, o menu Ferramentas ou uma ferramenta sem modo em andamento estão visíveis, sem interação
- **THEN** o lume desenha no máximo 1 quadro por minuto (relógio), além das leituras periódicas que a própria tela exibe

#### Scenario: Magnification terminada
- **WHEN** a animação de Magnification do dock termina
- **THEN** o lume deixa de desenhar quadros até a próxima mudança visível

#### Scenario: Padrão estático das Ferramentas
- **WHEN** um padrão de diagnóstico sólido ou a limpeza "Branco" está na tela e o texto de dica já sumiu
- **THEN** o lume não desenha quadros até a próxima tecla ou o fim do tempo

#### Scenario: Modo animado das Ferramentas termina
- **WHEN** o usuário sai do teste de movimento ou o destravamento de pixel termina
- **THEN** o lume volta a desenhar só sob demanda

## ADDED Requirements

### Requirement: Memória das Ferramentas
As Ferramentas SHALL caber no orçamento de memória em primeiro plano (PSS até 64 MB) com qualquer teste em andamento, e SHALL devolver a memória ao sair: ao voltar do menu Ferramentas às Configurações, o PSS SHALL ficar a até 2 MB do valor medido antes de entrar. Nenhum teste SHALL manter mais que 1 MB de dados de teste em memória.

#### Scenario: Teste de velocidade na TCL
- **WHEN** o teste de velocidade da internet está em andamento na TCL
- **THEN** o `dumpsys meminfo app.lume.tv` mostra PSS abaixo de 64 MB

#### Scenario: Memória devolvida
- **WHEN** o usuário roda todos os testes e volta às Configurações
- **THEN** o PSS fica a até 2 MB do valor medido antes de abrir as Ferramentas
