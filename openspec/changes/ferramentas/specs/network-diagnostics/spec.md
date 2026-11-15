## Purpose

Define a ferramenta Rede: como a TV está conectada (tipo, endereço, gateway), e a qualidade do sinal do Wi-Fi, sem abrir nenhuma conexão.

## ADDED Requirements

### Requirement: Dados da conexão
A ferramenta Rede SHALL mostrar o tipo de conexão (Wi-Fi, cabo Ethernet, outro ou sem conexão), o endereço IPv4, preferindo o cabo ao Wi-Fi como em `device-stats`, e o gateway. Um dado que o sistema não expõe SHALL aparecer como "—". Os dados SHALL ser lidos ao abrir a ferramenta e a cada 2 segundos enquanto ela estiver visível, e só com a interface parada.

#### Scenario: Conectado por Wi-Fi
- **WHEN** a TV está conectada só por Wi-Fi e a ferramenta Rede é aberta
- **THEN** a tela mostra "Wi-Fi", o endereço IPv4 e o gateway, ou "—" no gateway se o sistema não o expuser

#### Scenario: Sem conexão
- **WHEN** a TV não tem interface com endereço IPv4
- **THEN** a tela mostra "Sem conexão", e o endereço e o gateway aparecem como "—"

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

### Requirement: Sem tráfego de rede
A ferramenta Rede SHALL ler só dados locais do sistema (interfaces, rotas e o sinal do Wi-Fi) e SHALL NOT abrir nenhuma conexão de rede nem medir a velocidade da internet.

#### Scenario: Ferramenta aberta
- **WHEN** a ferramenta Rede fica aberta por um minuto
- **THEN** o Lume não abre nenhuma conexão de rede
