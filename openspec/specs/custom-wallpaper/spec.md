# custom-wallpaper Specification

## Purpose
Define o papel de parede próprio do usuário ("Minha imagem"): o slot único, a edição feita no celular (as regras de
tolerância de tamanho e proporção, os modos de enquadramento, o zoom, a posição e a repetição), a confirmação na TV e a
persistência do resultado.

## Requirements

### Requirement: Slot único "Minha imagem"
O Lume SHALL ter um único papel de parede próprio. Uma imagem nova recebida SHALL substituir a anterior só quando o usuário pressionar Aplicar na tela Confirmar da TV. Se ele cancelar, a imagem anterior continua valendo. Remover a imagem SHALL apagar o arquivo e o cache dela, e, se ela era a ativa, o papel de parede SHALL voltar à Aurora.

#### Scenario: Primeiro envio aplicado
- **WHEN** o usuário ajusta a foto no celular, envia e pressiona Aplicar na tela Confirmar da TV
- **THEN** a imagem passa a ser o papel de parede ativo, e o card do editor passa a se chamar "Minha imagem" com a miniatura dela

#### Scenario: Troca cancelada
- **WHEN** já existe "Minha imagem" e o usuário envia outra imagem mas pressiona Cancelar na tela Confirmar
- **THEN** a imagem anterior continua sendo "Minha imagem"

#### Scenario: Remover a imagem ativa
- **WHEN** "Minha imagem" é o papel de parede ativo e o usuário escolhe Remover e confirma
- **THEN** o arquivo e o cache são apagados, o fundo volta à Aurora e o card volta a "+ Do celular"

### Requirement: Edição no celular
A edição SHALL acontecer na página do Lume no celular, no tablet ou no computador, sobre uma prévia 16:9 da tela da TV que muda na hora a cada ajuste. A página SHALL compor o resultado final em 1920x1080 e enviá-lo à TV. A TV SHALL NOT receber a foto original nem compor o enquadramento.

#### Scenario: Ajuste com o dedo
- **WHEN** o usuário troca de modo ou mexe no zoom na página
- **THEN** a prévia da página mostra o novo enquadramento sem esperar a TV

#### Scenario: Envio do resultado
- **WHEN** o usuário toca em "Enviar para a TV"
- **THEN** a página envia uma imagem de 1920x1080 já enquadrada, e a TV a mostra na tela Confirmar

### Requirement: Tolerância de proporção
O modo inicial na página SHALL vir da proporção L/A da foto. De 1,70 a 1,86 (16:9 com ±5%), começa em Preencher, sem faixas. Acima de 1,86 ou abaixo de 1,70, começa em Ajustar, com faixas pretas em cima e embaixo (panorâmica) ou nas laterais (vertical, 4:3). O usuário SHALL poder trocar de modo depois.

#### Scenario: Quase 16:9
- **WHEN** o usuário escolhe uma foto de 2000x1080 (1,85)
- **THEN** a edição começa em Preencher, sem faixas

#### Scenario: Vertical
- **WHEN** o usuário escolhe uma foto de 1080x1920
- **THEN** a edição começa em Ajustar, com faixas pretas nas laterais

#### Scenario: Panorâmica
- **WHEN** o usuário escolhe uma foto de 3000x1000
- **THEN** a edição começa em Ajustar, com faixas pretas em cima e embaixo

### Requirement: Tolerância de resolução
Com o lado menor de 1080 px ou mais, todos os modos SHALL ficar disponíveis sem aviso. De 720 a 1079 px, todos ficam disponíveis com o aviso "Pode ficar menos nítida". De 64 a 719 px, só Repetir fica disponível, e a edição começa nele. Abaixo de 64 px, a página recusa a foto.

#### Scenario: Imagem 1280x720
- **WHEN** o usuário escolhe uma imagem de 1280x720
- **THEN** a edição começa em Preencher, com o aviso "Pode ficar menos nítida"

#### Scenario: Textura pequena
- **WHEN** o usuário escolhe uma imagem de 256x256
- **THEN** a edição começa em Repetir, e os outros modos aparecem desabilitados com o motivo

### Requirement: Modos de enquadramento
A página SHALL oferecer cinco modos, todos resultando em 1920x1080. **Ajustar**: a imagem inteira, centrada, com faixas pretas. **Preencher**: cobre a tela e corta o excedente. **Esticar**: deforma até 16:9. **Fundo desfocado**: como Ajustar, com as faixas preenchidas pela própria imagem ampliada e desfocada. **Repetir**: a imagem inteira no centro, como em Ajustar, e cópias dela em volta até cobrir a tela.

#### Scenario: Ajustar com foto vertical
- **WHEN** uma foto de 1080x1920 está em Ajustar
- **THEN** o resultado tem a foto com a altura toda (1080 px), centrada, com 608 px de largura e faixas pretas de 656 px de cada lado

#### Scenario: Esticar
- **WHEN** uma foto de 1600x1200 está em Esticar
- **THEN** o resultado ocupa a tela inteira sem faixas nem corte, deformado na horizontal

#### Scenario: Fundo desfocado
- **WHEN** uma foto vertical está em Fundo desfocado
- **THEN** a imagem inteira fica nítida no centro, e as laterais mostram a mesma imagem ampliada até cobrir a tela e desfocada, sem faixas pretas

