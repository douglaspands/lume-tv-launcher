## MODIFIED Requirements

### Requirement: Permissões mínimas
O lume SHALL NOT solicitar nenhuma permissão perigosa (runtime), permissão de localização nem `QUERY_ALL_PACKAGES`. As permissões declaradas SHALL ser apenas permissões normais, concedidas na instalação. A primeira é `REQUEST_DELETE_PACKAGES`, usada só para abrir o desinstalador do sistema a pedido do usuário. A segunda é `INTERNET`, exigida pelo Android para abrir o socket que lista as interfaces de rede (a conexão em Informações da TV) e para escutar na rede local enquanto as telas Receber imagem e Confirmar estão abertas e por até 10 s depois de Aplicar, só para avisar a página (capacidade `phone-companion`); ela SHALL NOT ser usada para abrir conexões de saída nem para escutar fora disso. A visibilidade dos apps SHALL ser declarada por consultas de intent (LEANBACK_LAUNCHER e LAUNCHER).

#### Scenario: Inspeção do manifesto
- **WHEN** o manifesto final do APK é inspecionado (`aapt dump permissions`)
- **THEN** aparecem só `android.permission.REQUEST_DELETE_PACKAGES` e `android.permission.INTERNET`, sem `QUERY_ALL_PACKAGES` nem permissão de localização

#### Scenario: Nenhum tráfego de rede
- **WHEN** o lume é usado em todas as telas, inclusive as Ferramentas, exceto a tela Receber imagem
- **THEN** o processo não abre nenhuma conexão de rede nem escuta em nenhuma porta

#### Scenario: Só escuta, nunca conecta
- **WHEN** a tela Receber imagem está aberta
- **THEN** o processo escuta numa única porta TCP e não abre nenhuma conexão de saída

#### Scenario: Desinstalar no Android 9 e 11
- **WHEN** o usuário escolhe "Desinstalar" no menu de contexto no Android TV 9 ou 11
- **THEN** o diálogo de desinstalação do sistema aparece, sem nenhum pedido de permissão

### Requirement: Versões do Android suportadas
O Lume SHALL instalar e funcionar em Android TV 9 (API 28) a Android TV 11 (API 30), e SHALL ser compilado contra um nível de API alvo aceito pela Play Store no momento do release. O botão de entradas e a engrenagem da barra superior SHALL funcionar nas duas pontas dessa faixa. As telas Receber imagem e Confirmar e o servidor do celular (capacidade `phone-companion`) SHALL funcionar nas duas pontas também, usando só o que o NDK oferece desde a API 28 (sockets, `SIOCGIFCONF`, `AAssetManager`, `/dev/urandom`), sem Java novo.

#### Scenario: Android TV 9
- **WHEN** o APK é instalado num emulador Android TV 9 (API 28)
- **THEN** a instalação é bem-sucedida, todas as telas funcionam e a engrenagem abre as configurações do Android

#### Scenario: Android TV 11
- **WHEN** o APK é instalado na TCL 55C715 ou na AIWA SMART TV (Android TV 11), ou em emulador Android TV 11
- **THEN** a instalação é bem-sucedida, o catálogo lista os apps instalados e o botão de entradas abre o menu de entradas da TV

#### Scenario: Visibilidade da ação de entradas no Android 11
- **WHEN** o Lume consulta o sistema se existe quem atenda a `com.android.tv.action.VIEW_INPUTS` no Android 11
- **THEN** a consulta enxerga a activity de entradas do fabricante, porque o manifesto declara a ação em `<queries>`

#### Scenario: Versão anterior
- **WHEN** alguém tenta instalar o APK em Android 8.1 ou anterior
- **THEN** o sistema recusa a instalação

#### Scenario: Imagem do celular no Android TV 9
- **WHEN** a tela Receber imagem é aberta no emulador Android TV 9 (API 28) e o navegador do computador abre a página pela porta encaminhada (`adb forward`)
- **THEN** a tela mostra o QR com o IPv4 da interface do emulador, a página abre com o código, o envio chega à tela Confirmar e Aplicar grava "Minha imagem", como no Android 11
