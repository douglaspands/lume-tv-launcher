## Purpose

Define a ferramenta Rede: como a TV está conectada (tipo, endereço, gateway), a qualidade do sinal do Wi-Fi e um teste de velocidade da internet que só roda quando o usuário pede.

## ADDED Requirements

### Requirement: Dados da conexão
A ferramenta Rede SHALL mostrar o tipo de conexão (Wi-Fi, cabo Ethernet, outro ou sem conexão), o endereço IPv4, preferindo o cabo ao Wi-Fi como em `device-stats`, e o gateway. Um dado que o sistema não expõe SHALL aparecer como "—". Os dados SHALL ser lidos ao abrir a ferramenta e a cada 2 segundos enquanto ela estiver visível, e só com a interface parada.

#### Scenario: Conectado por Wi-Fi
- **WHEN** a TV está conectada só por Wi-Fi e a ferramenta Rede é aberta
- **THEN** a tela mostra "Wi-Fi", o endereço IPv4 e o gateway, ou "—" no gateway se o sistema não o expuser

#### Scenario: Sem conexão
- **WHEN** a TV não tem interface com endereço IPv4
- **THEN** a tela mostra "Sem conexão", e o teste de velocidade fica indisponível

### Requirement: Sinal do Wi-Fi
Quando a conexão for Wi-Fi, a ferramenta SHALL mostrar a intensidade do sinal em dBm, com uma barra e uma classificação ("Ótimo" a partir de -55 dBm, "Bom" de -56 a -67, "Regular" de -68 a -75, "Fraco" abaixo de -75), a velocidade do link em Mbps e a faixa (2,4 GHz, 5 GHz ou 6 GHz). Se o sistema não fornecer o sinal, a seção SHALL mostrar "—".

#### Scenario: Sinal bom
- **WHEN** o Wi-Fi informa -60 dBm, 300 Mbps e 5 GHz
- **THEN** a tela mostra "-60 dBm", a classificação "Bom", "300 Mbps" e "5 GHz"

#### Scenario: Conexão por cabo
- **WHEN** a TV está conectada por cabo Ethernet
- **THEN** a seção do sinal do Wi-Fi não aparece

### Requirement: Privacidade da rede
A ferramenta Rede SHALL NOT mostrar nem ler o nome da rede (SSID), e o Lume SHALL NOT pedir permissão de localização.

#### Scenario: Sem nome da rede
- **WHEN** a TV está conectada por Wi-Fi e a ferramenta Rede é aberta
- **THEN** o nome da rede não aparece, e nenhum pedido de permissão é exibido

### Requirement: Teste de velocidade da internet
O teste de velocidade SHALL começar só quando o usuário pressionar OK em "Testar velocidade", e SHALL usar o serviço público `speed.cloudflare.com` por HTTPS. Ele mede, nesta ordem, a latência (mediana de 5 pedidos pequenos), o download (até 25 MB ou 10 segundos, o que vier primeiro) e o upload (até 10 MB ou 10 segundos), e mostra os resultados finais em ms e Mbps.

#### Scenario: Teste completo
- **WHEN** o usuário pressiona OK em "Testar velocidade" com a internet funcionando
- **THEN** a tela mostra latência, download e upload, nesta ordem, e os três valores finais ao terminar

#### Scenario: Nunca automático
- **WHEN** a ferramenta Rede está aberta e o usuário não pressiona "Testar velocidade"
- **THEN** o Lume não abre nenhuma conexão de rede

### Requirement: Progresso, cancelamento e falha do teste de velocidade
Durante o teste, a tela SHALL mostrar a fase atual e o valor parcial, atualizados no máximo 4 vezes por segundo. Voltar SHALL cancelar o teste e fechar as conexões. Uma falha (sem internet, servidor fora, tempo esgotado) SHALL aparecer como mensagem na tela, sem travar a interface, e o teste SHALL poder ser repetido.

#### Scenario: Cancelar
- **WHEN** o download está em andamento e o usuário pressiona Voltar
- **THEN** o teste para, as conexões fecham em até 1 segundo, nenhum resultado parcial fica como final, e a ferramenta continua aberta; um segundo Voltar fecha a ferramenta

#### Scenario: Sem internet
- **WHEN** a TV está na rede local, mas sem acesso à internet, e o usuário inicia o teste
- **THEN** a tela mostra uma mensagem de falha em até 10 segundos, e o teste pode ser repetido

### Requirement: Aviso de serviço externo
A ferramenta Rede SHALL avisar, junto do item "Testar velocidade", que o teste usa um serviço externo (Cloudflare) e consome dados.

#### Scenario: Aviso visível
- **WHEN** a ferramenta Rede é aberta
- **THEN** o aviso sobre o serviço externo e o consumo de dados aparece junto do item "Testar velocidade"
