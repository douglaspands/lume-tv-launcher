# audio-diagnostics Specification

## Purpose

Define a ferramenta Áudio: tons para conferir os canais esquerdo e direito, varredura de frequência para achar ruídos dos alto-falantes e teste de sincronia entre o som e a imagem para medir o atraso da saída de áudio.

## Requirements

### Requirement: Tons por canal
A ferramenta Áudio SHALL oferecer os itens "Esquerdo", "Direito" e "Os dois", que tocam um tom senoidal de 440 Hz no canal correspondente, enquanto o item estiver ativo, com no máximo -12 dBFS. O tom SHALL começar e terminar com uma rampa de volume de pelo menos 20 ms, para não estalar. OK num item SHALL ligar ou desligar o tom dele, e ligar um tom SHALL desligar qualquer outro.

#### Scenario: Canal esquerdo
- **WHEN** o usuário pressiona OK em "Esquerdo"
- **THEN** o tom toca só no canal esquerdo, e o item mostra que está tocando

#### Scenario: Troca de canal
- **WHEN** o tom "Esquerdo" toca e o usuário pressiona OK em "Direito"
- **THEN** o tom esquerdo para, e o tom toca só no canal direito

#### Scenario: Sem estalo
- **WHEN** um tom começa ou termina
- **THEN** o volume sobe ou desce em pelo menos 20 ms

### Requirement: Varredura de frequência
O item "Varredura" SHALL tocar, nos dois canais e com no máximo -12 dBFS, um tom que sobe de 20 Hz a 20 kHz em escala logarítmica, em 30 segundos. A tela SHALL mostrar a frequência atual. Ao fim, a varredura SHALL parar sozinha. Assim o usuário encontra a frequência em que o alto-falante chia, vibra ou some.

#### Scenario: Frequência visível
- **WHEN** a varredura está em andamento
- **THEN** a tela mostra a frequência que está tocando, em Hz

#### Scenario: Fim da varredura
- **WHEN** a varredura chega a 20 kHz
- **THEN** o som para, e o item volta ao estado parado

### Requirement: Sincronia entre áudio e imagem
O item "Sincronia" SHALL repetir, uma vez por segundo, um bipe curto (1 kHz, 50 ms) junto com um quadrado branco de 50 ms no centro da tela. O quadrado SHALL ocupar menos de 5% da área e piscar só 1 vez por segundo. O bipe SHALL ter rampas curtas (de 1 a 5 ms), exceção à rampa de 20 ms dos tons: um início nítido é o que o usuário casa com o quadrado, e numa rampa longa o ponto exato do som fica vago.

#### Scenario: Bipe e quadrado
- **WHEN** o teste de sincronia começa com ajuste de 0 ms
- **THEN** a cada segundo toca um bipe de 50 ms e aparece o quadrado branco de 50 ms no centro, ao mesmo tempo

### Requirement: Ajuste do atraso da sincronia
No teste de sincronia, as setas Esquerda e Direita SHALL ajustar, em passos de 10 ms entre -300 e +300 ms, o atraso do quadrado em relação ao bipe. A tela SHALL mostrar o ajuste e a interpretação ("o som está X ms atrasado" ou "adiantado"), para o usuário corrigir no menu de som da TV ou da soundbar. O ajuste SHALL valer só enquanto o teste estiver aberto e SHALL NOT mudar o áudio de outros apps.

#### Scenario: Ajuste do atraso
- **WHEN** o teste de sincronia está em 0 ms e o usuário pressiona Direita 8 vezes
- **THEN** o quadrado passa a aparecer 80 ms depois do bipe, e a tela diz que o som está 80 ms atrasado

#### Scenario: Sem efeito fora do teste
- **WHEN** o usuário sai do teste de sincronia
- **THEN** o ajuste é descartado, e nenhuma configuração de áudio da TV muda

### Requirement: Saída de áudio sob demanda
A saída de áudio SHALL ser aberta só quando um tom, a varredura ou a sincronia começa, e SHALL ser fechada quando o som para, a ferramenta fecha ou o Lume vai para segundo plano. Se o sistema não abrir a saída de áudio, a ferramenta SHALL mostrar "Áudio indisponível" no item, sem travar a interface.

#### Scenario: Segundo plano
- **WHEN** um tom toca e o usuário pressiona Home com o Lume já visível, ou outro app abre
- **THEN** o som para, e a saída de áudio é fechada

#### Scenario: Falha ao abrir
- **WHEN** a saída de áudio não pode ser aberta
- **THEN** o item mostra "Áudio indisponível", e a interface continua respondendo