### Requirement: Zoom e posição em Preencher
Em Preencher, o usuário SHALL poder ampliar de 100% a 200% com o gesto de pinça, com o slider de zoom (de 1 em 1%) ou com a roda do mouse, e mover o enquadramento arrastando, sem nunca deixar área vazia. Em 100%, a imagem cobre a tela pelo lado menor. A posição inicial SHALL ser o centro.

#### Scenario: Ampliar e mover
- **WHEN** o usuário amplia para 150% e arrasta a imagem para a direita até o fim
- **THEN** o enquadramento para quando a borda esquerda da foto encosta na borda da tela

#### Scenario: Limite do zoom
- **WHEN** o zoom está em 200% e o usuário tenta ampliar
- **THEN** o zoom continua em 200%

### Requirement: Modo Repetir
Em Repetir, a cópia central SHALL ficar onde Ajustar põe a imagem, e as cópias SHALL se repetir em volta dela até cobrir a tela, sem faixa preta. O usuário SHALL escolher a variante **Lado a lado** (cópias iguais) ou **Espelhado** (vizinhas espelhadas, sem costura), a escala de 25% a 100% em passos de 25%, e deslocar o padrão arrastando. O padrão inicial SHALL ser Lado a lado a 100%.

#### Scenario: Foto vertical em Repetir
- **WHEN** uma foto de 1080x1920 está em Repetir, Lado a lado, 100%
- **THEN** a imagem inteira fica no centro com 608 px de largura, como em Ajustar, e as laterais mostram cópias iguais dela, encostadas, sem faixa preta

#### Scenario: Espelhado sem costura
- **WHEN** uma foto de 640x480 está em Repetir, Espelhado, 100%
- **THEN** cada cópia é o espelho da vizinha, de modo que as bordas se encontram sem salto de cor

#### Scenario: Deslocar o padrão
- **WHEN** o usuário arrasta a prévia para a direita em Repetir
- **THEN** o mosaico inteiro anda para a direita, e o padrão continua cobrindo toda a tela

### Requirement: Tamanho das cópias em Repetir
Em 100%, cada cópia de Repetir SHALL ter o tamanho da imagem em Ajustar ou, se for menor, o tamanho real dela numa tela de 1080 linhas. A escala SHALL NOT ampliar a imagem além disso.

#### Scenario: Textura pequena em Repetir
- **WHEN** uma imagem de 256x256 está em Repetir a 100%
- **THEN** cada cópia tem 256x256 no resultado de 1920x1080, e o mosaico cobre a tela

#### Scenario: Foto grande em Repetir
- **WHEN** uma foto de 2730x1536 está em Repetir a 50%
- **THEN** cada cópia tem 960x540, a metade da imagem em Ajustar

### Requirement: Confirmação na TV
Ao receber uma imagem, a TV SHALL mostrá-la em tela cheia, sem escurecimento, como seria o papel de parede, com um cartão pequeno na base: a pergunta "Usar como papel de parede?" e os botões Cancelar e Aplicar. Uma imagem nova recebida enquanto a tela está aberta SHALL substituir a mostrada. Voltar SHALL equivaler a Cancelar, que volta à tela Receber imagem com o mesmo código, para o usuário ajustar e enviar de novo.

#### Scenario: Conferir na TV
- **WHEN** o celular envia uma imagem
- **THEN** a TV mostra o resultado em tela cheia com Cancelar e Aplicar, com o foco em Aplicar

#### Scenario: Reenviar com outro ajuste
- **WHEN** a tela Confirmar está aberta e o usuário muda o ajuste no celular e envia de novo
- **THEN** a TV passa a mostrar a nova imagem, sem o usuário apertar nada no controle

#### Scenario: Cancelar e continuar ajustando
- **WHEN** o usuário pressiona Cancelar ou Voltar na tela Confirmar
- **THEN** nada é aplicado e a TV volta à tela Receber imagem com o mesmo código, e o celular pode enviar de novo

### Requirement: Aplicar e persistir
Aplicar SHALL gravar a imagem recebida (1920x1080) no armazenamento interno do app, fora do cache, e então ativar "Minha imagem". A TV SHALL NOT guardar a foto original nem os parâmetros do enquadramento. Para mudar o ajuste depois, o usuário envia uma nova imagem ("Trocar imagem").

#### Scenario: Reinício da TV
- **WHEN** o usuário aplica "Minha imagem" e a TV é desligada da tomada e religada
- **THEN** o Lume abre com a mesma imagem

#### Scenario: Imagem ausente
- **WHEN** a configuração aponta "Minha imagem" mas o arquivo não existe ou não decodifica
- **THEN** o Lume usa a Aurora, registra o problema no log e o card volta a "+ Do celular"

### Requirement: Memória da imagem própria
Na TV, a imagem recebida (1920x1080) SHALL ficar em memória só enquanto a tela Confirmar está aberta ou o cache está sendo refeito, e SHALL ser liberada depois. Ela SHALL ter memória própria, devolvida ao sistema ao sair (ver `resource-efficiency`): durante a confirmação, a memória privada sem a GPU SHALL caber no orçamento de primeiro plano (32 MB), e ao sair SHALL voltar a até 2 MB do valor anterior. O arquivo gravado SHALL ocupar no máximo 9 MB em disco.

#### Scenario: Confirmação na AIWA
- **WHEN** a tela Confirmar está aberta na AIWA
- **THEN** o Lume continua respondendo, e a memória privada sem a GPU fica abaixo de 32 MB

#### Scenario: Memória devolvida
- **WHEN** o usuário aplica e volta ao editor
- **THEN** a memória privada suja fica a até 2 MB do valor medido antes de abrir a tela Receber imagem
