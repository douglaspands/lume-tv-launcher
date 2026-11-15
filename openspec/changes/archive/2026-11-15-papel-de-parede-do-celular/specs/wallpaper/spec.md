## MODIFIED Requirements

### Requirement: Papel de parede de aurora boreal
O lume SHALL desenhar, atrás do conteúdo de todas as telas (Início, Apps, Configurações, Sobre e edição), o papel de parede selecionado pelo usuário a partir de um catálogo integrado de 6 opções, ou a imagem própria "Minha imagem" (capacidade `custom-wallpaper`), ou a Aurora boreal por padrão, cobrindo toda a área visível. As imagens do catálogo SHALL respeitar a paleta do tema Lume (tons escuros de azul-petróleo, ardósia, índigo e ciano), com topo escuro sob a barra superior e metade inferior escura para acomodar o dock; a imagem própria é do usuário e não segue essa regra.

#### Scenario: Papel de parede na tela Início
- **WHEN** a tela Início é exibida
- **THEN** o papel de parede ativo aparece atrás da barra superior e do dock, ocupando a tela inteira, com a parte de baixo escura

#### Scenario: Papel de parede nas outras telas
- **WHEN** o usuário abre a aba Apps, as Configurações, a tela Sobre ou o modo edição
- **THEN** o mesmo papel de parede ativo aparece atrás do conteúdo

#### Scenario: Proporção diferente de 16:9
- **WHEN** a tela tem uma proporção diferente de 16:9
- **THEN** o papel de parede cobre toda a superfície visível, inclusive as sobras laterais ou verticais fora do canvas lógico, sem faixas vazias

#### Scenario: Minha imagem ativa
- **WHEN** "Minha imagem" é o papel de parede ativo
- **THEN** ela aparece atrás de todas as telas, com o enquadramento aplicado, e é escurecida nas telas de texto como os papéis do catálogo

### Requirement: Papel de parede pronto ao acordar
O lume SHALL guardar em cache, no armazenamento do app, a imagem já decodificada do papel de parede ativo (RGB cru em 1920x1080), de modo que, ao acordar, só leia o cache e a envie à GPU, sem decodificar de novo; a redução para um buffer menor, que custa poucos milissegundos de GPU, continua a ser feita a cada carga. O cache SHALL ser refeito quando o papel de parede, a versão da imagem, a revisão de "Minha imagem" ou o tamanho do buffer mudam, e a falta dele SHALL NOT impedir o papel de parede de aparecer.

#### Scenario: Retorno com cache
- **WHEN** o lume acorda com o mesmo papel de parede e a mesma versão da última vez
- **THEN** a imagem vem do cache, sem decodificação

#### Scenario: Troca invalida o cache
- **WHEN** o usuário escolhe outro papel de parede, ou o app é atualizado para outra versão
- **THEN** a imagem é decodificada de novo e o cache passa a guardar só a nova versão

#### Scenario: Cache apagado pelo sistema
- **WHEN** o Android apagou o cache do app e o lume acorda
- **THEN** o papel de parede é preparado a partir do APK e aparece no primeiro quadro completo, como sem cache

#### Scenario: Cache de "Minha imagem" apagado pelo sistema
- **WHEN** "Minha imagem" é o papel de parede ativo e o Android apagou o cache do app
- **THEN** o lume decodifica de novo a imagem guardada fora do cache; enquanto isso, o fundo mostra a cor de fundo do tema, sem travar a navegação
