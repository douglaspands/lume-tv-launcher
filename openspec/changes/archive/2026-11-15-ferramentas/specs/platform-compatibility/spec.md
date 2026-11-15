## MODIFIED Requirements

### Requirement: Permissões mínimas
O lume SHALL NOT solicitar nenhuma permissão perigosa (runtime), permissão de localização nem `QUERY_ALL_PACKAGES`. As permissões declaradas SHALL ser apenas permissões normais, concedidas na instalação. A primeira é `REQUEST_DELETE_PACKAGES`, usada só para abrir o desinstalador do sistema a pedido do usuário. A segunda é `INTERNET`, exigida pelo Android para abrir o socket que lista as interfaces de rede (a conexão em Informações da TV); ela SHALL NOT ser usada para abrir conexões. A visibilidade dos apps SHALL ser declarada por consultas de intent (LEANBACK_LAUNCHER e LAUNCHER).

#### Scenario: Inspeção do manifesto
- **WHEN** o manifesto final do APK é inspecionado (`aapt dump permissions`)
- **THEN** aparecem só `android.permission.REQUEST_DELETE_PACKAGES` e `android.permission.INTERNET`, sem `QUERY_ALL_PACKAGES` nem permissão de localização

#### Scenario: Nenhum tráfego de rede
- **WHEN** o lume é usado em todas as telas, inclusive as Ferramentas
- **THEN** o processo não abre nenhuma conexão de rede

#### Scenario: Desinstalar no Android 9 e 11
- **WHEN** o usuário escolhe "Desinstalar" no menu de contexto no Android TV 9 ou 11
- **THEN** o diálogo de desinstalação do sistema aparece, sem nenhum pedido de permissão
