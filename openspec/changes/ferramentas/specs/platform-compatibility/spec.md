## MODIFIED Requirements

### Requirement: Permissões mínimas
O lume SHALL NOT solicitar nenhuma permissão perigosa (runtime), permissão de localização nem `QUERY_ALL_PACKAGES`. As permissões declaradas SHALL ser apenas permissões normais, concedidas na instalação. A primeira é `REQUEST_DELETE_PACKAGES`, usada só para abrir o desinstalador do sistema a pedido do usuário. A segunda é `INTERNET`, usada para listar as interfaces de rede (a conexão em Informações da TV e na ferramenta Rede) e para o teste de velocidade da internet. A terceira é `ACCESS_WIFI_STATE`, usada só para ler o sinal do Wi-Fi na ferramenta Rede. O lume SHALL abrir conexões de rede apenas durante o teste de velocidade iniciado pelo usuário (`network-diagnostics`). A visibilidade dos apps SHALL ser declarada por consultas de intent (LEANBACK_LAUNCHER e LAUNCHER).

#### Scenario: Inspeção do manifesto
- **WHEN** o manifesto final do APK é inspecionado (`aapt dump permissions`)
- **THEN** aparecem só `android.permission.REQUEST_DELETE_PACKAGES`, `android.permission.INTERNET` e `android.permission.ACCESS_WIFI_STATE`, sem `QUERY_ALL_PACKAGES` nem permissão de localização

#### Scenario: Nenhum tráfego de rede
- **WHEN** o lume é usado em todas as telas, sem iniciar o teste de velocidade
- **THEN** o processo não abre nenhuma conexão de rede

#### Scenario: Tráfego só no teste
- **WHEN** o usuário inicia o teste de velocidade na ferramenta Rede
- **THEN** o processo abre conexões só com `speed.cloudflare.com`, e todas fecham ao fim ou ao cancelar o teste

#### Scenario: Desinstalar no Android 9 e 11
- **WHEN** o usuário escolhe "Desinstalar" no menu de contexto no Android TV 9 ou 11
- **THEN** o diálogo de desinstalação do sistema aparece, sem nenhum pedido de permissão
