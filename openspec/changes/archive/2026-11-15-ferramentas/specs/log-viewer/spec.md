## Purpose

Define a ferramenta Log: as mensagens recentes do próprio Lume numa tela só delas, da mais nova para a mais antiga, para achar falhas pela TV, sem computador nem `adb`.

## ADDED Requirements

### Requirement: Arquivo de log do Lume
Cada linha que o Lume escreve no log do sistema SHALL ir também para o arquivo `lume.log` na pasta privada do Lume, no formato `threadtime` do `logcat`, numa linha só e com no máximo 1 KB (o excesso é cortado com "…"). Quando o arquivo passar de 256 KB, ele SHALL virar `lume.log.1`, substituindo o anterior, e um arquivo novo SHALL começar, de modo que o log nunca ocupe mais de 512 KB. As linhas da abertura SHALL esperar na memória (até 64) até a pasta ser conhecida.

#### Scenario: Rotação
- **WHEN** o arquivo de log chega a 256 KB
- **THEN** ele passa a se chamar `lume.log.1`, o `lume.log.1` anterior é apagado e as próximas linhas vão para um `lume.log` novo

#### Scenario: Log do sistema curto demais
- **WHEN** o buffer do `logcat` da TV guarda só alguns segundos (na AIWA, 64 KB)
- **THEN** as mensagens do Lume continuam no arquivo e aparecem no Log

#### Scenario: Falha antes de fechar
- **WHEN** o Lume entra em pânico e fecha
- **THEN** a mensagem do pânico está no arquivo de log e aparece no Log na próxima abertura

### Requirement: Leitura do log do Lume
A ferramenta Log SHALL ler, ao abrir, as últimas 500 linhas dos dois arquivos de log do Lume, incluindo as de execuções anteriores. A leitura SHALL rodar fora da thread de desenho e SHALL NOT pedir permissão nova nem usar Java. As linhas SHALL aparecer como o Lume as escreveu, em inglês, sem tradução. As linhas e o estado da ferramenta SHALL ser descartados ao sair.

#### Scenario: Abrir o log
- **WHEN** o usuário abre "Log" no menu Ferramentas
- **THEN** a tela mostra "Lendo o log…" e, em seguida, as mensagens do Lume

#### Scenario: Log ilegível
- **WHEN** o sistema não deixa ler o log
- **THEN** a tela mostra "Não foi possível ler o log", e Voltar continua saindo

#### Scenario: Leitura nova a cada abertura
- **WHEN** o usuário sai do Log e o abre de novo
- **THEN** o log é lido outra vez, com as mensagens escritas nesse meio-tempo

### Requirement: Tela do log
O log SHALL ocupar uma tela só dele: o caminho "Configurações › Ferramentas › Log" no topo e um `.panel` com toda a largura do conteúdo, sem itens de ação. Cada mensagem SHALL mostrar a data, a hora e o nível à esquerda, em `--text-muted`, e a origem e o texto à direita, em `--text`, quebrados em até 6 linhas. Erros (`E`) e falhas fatais (`F`) SHALL usar `--danger-fg`. A mensagem mais nova SHALL ficar no topo.

#### Scenario: Mais nova primeiro
- **WHEN** o log é exibido
- **THEN** a primeira mensagem do painel é a mais recente

#### Scenario: Erro destacado
- **WHEN** uma mensagem de nível `E` está na tela
- **THEN** o texto dela aparece na cor `--danger-fg`

### Requirement: Navegação no log
Na tela do log, Cima e Baixo SHALL rolar uma mensagem por vez, e Baixo SHALL parar quando a mensagem mais antiga estiver inteira na tela. OK SHALL alternar entre "Tudo" e "Só falhas" (avisos `W`, erros `E` e falhas fatais `F`), voltando ao topo, e o título do painel SHALL dizer qual dos dois está ativo, com a dica do OK à direita. Voltar SHALL voltar ao menu Ferramentas com o foco em "Log".

#### Scenario: Só falhas
- **WHEN** o log está em "Tudo" e o usuário pressiona OK
- **THEN** só ficam os avisos e erros, da mais nova para a mais antiga, e o título diz "Só falhas"

#### Scenario: Fim da rolagem
- **WHEN** o usuário segura Baixo até o fim
- **THEN** a rolagem para com a mensagem mais antiga inteira na tela

#### Scenario: Sair
- **WHEN** o usuário pressiona Voltar no log
- **THEN** o menu Ferramentas aparece com o foco em "Log"
